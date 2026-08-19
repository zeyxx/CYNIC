//! Local registry-backed mesh resolver.
//!
//! Replaces direct reads of the hardcoded `CANONICAL_SYNAPSE_NODES` const with
//! a `PeerResolver` port implementation. The desired topology comes from a
//! declared registry seed — today `infra/synapse-nodes.seed.json` (a versioned,
//! signed artifact), falling back to the prior const when the file is absent.
//!
//! Resilience (design B: dynamic mesh + signed fallback):
//!   * `from_repo_infra` reads the seed file; if it is missing/unparseable the
//!     resolver degrades to the declared const (fail-soft, not fail-crash).
//!   * `fallback_peers` reads the rolling signed `known-peers.vN.<ts>.json`
//!     bundle and verifies its ed25519 signature against the machine-peer
//!     public key before trusting it. Invalid/missing signature => empty
//!     fallback (fail-closed; never silently trust an unsigned peer list).
//!
//! Observed reality (mesh-live + OS-substrate health) is folded in via the pure
//! `select_peer` / `select_all_peers` helpers in `cynic-protocol`.

use std::path::Path;

use ed25519_dalek::{Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};

use cynic_protocol::peer_resolver::{
    PeerEndpoint, PeerResolver, PeerTransport, SignedPeerEndpoint,
};
use cynic_protocol::build_node::NodeHealthView;

/// Declared seed node as stored in `infra/synapse-nodes.seed.json`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct SeedSynapseNode {
    pub id: String,
    pub role: String,
    pub ip: String,
    pub probe_ports: Vec<u16>,
}

#[derive(Debug, Deserialize)]
struct SeedFile {
    #[allow(dead_code)]
    schema: String,
    nodes: Vec<SeedSynapseNode>,
}

/// Signed rolling fallback bundle (`infra/known-peers.<ts>.json`).
#[derive(Debug, Deserialize, Serialize)]
struct SignedBundle {
    #[allow(dead_code)]
    schema: String,
    #[allow(dead_code)]
    signed_by: String,
    signature: String,
    nodes: Vec<SeedSynapseNode>,
}

fn transport_for(role: &str, ip: &str) -> PeerTransport {
    if role == "cortex_primary" || ip.starts_with("127.") {
        PeerTransport::Uds
    } else if role == "storage_primary" {
        PeerTransport::Ws
    } else {
        PeerTransport::Http
    }
}

/// Resolver backed by a local seed (registry / declared nodes) plus an optional
/// signed rolling fallback.
pub struct LocalRegistryMeshResolver {
    seed: Vec<SeedSynapseNode>,
    /// Directory holding `known-peers.vN.<ts>.json` + `known-peers.pub`.
    fallback_dir: Option<std::path::PathBuf>,
}

impl LocalRegistryMeshResolver {
    pub fn from_seed(seed: Vec<SeedSynapseNode>) -> Self {
        Self {
            seed,
            fallback_dir: None,
        }
    }

    pub fn from_declared(
        nodes: &[crate::usecases::federated_mesh_probe_uc::DeclaredSynapseNode],
    ) -> Self {
        let seed = nodes
            .iter()
            .map(|n| SeedSynapseNode {
                id: n.id.to_string(),
                role: n.role.to_string(),
                ip: n.ip.to_string(),
                probe_ports: n.probe_ports.to_vec(),
            })
            .collect();
        Self::from_seed(seed)
    }

    /// Build the resolver from `infra/`: read `synapse-nodes.seed.json`; if it
    /// is missing or unparseable, degrade to the declared const. The fallback
    /// directory is set so `fallback_peers` can load the signed bundle.
    pub fn from_repo_infra(
        infra_dir: &Path,
        declared: &[crate::usecases::federated_mesh_probe_uc::DeclaredSynapseNode],
    ) -> Self {
        let seed = read_seed_file(infra_dir).unwrap_or_else(|| {
            declared
                .iter()
                .map(|n| SeedSynapseNode {
                    id: n.id.to_string(),
                    role: n.role.to_string(),
                    ip: n.ip.to_string(),
                    probe_ports: n.probe_ports.to_vec(),
                })
                .collect()
        });
        let fallback_dir = if infra_dir.exists() {
            Some(infra_dir.to_path_buf())
        } else {
            None
        };
        Self {
            seed,
            fallback_dir,
        }
    }

    /// Convenience: resolve `infra/` via `repo_infra_dir()` and build the
    /// resolver. Callers that just want the repo-default topology use this.
    pub fn from_repo_infra_default(
        declared: &[crate::usecases::federated_mesh_probe_uc::DeclaredSynapseNode],
    ) -> Self {
        Self::from_repo_infra(&Self::repo_infra_dir(), declared)
    }

    fn to_endpoint(&self, node: &SeedSynapseNode) -> PeerEndpoint {
        PeerEndpoint {
            node_id: node.id.clone(),
            role: node.role.clone(),
            transport: transport_for(&node.role, &node.ip),
            // Seed is the *desired* topology; addr carries the declared IP so
            // the probe/observer can confirm reachability.
            addr: node.ip.clone(),
            probe_ports: node.probe_ports.clone(),
            // Desired-until-observed: a seed node is assumed reachable until a
            // mesh probe or OS-health observation proves otherwise.
            reachable: true,
            health: NodeHealthView::default(),
        }
    }
}

impl PeerResolver for LocalRegistryMeshResolver {
    fn resolve_peers(&self, role: &str) -> Vec<PeerEndpoint> {
        self.seed
            .iter()
            .filter(|n| n.role == role)
            .map(|n| self.to_endpoint(n))
            .collect()
    }

    fn resolve_all_peers(&self) -> Vec<PeerEndpoint> {
        self.seed.iter().map(|n| self.to_endpoint(n)).collect()
    }

    fn fallback_peers(&self, role: &str) -> Vec<SignedPeerEndpoint> {
        let Some(dir) = &self.fallback_dir else {
            return vec![];
        };
        // Locate the most recent known-peers.<ts>.json signed bundle.
        let Ok(readdir) = std::fs::read_dir(dir) else {
            return vec![];
        };
        let mut bundles: Vec<(u64, std::path::PathBuf)> = readdir
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let name = e.file_name().to_string_lossy().to_string();
                // known-peers.<ts>.json
                let ts = name
                    .strip_prefix("known-peers.")
                    .and_then(|s| s.strip_suffix(".json"))
                    .and_then(|s| s.parse::<u64>().ok())?;
                Some((ts, e.path()))
            })
            .collect();
        bundles.sort_by_key(|(ts, _)| *ts);
        let Some((_, path)) = bundles.into_iter().last() else {
            return vec![];
        };

        // Public key for verification (non-secret, lives next to the bundle).
        let pub_path = dir.join("known-peers.pub");
        let Ok(pub_hex) = std::fs::read_to_string(&pub_path) else {
            return vec![];
        };
        let Ok(pub_bytes) = hex_decode(&pub_hex.trim()) else {
            return vec![];
        };
        let pub_arr: [u8; 32] = match pub_bytes.as_slice().try_into() {
            Ok(a) => a,
            Err(_) => return vec![],
        };
        let Ok(verifying) = VerifyingKey::from_bytes(&pub_arr) else {
            return vec![];
        };

        let Ok(raw) = std::fs::read_to_string(&path) else {
            return vec![];
        };
        let Ok(bundle) = serde_json::from_str::<SignedBundle>(&raw) else {
            return vec![];
        };
        let Ok(sig_bytes) = hex_decode(&bundle.signature) else {
            return vec![];
        };
        let sig_arr: [u8; 64] = match sig_bytes.as_slice().try_into() {
            Ok(a) => a,
            Err(_) => return vec![],
        };
        let Ok(signature) = ed25519_dalek::Signature::from_slice(&sig_arr) else {
            return vec![];
        };
        // Reconstruct the canonical signed payload (all fields except the
        // signature) and verify. This is robust to file formatting.
        let payload = SignedBundle {
            schema: bundle.schema.clone(),
            signed_by: bundle.signed_by.clone(),
            signature: String::new(),
            nodes: bundle.nodes.clone(),
        };
        let Ok(payload_bytes) = serde_json::to_vec(&payload) else {
            return vec![];
        };
        if verifying.verify(&payload_bytes, &signature).is_err() {
            // Invalid signature: fail-closed, do not trust the fallback.
            return vec![];
        }

        bundle
            .nodes
            .into_iter()
            .filter(|n| n.role == role)
            .map(|n| SignedPeerEndpoint {
                endpoint: PeerEndpoint {
                    node_id: n.id.clone(),
                    role: n.role.clone(),
                    transport: transport_for(&n.role, &n.ip),
                    addr: n.ip.clone(),
                    probe_ports: n.probe_ports.clone(),
                    reachable: true,
                    health: NodeHealthView::default(),
                },
                signature_ed25519: bundle.signature.clone(),
                generated_at_unix_s: 0,
                version: 1,
            })
            .collect()
    }
}

impl LocalRegistryMeshResolver {
    /// Resolve the repo `infra/` directory at runtime. Builds from the crate's
    /// manifest dir (crates/cynic-node => ../../infra) and falls back to a cwd
    /// relative `infra/` so the resolver finds the seed + signed fallback even
    /// when invoked from a different working directory.
    pub fn repo_infra_dir() -> std::path::PathBuf {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let from_manifest = std::path::Path::new(&manifest).join("../../infra");
        if from_manifest.exists() {
            from_manifest
        } else {
            std::path::PathBuf::from("infra")
        }
    }

    /// All seed peers (every declared role), for mesh-wide liveness probing.
    pub fn resolve_all_peers(&self) -> Vec<PeerEndpoint> {
        self.seed.iter().map(|n| self.to_endpoint(n)).collect()
    }
}

/// Helper: pick the highest-priority reachable, healthy peer for `role`.
/// Returns `None` when no candidate is dispatchable — caller must then emit a
/// `TopologyDriftSignal` (drift is data, not a crash).
pub fn select_peer_for_role(
    resolver: &LocalRegistryMeshResolver,
    role: &str,
) -> Option<PeerEndpoint> {
    cynic_protocol::peer_resolver::select_peer(&resolver.fallback_peers(role), role)
}

/// Helper: all reachable, healthy seed peers, ordered by role priority.
pub fn select_all_healthy(resolver: &LocalRegistryMeshResolver) -> Vec<PeerEndpoint> {
    cynic_protocol::peer_resolver::select_all_peers(&resolver.resolve_all_peers())
}

/// Read the declared seed from `infra/synapse-nodes.seed.json`.
fn read_seed_file(infra_dir: &Path) -> Option<Vec<SeedSynapseNode>> {
    let path = infra_dir.join("synapse-nodes.seed.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    let parsed: SeedFile = serde_json::from_str(&raw).ok()?;
    Some(parsed.nodes)
}

fn hex_decode(s: &str) -> Result<Vec<u8>, std::num::ParseIntError> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err("invalid hex length".parse::<u8>().unwrap_err());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16))
        .collect()
}
