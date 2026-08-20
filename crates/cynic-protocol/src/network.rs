use serde::{Deserialize, Serialize};

pub const NETWORK_MESH_PLAN_SCHEMA: &str = "cynic.network_mesh_plan.v1";
pub const MESH_ROLE_MAP_SCHEMA: &str = "cynic.mesh_role_map.v1";

/// What a mesh provider is actually responsible for.
///
/// These are separate concerns and one provider does not imply the other:
/// carrying packets is not the same job as deciding who may join. Conflating
/// them is how a coordination provider ends up treated as network truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeshRole {
    /// Moves packets between nodes. Owns interfaces, keys, routes.
    Transport,
    /// Decides membership: enrolment, key distribution, revocation, discovery.
    /// Holds no packet path of its own.
    Coordination,
}

/// Where a provider's control plane runs, and therefore who can revoke it.
///
/// This is the distinction that decides whether a provider may be kept at all.
/// `Sovereign` means the control plane runs on hardware the operator owns.
/// `Saas` means a third party can revoke, rate-limit, observe, or price the
/// organism's ability to form its own network — which is disqualifying for a
/// runtime dependency regardless of how convenient the client is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MeshSovereignty {
    /// Control plane on operator-owned substrate (WireGuard keys, self-hosted
    /// Headscale). Failure is the operator's to repair.
    Sovereign,
    /// Third-party-hosted control plane. Disqualified as a runtime dependency.
    Saas,
    /// No control plane: static declarative configuration only.
    None,
}

/// A provider CYNIC knows about, with its role and sovereignty pinned.
///
/// The CLI advertises `wireguard|headscale|tailscale|static` as a bare string
/// with no type behind it, so nothing prevented a SaaS control plane from being
/// selected as if it were equivalent to a sovereign one. This table is the
/// missing constraint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MeshProviderBinding {
    pub provider: String,
    pub role: MeshRole,
    pub sovereignty: MeshSovereignty,
    /// True when this provider is the declared owner of its role today.
    pub canonical_for_role: bool,
    /// Why this provider is admitted, retained pending work, or refused.
    pub rationale: &'static str,
}

/// Every mesh provider CYNIC recognises, and its standing.
///
/// Observed on this fleet 2026-08-20: `wg0` is up on the hypervisor with four
/// live peers; no tailscale interface exists on any node and `tailscaled` is
/// inactive. Headscale is running on LXC 100 and answers on :8080 with zero
/// clients — it is retained for the coordination role, which has no other
/// owner, not for transport.
pub fn mesh_provider_bindings() -> Vec<MeshProviderBinding> {
    vec![
        MeshProviderBinding {
            provider: "wireguard".to_string(),
            role: MeshRole::Transport,
            sovereignty: MeshSovereignty::Sovereign,
            canonical_for_role: true,
            rationale: "keys and interfaces live on operator substrate; no external control plane",
        },
        MeshProviderBinding {
            provider: "headscale".to_string(),
            role: MeshRole::Coordination,
            sovereignty: MeshSovereignty::Sovereign,
            canonical_for_role: true,
            rationale: "self-hosted control plane; owns enrolment and revocation, never the packet path",
        },
        MeshProviderBinding {
            provider: "tailscale".to_string(),
            role: MeshRole::Coordination,
            sovereignty: MeshSovereignty::Saas,
            canonical_for_role: false,
            rationale: "third-party control plane may revoke or observe mesh formation; refused as a runtime dependency",
        },
        MeshProviderBinding {
            provider: "static".to_string(),
            role: MeshRole::Transport,
            sovereignty: MeshSovereignty::None,
            canonical_for_role: false,
            rationale: "declarative fallback with no control plane; no discovery, no revocation",
        },
    ]
}

/// Whether a provider may be a runtime dependency, and why not when it may not.
///
/// Refusal is a typed verdict rather than a filtered-out entry so the reason
/// survives into reports instead of a provider silently disappearing.
pub fn admit_mesh_provider(provider: &str) -> Result<MeshProviderBinding, String> {
    match mesh_provider_bindings()
        .into_iter()
        .find(|b| b.provider == provider)
    {
        None => Err(format!("unknown mesh provider: {provider}")),
        Some(binding) if binding.sovereignty == MeshSovereignty::Saas => Err(format!(
            "{provider} refused: {} control plane is not operator-owned",
            binding.provider
        )),
        Some(binding) => Ok(binding),
    }
}

/// The provider that owns a role today, if any is canonical for it.
pub fn canonical_provider_for_role(role: MeshRole) -> Option<MeshProviderBinding> {
    mesh_provider_bindings()
        .into_iter()
        .find(|b| b.role == role && b.canonical_for_role)
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshPlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub provider: String,
    pub registry_path: String,
    pub service_plane: String,
    pub cidr: Option<String>,
    pub dns_zone: Option<String>,
    pub desired_peer_count: usize,
    pub desired_peers: Vec<NetworkMeshPeer>,
    pub routes: Vec<NetworkMeshRoute>,
    pub dns: Vec<NetworkMeshDnsRecord>,
    pub acl: Vec<NetworkMeshAclRule>,
    pub actuator_required: bool,
    pub can_apply: bool,
    pub apply_requires: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshPeer {
    pub id: String,
    pub role: Option<String>,
    pub status: Option<String>,
    pub address: String,
    pub local_address: Option<String>,
    pub dns_names: Vec<String>,
    pub services: Vec<NetworkMeshService>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshService {
    pub name: String,
    pub port: Option<u16>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshRoute {
    pub destination: String,
    pub via_peer: String,
    pub provider: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshDnsRecord {
    pub name: String,
    pub address: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkMeshAclRule {
    pub id: String,
    pub source: String,
    pub destination: String,
    pub port: Option<u16>,
    pub protocol: String,
    pub action: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_mesh_plan_schema_is_stable() {
        let plan = NetworkMeshPlan {
            schema: NETWORK_MESH_PLAN_SCHEMA,
            status: "planned",
            provider: "wireguard".to_string(),
            registry_path: "registry.json".to_string(),
            service_plane: "synapse".to_string(),
            cidr: Some("10.0.0.0/24".to_string()),
            dns_zone: Some("mesh.test".to_string()),
            desired_peer_count: 0,
            desired_peers: Vec::new(),
            routes: Vec::new(),
            dns: Vec::new(),
            acl: Vec::new(),
            actuator_required: true,
            can_apply: false,
            apply_requires: vec!["network_actuator_bundle"],
        };
        assert_eq!(plan.schema, "cynic.network_mesh_plan.v1");
        assert!(!plan.can_apply);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tailscale_is_refused_as_a_runtime_dependency() {
        let err = admit_mesh_provider("tailscale").expect_err("SaaS must not be admitted");
        assert!(err.contains("not operator-owned"), "{err}");
    }

    #[test]
    fn sovereign_providers_are_admitted() {
        for provider in ["wireguard", "headscale"] {
            let binding = admit_mesh_provider(provider)
                .unwrap_or_else(|e| panic!("{provider} should be admitted: {e}"));
            assert_eq!(binding.sovereignty, MeshSovereignty::Sovereign);
        }
    }

    /// Headscale is self-hosted, so it survives the SaaS filter — but it must
    /// never be admitted as transport. That role belongs to WireGuard, which
    /// owns the only live interface on this fleet.
    #[test]
    fn headscale_coordinates_and_does_not_transport() {
        let headscale = admit_mesh_provider("headscale").expect("self-hosted");
        assert_eq!(headscale.role, MeshRole::Coordination);

        let transport =
            canonical_provider_for_role(MeshRole::Transport).expect("transport must have an owner");
        assert_eq!(transport.provider, "wireguard");
    }

    /// Each role has exactly one canonical owner: two owners is an unresolved
    /// authority conflict, zero is an unowned capability.
    #[test]
    fn every_role_has_exactly_one_canonical_owner() {
        for role in [MeshRole::Transport, MeshRole::Coordination] {
            let owners: Vec<_> = mesh_provider_bindings()
                .into_iter()
                .filter(|b| b.role == role && b.canonical_for_role)
                .collect();
            assert_eq!(owners.len(), 1, "role {role:?} has {} owners", owners.len());
        }
    }

    /// A canonical owner that is not sovereign would mean the organism depends on
    /// a third party to form its own network.
    #[test]
    fn no_canonical_owner_is_saas() {
        for binding in mesh_provider_bindings() {
            if binding.canonical_for_role {
                assert_ne!(
                    binding.sovereignty,
                    MeshSovereignty::Saas,
                    "{} is canonical but SaaS",
                    binding.provider
                );
            }
        }
    }

    #[test]
    fn an_unknown_provider_is_named_not_silently_dropped() {
        let err = admit_mesh_provider("zerotier").expect_err("unknown");
        assert!(err.contains("zerotier"), "{err}");
    }
}
