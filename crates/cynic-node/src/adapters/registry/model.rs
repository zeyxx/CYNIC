use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct Registry {
    pub(super) network_policy: Option<RegistryNetworkPolicy>,
    pub(super) nodes: Option<Vec<RegistryNode>>,
    pub(super) organs: Option<Vec<RegistryOrgan>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryNetworkPolicy {
    pub(super) canonical_service_plane: Option<String>,
    pub(super) planes: Option<RegistryPlanes>,
    pub(super) reservations: Option<Vec<RegistryReservation>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryPlanes {
    #[serde(alias = "synapse")]
    pub(super) service: Option<RegistryPlane>,
    #[serde(alias = "management")]
    pub(super) admin: Option<RegistryPlane>,
    #[serde(alias = "overlay")]
    pub(super) mesh: Option<RegistryPlane>,
    pub(super) proxmox_guest: Option<RegistryPlane>,
}

impl RegistryPlanes {
    /// Resolve a plane by canonical name.
    ///
    /// This is the only read path for plane data: it keeps every consumer off
    /// the individual fields, so adding a plane cannot silently produce a
    /// declared-but-never-read field. `proxmox_guest` is a lab-topology plane,
    /// not a universal one, so it stays resolvable but out of the canonical trio.
    pub(super) fn get(&self, plane: &str) -> Option<&RegistryPlane> {
        match plane {
            "service" => self.service.as_ref(),
            "admin" => self.admin.as_ref(),
            "mesh" => self.mesh.as_ref(),
            "proxmox_guest" => self.proxmox_guest.as_ref(),
            _ => None,
        }
    }

    /// Declared plane CIDRs, canonical order. Exposed as observed state so a
    /// plane that is declared in the registry is never invisible to consumers.
    pub(super) fn declared_plane_cidrs(&self) -> BTreeMap<String, String> {
        self.declared_planes()
            .into_iter()
            .filter_map(|plane| {
                let cidr = self.get(plane)?.cidr.clone().filter(|c| !c.is_empty())?;
                Some((plane.to_string(), cidr))
            })
            .collect()
    }

    /// Canonical names of every declared plane, in canonical order.
    pub(super) fn declared_planes(&self) -> Vec<&'static str> {
        [
            ("service", self.service.is_some()),
            ("admin", self.admin.is_some()),
            ("mesh", self.mesh.is_some()),
            ("proxmox_guest", self.proxmox_guest.is_some()),
        ]
        .into_iter()
        .filter(|(_, present)| *present)
        .map(|(name, _)| name)
        .collect()
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryPlane {
    pub(super) cidr: Option<String>,
    pub(super) dns_zone: Option<String>,
    pub(super) canonical: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryReservation {
    pub(super) name: Option<String>,
    #[serde(alias = "synapse")]
    pub(super) service: Option<String>,
    #[serde(alias = "management")]
    pub(super) admin: Option<String>,
    #[serde(alias = "overlay")]
    pub(super) mesh: Option<String>,
    pub(super) dns: Option<Vec<String>>,
    #[serde(default)]
    pub(super) addresses: BTreeMap<String, String>,
}

impl RegistryReservation {
    /// Address reserved on `plane`, canonical plane name.
    pub(super) fn address_on(&self, plane: &str) -> Option<&str> {
        let value = match plane {
            "service" => self.service.as_deref(),
            "admin" => self.admin.as_deref(),
            "mesh" => self.mesh.as_deref(),
            _ => None,
        };
        value.filter(|value| !value.is_empty())
    }

    /// Address on the first plane that reserves one, canonical order.
    pub(super) fn best_address(&self) -> Option<&str> {
        ["service", "admin", "mesh"]
            .into_iter()
            .find_map(|plane| self.address_on(plane))
            .or_else(|| {
                self.addresses
                    .values()
                    .find(|value| !value.is_empty())
                    .map(String::as_str)
            })
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryNode {
    pub(super) name: Option<String>,
    pub(super) id: Option<String>,
    pub(super) role: Option<String>,
    #[serde(default)]
    pub(super) roles: Option<Vec<String>>,
    pub(super) status: Option<String>,
    pub(super) constellation: Option<String>,
    pub(super) aliases: Option<Vec<String>>,
    pub(super) network: Option<RegistryNodeNetwork>,
    pub(super) services: Option<Vec<RegistryService>>,
    pub(super) vms: Option<Vec<RegistryVm>>,
    pub(super) proxmox_id: Option<String>,
    pub(super) proxmox: Option<RegistryProxmox>,
    pub(super) source_workspace_layout: Option<RegistrySourceWorkspaceLayout>,
    #[serde(default)]
    pub(super) energy_profile: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(super) struct RegistrySourceWorkspaceLayout {
    pub(super) repository_store: String,
    pub(super) workspace_store: String,
    pub(super) principal: String,
    #[serde(default)]
    pub(super) dedicated_worktrees: bool,
    #[serde(default)]
    pub(super) repositories: Vec<RegistrySourceRepository>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub(super) struct RegistrySourceRepository {
    pub(super) id: String,
    pub(super) relative_path: String,
    pub(super) transport_uri: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryVm {
    pub(super) name: Option<String>,
    pub(super) aliases: Option<Vec<String>>,
    pub(super) id: Option<String>,
    pub(super) ip: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryNodeNetwork {
    /// Canonical service plane address (legacy alias: `synapse`).
    #[serde(alias = "synapse")]
    pub(super) service: Option<String>,
    /// Canonical admin plane address (legacy alias: `management`).
    #[serde(alias = "management")]
    pub(super) admin: Option<String>,
    /// Canonical mesh overlay address (legacy alias: `overlay`).
    #[serde(alias = "overlay")]
    pub(super) mesh: Option<String>,
    /// Underlay / L2 adjacency address of the node itself. Not a plane.
    pub(super) local: Option<String>,
    pub(super) tailscale: Option<String>,
    pub(super) public_ingress: Option<String>,
}

impl RegistryNodeNetwork {
    /// Address on `plane`, canonical plane name.
    ///
    /// `local` is deliberately not a plane (RFC-NET-001): it is the node's own
    /// underlay / L2-adjacency address, reachable through `best_ip` only.
    pub(super) fn address_on(&self, plane: &str) -> Option<&str> {
        let value = match plane {
            "service" => self.service.as_deref(),
            "admin" => self.admin.as_deref(),
            "mesh" => self.mesh.as_deref(),
            _ => None,
        };
        value.filter(|value| !value.is_empty())
    }

    /// The node's own underlay / L2-adjacency address. Not a plane.
    pub(super) fn underlay(&self) -> Option<&str> {
        self.local.as_deref().filter(|value| !value.is_empty())
    }

    /// Best address for L2-adjacent (same-switch) contact: underlay first, then
    /// the service plane, then admin. Mesh is excluded on purpose — it is not
    /// reachable without a tunnel.
    pub(super) fn best_ip(&self) -> Option<&str> {
        self.underlay()
            .or_else(|| self.address_on("service"))
            .or_else(|| self.address_on("admin"))
            .filter(|value| *value != "dhcp")
    }

    /// Address reachable from outside the fleet, if any.
    pub(super) fn ingress_ip(&self) -> Option<&str> {
        self.public_ingress
            .as_deref()
            .or(self.tailscale.as_deref())
            .filter(|value| !value.is_empty())
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryProxmox {
    pub(super) id: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryService {
    pub(super) name: Option<String>,
    pub(super) port: Option<u16>,
    pub(super) status: Option<String>,
    pub(super) optional: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub(super) struct RegistryOrgan {
    pub(super) name: Option<String>,
}
