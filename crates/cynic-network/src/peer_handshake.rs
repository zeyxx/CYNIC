//! WireGuard peer handshake observation types and port (pure).

use serde::Serialize;

pub const PEER_HANDSHAKE_SCHEMA: &str = "cynic.wg_peer_handshake_observation.v1";

#[derive(Debug, Clone, Serialize)]
pub struct WireguardHandshakeObservation {
    pub schema: &'static str,
    pub interface: String,
    pub observed_at_unix_s: u64,
    pub peers: Vec<PeerHandshakeStatus>,
    pub stale_count: usize,
    pub dead_count: usize,
    pub ok_count: usize,
    pub unknown_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeerHandshakeStatus {
    pub public_key_prefix: String,
    pub last_handshake_unix_s: u64,
    pub age_s: u64,
    pub state: HandshakeState,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HandshakeState {
    Ok,
    Stale,
    Dead,
    NeverSeen,
}

pub trait PeerHandshakePort {
    fn observe(&self, interface: &str, stale_s: u64, dead_s: u64) -> WireguardHandshakeObservation;
}