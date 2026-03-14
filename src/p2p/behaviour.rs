use libp2p::kad::store::MemoryStore;
use libp2p::kad::{Kademlia, KademliaEvent};
use libp2p::gossipsub::{Gossipsub, GossipsubEvent, MessageAuthenticity, ConfigBuilder};
use libp2p::swarm::NetworkBehaviour;
use libp2p::{identify, mdns, PeerId};
use serde::{Deserialize, Serialize};

// Define the "SocialKube" Gossip Topic
pub const SOCIALKUBE_TASK_TOPIC: &str = "socialkube-inference-tasks";

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "SocialKubeEvent")]
pub struct SocialKubeBehaviour {
    // Kademlia DHT for finding peers across the internet (Israel-wide)
    pub kademlia: Kademlia<MemoryStore>,
    
    // Gossipsub for broadcasting inference tasks and swarm updates
    pub gossipsub: Gossipsub,
    
    // MDNS for finding other SocialKube nodes on the same local network (LAN)
    pub mdns: mdns::tokio::Behaviour,
    
    // Identify protocol to exchange public keys and supported protocols
    pub identify: identify::Behaviour,
}

#[derive(Debug)]
pub enum SocialKubeEvent {
    Kad(KademliaEvent),
    Gossip(GossipsubEvent),
    Mdns(mdns::Event),
    Identify(identify::Event),
}

// Convert sub-module events into our global SocialKubeEvent
impl From<KademliaEvent> for SocialKubeEvent {
    fn from(event: KademliaEvent) -> Self {
        SocialKubeEvent::Kad(event)
    }
}

impl From<GossipsubEvent> for SocialKubeEvent {
    fn from(event: GossipsubEvent) -> Self {
        SocialKubeEvent::Gossip(event)
    }
}

impl From<mdns::Event> for SocialKubeEvent {
    fn from(event: mdns::Event) -> Self {
        SocialKubeEvent::Mdns(event)
    }
}

impl From<identify::Event> for SocialKubeEvent {
    fn from(event: identify::Event) -> Self {
        SocialKubeEvent::Identify(event)
    }
}

impl SocialKubeBehaviour {
    pub fn new(local_peer_id: PeerId, local_key: libp2p::identity::Keypair) -> Result<Self, Box<dyn std::error::Error>> {
        
        // 1. Setup Kademlia (The "Find Me" system)
        let store = MemoryStore::new(local_peer_id);
        let kademlia = Kademlia::new(local_peer_id, store);

        // 2. Setup Gossipsub (The "Broadcast" system)
        let gossipsub_config = ConfigBuilder::default()
            .heartbeat_interval(std::time::Duration::from_secs(1))
            .validation_mode(libp2p::gossipsub::ValidationMode::Strict)
            .build()?;
            
        let gossipsub = Gossipsub::new(
            MessageAuthenticity::Signed(local_key.clone()),
            gossipsub_config,
        )?;

        // 3. Setup MDNS (Local Discovery)
        let mdns = mdns::tokio::Behaviour::new(mdns::Config::default(), local_peer_id)?;

        // 4. Setup Identify
        let identify = identify::Behaviour::new(identify::Config::new(
            "/socialkube/1.0.0".into(),
            local_key.public(),
        ));

        Ok(Self {
            kademlia,
            gossipsub,
            mdns,
            identify,
        })
    }
}