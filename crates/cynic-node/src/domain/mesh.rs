<<<<<<< HEAD
use std::net::IpAddr;

/// Domain Entity: A WireGuard Peer Identity
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerIdentity {
    pub public_key: String,
    pub internal_ip: IpAddr,
    pub endpoint: Option<String>, // Can be None if roaming/NAT
}

/// Domain Entity: The Organism's Mesh Topology
#[derive(Debug, Clone)]
pub struct MeshTopology {
    pub network_cidr: String, // e.g., "10.20.0.0/16"
    pub peers: Vec<PeerIdentity>,
}

impl MeshTopology {
    pub fn new(network_cidr: String) -> Self {
        Self {
            network_cidr,
            peers: Vec::new(),
        }
    }

    pub fn add_peer(&mut self, peer: PeerIdentity) {
        self.peers.push(peer);
=======
use std::collections::BTreeMap;
use std::net::IpAddr;
use serde::{Deserialize, Serialize};

pub const SYNAPSE_P2P_SCHEMA_V1: &str = "cynic.synapse_p2p.v1";

/// Domain Entity: A WireGuard Peer Identity
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PeerIdentity {
    pub public_key: String,
    pub internal_ip: IpAddr,
    pub endpoint: Option<String>,
    pub node_id: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeerConnectivityStatus {
    Active,
    Syncing,
    Offline,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SynapsePeerState {
    pub identity: PeerIdentity,
    pub status: PeerConnectivityStatus,
    pub last_seen_timestamp: i64,
    pub latest_merkle_root: Option<String>,
    pub active_leases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SynapseP2PMessage {
    Handshake {
        schema: &'static str,
        node_id: String,
        protocol_version: u32,
        timestamp: i64,
    },
    LeaseAnnouncement {
        agent_id: String,
        target: String,
        fencing_token: u64,
        expires_at: String,
    },
    MerkleRootBroadcast {
        epoch: u64,
        merkle_root_hex: String,
        timestamp: i64,
    },
    Heartbeat {
        node_id: String,
        timestamp: i64,
    },
}

/// Domain Entity: The Organism's Synapse Mesh Topology
#[derive(Debug, Clone, Default)]
pub struct SynapseMeshTopology {
    pub network_cidr: String,
    pub peers: BTreeMap<String, SynapsePeerState>,
}

impl SynapseMeshTopology {
    pub fn new(network_cidr: &str) -> Self {
        Self {
            network_cidr: network_cidr.to_string(),
            peers: BTreeMap::new(),
        }
    }

    pub fn register_peer(&mut self, identity: PeerIdentity, now: i64) {
        let node_id = identity.node_id.clone();
        let state = SynapsePeerState {
            identity,
            status: PeerConnectivityStatus::Active,
            last_seen_timestamp: now,
            latest_merkle_root: None,
            active_leases: Vec::new(),
        };
        self.peers.insert(node_id, state);
    }

    pub fn handle_message(&mut self, sender_node_id: &str, msg: SynapseP2PMessage, now: i64) -> Result<(), String> {
        let Some(peer) = self.peers.get_mut(sender_node_id) else {
            return Err(format!("Unknown peer node_id: {sender_node_id}"));
        };

        peer.last_seen_timestamp = now;
        peer.status = PeerConnectivityStatus::Active;

        match msg {
            SynapseP2PMessage::Handshake { .. } => {}
            SynapseP2PMessage::Heartbeat { .. } => {}
            SynapseP2PMessage::LeaseAnnouncement { target, .. } => {
                if !peer.active_leases.contains(&target) {
                    peer.active_leases.push(target);
                }
            }
            SynapseP2PMessage::MerkleRootBroadcast { merkle_root_hex, .. } => {
                peer.latest_merkle_root = Some(merkle_root_hex);
            }
        }

        Ok(())
    }

    pub fn active_peers_count(&self, now: i64, timeout_secs: i64) -> usize {
        self.peers
            .values()
            .filter(|p| now - p.last_seen_timestamp <= timeout_secs)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[test]
    fn test_mesh_registration_and_message_handling() {
        let mut mesh = SynapseMeshTopology::new("10.10.10.0/24");

        let peer_a = PeerIdentity {
            public_key: "pubkey-node-proxmox-1".to_string(),
            internal_ip: IpAddr::V4(Ipv4Addr::new(10, 10, 10, 101)),
            endpoint: Some("192.168.1.50:51820".to_string()),
            node_id: "node-proxmox-1".to_string(),
        };

        mesh.register_peer(peer_a, 1_720_000_000);
        assert_eq!(mesh.peers.len(), 1);

        // Receive lease announcement
        let lease_msg = SynapseP2PMessage::LeaseAnnouncement {
            agent_id: "agent-codex".to_string(),
            target: "crates/cynic-inference".to_string(),
            fencing_token: 1001,
            expires_at: "2026-08-14T00:00:00Z".to_string(),
        };

        mesh.handle_message("node-proxmox-1", lease_msg, 1_720_000_010).expect("handle msg");
        let peer = mesh.peers.get("node-proxmox-1").expect("peer");
        assert_eq!(peer.active_leases, vec!["crates/cynic-inference"]);

        // Receive Merkle root broadcast
        let merkle_msg = SynapseP2PMessage::MerkleRootBroadcast {
            epoch: 42,
            merkle_root_hex: "abcdef1234567890".to_string(),
            timestamp: 1_720_000_020,
        };

        mesh.handle_message("node-proxmox-1", merkle_msg, 1_720_000_020).expect("handle msg");
        let peer = mesh.peers.get("node-proxmox-1").expect("peer");
        assert_eq!(peer.latest_merkle_root, Some("abcdef1234567890".to_string()));
>>>>>>> github/main
    }
}
