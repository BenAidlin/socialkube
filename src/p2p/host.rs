use libp2p::{
    identity,
    noise,
    swarm::Swarm,
    SwarmBuilder,
    tcp, yamux, PeerId,
};
use crate::p2p::behaviour::SocialKubeBehaviour;

pub async fn build_swarm() -> anyhow::Result<Swarm<SocialKubeBehaviour>> {
    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());

    let swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            SocialKubeBehaviour::new(local_peer_id, key.clone()).expect("Failed to initialize behaviour")
        })?
        .build();

    Ok(swarm)
}
