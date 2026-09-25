<<<<<<< HEAD
use libp2p::PeerId;
use libp2p::identity;
use libp2p::swarm::SwarmEvent;
use libp2p::{
    Swarm, SwarmBuilder, gossipsub, kad, mdns, noise, swarm::NetworkBehaviour, tcp, yamux,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;
use tokio::sync::{broadcast, mpsc};

use crate::domain::UniversalObserverEvent;
use cynic_protocol::RuntimeProof;

pub const TOPIC_EVENTS: &str = "cynic.events.v1";
pub const TOPIC_PROOFS: &str = "cynic.proofs.v1";
pub const TOPIC_LEGACY_PUMPFUN: &str = "cynic-odysseum-pumpfun-events";

/// Le comportement réseau (Behavior) qui combine mDNS (découverte locale),
/// Gossipsub (diffusion multi-topic des preuves et événements) et Kademlia (DHT distribuée).
=======
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

use cynic_protocol::Epoch;

/// Le comportement réseau (Behavior) qui combine mDNS (découverte locale),
/// Gossipsub (diffusion des messages) et Kademlia (Registre DHT distribué).
>>>>>>> github/main
#[derive(NetworkBehaviour)]
pub struct FederationBehaviour {
    pub gossipsub: gossipsub::Behaviour,
    pub mdns: mdns::tokio::Behaviour,
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
}

pub struct FederationNode {
    pub swarm: Swarm<FederationBehaviour>,
<<<<<<< HEAD
    pub topic_events: gossipsub::IdentTopic,
    pub topic_proofs: gossipsub::IdentTopic,
    pub topic_legacy: gossipsub::IdentTopic,
=======
    pub gossip_topic: gossipsub::IdentTopic,
    /// Topic dédié au gossip des epoch certificates entre pairs P2P.
    pub epoch_topic: gossipsub::IdentTopic,
    /// Clé d'élection pour signer les epoch certificates (élection de leader).
    pub election_key: libp2p::identity::Keypair,
    /// PeerId public associé à la clé d'élection.
    pub election_pubkey: libp2p::identity::PublicKey,
    /// Epoch logique courant du nœud.
    pub current_epoch: cynic_protocol::Epoch,
    /// Timestamp de la dernière diffusion d'epoch certificate.
    pub last_epoch_broadcast: std::time::Instant,
>>>>>>> github/main
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
<<<<<<< HEAD
            .field("topic_events", &self.topic_events)
            .field("topic_proofs", &self.topic_proofs)
            .field("topic_legacy", &self.topic_legacy)
=======
            .field("gossip_topic", &self.gossip_topic)
>>>>>>> github/main
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
<<<<<<< HEAD
                // Configuration de Gossipsub
=======
                // Configuration de Gossipsub (le protocole de diffusion des E-Scores)
>>>>>>> github/main
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

<<<<<<< HEAD
                // Configuration de mDNS (découverte automatique locale)
=======
                // Configuration de mDNS (découverte automatique)
>>>>>>> github/main
                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?;

<<<<<<< HEAD
                // Configuration de Kademlia (Registre DHT décentralisé)
=======
                // Configuration de Kademlia (Notre SwarmRegistry distribué gratuit !)
>>>>>>> github/main
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

<<<<<<< HEAD
        // 3. Définition des Topics canoniques
        let topic_events = gossipsub::IdentTopic::new(TOPIC_EVENTS);
        let topic_proofs = gossipsub::IdentTopic::new(TOPIC_PROOFS);
        let topic_legacy = gossipsub::IdentTopic::new(TOPIC_LEGACY_PUMPFUN);

        Ok(Self {
            swarm,
            topic_events,
            topic_proofs,
            topic_legacy,
        })
    }

    /// Lance le nœud P2P et s'abonne à tous les topics de la fédération
=======
        // 3. Définition du Topic d'échange
        let gossip_topic = gossipsub::IdentTopic::new("cynic-odysseum-pumpfun-events");
        let epoch_topic = gossipsub::IdentTopic::new("cynic-odysseum-epoch-certificates");

        // 4. Génération de la clé de l'élection pour signer les epoch certificates
        let election_key = identity::Keypair::generate_ed25519();
        let election_pubkey = election_key.public();

        Ok(Self {
            swarm,
            gossip_topic,
            epoch_topic,
            election_key,
            election_pubkey,
            current_epoch: Epoch::ZERO,
            last_epoch_broadcast: std::time::Instant::now(),
        })
    }

    /// Retourne l'epoch logique actuel du nœud.
    pub fn current_epoch(&self) -> Epoch {
        self.current_epoch
    }

    /// Publie un EpochCertificate sur le réseau via Gossipsub.
    pub fn broadcast_epoch_certificate(
        &mut self,
        cert: &cynic_protocol::EpochCertificate,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let data = serde_json::to_vec(cert)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.epoch_topic.clone(), data)?;
        Ok(())
    }

    /// Génère et broadcast un EpochCertificate pour l'epoch courant.
    pub fn announce_epoch(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let cert = cynic_protocol::EpochCertificate {
            epoch: self.current_epoch,
            producer_id: self.election_pubkey.to_peer_id().to_string(),
            proof_of_progress: format!("blockhash:{}:slot:{}", self.current_epoch.id(), self.current_epoch.id()),
            signed_at_unix_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis())
                .unwrap_or(0),
        };
        self.current_epoch = self.current_epoch.next();
        self.broadcast_epoch_certificate(&cert)?;
        self.last_epoch_broadcast = std::time::Instant::now();
        log::info!("📢 Epoch {} annoncé sur le réseau P2P", cert.epoch.id());
        Ok(())
    }

    /// Lance le nœud P2P et s'abonne aux topics de la fédération
>>>>>>> github/main
    pub async fn start(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.swarm
            .behaviour_mut()
            .gossipsub
<<<<<<< HEAD
            .subscribe(&self.topic_events)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.topic_proofs)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.topic_legacy)?;

        // Écoute sur toutes les interfaces réseau
        self.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;
        log::info!("🚀 Nœud Fédération (Kademlia DHT + Multi-Topic Gossipsub) Démarré.");
        Ok(())
    }

    /// Publie un événement universel multi-domaine sur le réseau P2P
    pub fn broadcast_universal_event(
        &mut self,
        event: &UniversalObserverEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bytes = serde_json::to_vec(event)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.topic_events.clone(), bytes)?;
        Ok(())
    }

    /// Publie une preuve d'exécution RuntimeProof à toute la fédération
    pub fn broadcast_runtime_proof(
        &mut self,
        proof: &RuntimeProof,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bytes = serde_json::to_vec(proof)?;
        self.swarm
            .behaviour_mut()
            .gossipsub
            .publish(self.topic_proofs.clone(), bytes)?;
        Ok(())
    }

    /// Exécute la boucle d'événements du Swarm libp2p avec pont bidirectionnel
    pub async fn run(
        mut self,
        peer_count: Arc<AtomicU64>,
        tx_incoming_events: broadcast::Sender<UniversalObserverEvent>,
        mut rx_outgoing_events: mpsc::Receiver<UniversalObserverEvent>,
=======
            .subscribe(&self.gossip_topic)?;
        // S'abonner également au topic des epoch certificates
        self.swarm
            .behaviour_mut()
            .gossipsub
            .subscribe(&self.epoch_topic)?;
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
>>>>>>> github/main
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        use futures_util::StreamExt;

        loop {
            tokio::select! {
<<<<<<< HEAD
                // Diffusion d'un événement produit localement vers le réseau P2P
                Some(event) = rx_outgoing_events.recv() => {
                    if let Err(e) = self.broadcast_universal_event(&event) {
                        log::warn!("Échec de diffusion P2P Gossipsub de l'événement {}: {}", event.id, e);
                    }
                }
                // Réception et traitement des événements réseau Swarm
                swarm_event = self.swarm.select_next_some() => {
                    match swarm_event {
=======
                event = self.swarm.select_next_some() => {
                    match event {
>>>>>>> github/main
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
<<<<<<< HEAD
                            let topic_str = message.topic.as_str();
                            log::info!(
                                "✉️ P2P Gossipsub [{}]: Message reçu de {} (ID: {})",
                                topic_str,
                                propagation_source,
                                message_id
                            );

                            if topic_str == TOPIC_EVENTS {
                                if let Ok(event) = serde_json::from_slice::<UniversalObserverEvent>(&message.data) {
                                    let _ = tx_incoming_events.send(event);
                                }
=======
                            // Vérifier si c'est un epoch certificate (topic epoch-certificates)
                            if message.topic == Some(self.epoch_topic.hash()) {
                                if let Ok(cert) = serde_json::from_slice::<cynic_protocol::EpochCertificate>(&message.data) {
                                    // Valider et intégrer l'epoch certificate reçu
                                    if cert.epoch.id() > self.current_epoch.id() {
                                        self.current_epoch = cert.epoch;
                                        log::info!(
                                            "📥 Epoch certificate reçu de {} : epoch {} (nouveau max)",
                                            propagation_source, cert.epoch.id()
                                        );
                                    }
                                } else {
                                    log::warn!(
                                        "⚠️ Epoch certificate invalide reçu de {} (ID: {})",
                                        propagation_source, message_id
                                    );
                                }
                            } else {
                                log::info!(
                                    "✉️ P2P Gossipsub: Message reçu de {} (ID: {}): {:?}",
                                    propagation_source,
                                    message_id,
                                    String::from_utf8_lossy(&message.data)
                                );
>>>>>>> github/main
                            }
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
