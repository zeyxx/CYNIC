use serde::Serialize;

use crate::network_command_surface::NetworkCommandSurface;

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MeshActuatorPolicyProjection {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) source: &'static str,
    pub(crate) allowed_profiles: Vec<MeshActuatorProfile>,
    pub(crate) forbidden_tokens: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct MeshActuatorProfile {
    pub(crate) name: &'static str,
    pub(crate) description: &'static str,
    pub(crate) command_count: usize,
    pub(crate) command_prefixes: Vec<NetworkCommandSurface>,
}

pub(super) fn mesh_actuator_policy(
    firewall_preflight: &[NetworkCommandSurface],
    tunnel_install: &[NetworkCommandSurface],
    route_preflight: &[NetworkCommandSurface],
    route_install: &[NetworkCommandSurface],
    route_observation: &[NetworkCommandSurface],
    dns_install: &[NetworkCommandSurface],
    post_apply_observation: &[NetworkCommandSurface],
    rollback_plan: &[NetworkCommandSurface],
) -> MeshActuatorPolicyProjection {
    MeshActuatorPolicyProjection {
        schema: "cynic.network_mesh_actuator_policy_projection.v1",
        status: "planned",
        mutates: false,
        source: "cynic.network_mesh_apply_plan.v1",
        allowed_profiles: vec![
            profile(
                "mesh-firewall-preflight",
                "Prepare or validate firewall transaction before tunnel activation.",
                firewall_preflight,
            ),
            profile(
                "mesh-tunnel-install",
                "Install provider tunnel configuration after secret material is supplied.",
                tunnel_install,
            ),
            profile(
                "mesh-route-preflight",
                "Validate declared mesh route bindings before route mutation.",
                route_preflight,
            ),
            profile(
                "mesh-route-install",
                "Install mesh routes after tunnel activation.",
                route_install,
            ),
            profile(
                "mesh-route-observe",
                "Observe mesh route state after route mutation.",
                route_observation,
            ),
            profile("mesh-dns-install", "Install mesh DNS records.", dns_install),
            profile(
                "mesh-observe",
                "Observe mesh convergence after apply.",
                post_apply_observation,
            ),
            profile(
                "mesh-rollback",
                "Rollback routes, DNS, firewall, and tunnel configuration.",
                rollback_plan,
            ),
        ],
        forbidden_tokens: vec![
            "sh", "bash", "-c", "curl", "wget", "nc", "python", "python3", "perl", "ruby", ">",
            ">>", "|", "&&", ";",
        ],
    }
}

fn profile(
    name: &'static str,
    description: &'static str,
    commands: &[NetworkCommandSurface],
) -> MeshActuatorProfile {
    MeshActuatorProfile {
        name,
        description,
        command_count: commands.len(),
        command_prefixes: commands.to_vec(),
    }
}
