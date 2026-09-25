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
    }
}
