mod app;
mod routes;
mod static_files;

use clap::Parser;
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Parser)]
#[command(name = "rust_tool_server")]
#[command(about = "RustTool web API server")]
struct ServerArgs {
    #[arg(long, env = "RUSTTOOL_SERVER_HOST", default_value = "127.0.0.1")]
    host: String,
    #[arg(long, env = "RUSTTOOL_SERVER_PORT", default_value_t = 5172)]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = ServerArgs::parse();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rust_tool_server=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app = app::build_app();
    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    let listener = TcpListener::bind(addr).await?;

    tracing::info!("RustTool server listening on http://{addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
