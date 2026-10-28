//! Mesh domain entities and ports (WireGuard overlay).
//!
//! FCIS: pure data models + trait ports; side effects live in adapters.

use std::error::Error;
use std::net::IpAddr;

/// A WireGuard peer identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerIdentity {
    pub public_key: String,
    pub internal_ip: IpAddr,
    pub endpoint: Option<String>,
}

/// The organism's mesh topology.
#[derive(Debug, Clone)]
pub struct MeshTopology {
    pub network_cidr: String,
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
    }
}

/// Port: apply mesh configuration to the underlying OS/kernel.
pub trait MeshAdapter {
    fn initialize_interface(&self, local_ip: &str, private_key: &str)
        -> Result<(), Box<dyn Error>>;
    fn sync_topology(&self, topology: &MeshTopology) -> Result<(), Box<dyn Error>>;
}

/// Port: fetch canonical mesh truth from SSoT.
pub trait MeshStateProvider {
    async fn fetch_known_peers(&self) -> Result<Vec<PeerIdentity>, Box<dyn Error>>;
}