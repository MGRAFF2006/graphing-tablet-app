use clap::Parser;
use graphpad_core::{run_host, HostConfig, DEFAULT_PORT};
use tracing::info;
use tracing_subscriber::EnvFilter;

#[derive(Parser, Debug)]
#[command(
    name = "graphpad-host",
    about = "GraphPad host daemon — virtual graphics tablet from iPad pen input"
)]
struct Args {
    /// Address to bind (default: all interfaces)
    #[arg(long, default_value = "0.0.0.0")]
    bind: String,

    /// TCP port for pen stream
    #[arg(short, long, default_value_t = DEFAULT_PORT)]
    port: u16,

    /// Host name sent to clients
    #[arg(long, default_value = "GraphPad Host")]
    name: String,

    /// Log pen events without creating a virtual device (Linux only)
    #[arg(long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    let bind_addr = format!("{}:{}", args.bind, args.port);

    info!("GraphPad host daemon v{}", env!("CARGO_PKG_VERSION"));
    info!("Bind address: {bind_addr}");

    #[cfg(target_os = "linux")]
    {
        if args.dry_run {
            info!("Dry-run mode — logging pen events only");
            run_host(
                HostConfig {
                    bind_addr,
                    host_name: args.name,
                },
                || Ok(graphpad_input_windows::LogOnlyBackend),
            )
            .await?;
        } else {
            run_host(
                HostConfig {
                    bind_addr,
                    host_name: args.name,
                },
                || graphpad_input_linux::LinuxTablet::new(),
            )
            .await?;
        }
    }

    #[cfg(target_os = "windows")]
    {
        if args.dry_run {
            run_host(
                HostConfig {
                    bind_addr,
                    host_name: args.name,
                },
                || Ok(graphpad_input_windows::LogOnlyBackend),
            )
            .await?;
        } else {
            anyhow::bail!(
                "Windows digitizer mode is not yet available. Use --dry-run to test the protocol."
            );
        }
    }

    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    {
        run_host(
            HostConfig {
                bind_addr,
                host_name: args.name,
            },
            || Ok(graphpad_input_windows::LogOnlyBackend),
        )
        .await?;
    }

    Ok(())
}
