use libp2p::identity;
use libp2p::swarm::SwarmEvent;
use libp2p::PeerId;
use libp2p::{
    gossipsub, kad, mdns, noise, swarm::NetworkBehaviour, tcp, yamux, Swarm, SwarmBuilder,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

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
    pub epoch_topic: gossipsub::IdentTopic,
}

impl core::fmt::Debug for FederationBehaviour {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let _ = self;
        f.debug_struct("FederationBehaviour").finish()
    }
}

impl core::fmt::Debug for FederationNode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FederationNode")
            .field("gossip_topic", &self.gossip_topic)
            .finish()
    }
}

impl FederationNode {
    pub fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
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
                )
                .expect("Erreur init Gossipsub");

                // Configuration de mDNS (découverte automatique)
                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?;

                // Configuration de Kademlia (Notre SwarmRegistry distribué gratuit !)
                let store = kad::store::MemoryStore::new(key.public().to_peer_id());
                let kad_config = kad::Config::default();
                let kademlia =
                    kad::Behaviour::with_config(key.public().to_peer_id(), store, kad_config);

                Ok(FederationBehaviour {
                    gossipsub,
                    mdns,
                    kademlia,
                })
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
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.gossip_topic)?;

        // On écoute sur toutes les interfaces
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        log::info!("🚀 Nœud Fédération (Kademlia+Gossip) Démarré.");
        Ok(())
    }

    /// Publie un événement à tout le réseau
    pub fn broadcast_event(
        &mut self,
        data: Vec<u8>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.gossip_topic.clone(), data)?;
        Ok(())
    }

    /// Exécute la boucle d'événements du Swarm libp2p et met à jour dynamiquement le compteur de pairs.
    pub async fn run(
        mut self,
        peer_count: Arc<AtomicU64>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use futures_util::StreamExt;

        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            log::info!("📢 P2P: Écoute locale sur l'adresse : {}", address);
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            log::info!("🤝 P2P: Connexion établie avec le pair : {}", peer_id);
                            let count = self.swarm.network_info().num_peers() as u64;
                            peer_count.store(count, Ordering::Relaxed);
                        }
                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            log::info!("🚪 P2P: Connexion fermée avec le pair : {}", peer_id);
                            let count = self.swarm.network_info().num_peers() as u64;
                            peer_count.store(count, Ordering::Relaxed);
                        }
                        SwarmEvent::Behaviour(FederationBehaviourEvent::Mdns(mdns::Event::Discovered(list))) => {
                            for (peer_id, multiaddr) in list {
                                log::info!("🔍 P2P mDNS: Nouveau pair découvert: {} ({})", peer_id, &multiaddr);
                                self.swarm.behaviour_mut().kademlia.add_address(&peer_id, multiaddr);
                            }
                        }
                        SwarmEvent::Behaviour(FederationBehaviourEvent::Mdns(mdns::Event::Expired(list))) => {
                            for (peer_id, multiaddr) in list {
                                log::info!("⏳ P2P mDNS: Pair expiré: {} ({})", peer_id, &multiaddr);
                                self.swarm.behaviour_mut().kademlia.remove_address(&peer_id, &multiaddr);
                            }
                        }
                        SwarmEvent::Behaviour(FederationBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                            propagation_source,
                            message_id,
                            message,
                        })) => {
                            log::info!(
                                "✉️ P2P Gossipsub: Message reçu de {} (ID: {}): {:?}",
                                propagation_source,
                                message_id,
                                String::from_utf8_lossy(&message.data)
                            );
                        }
                        SwarmEvent::Behaviour(FederationBehaviourEvent::Kademlia(kad::Event::RoutingUpdated {
                            peer,
                            is_new_peer,
                            ..
                        })) => {
                            if is_new_peer {
                                log::info!("⚙️ P2P Kademlia: Pair ajouté à la table de routage DHT: {}", peer);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }
}
