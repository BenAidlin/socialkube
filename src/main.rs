mod p2p;
mod engine;
mod economy;
mod api;

use tracing::{info, info_span};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging: stdout + rolling log file
    let file_appender = tracing_appender::rolling::daily("logs", "socialkube.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with(fmt::layer().with_writer(std::io::stdout))
        .with(fmt::layer().with_ansi(false).with_writer(non_blocking))
        .init();

    let _span = info_span!("node_main").entered();
    info!("--- SocialKube Node Starting ---");
    
    // Initialize P2P Host
    let (peer_id, _keys) = p2p::host::setup_node().await?;
    info!("Node initialized with PeerID: {}", peer_id);

    // Keep the process alive
    tokio::signal::ctrl_c().await?;
    info!("SocialKube Node Shutting Down...");
    Ok(())
}
