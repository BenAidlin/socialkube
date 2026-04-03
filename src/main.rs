mod p2p;
mod engine;
mod economy;
mod api;
mod error;
mod config;

use tracing::{info, info_span, error};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use libp2p::futures::StreamExt;
use libp2p::{swarm::SwarmEvent, mdns, gossipsub};
use std::time::Duration;
use std::sync::Arc;
use crate::p2p::behaviour::SOCIALKUBE_TASK_TOPIC;
use tower_http::cors::{Any, CorsLayer};

use crate::engine::backend::{QwenBackend, ModelBackend};
use crate::engine::downloader::ModelDownloader;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

    // Initial Shard Assignment
    let shard_assignments = Arc::new(engine::sharder::calculate_shard_assignment(&hw_profile));
    info!("Initial assignment: Node hosting {} model(s)", shard_assignments.len());

    // Initialize Shared Engine Backend
    let shared_backend: Arc<tokio::sync::Mutex<Option<Box<dyn ModelBackend>>>> = Arc::new(tokio::sync::Mutex::new(None));
    let shared_memory = Arc::new(engine::memory::ConversationMemory::default());
    
    let app_state = api::routes::AppState {
        shard_assignments: shard_assignments.as_ref().clone(),
        hw_profile: hw_profile.clone(),
        backend: shared_backend.clone(),
        memory: shared_memory.clone(),
    };

    // Trigger Model Downloads and Backend Initialization
    if let Ok(downloader) = ModelDownloader::new() {
        let assignments_clone = shard_assignments.clone();
        let backend_clone = shared_backend.clone();
        tokio::spawn(async move {
            let downloaded = downloader.check_and_download_models(&assignments_clone).await;
            
            // If we downloaded the base model, initialize the backend
            if let Some((_id, paths, tokenizer_path)) = downloaded.iter().find(|(id, _, _)| id == config::DEFAULT_MODEL_ID) {
                info!("Initializing Inference Backend for {} ({} files)...", config::DEFAULT_MODEL_ID, paths.len());
                
                // Copy tokenizer from the downloaded location to current directory
                let tokenizer_dst = config::get_tokenizer_path();
                if let Err(e) = std::fs::copy(&tokenizer_path, &tokenizer_dst) {
                    error!("Failed to copy tokenizer from {:?} to {:?}: {:?}", tokenizer_path, tokenizer_dst, e);
                }

                match QwenBackend::new() {
                    Ok(mut backend) => {
                        if let Err(e) = backend.load_model(paths.clone()) {
                            error!("Failed to load model into backend: {:?}", e);
                        } else {
                            let mut lock = backend_clone.lock().await;
                            *lock = Some(Box::new(backend));
                            info!("Inference Backend is READY.");
                        }
                    }
                    Err(e) => error!("Failed to initialize backend: {:?}", e),
                }
            }
        });
    }

    // Start Axum API Server
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = api::routes::create_router(app_state.clone()).layer(cors);
    
    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("API Server listening on http://{}", addr);

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!("API Server error: {:?}", e);
        }
    });

    // Initialize P2P Host
    let mut swarm = p2p::host::build_swarm().await?;
    let local_peer_id = swarm.local_peer_id().to_string();
    info!("Node initialized with PeerID: {}", local_peer_id);

    // Initialize Economy Ledger
    let db_name = format!("ledger/socialkube_{}.db", &local_peer_id);
    let ledger = economy::ledger::Ledger::new(&db_name).map_err(|e| anyhow::anyhow!("Ledger error: {:?}", e))?;

    // Subscribe to Gossipsub topic
    let topic = gossipsub::IdentTopic::new(SOCIALKUBE_TASK_TOPIC);
    swarm.behaviour_mut().gossipsub.subscribe(&topic)?;
    info!("Subscribed to topic: {}", SOCIALKUBE_TASK_TOPIC);

    // Listen on all interfaces
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let mut tick = tokio::time::interval(Duration::from_secs(10));

    loop {
        tokio::select! {
            _ = tick.tick() => {
                let message = format!("Heartbeat from {}", local_peer_id);
                if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic.clone(), message.as_bytes()) {
                    error!("Publish error: {:?}", e);
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

                        // TEST: Trigger an inference request
                        if let Ok(can_spend) = ledger.spend_credits(&local_peer_id, 10) {
                            if can_spend {
                                let request = p2p::behaviour::InferenceRequest {
                                    model_id: config::DEFAULT_MODEL_ID.to_string(),
                                    prompt: "What is the capital of Israel?".to_string(),
                                    shard_index: 0,
                                };
                                info!("Sending InferenceRequest to {}...", peer_id);
                                swarm.behaviour_mut().request_response.send_request(&peer_id, request);
                            } else {
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
                                    let _ = ledger.add_credits(&peer.to_string(), 10);
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
