//! Local registry-backed mesh resolver.
//!
//! The desired topology comes from `infra/registry.json`, the single source
//! of truth for the fleet (AGENTS.md authority order). It used to come from a
//! separate `service-nodes.seed.json` (deleted) (now deleted): a second declarative copy
//! topology, declaring 5 nodes where the registry declares 12, and disagreeing
//! with it on `local-kernel`. Two sources with no comparison is a competing
//! authority, so that seed is deleted and the registry is read directly.
//!
//! Resilience (design B: dynamic mesh + signed fallback):
//!   * `from_repo_infra` reads the registry; if it is missing/unparseable the
//!     resolver yields zero declared nodes (the caller reports `isolated` —
//!     an explicit unknown, not a duplicated answer).
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

use cynic_build::build_node::NodeHealthView;
use cynic_node_protocol::peer_resolver::{
    PeerEndpoint, PeerResolver, PeerTransport, SignedPeerEndpoint,
};

/// Declared node, projected from the registry into the shape the resolver
/// consumes.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub struct DeclaredNode {
    pub id: String,
    pub role: String,
    pub ip: String,
    pub probe_ports: Vec<u16>,
}

/// Signed rolling fallback bundle (`infra/known-peers.<ts>.json`).
#[derive(Debug, Deserialize, Serialize)]
struct SignedBundle {
    #[allow(dead_code)]
    schema: String,
    #[allow(dead_code)]
    signed_by: String,
    signature: String,
    nodes: Vec<DeclaredNode>,
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

/// Resolver backed by the declared registry topology plus an optional
/// signed rolling fallback.
pub struct LocalRegistryMeshResolver {
    declared: Vec<DeclaredNode>,
    /// Directory holding `known-peers.vN.<ts>.json` + `known-peers.pub`.
    fallback_dir: Option<std::path::PathBuf>,
}

impl LocalRegistryMeshResolver {
    pub fn from_declared(declared: Vec<DeclaredNode>) -> Self {
        Self {
            declared,
            fallback_dir: None,
        }
    }

    /// Build the resolver from `infra/`: read `registry.json`.
    ///
    /// The registry is the authority (AGENTS.md authority order). It used to
    /// read a separate `service-nodes.seed.json` (deleted) (now deleted), a second copy of
    /// the same topology: the seed declared 5 nodes with `local-kernel` at
    /// `127.0.0.1`/`cortex_primary` while the registry declares
    /// `10.10.10.100`/`kernel_primary`, and the registry knows 12 nodes. Two
    /// sources, no comparison, both plausible — the definition of a competing
    /// authority. The seed is deleted; the registry is read directly.
    ///
    /// A missing or unparseable registry yields an empty topology and the
    /// caller reports `isolated` — an explicit unknown, not a duplicated
    /// answer.
    ///
    /// The fallback directory is set so `fallback_peers` can load the signed
    /// rolling bundle.
    pub fn from_repo_infra(infra_dir: &Path) -> Self {
        let declared = read_registry_nodes(infra_dir).unwrap_or_default();
        let fallback_dir = if infra_dir.exists() {
            Some(infra_dir.to_path_buf())
        } else {
            None
        };
        Self { declared, fallback_dir }
    }

    /// Convenience: resolve `infra/` via `repo_infra_dir()` and build the
    /// resolver. Callers that just want the repo-default topology use this.
    pub fn from_repo_infra_default() -> Self {
        Self::from_repo_infra(&Self::repo_infra_dir())
    }

    fn to_endpoint(&self, node: &DeclaredNode) -> PeerEndpoint {
        PeerEndpoint {
            node_id: node.id.clone(),
            role: node.role.clone(),
            transport: transport_for(&node.role, &node.ip),
            // The registry is the *desired* topology; addr carries the
            // declared IP so the probe/observer can confirm reachability.
            addr: node.ip.clone(),
            probe_ports: node.probe_ports.clone(),
            // Desired-until-observed: a declared node is assumed reachable until a
            // mesh probe or OS-health observation proves otherwise.
            reachable: true,
            health: NodeHealthView {
                critical_active: false,
                warning_active: false,
                active_osha_codes: vec![],
                available_mem_mib: 0,
            },
        }
    }
}

impl PeerResolver for LocalRegistryMeshResolver {
    fn resolve_peers(&self, role: &str) -> Vec<PeerEndpoint> {
        self.declared
            .iter()
            .filter(|n| n.role == role)
            .map(|n| self.to_endpoint(n))
            .collect()
    }

    fn resolve_all_peers(&self) -> Vec<PeerEndpoint> {
        self.declared.iter().map(|n| self.to_endpoint(n)).collect()
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
        let Some((ts, path)) = bundles.into_iter().last() else {
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
            .map(|n| {
                let endpoint = PeerEndpoint {
                    node_id: n.id.clone(),
                    role: n.role.clone(),
                    transport: transport_for(&n.role, &n.ip),
                    addr: n.ip.clone(),
                    probe_ports: n.probe_ports.clone(),
                    reachable: true,
                    health: NodeHealthView {
                        critical_active: false,
                        warning_active: false,
                        active_osha_codes: vec![],
                        available_mem_mib: 0,
                    },
                };
                // The signature verified above, so admitting the wrapper is
                // justified. `generated_at_unix_s` is the bundle timestamp we
                // already sorted on; it used to be hardcoded to 0, which threw
                // away the only freshness datum a fallback peer carries and left
                // no way to tell a rolling bundle from a stale one.
                SignedPeerEndpoint::from_verified(endpoint, bundle.signature.clone(), ts, 1)
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap_or_default()
    }
}

impl LocalRegistryMeshResolver {
    /// Resolve the repo `infra/` directory at runtime. Builds from the crate's
    /// manifest dir (crates/cynic-node => ../../infra) and falls back to a cwd
    /// relative `infra/` so the resolver finds the registry + signed fallback
    /// even when invoked from a different working directory.
    pub fn repo_infra_dir() -> std::path::PathBuf {
        let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
        let from_manifest = std::path::Path::new(&manifest).join("../../infra");
        if from_manifest.exists() {
            from_manifest
        } else {
            std::path::PathBuf::from("infra")
        }
    }

    /// All declared peers (every declared role), for mesh-wide liveness probing.
    pub fn resolve_all_peers(&self) -> Vec<PeerEndpoint> {
        self.declared.iter().map(|n| self.to_endpoint(n)).collect()
    }
}

/// Helper: pick the highest-priority reachable, healthy peer for `role`.
///
/// Returns the *signed* endpoint: `fallback_peers` is fail-closed and only
/// yields peers whose ed25519 signature verified, so the wrapper is the
/// authentication evidence. Callers that only need the address read
/// `.endpoint`; keeping the wrapper means a verified fallback peer stays
/// distinguishable from an unverified declared entry.
///
/// Returns `None` when no candidate is dispatchable — caller must then emit a
/// `TopologyDriftSignal` (drift is data, not a crash).
pub fn select_peer_for_role(
    resolver: &LocalRegistryMeshResolver,
    role: &str,
) -> Option<cynic_node_protocol::peer_resolver::SignedPeerEndpoint> {
    cynic_node_protocol::peer_resolver::select_signed_peer(&resolver.fallback_peers(role), role)
}

/// Helper: all reachable, healthy declared peers, ordered by role priority.
pub fn select_all_healthy(resolver: &LocalRegistryMeshResolver) -> Vec<PeerEndpoint> {
    cynic_node_protocol::peer_resolver::select_all_peers(&resolver.resolve_all_peers())
}

/// Read the declared fleet topology from `infra/registry.json`.
///
/// The registry is the single source of truth for the fleet (AGENTS.md
/// authority order). This function is the only reader of that authority for
/// mesh peer resolution: it projects each declared node into the declared shape
/// the resolver consumes, deriving the service-plane address from the
/// canonical `service` key and the probe ports from the node's declared
/// services.
///
/// A node without a service-plane address is skipped: an address-less node
/// cannot be probed, and emitting it would produce a peer that always fails
/// its probe for a reason the registry never declared.
fn read_registry_nodes(infra_dir: &Path) -> Option<Vec<DeclaredNode>> {
    let path = infra_dir.join("registry.json");
    let raw = std::fs::read_to_string(&path).ok()?;
    let parsed: RegistryFile = serde_json::from_str(&raw).ok()?;
    let service_cidr = parsed.service_plane_cidr();
    let nodes = parsed
        .nodes
        .into_iter()
        .filter_map(|node| {
            let ip = node.service_address()?;
            // The address must belong to the plane the registry declares for
            // it. The registry declares `cynic-compute-01` under both `lan`
            // and `service` with the same `192.168.0.25` address, while the
            // service plane CIDR is `10.10.10.0/24`: that address is not on
            // the service plane, so the node is not a service-plane peer.
            if !service_cidr
                .as_deref()
                .map_or(true, |cidr| {
                    cynic_network::plane_vocabulary::address_in_cidr(&ip, cidr)
                })
            {
                return None;
            }
            let probe_ports = node.probe_ports();
            Some(DeclaredNode {
                id: node.id,
                role: node.role,
                ip,
                probe_ports,
            })
        })
        .collect();
    Some(nodes)
}

/// The subset of the registry this reader needs.
#[derive(Debug, Deserialize)]
struct RegistryFile {
    #[allow(dead_code)]
    schema: String,
    #[serde(default)]
    network_policy: RegistryNetworkPolicy,
    nodes: Vec<RegistryNode>,
}

#[derive(Debug, Default, Deserialize)]
struct RegistryNetworkPolicy {
    #[serde(default)]
    planes: RegistryPlanes,
}

#[derive(Debug, Default, Deserialize)]
struct RegistryPlanes {
    #[serde(default)]
    service: Option<RegistryPlane>,
    /// Legacy registry key for the service plane. Read so a runtime registry
    /// that still declares the old spelling keeps resolving; the registry in
    /// this repo declares the canonical `service` key only.
    #[serde(default, rename = "synapse")]
    legacy_synapse: Option<RegistryPlane>,
}

#[derive(Debug, Deserialize)]
struct RegistryPlane {
    #[serde(default)]
    cidr: Option<String>,
}

impl RegistryFile {
    /// CIDR the registry declares for the service plane, if any.
    fn service_plane_cidr(&self) -> Option<String> {
        self.network_policy
            .planes
            .service
            .as_ref()
            .or(self.network_policy.planes.legacy_synapse.as_ref())
            .and_then(|plane| plane.cidr.as_deref())
            .map(str::trim)
            .filter(|cidr| !cidr.is_empty())
            .map(str::to_string)
    }
}

/// One declared registry node, as this reader sees it.
#[derive(Debug, Deserialize)]
struct RegistryNode {
    id: String,
    #[serde(default)]
    role: String,
    #[serde(default)]
    network: RegistryNodeNetwork,
    #[serde(default)]
    services: Vec<RegistryNodeService>,
}

#[derive(Debug, Default, Deserialize)]
struct RegistryNodeNetwork {
    #[serde(default)]
    service: Option<String>,
    /// Legacy registry key for the service plane. Read so a runtime registry
    /// that still declares the old spelling keeps resolving; the registry in
    /// this repo declares the canonical `service` key only.
    #[serde(default, rename = "synapse")]
    legacy_synapse: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RegistryNodeService {
    #[serde(default)]
    port: Option<u16>,
}

impl RegistryNode {
    /// Service-plane address, canonical key first, legacy key as fallback.
    fn service_address(&self) -> Option<String> {
        self.network
            .service
            .as_deref()
            .or(self.network.legacy_synapse.as_deref())
            .map(str::trim)
            .filter(|address| !address.is_empty())
            .map(str::to_string)
    }

    /// Ports to probe, from the node's declared services.
    ///
    /// The registry declares services, not probe ports, so the service ports
    /// are the probe targets. SSH (22) is not derived: the registry models
    /// the SSH transport as `transports.ssh`, not as a service, so adding it
    /// here would invent a port the registry never declared.
    fn probe_ports(&self) -> Vec<u16> {
        self.services
            .iter()
            .filter_map(|service| service.port)
            .collect()
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn registry(nodes: &str) -> String {
        registry_with_cidr(nodes, "10.10.10.0/24")
    }

    fn registry_with_cidr(nodes: &str, cidr: &str) -> String {
        format!(
            r#"{{"schema":"cynic.registry.v1.minimal",
                "network_policy":{{"planes":{{"service":{{"cidr":"{cidr}"}}}}}},
                "nodes":[{nodes}]}}"#,
            nodes = nodes,
            cidr = cidr
        )
    }

    /// The seed this reader replaced declared 5 nodes and disagreed with the
    /// registry on `local-kernel` (`127.0.0.1`/`cortex_primary` in the seed,
    /// `10.10.10.100`/`kernel_primary` in the registry). The registry is the
    /// authority, so the reader must take the registry's word for both.
    #[test]
    fn the_registry_is_the_authority_for_the_declared_topology() {
        let dir = std::env::temp_dir().join("cynic-mesh-resolver-authority");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(
            dir.join("registry.json"),
            registry(
                r#"{"id":"local-kernel","role":"kernel_primary",
                    "network":{"service":"10.10.10.100"},
                    "services":[{"name":"cynic-kernel","port":3030}]}"#,
            ),
        )
        .expect("write registry");

        let nodes = read_registry_nodes(&dir).expect("registry is read");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].id, "local-kernel");
        assert_eq!(nodes[0].role, "kernel_primary");
        assert_eq!(nodes[0].ip, "10.10.10.100");

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A node whose declared service-plane address falls outside the CIDR the
    /// registry declares for that plane is not a service-plane peer.
    /// `cynic-compute-01` is exactly that case in the live registry: it
    /// declares `192.168.0.25` under both `lan` and `service` while the
    /// service plane is `10.10.10.0/24`. Admitting it would produce a peer
    /// that always fails its probe for a reason the registry never declared.
    #[test]
    fn a_node_outside_the_service_plane_cidr_is_not_admitted() {
        let dir = std::env::temp_dir().join("cynic-mesh-resolver-plane");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(
            dir.join("registry.json"),
            registry(
                r#"{"id":"cynic-compute-01","role":"embedding_runner",
                    "network":{"lan":"192.168.0.25","service":"192.168.0.25"},
                    "services":[{"name":"embedding-server","port":8080}]}"#,
            ),
        )
        .expect("write registry");

        let nodes = read_registry_nodes(&dir).expect("registry is read");
        assert!(
            nodes.is_empty(),
            "an address outside the service-plane CIDR must not enter the mesh: {nodes:?}"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The legacy registry key is still read so a runtime registry that
    /// declares it keeps resolving. The registry in this repo declares the
    /// canonical `service` key only; the alias is a read path, not a writer.
    #[test]
    fn the_legacy_service_plane_key_is_still_read() {
        let dir = std::env::temp_dir().join("cynic-mesh-resolver-legacy");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(
            dir.join("registry.json"),
            registry(
                r#"{"id":"cynic-builder","role":"build_runner",
                    "network":{"synapse":"10.10.10.119"},
                    "services":[{"name":"cynic-node","port":9090}]}"#,
            ),
        )
        .expect("write registry");

        let nodes = read_registry_nodes(&dir).expect("registry is read");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].ip, "10.10.10.119");
        assert_eq!(nodes[0].probe_ports, vec![9090]);

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A node whose address is inside the plane CIDR is admitted even when it
    /// declares no service port: the address is the registry's declaration of
    /// the node on that plane, and the probe then reports it unreachable
    /// because there is no port to reach. Dropping it from the topology would
    /// hide a declared node instead of reporting an unproven one.
    #[test]
    fn a_node_inside_the_plane_cidr_is_admitted_even_without_a_port() {
        let dir = std::env::temp_dir().join("cynic-mesh-resolver-noport");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(
            dir.join("registry.json"),
            registry(
                r#"{"id":"cynic-host-proxmox","role":"hypervisor",
                    "network":{"service":"10.10.10.1"},"services":[]}"#,
            ),
        )
        .expect("write registry");

        let nodes = read_registry_nodes(&dir).expect("registry is read");
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].ip, "10.10.10.1");
        assert!(nodes[0].probe_ports.is_empty());

        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A registry that declares no service-plane CIDR admits every node with a
    /// service address: the plane is undeclared, so nothing can be excluded on
    /// plane grounds. This is the "unknown, not a guess" path.
    #[test]
    fn a_registry_without_a_plane_cidr_admits_every_addressed_node() {
        let dir = std::env::temp_dir().join("cynic-mesh-resolver-nocidr");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("temp dir");
        std::fs::write(
            dir.join("registry.json"),
            r#"{"schema":"cynic.registry.v1.minimal",
                "nodes":[{"id":"cynic-compute-01","role":"embedding_runner",
                    "network":{"service":"192.168.0.25"},"services":[]}]}"#,
        )
        .expect("write registry");

        let nodes = read_registry_nodes(&dir).expect("registry is read");
        assert_eq!(nodes.len(), 1);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
