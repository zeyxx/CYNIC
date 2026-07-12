use std::path::Path;

use serde::Serialize;

use crate::adapters::local_network_mesh_bundle::LocalNetworkMeshBundleReader;
use crate::network_command_surface::NetworkCommandSurface;
use crate::ports::network_mesh_bundle_port::NetworkMeshBundlePort;

mod command_plan;
#[cfg(kani)]
mod command_plan_verification;
mod context;
mod policy;
#[cfg(kani)]
mod policy_verification;
mod profile_execution;

use command_plan::{
    dns_install, firewall_preflight, post_apply_observation, preflight, rollback_conditions,
    rollback_plan, route_install, route_observation, route_preflight, tunnel_install,
};
use context::NetworkMeshApplyPlanContext;
use policy::{MeshActuatorPolicyProjection, mesh_actuator_policy};
use profile_execution::{NetworkMeshApplyProfileReport, apply_network_mesh_profile_from_plan};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NetworkMeshApplyPlanReport {
    pub(crate) schema: &'static str,
    pub(crate) status: &'static str,
    pub(crate) mutates: bool,
    pub(crate) provider: String,
    pub(crate) bundle_dir: String,
    pub(crate) bundle_complete: bool,
    pub(crate) desired_peer_count: usize,
    pub(crate) preflight: Vec<NetworkCommandSurface>,
    pub(crate) firewall_preflight: Vec<NetworkCommandSurface>,
    pub(crate) tunnel_install: Vec<NetworkCommandSurface>,
    pub(crate) route_preflight: Vec<NetworkCommandSurface>,
    pub(crate) route_install: Vec<NetworkCommandSurface>,
    pub(crate) route_observation: Vec<NetworkCommandSurface>,
    pub(crate) dns_install: Vec<NetworkCommandSurface>,
    pub(crate) post_apply_observation: Vec<NetworkCommandSurface>,
    pub(crate) rollback_conditions: Vec<&'static str>,
    pub(crate) rollback_plan: Vec<NetworkCommandSurface>,
    pub(crate) actuator_policy: MeshActuatorPolicyProjection,
    pub(crate) apply_requires: Vec<&'static str>,
    pub(crate) can_apply: bool,
}

pub(crate) fn build_network_mesh_apply_plan(
    bundle_dir: &str,
) -> Result<NetworkMeshApplyPlanReport, String> {
    build_network_mesh_apply_plan_with_reader_and_context(
        bundle_dir,
        &LocalNetworkMeshBundleReader,
        NetworkMeshApplyPlanContext::from_current_runtime(),
    )
}

fn build_network_mesh_apply_plan_with_reader_and_context(
    bundle_dir: &str,
    bundle_reader: &impl NetworkMeshBundlePort,
    context: NetworkMeshApplyPlanContext,
) -> Result<NetworkMeshApplyPlanReport, String> {
    let bundle_dir = Path::new(bundle_dir);
    let bundle = bundle_reader.load_mesh_bundle(bundle_dir)?;
    let manifest = bundle.manifest;
    let plan = bundle.plan;

    let preflight = preflight(&manifest, &plan);
    let firewall_preflight = firewall_preflight(&plan);
    let tunnel_install = tunnel_install(bundle_dir, &manifest, &plan);
    let route_preflight = route_preflight(&plan);
    let route_install = route_install(&plan, &context.runtime);
    let route_observation = route_observation(&plan, &context.runtime);
    let dns_install = dns_install(bundle_dir);
    let post_apply_observation = post_apply_observation(&plan, &context.runtime);
    let rollback_plan = rollback_plan(&plan, &context.runtime);
    let actuator_policy = mesh_actuator_policy(
        &firewall_preflight,
        &tunnel_install,
        &route_preflight,
        &route_install,
        &route_observation,
        &dns_install,
        &post_apply_observation,
        &rollback_plan,
    );

    Ok(NetworkMeshApplyPlanReport {
        schema: "cynic.network_mesh_apply_plan.v1",
        status: if bundle.complete {
            "planned"
        } else {
            "blocked"
        },
        mutates: false,
        provider: manifest.provider.clone(),
        bundle_dir: bundle_dir.to_string_lossy().to_string(),
        bundle_complete: bundle.complete,
        desired_peer_count: plan.desired_peer_count,
        preflight,
        firewall_preflight,
        tunnel_install,
        route_preflight,
        route_install,
        route_observation,
        dns_install,
        post_apply_observation,
        rollback_conditions: rollback_conditions(),
        rollback_plan,
        actuator_policy,
        apply_requires: vec![
            "operator_confirms_mutation",
            "private_keys_materialized_outside_git",
            "bounded_network_actuator_installed",
            "firewall_policy_reviewed",
            "rollback_plan_available",
        ],
        can_apply: false,
    })
}

pub(crate) fn apply_network_mesh_profile(
    bundle_dir: &str,
    profile: &str,
    apply: bool,
    closed_environment: bool,
) -> Result<NetworkMeshApplyProfileReport, String> {
    let plan = build_network_mesh_apply_plan(bundle_dir)?;
    Ok(apply_network_mesh_profile_from_plan(
        bundle_dir,
        profile,
        apply,
        closed_environment,
        plan,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::network_mesh_apply::{
        MeshApplyBundle, MeshBundleManifest, MeshPlanFile, MeshPlanRouteFile,
    };
    use crate::network_command_surface::NetworkCommandInvocation;
    use crate::network_mesh_apply_plan::command_plan::MeshApplyRuntimeContext;
    use crate::network_mesh_bundle::build_network_mesh_bundle;
    use std::path::Path;

    struct FakeBundleReader;

    impl NetworkMeshBundlePort for FakeBundleReader {
        fn load_mesh_bundle(
            &self,
            _bundle_dir: &Path,
        ) -> Result<crate::domain::network_mesh_apply::MeshApplyBundle, String> {
            Ok(MeshApplyBundle {
                manifest: MeshBundleManifest {
                    schema: "cynic.network_mesh_bundle.v1".to_string(),
                    status: "ready".to_string(),
                    provider: "static".to_string(),
                    bundle_files: Vec::new(),
                },
                plan: MeshPlanFile {
                    provider: "static".to_string(),
                    cidr: Some("10.10.10.0/24".to_string()),
                    desired_peer_count: 1,
                    desired_peers: Vec::new(),
                    routes: vec![MeshPlanRouteFile {
                        destination: "10.10.10.0/24".to_string(),
                        via_peer: "node-a".to_string(),
                        provider: "static".to_string(),
                    }],
                    dns: Vec::new(),
                    acl: Vec::new(),
                },
                complete: true,
            })
        }
    }

    #[test]
    fn apply_plan_reads_bundle_without_mutating() {
        let out = tempfile::tempdir().expect("tempdir");
        let registry_path = format!(
            "{}/../../infra/registries/lab.json",
            env!("CARGO_MANIFEST_DIR")
        );
        build_network_mesh_bundle(&registry_path, None, out.path().to_str().expect("path"))
            .expect("bundle");
        let plan =
            build_network_mesh_apply_plan(out.path().to_str().expect("path")).expect("apply plan");
        assert_eq!(plan.schema, "cynic.network_mesh_apply_plan.v1");
        assert!(plan.bundle_complete);
        assert!(!plan.mutates);
        assert!(!plan.can_apply);
        assert!(!plan.rollback_plan.is_empty());
        assert_eq!(plan.actuator_policy.allowed_profiles.len(), 8);
        assert!(
            plan.actuator_policy
                .allowed_profiles
                .iter()
                .any(|profile| profile.name == "mesh-rollback")
        );
    }

    #[test]
    fn projected_actuator_policy_contains_no_shell_commands() {
        let out = tempfile::tempdir().expect("tempdir");
        let registry_path = format!(
            "{}/../../infra/registries/lab.json",
            env!("CARGO_MANIFEST_DIR")
        );
        build_network_mesh_bundle(&registry_path, None, out.path().to_str().expect("path"))
            .expect("bundle");
        let plan =
            build_network_mesh_apply_plan(out.path().to_str().expect("path")).expect("apply plan");

        for profile in &plan.actuator_policy.allowed_profiles {
            for command in &profile.command_prefixes {
                assert!(
                    !matches!(command.invocation, NetworkCommandInvocation::Shell { .. }),
                    "profile {} contains shell command {}",
                    profile.name,
                    command.role
                );
            }
        }
    }

    #[test]
    fn route_install_profile_is_argv_only_but_requires_closed_environment() {
        let out = tempfile::tempdir().expect("tempdir");
        let registry_path = format!(
            "{}/../../infra/registries/lab.json",
            env!("CARGO_MANIFEST_DIR")
        );
        build_network_mesh_bundle(&registry_path, None, out.path().to_str().expect("path"))
            .expect("bundle");
        let plan =
            build_network_mesh_apply_plan(out.path().to_str().expect("path")).expect("apply plan");
        let route_install = plan
            .actuator_policy
            .allowed_profiles
            .iter()
            .find(|profile| profile.name == "mesh-route-install")
            .expect("route install profile");
        assert!(route_install.command_prefixes.iter().all(|command| {
            matches!(command.invocation, NetworkCommandInvocation::Argv { .. })
        }));

        let report = apply_network_mesh_profile(
            out.path().to_str().expect("path"),
            "mesh-route-install",
            true,
            false,
        )
        .expect("apply profile");

        assert_eq!(report.status, "refused");
        assert!(report.executions.is_empty());
        assert!(
            report
                .violations
                .iter()
                .any(|item| item.code == "closed_environment_required")
        );
    }

    #[test]
    fn apply_plan_can_be_built_from_substituted_bundle_port() {
        let plan = build_network_mesh_apply_plan_with_reader_and_context(
            "/virtual/bundle",
            &FakeBundleReader,
            NetworkMeshApplyPlanContext {
                runtime: MeshApplyRuntimeContext {
                    mesh_interface: "cynic-mesh0".to_string(),
                    ip_bin: "/portable/bin/ip".to_string(),
                    wg_bin: "/portable/bin/wg".to_string(),
                    getent_bin: "/portable/bin/getent".to_string(),
                },
            },
        )
        .expect("apply plan");

        assert_eq!(plan.status, "planned");
        assert!(plan.bundle_complete);
        assert_eq!(plan.provider, "static");
        assert_eq!(plan.desired_peer_count, 1);
        assert_eq!(plan.route_install.len(), 1);
        assert!(plan.route_install.iter().all(|command| matches!(
            &command.invocation,
            NetworkCommandInvocation::Argv { argv } if argv.first() == Some(&"/portable/bin/ip".to_string())
        )));
    }
}
