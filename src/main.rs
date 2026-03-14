mod p2p;
mod engine;
mod economy;
mod api;

use tracing::{info, info_span};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use libp2p::futures::StreamExt;
use libp2p::{swarm::SwarmEvent, mdns};

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
    let mut swarm = p2p::host::build_swarm().await?;
    info!("Node initialized with PeerID: {}", swarm.local_peer_id());

    // Listen on all interfaces with a random OS-assigned port
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    loop {
        tokio::select! {
            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on {:?}", address);
                }
                SwarmEvent::Behaviour(p2p::behaviour::SocialKubeEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, multiaddr) in list {
                        info!("mDNS discovered: {} at {}", peer_id, multiaddr);
                        swarm.behaviour_mut().kademlia.add_address(&peer_id, multiaddr);
                    }
                }
                SwarmEvent::Behaviour(p2p::behaviour::SocialKubeEvent::Gossip(event)) => {
                     info!("Gossipsub event: {:?}", event);
                }
                // Handle other events as needed
                _ => {}
            },
            _ = tokio::signal::ctrl_c() => {
                info!("SocialKube Node Shutting Down...");
                break;
            }
        }
    }
    Ok(())
}
