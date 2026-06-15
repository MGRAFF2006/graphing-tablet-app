//! TCP session handling for GraphPad clients.

use std::sync::Arc;

use bytes::BytesMut;
use log::{debug, info, warn};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;

use crate::mapping::TabletMapper;
use crate::protocol::{
    self, Frame, FrameType, Hello, HostHello, PenEvent, ProtocolError, PROTOCOL_VERSION,
    PRESSURE_MAX, TABLET_AXIS_MAX,
};

/// Backend that applies mapped pen state to the host OS.
pub trait InputBackend: Send {
    fn apply_state(&mut self, state: crate::mapping::MappedPenState) -> anyhow::Result<()>;
    fn reset(&mut self) -> anyhow::Result<()>;
}

pub struct HostConfig {
    pub bind_addr: String,
    pub host_name: String,
}

pub struct SessionStats {
    pub frames_received: u64,
    pub pen_moves: u64,
    pub last_sequence: u32,
    pub dropped_sequences: u64,
}

pub struct PenSession<B: InputBackend> {
    backend: B,
    mapper: TabletMapper,
    in_contact: bool,
    stats: SessionStats,
}

impl<B: InputBackend> PenSession<B> {
    pub fn new(backend: B) -> Self {
        Self {
            backend,
            mapper: TabletMapper::default(),
            in_contact: false,
            stats: SessionStats {
                frames_received: 0,
                pen_moves: 0,
                last_sequence: 0,
                dropped_sequences: 0,
            },
        }
    }

    pub fn stats(&self) -> &SessionStats {
        &self.stats
    }

    pub fn handle_frame(&mut self, frame: Frame) -> anyhow::Result<()> {
        self.stats.frames_received += 1;

        match frame {
            Frame::Hello(hello) => {
                info!(
                    "Client connected: {} ({:.0}x{:.0} mm)",
                    hello.device_name, hello.screen_width_mm, hello.screen_height_mm
                );
            }
            Frame::Config(config) => {
                self.mapper.mapping_mode = config.mapping_mode;
                self.mapper.pressure_curve = config.pressure_curve;
                self.mapper.area_left = config.area_left;
                self.mapper.area_top = config.area_top;
                self.mapper.area_right = config.area_right;
                self.mapper.area_bottom = config.area_bottom;
                debug!("Updated mapping config: {:?}", config);
            }
            Frame::PenDown(event) => {
                self.in_contact = true;
                self.track_sequence(event.sequence);
                self.apply_pen_event(&event, true, false)?;
            }
            Frame::PenMove(event) => {
                self.stats.pen_moves += 1;
                self.track_sequence(event.sequence);
                self.apply_pen_event(&event, self.in_contact, false)?;
            }
            Frame::PenUp(event) => {
                self.in_contact = false;
                self.track_sequence(event.sequence);
                self.apply_pen_event(&event, false, false)?;
            }
            Frame::PenHover(event) => {
                self.track_sequence(event.sequence);
                self.apply_pen_event(&event, false, true)?;
            }
            Frame::Button(_) | Frame::Heartbeat(_) => {}
            Frame::HostHello(_) => {}
        }

        Ok(())
    }

    fn apply_pen_event(
        &mut self,
        event: &PenEvent,
        in_contact: bool,
        hovering: bool,
    ) -> anyhow::Result<()> {
        let state = self.mapper.map_pen_event(event, in_contact, hovering);
        self.backend.apply_state(state)
    }

    fn track_sequence(&mut self, sequence: u32) {
        if self.stats.last_sequence > 0 && sequence > self.stats.last_sequence + 1 {
            self.stats.dropped_sequences += (sequence - self.stats.last_sequence - 1) as u64;
            warn!(
                "Detected dropped pen frames: expected {}, got {}",
                self.stats.last_sequence + 1,
                sequence
            );
        }
        self.stats.last_sequence = sequence;
    }

    pub fn disconnect(&mut self) -> anyhow::Result<()> {
        self.in_contact = false;
        self.backend.reset()
    }
}

pub async fn run_host<B, F>(config: HostConfig, backend_factory: F) -> anyhow::Result<()>
where
    B: InputBackend + 'static,
    F: Fn() -> anyhow::Result<B> + Send + Sync + 'static,
{
    let listener = TcpListener::bind(&config.bind_addr).await?;
    info!("GraphPad host listening on {}", config.bind_addr);

    let factory = Arc::new(backend_factory);

    loop {
        let (stream, peer) = listener.accept().await?;
        info!("Incoming connection from {peer}");
        let host_name = config.host_name.clone();
        let factory = Arc::clone(&factory);
        tokio::spawn(async move {
            if let Err(err) = handle_client(stream, host_name, factory).await {
                warn!("Client session ended with error: {err:#}");
            }
        });
    }
}

async fn handle_client<B, F>(
    mut stream: TcpStream,
    host_name: String,
    backend_factory: Arc<F>,
) -> anyhow::Result<()>
where
    B: InputBackend + 'static,
    F: Fn() -> anyhow::Result<B>,
{
    let backend = backend_factory()?;
    let session = Arc::new(Mutex::new(PenSession::new(backend)));

    // Send host hello
    let host_hello = Frame::HostHello(HostHello {
        protocol_version: PROTOCOL_VERSION as u16,
        host_name,
        tablet_width: TABLET_AXIS_MAX as u32,
        tablet_height: TABLET_AXIS_MAX as u32,
        pressure_max: PRESSURE_MAX as u32,
    });
    let encoded = protocol::encode_frame(&host_hello);
    stream.write_all(&encoded).await?;

    let (reader, mut writer) = stream.into_split();
    let reader = Arc::new(Mutex::new(reader));
    let read_session = Arc::clone(&session);

    let read_task = tokio::spawn(async move {
        let mut buf = BytesMut::with_capacity(4096);
        let mut scratch = [0u8; 4096];
        loop {
            let n = {
                let mut reader = reader.lock().await;
                reader.read(&mut scratch).await?
            };
            if n == 0 {
                break;
            }
            buf.extend_from_slice(&scratch[..n]);

            while buf.len() >= 8 {
                let length = u16::from_be_bytes([buf[6], buf[7]]) as usize;
                let frame_len = 8 + length;
                if buf.len() < frame_len {
                    break;
                }
                let frame_bytes = buf.split_to(frame_len);
                match protocol::decode_frame(&frame_bytes) {
                    Ok(frame) => {
                        let mut session = read_session.lock().await;
                        if let Err(err) = session.handle_frame(frame) {
                            warn!("Failed to handle frame: {err:#}");
                        }
                    }
                    Err(ProtocolError::PayloadTooShort { .. }) => {
                        // Need more bytes — shouldn't happen after length check
                        break;
                    }
                    Err(err) => {
                        warn!("Protocol decode error: {err}");
                        break;
                    }
                }
            }
        }
        anyhow::Result::<()>::Ok(())
    });

    // Heartbeat writer (every 2s)
    let heartbeat_task = tokio::spawn(async move {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            let heartbeat = protocol::encode_frame(&Frame::Heartbeat(
                crate::protocol::Heartbeat {
                    timestamp_us: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_micros() as u64,
                },
            ));
            if writer.write_all(&heartbeat).await.is_err() {
                break;
            }
        }
    });

    let _ = read_task.await;
    heartbeat_task.abort();

    let mut session = session.lock().await;
    session.disconnect()?;
    info!(
        "Session closed — {} pen moves, {} dropped sequences",
        session.stats().pen_moves, session.stats().dropped_sequences
    );
    Ok(())
}

/// Convenience helper for clients sending HELLO.
pub fn client_hello_frame(device_name: &str, width_mm: f32, height_mm: f32) -> Frame {
    Frame::Hello(Hello {
        protocol_version: PROTOCOL_VERSION as u16,
        device_name: device_name.to_owned(),
        screen_width_mm: width_mm,
        screen_height_mm: height_mm,
        max_pressure: 1.0,
    })
}

pub fn frame_type_name(frame_type: FrameType) -> &'static str {
    match frame_type {
        FrameType::Hello => "HELLO",
        FrameType::HostHello => "HOST_HELLO",
        FrameType::PenDown => "PEN_DOWN",
        FrameType::PenMove => "PEN_MOVE",
        FrameType::PenUp => "PEN_UP",
        FrameType::PenHover => "PEN_HOVER",
        FrameType::Button => "BUTTON",
        FrameType::Heartbeat => "HEARTBEAT",
        FrameType::Config => "CONFIG",
    }
}
