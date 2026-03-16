mod p2p;
mod engine;
mod economy;
mod api;

use tracing::{info, info_span};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use libp2p::futures::StreamExt;
use libp2p::{swarm::SwarmEvent, mdns, gossipsub};
use std::time::Duration;
use crate::p2p::behaviour::SOCIALKUBE_TASK_TOPIC;

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

    // FR-3 Hardware Benchmarking
    let hw_profile = engine::benchmark::detect_hardware();
    info!("Local Hardware Profile: {:?}", hw_profile);

    // Initial Shard Assignment (1B Full + 8B partial)
    let shard_assignments = engine::sharder::calculate_shard_assignment(&hw_profile);
    info!("Initial assignment: Node hosting {} model(s)", shard_assignments.len());

    // Initialize P2P Host
    let mut swarm = p2p::host::build_swarm().await?;
    let local_peer_id = swarm.local_peer_id().to_string();
    info!("Node initialized with PeerID: {}", local_peer_id);

    // Initialize Economy Ledger with a unique filename for this peer in the 'ledger' directory
    let db_name = format!("ledger/socialkube_{}.db", &local_peer_id);
    let ledger = economy::ledger::Ledger::new(&db_name)?;

    // Subscribe to Gossipsub topic
    let topic = gossipsub::IdentTopic::new(SOCIALKUBE_TASK_TOPIC);
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
    info!("Subscribed to topic: {}", SOCIALKUBE_TASK_TOPIC);

    // Listen on all interfaces with a random OS-assigned port
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // Heartbeat ticker for testing
    let mut tick = tokio::time::interval(Duration::from_secs(10));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let message = format!("Heartbeat from {}", local_peer_id);
                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), message.as_bytes()) {
                    tracing::error!("Publish error: {:?}", e);
                }
            }
            event = swarm.select_next_some() => match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on {:?}", address);
                }
                SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                    info!("Successfully connected to {} at {:?}", peer_id, endpoint);
                }
                SwarmEvent::Behaviour(p2p::behaviour::SocialKubeEvent::Mdns(mdns::Event::Discovered(list))) => {
                    for (peer_id, multiaddr) in list {
                        info!("mDNS discovered: {} at {}", peer_id, multiaddr);
                        swarm.behaviour_mut().kademlia.add_address(&peer_id, multiaddr.clone());
                        let _ = swarm.dial(multiaddr);

                        // TEST: Trigger an inference request to the newly discovered peer
                        if let Ok(can_spend) = ledger.spend_credits(&local_peer_id, 10) {
                            if can_spend {
                                let request = p2p::behaviour::InferenceRequest {
                                    model_id: "llama-3-8b".to_string(),
                                    prompt: "What is the capital of Israel?".to_string(),
                                    shard_index: 0,
                                };
                                info!("Sending InferenceRequest to {}...", peer_id);
                                swarm.behaviour_mut().request_response.send_request(&peer_id, request);
                            } else {
                                info!("Not enough credits to send request to {}", peer_id);
                                // For testing purposes, give ourselves some credits if we are empty
                                let _ = ledger.add_credits(&local_peer_id, 100);
                            }
                        }
                    }
                }
                SwarmEvent::Behaviour(p2p::behaviour::SocialKubeEvent::Gossip(gossipsub::Event::Message { 
                    propagation_source, 
                    message, 
                    .. 
                })) => {
                    let msg_str = String::from_utf8_lossy(&message.data);
                    info!("Got message: '{}' from {:?}", msg_str, propagation_source);
                }
                SwarmEvent::Behaviour(p2p::behaviour::SocialKubeEvent::RequestResponse(event)) => {
                    match event {
                        libp2p::request_response::Event::Message { peer, message } => {
                            match message {
                                libp2p::request_response::Message::Request { request, channel, .. } => {
                                    info!("Received InferenceRequest from {}: {:?}", peer, request);
                                    
                                    // 1. Credit the worker locally for the effort
                                    let _ = ledger.add_credits(&peer.to_string(), 10);
                                    
                                    // 2. Respond with a placeholder result
                                    let response = p2p::behaviour::InferenceResponse {
                                        result: format!("Computed result for: {}", request.prompt),
                                    };
                                    let _ = swarm.behaviour_mut().request_response.send_response(channel, response);
                                }
                                libp2p::request_response::Message::Response { response, .. } => {
                                    info!("Received InferenceResponse from {}: {:?}", peer, response);
                                }
                            }
                        }
                        _ => {}
                    }
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
