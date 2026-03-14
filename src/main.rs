mod p2p;
mod engine;
mod economy;
mod api;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("--- SocialKube Node Starting ---");
    
    // Initialize P2P Host
    let (peer_id, _keys) = p2p::host::setup_node().await?;
    println!("Node initialized with PeerID: {}", peer_id);

    // Keep the process alive
    tokio::signal::ctrl_c().await?;
    println!("SocialKube Node Shutting Down...");
    Ok(())
}
