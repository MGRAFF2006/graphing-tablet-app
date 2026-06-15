use graphpad_core::mapping::{MappedPenEvent, Mapper, PressureCurve};
use graphpad_core::protocol::{Frame, FrameReader};
use std::env;
use std::io;
use std::net::{TcpListener, TcpStream};

const DEFAULT_LISTEN_ADDR: &str = "0.0.0.0:47391";

fn main() -> io::Result<()> {
    let config = Config::from_args(env::args().skip(1))?;
    let mapper = Mapper::new(Default::default(), config.pressure_curve);
    let mut backend = Backend::open(config.backend, mapper.extents())?;

    println!(
        "GraphPad daemon listening on {} with {} backend",
        config.listen_addr,
        backend.name()
    );
    let listener = TcpListener::bind(&config.listen_addr)?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let peer = stream
                    .peer_addr()
                    .map(|addr| addr.to_string())
                    .unwrap_or_else(|_| "unknown peer".to_string());
                println!("client connected: {peer}");
                if let Err(err) = handle_client(stream, mapper, &mut backend) {
                    eprintln!("client disconnected with error: {err}");
                }
                println!("client disconnected: {peer}");
            }
            Err(err) => eprintln!("accept failed: {err}"),
        }
    }

    Ok(())
}

fn handle_client(stream: TcpStream, mapper: Mapper, backend: &mut Backend) -> io::Result<()> {
    stream.set_nodelay(true)?;
    let mut reader = FrameReader::new(stream);

    loop {
        match reader.read_frame() {
            Ok(Frame::Hello(hello)) => {
                println!(
                    "HELLO device=\"{}\" size={}x{} pressure={} protocol={}",
                    hello.device_name,
                    hello.width_px,
                    hello.height_px,
                    hello.max_pressure,
                    hello.protocol_version
                );
            }
            Ok(Frame::Pen(event)) => backend.emit_pen(mapper.map_pen_event(event))?,
            Ok(Frame::Button(event)) => {
                println!(
                    "BUTTON sequence={} buttons=0b{:08b} timestamp_us={}",
                    event.sequence, event.buttons, event.timestamp_us
                );
            }
            Ok(Frame::Heartbeat { timestamp_us }) => {
                println!("HEARTBEAT timestamp_us={timestamp_us}");
            }
            Ok(Frame::Config(config)) => {
                println!(
                    "CONFIG display={}x{} mode={:?} preserve_aspect={}",
                    config.display_width,
                    config.display_height,
                    config.mapping_mode,
                    config.preserve_aspect_ratio
                );
            }
            Err(err) => {
                return Err(io::Error::new(io::ErrorKind::InvalidData, err));
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BackendKind {
    Auto,
    Log,
    Uinput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Config {
    listen_addr: &'static str,
    backend: BackendKind,
    pressure_curve: PressureCurve,
}

impl Config {
    fn from_args(args: impl Iterator<Item = String>) -> io::Result<Self> {
        let mut listen_addr = DEFAULT_LISTEN_ADDR;
        let mut backend = BackendKind::Auto;
        let mut pressure_curve = PressureCurve::Linear;
        let mut pending_key: Option<String> = None;

        for arg in args {
            if let Some(key) = pending_key.take() {
                match key.as_str() {
                    "--listen" => listen_addr = leak_arg(arg),
                    "--backend" => backend = parse_backend(&arg)?,
                    "--pressure-curve" => pressure_curve = parse_pressure_curve(&arg)?,
                    _ => unreachable!(),
                }
                continue;
            }

            if let Some((key, value)) = arg.split_once('=') {
                match key {
                    "--listen" => listen_addr = leak_arg(value.to_string()),
                    "--backend" => backend = parse_backend(value)?,
                    "--pressure-curve" => pressure_curve = parse_pressure_curve(value)?,
                    "--help" | "-h" => {
                        print_help();
                        std::process::exit(0);
                    }
                    _ => return Err(invalid_arg(format!("unknown option: {key}"))),
                }
                continue;
            }

            match arg.as_str() {
                "--listen" | "--backend" | "--pressure-curve" => pending_key = Some(arg),
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => return Err(invalid_arg(format!("unknown option: {arg}"))),
            }
        }

        if let Some(key) = pending_key {
            return Err(invalid_arg(format!("missing value for {key}")));
        }

        Ok(Self {
            listen_addr,
            backend,
            pressure_curve,
        })
    }
}

fn parse_backend(value: &str) -> io::Result<BackendKind> {
    match value {
        "auto" => Ok(BackendKind::Auto),
        "log" => Ok(BackendKind::Log),
        "uinput" => Ok(BackendKind::Uinput),
        _ => Err(invalid_arg(format!(
            "unsupported backend {value:?}; expected auto, log, or uinput"
        ))),
    }
}

fn parse_pressure_curve(value: &str) -> io::Result<PressureCurve> {
    match value {
        "linear" => Ok(PressureCurve::Linear),
        "soft" => Ok(PressureCurve::Soft),
        "firm" => Ok(PressureCurve::Firm),
        _ => Err(invalid_arg(format!(
            "unsupported pressure curve {value:?}; expected linear, soft, or firm"
        ))),
    }
}

fn leak_arg(value: String) -> &'static str {
    Box::leak(value.into_boxed_str())
}

fn invalid_arg(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}

fn print_help() {
    println!(
        "graphpad-daemon\n\n\
         Usage: graphpad-daemon [--listen ADDR:PORT] [--backend auto|log|uinput] \\\n\
         [--pressure-curve linear|soft|firm]\n\n\
         Default listen address: {DEFAULT_LISTEN_ADDR}"
    );
}

enum Backend {
    Log,
    #[cfg(target_os = "linux")]
    Uinput(graphpad_input_linux::LinuxTablet),
}

impl Backend {
    fn open(kind: BackendKind, extents: graphpad_core::mapping::TabletExtents) -> io::Result<Self> {
        match kind {
            BackendKind::Log => Ok(Self::Log),
            BackendKind::Auto => Self::open_auto(extents),
            BackendKind::Uinput => Self::open_uinput(extents),
        }
    }

    fn open_auto(extents: graphpad_core::mapping::TabletExtents) -> io::Result<Self> {
        #[cfg(target_os = "linux")]
        {
            match graphpad_input_linux::LinuxTablet::open(extents) {
                Ok(tablet) => return Ok(Self::Uinput(tablet)),
                Err(err) => {
                    eprintln!("could not open /dev/uinput ({err}); falling back to log backend");
                }
            }
        }

        Ok(Self::Log)
    }

    fn open_uinput(extents: graphpad_core::mapping::TabletExtents) -> io::Result<Self> {
        #[cfg(target_os = "linux")]
        {
            return graphpad_input_linux::LinuxTablet::open(extents).map(Self::Uinput);
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = extents;
            Err(io::Error::new(
                io::ErrorKind::Unsupported,
                "uinput backend is only available on Linux",
            ))
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Log => "log",
            #[cfg(target_os = "linux")]
            Self::Uinput(_) => "uinput",
        }
    }

    fn emit_pen(&mut self, event: MappedPenEvent) -> io::Result<()> {
        match self {
            Self::Log => {
                println!(
                    "PEN seq={} x={} y={} pressure={} tilt=({}, {}) down={} hover={} buttons=0b{:08b}",
                    event.sequence,
                    event.x,
                    event.y,
                    event.pressure,
                    event.tilt_x,
                    event.tilt_y,
                    event.tool_down,
                    event.hover,
                    event.buttons
                );
                Ok(())
            }
            #[cfg(target_os = "linux")]
            Self::Uinput(tablet) => tablet.emit_pen(event),
        }
    }
}
