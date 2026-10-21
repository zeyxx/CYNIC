//! Reading the declared fleet topology from the registry.
//!
//! The registry (`infra/registry.json`) is the single source of truth for
//! the fleet (AGENTS.md authority order). This module is the only reader of
//! that authority for mesh peer resolution: it projects each declared node
//! into the shape the resolver consumes.
//!
//! It is a separate module because reading the registry and resolving
//! peers are two responsibilities: the reader knows the registry's shape
//! and how a declared node becomes a peer candidate, and the resolver
//! knows what to do with the candidates.

use std::path::Path;

use serde::Deserialize;

use crate::usecases::federated_mesh_resolver::DeclaredNode;

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
pub(crate) fn read_registry_nodes(infra_dir: &Path) -> Option<Vec<DeclaredNode>> {
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
pub(crate) struct RegistryFile {
    #[allow(dead_code)]
    schema: String,
    #[serde(default)]
    network_policy: RegistryNetworkPolicy,
    nodes: Vec<RegistryNode>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RegistryNetworkPolicy {
    #[serde(default)]
    planes: RegistryPlanes,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RegistryPlanes {
    #[serde(default)]
    service: Option<RegistryPlane>,
    /// Legacy registry key for the service plane. Read so a runtime registry
    /// that still declares the old spelling keeps resolving; the registry in
    /// this repo declares the canonical `service` key only.
    #[serde(default, rename = "synapse")]
    legacy_synapse: Option<RegistryPlane>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegistryPlane {
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
pub(crate) struct RegistryNode {
    id: String,
    #[serde(default)]
    role: String,
    #[serde(default)]
    network: RegistryNodeNetwork,
    #[serde(default)]
    services: Vec<RegistryNodeService>,
}

#[derive(Debug, Default, Deserialize)]
pub(crate) struct RegistryNodeNetwork {
    #[serde(default)]
    service: Option<String>,
    /// Legacy registry key for the service plane. Read so a runtime registry
    /// that still declares the old spelling keeps resolving; the registry in
    /// this repo declares the canonical `service` key only.
    #[serde(default, rename = "synapse")]
    legacy_synapse: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct RegistryNodeService {
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
