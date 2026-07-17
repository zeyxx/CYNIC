use libp2p::{
    gossipsub, kad, mdns, noise, swarm::NetworkBehaviour, tcp, yamux, Swarm, SwarmBuilder,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;
use libp2p::identity;
use libp2p::PeerId;

/// Le comportement réseau (Behavior) qui combine mDNS (découverte locale),
/// Gossipsub (diffusion des messages) et Kademlia (Registre DHT distribué).
#[derive(NetworkBehaviour)]
pub struct FederationBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
}

pub struct FederationNode {
    pub swarm: Swarm<FederationBehaviour>,
    pub gossip_topic: gossipsub::IdentTopic,
}

impl FederationNode {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // 1. Génération de l'identité cryptographique du nœud Argo
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());
        log::info!("Identité du Nœud P2P (Argo) : {}", local_peer_id);

        // 2. Création du Swarm (Transport TCP + Sécurité Noise + Multiplexing Yamux)
        let swarm = SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )?
            .with_behaviour(|key| {
                // Configuration de Gossipsub (le protocole de diffusion des E-Scores)
                let message_id_fn = |message: &gossipsub::Message| {
                    let mut s = DefaultHasher::new();
                    message.data.hash(&mut s);
                    gossipsub::MessageId::from(s.finish().to_string())
                };

                let gossipsub_config = gossipsub::ConfigBuilder::default()
                    .heartbeat_interval(Duration::from_secs(1))
                    .validation_mode(gossipsub::ValidationMode::Strict)
                    .message_id_fn(message_id_fn)
                    .build()
                    .expect("Configuration Gossipsub invalide");

                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                ).expect("Erreur init Gossipsub");

                // Configuration de mDNS (découverte automatique)
                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?;

                // Configuration de Kademlia (Notre SwarmRegistry distribué gratuit !)
                let store = kad::store::MemoryStore::new(key.public().to_peer_id());
                let kad_config = kad::Config::default();
                let kademlia = kad::Behaviour::with_config(key.public().to_peer_id(), store, kad_config);

                Ok(FederationBehaviour { gossipsub, mdns, kademlia })
            })?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        // 3. Définition du Topic d'échange
        let gossip_topic = gossipsub::IdentTopic::new("cynic-odysseum-pumpfun-events");

        Ok(Self {
            swarm,
            gossip_topic,
        })
    }

    /// Lance le nœud P2P et s'abonne au topic de la fédération
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.swarm.behaviour_mut().gossipsub.subscribe(&self.gossip_topic)?;
        
        // On écoute sur toutes les interfaces
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        log::info!("🚀 Nœud Fédération (Kademlia+Gossip) Démarré.");
        Ok(())
    }

    /// Publie un événement à tout le réseau
    pub fn broadcast_event(&mut self, data: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
        self.swarm.behaviour_mut().gossipsub.publish(self.gossip_topic.clone(), data)?;
        Ok(())
    }
}
