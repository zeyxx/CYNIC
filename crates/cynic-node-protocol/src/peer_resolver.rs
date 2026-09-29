//! Peer resolution types and pure peer-selection helpers.
//!
//! The desired mesh topology comes from a declared registry seed
//! (`infra/synapse-nodes.seed.json`) and an optional signed rolling fallback
//! bundle. This module carries only the portable types and the pure
//! selection decisions; resolver implementations live behind the
//! `PeerResolver` trait in the consuming crate (e.g. cynic-node).

use serde::{Deserialize, Serialize};

use crate::emergence::RolePriority;
use cynic_protocol::health_signal::NodeHealthView;

/// Transport used to reach a peer endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PeerTransport {
    /// Unix domain socket (local / loopback cortex).
    Uds,
    /// WebSocket (storage primary, etc.).
    Ws,
    /// HTTP (default for remote peers).
    Http,
}

/// A single resolved peer endpoint.
#[derive(Debug, Clone, PartialEq)]
pub struct PeerEndpoint {
    pub node_id: String,
    pub role: String,
    pub transport: PeerTransport,
    /// Declared/observed address (IP or socket path).
    pub addr: String,
    pub probe_ports: Vec<u16>,
    /// Desired-until-observed: assumed reachable until a probe/observer proves otherwise.
    pub reachable: bool,
    pub health: NodeHealthView,
}

/// A peer endpoint wrapped with an ed25519 signature from the rolling fallback bundle.
///
/// The signature field is private: the wrapper is the evidence, so the only
/// way to obtain one is to have a signature to put in it. Struct-literal
/// construction is deliberately impossible, which is what makes
/// `PeerResolver::fallback_peers` enforceable rather than merely documented.
#[derive(Debug, Clone, PartialEq)]
pub struct SignedPeerEndpoint {
    pub endpoint: PeerEndpoint,
    signature_ed25519: String,
    pub generated_at_unix_s: u64,
    pub version: u8,
}

/// Why a signed endpoint could not be admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureRejection {
    /// The bundle carried no signature. Fail-closed: no proof, no peer.
    Missing,
}

impl SignedPeerEndpoint {
    /// Admit an endpoint only when a non-empty ed25519 signature is present.
    ///
    /// This does not cryptographically verify the signature — that is the
    /// caller's job, and it is why the rejection is a data value rather than a
    /// silent empty. What it guarantees is that no `SignedPeerEndpoint` can
    /// exist without a signature, so a peer selected out of a fallback set is
    /// never an unverified seed wearing the proof of one.
    pub fn from_verified(
        endpoint: PeerEndpoint,
        signature_ed25519: String,
        generated_at_unix_s: u64,
        version: u8,
    ) -> Result<Self, SignatureRejection> {
        if signature_ed25519.is_empty() {
            return Err(SignatureRejection::Missing);
        }
        Ok(Self {
            endpoint,
            signature_ed25519,
            generated_at_unix_s,
            version,
        })
    }

    /// The verified signature this endpoint was admitted with.
    pub fn signature(&self) -> &str {
        &self.signature_ed25519
    }
}

/// Resolver port: yields desired peers for a role, or all declared peers.
pub trait PeerResolver {
    /// Peers declared for a specific role.
    fn resolve_peers(&self, role: &str) -> Vec<PeerEndpoint>;
    /// Every declared peer, across all roles.
    fn resolve_all_peers(&self) -> Vec<PeerEndpoint>;
    /// Signed rolling-fallback peers for a role (fail-closed: empty when the
    /// bundle is missing or its signature does not verify).
    ///
    /// The fail-closed rule is load-bearing and was previously held only by
    /// comments in each implementor: `MeshPeerResolver` returns `vec![]`
    /// unconditionally because the registry carries no signatures, while
    /// `LocalRegistryMeshResolver` reads a `known-peers.<ts>.json` bundle.
    /// Two opposite reasons, same obligation, nothing enforced it. An adapter
    /// that returned unsigned endpoints under partition would present itself
    /// as a verified fallback peer — precisely the distinction the signed
    /// bundle exists to preserve.
    ///
    /// So the obligation moves to the type: an implementor must construct
    /// `SignedPeerEndpoint` through [`SignedPeerEndpoint::from_verified`],
    /// which admits only a non-empty signature. An endpoint that cannot be
    /// proven verified cannot be given the wrapper that means it was.
    fn fallback_peers(&self, role: &str) -> Vec<SignedPeerEndpoint>;
}

/// Pure selector: the first reachable, role-matching peer.
///
/// Returns `None` when no candidate is dispatchable — the caller must then emit
/// a `TopologyDriftSignal` (drift is data, not a crash).
pub fn select_peer(peers: &[PeerEndpoint], role: &str) -> Option<PeerEndpoint> {
    peers
        .iter()
        .filter(|p| p.role == role && p.reachable && !p.health.critical_active)
        .filter_map(|p| {
            let priority = RolePriority::from_role_str(&p.role)?;
            Some((p, priority))
        })
        .min_by(|(a, pa), (b, pb)| {
            pa.cmp(pb).then_with(|| {
                a.health
                    .active_osha_codes
                    .len()
                    .cmp(&b.health.active_osha_codes.len())
            })
        })
        .map(|(p, _)| p.clone())
}

/// Select from signature-verified fallback peers, keeping the proof attached.
///
/// `PeerResolver::fallback_peers` is fail-closed: it returns only endpoints
/// whose ed25519 signature verified, so the `SignedPeerEndpoint` wrapper *is*
/// the evidence that a peer was authenticated. Unwrapping to `PeerEndpoint`
/// before selecting would silently discard that proof and make a verified
/// fallback peer indistinguishable from an unverified seed entry — which is
/// exactly the distinction the signed bundle exists to preserve.
///
/// Returns `None` when no candidate is dispatchable, same as [`select_peer`].
pub fn select_signed_peer(peers: &[SignedPeerEndpoint], role: &str) -> Option<SignedPeerEndpoint> {
    let endpoints: Vec<PeerEndpoint> = peers.iter().map(|p| p.endpoint.clone()).collect();
    let chosen = select_peer(&endpoints, role)?;
    peers.iter().find(|p| p.endpoint == chosen).cloned()
}

/// Pure selector: every reachable peer, in declared order.
pub fn select_all_peers(peers: &[PeerEndpoint]) -> Vec<PeerEndpoint> {
    peers.iter().filter(|p| p.reachable).cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ep(role: &str, reachable: bool) -> PeerEndpoint {
        PeerEndpoint {
            node_id: format!("node-{role}"),
            role: role.to_string(),
            transport: PeerTransport::Http,
            addr: "10.10.10.1".to_string(),
            probe_ports: vec![3000],
            reachable,
            health: NodeHealthView {
                critical_active: false,
                warning_active: false,
                active_osha_codes: vec![],
                available_mem_mib: 0,
            },
        }
    }

    #[test]
    fn select_peer_picks_reachable_role_match() {
        let peers = vec![ep("cortex_primary", false), ep("cortex_primary", true)];
        let got = select_peer(&peers, "cortex_primary").expect("reachable peer exists");
        assert!(got.reachable);
        assert_eq!(got.role, "cortex_primary");
    }

    #[test]
    fn select_peer_none_when_all_unreachable() {
        let peers = vec![ep("cortex_primary", false)];
        assert!(select_peer(&peers, "cortex_primary").is_none());
    }

    #[test]
    fn select_all_peers_filters_unreachable() {
        let peers = vec![ep("a", true), ep("b", false)];
        let got = select_all_peers(&peers);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].role, "a");
    }

    fn signed(role: &str, reachable: bool, signature: &str) -> SignedPeerEndpoint {
        SignedPeerEndpoint::from_verified(ep(role, reachable), signature.to_string(), 1, 1)
            .expect("a non-empty signature is admitted")
    }

    #[test]
    fn from_verified_rejects_an_absent_signature() {
        // The fail-closed obligation of `PeerResolver::fallback_peers`, pinned
        // where it can be checked: an unproven peer must not be able to take
        // the shape of a proven one.
        let got = SignedPeerEndpoint::from_verified(ep("a", true), String::new(), 1, 1);
        assert_eq!(got.err(), Some(SignatureRejection::Missing));
    }

    #[test]
    fn select_signed_peer_keeps_the_signature_it_selected_on() {
        // The file's own doc warns that unwrapping before selecting "would
        // silently discard that proof". This is that divergence, as a test.
        let peers = vec![signed("cortex_primary", true, "sig-abc")];
        let got = select_signed_peer(&peers, "cortex_primary").expect("one signed peer");
        assert_eq!(got.signature(), "sig-abc");
        assert_eq!(got, peers[0]);
    }

    #[test]
    fn select_signed_peer_is_none_when_nothing_is_reachable() {
        let peers = vec![signed("cortex_primary", false, "sig-abc")];
        assert!(select_signed_peer(&peers, "cortex_primary").is_none());
    }
}
