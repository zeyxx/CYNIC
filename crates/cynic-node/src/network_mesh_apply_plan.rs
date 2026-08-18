use std::path::Path;

use serde::Serialize;

use crate::adapters::local_network_mesh_bundle::LocalNetworkMeshBundleReader;
use crate::network_command_surface::NetworkCommandSurface;
use crate::ports::network_mesh_bundle_port::NetworkMeshBundlePort;

mod canary_admission;
mod command_plan;
#[cfg(kani)]
mod command_plan_verification;
mod context;
mod policy;
#[cfg(kani)]
mod policy_verification;
mod profile_execution;
mod target_transaction;

use canary_admission::{
    MeshCanaryAdmissionReport, MeshCanaryEvidence, assess_mesh_canary_admission,
};
use command_plan::{
    dns_install, firewall_preflight, post_apply_observation, preflight, rollback_conditions,
    rollback_plan, route_install, route_observation, route_preflight, tunnel_install,
};
use context::NetworkMeshApplyPlanContext;
use policy::{MeshActuatorPolicyProjection, mesh_actuator_policy};
pub(crate) use profile_execution::NetworkMeshApplyProfileReport;
use profile_execution::apply_network_mesh_profile_from_plan;
pub(crate) use target_transaction::{
    apply_network_mesh_target_transaction_with_runner, target_transaction_argv,
};

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
    pub(crate) canary_admission: MeshCanaryAdmissionReport,
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
    let canary_admission = assess_mesh_canary_admission(&MeshCanaryEvidence::default());

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
        canary_admission,
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
    use crate::domain::network_execution::NetworkExecutionBoundary;
    use crate::domain::network_mesh_apply::{
        MeshApplyBundle, MeshBundleManifest, MeshPlanFile, MeshPlanPeerFile, MeshPlanRouteFile,
    };
    use crate::network_actuator::CommandExecution;
    use crate::network_command_surface::NetworkCommandInvocation;
    use crate::network_mesh_apply_plan::command_plan::MeshApplyRuntimeContext;
    use crate::network_mesh_apply_plan::profile_execution::apply_network_mesh_profile_from_plan_with_runner;
    use crate::network_mesh_apply_plan::target_transaction::{
        apply_network_mesh_target_transaction_with_runner, target_transaction_argv,
    };
    use crate::network_mesh_bundle::build_network_mesh_bundle;
    use crate::ports::network_command_runner_port::NetworkCommandRunnerPort;
    use cynic_host::runtime_harness::{
        RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA, RuntimeHarnessKind,
        RuntimeHarnessTargetAttestation, closed_local_lab_context,
    };
    use std::cell::Cell;
    use std::collections::VecDeque;
    use std::path::Path;
    use std::sync::Mutex;

    struct FakeBundleReader;

    struct FakeIsolatedNetworkRunner {
        calls: Cell<usize>,
        attestation: RuntimeHarnessTargetAttestation,
        exit_code: i32,
    }

    struct ScriptedIsolatedNetworkRunner {
        calls: Mutex<Vec<String>>,
        outcomes: Mutex<VecDeque<(Option<i32>, String)>>,
        attestation: RuntimeHarnessTargetAttestation,
    }

    impl NetworkCommandRunnerPort for FakeIsolatedNetworkRunner {
        fn execution_boundary(&self) -> NetworkExecutionBoundary {
            NetworkExecutionBoundary::IsolatedHarnessTarget
        }

        fn target_attestation(&self) -> Option<&RuntimeHarnessTargetAttestation> {
            Some(&self.attestation)
        }

        fn run_command(
            &self,
            command: &crate::network_command_surface::NetworkCommandSurface,
        ) -> CommandExecution {
            self.calls.set(self.calls.get() + 1);
            let argv = match &command.invocation {
                NetworkCommandInvocation::Argv { argv } => argv.clone(),
                _ => Vec::new(),
            };
            CommandExecution {
                role: command.role.clone(),
                argv,
                exit_code: Some(self.exit_code),
                stdout: String::new(),
                stderr: String::new(),
            }
        }
    }

    impl NetworkCommandRunnerPort for ScriptedIsolatedNetworkRunner {
        fn execution_boundary(&self) -> NetworkExecutionBoundary {
            NetworkExecutionBoundary::IsolatedHarnessTarget
        }

        fn target_attestation(&self) -> Option<&RuntimeHarnessTargetAttestation> {
            Some(&self.attestation)
        }

        fn run_command(
            &self,
            command: &crate::network_command_surface::NetworkCommandSurface,
        ) -> CommandExecution {
            self.calls.lock().expect("calls").push(command.role.clone());
            let (exit_code, stdout) = self
                .outcomes
                .lock()
                .expect("outcomes")
                .pop_front()
                .expect("scripted outcome");
            let argv = match &command.invocation {
                NetworkCommandInvocation::Argv { argv } => argv.clone(),
                _ => Vec::new(),
            };
            CommandExecution {
                role: command.role.clone(),
                argv,
                exit_code,
                stdout,
                stderr: String::new(),
            }
        }
    }

    fn scripted_runner(
        outcomes: impl IntoIterator<Item = (Option<i32>, &'static str)>,
    ) -> ScriptedIsolatedNetworkRunner {
        ScriptedIsolatedNetworkRunner {
            calls: Mutex::new(Vec::new()),
            outcomes: Mutex::new(
                outcomes
                    .into_iter()
                    .map(|(exit_code, stdout)| (exit_code, stdout.to_string()))
                    .collect(),
            ),
            attestation: attested_lab_target(),
        }
    }

    fn attested_lab_target() -> RuntimeHarnessTargetAttestation {
        let context = closed_local_lab_context();
        RuntimeHarnessTargetAttestation {
            schema: RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA.to_string(),
            harness_kind: RuntimeHarnessKind::ClosedLocalLab,
            observed_at_unix_ms: 1,
            expires_at_unix_ms: u64::MAX,
            target_id: "lab-kernel".to_string(),
            provider: "qemu-libvirt".to_string(),
            transport: "ssh".to_string(),
            transport_target: "cynic@172.30.41.10".to_string(),
            declared_isolation_boundary: context.isolation_boundary.to_string(),
            observed_target_boundary: "libvirt-network:cynic-lab-isolated".to_string(),
            target_identity_verified: true,
            transport_verified: true,
            isolated_network_verified: true,
            operator_host_network_unchanged: true,
            proof_refs: vec!["runtime-proof://lab/target/lab-kernel".to_string()],
        }
    }

    impl NetworkMeshBundlePort for FakeBundleReader {
        fn load_mesh_bundle(
            &self,
            _bundle_dir: &Path,
        ) -> Result<crate::domain::network_mesh_apply::MeshApplyBundle, String> {
            Ok(MeshApplyBundle {
                manifest: MeshBundleManifest {
                    schema: "cynic.network_mesh_bundle.v1".to_string(),
                    status: "planned".to_string(),
                    provider: "static".to_string(),
                    bundle_files: Vec::new(),
                },
                plan: MeshPlanFile {
                    schema: "cynic.network_mesh_plan.v1".to_string(),
                    status: "planned".to_string(),
                    provider: "static".to_string(),
                    cidr: Some("10.10.10.0/24".to_string()),
                    desired_peer_count: 1,
                    desired_peers: vec![MeshPlanPeerFile {
                        id: "node-a".to_string(),
                    }],
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
    fn closed_environment_flag_alone_never_executes_local_network_mutation() {
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

        let report = apply_network_mesh_profile_from_plan(
            "/virtual/bundle",
            "mesh-route-install",
            true,
            true,
            plan,
        );

        assert_eq!(report.status, "refused");
        assert!(report.executions.is_empty());
        assert!(!report.execution_admission.admitted);
        assert!(
            report
                .violations
                .iter()
                .any(|violation| { violation.code == "network_execution_boundary_unverified" })
        );
    }

    #[test]
    fn isolated_runner_executes_only_after_harness_target_admission() {
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
        let harness = closed_local_lab_context();
        let runner = FakeIsolatedNetworkRunner {
            calls: Cell::new(0),
            attestation: attested_lab_target(),
            exit_code: 0,
        };

        let report = apply_network_mesh_profile_from_plan_with_runner(
            "/virtual/bundle",
            "mesh-route-install",
            true,
            true,
            Some(&harness),
            &runner,
            plan,
        );

        assert_eq!(report.status, "completed");
        assert!(report.execution_admission.admitted);
        assert_eq!(runner.calls.get(), report.commands.len());
        assert_eq!(report.executions.len(), report.commands.len());
    }

    #[test]
    fn profile_execution_stops_after_first_failed_command() {
        let mut plan = build_network_mesh_apply_plan_with_reader_and_context(
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
        let profile = plan
            .actuator_policy
            .allowed_profiles
            .iter_mut()
            .find(|profile| profile.name == "mesh-route-install")
            .expect("profile");
        profile
            .command_prefixes
            .push(profile.command_prefixes[0].clone());
        profile.command_count = profile.command_prefixes.len();
        let harness = closed_local_lab_context();
        let runner = FakeIsolatedNetworkRunner {
            calls: Cell::new(0),
            attestation: attested_lab_target(),
            exit_code: 1,
        };

        let report = apply_network_mesh_profile_from_plan_with_runner(
            "/virtual/bundle",
            "mesh-route-install",
            true,
            true,
            Some(&harness),
            &runner,
            plan,
        );

        assert_eq!(report.status, "failed");
        assert_eq!(report.commands.len(), 2);
        assert_eq!(report.executions.len(), 1);
        assert_eq!(runner.calls.get(), 1);
    }

    #[test]
    fn target_transaction_proves_empty_before_and_expected_route_after() {
        let plan = fake_apply_plan();
        let harness = closed_local_lab_context();
        let runner = scripted_runner([
            (Some(0), ""),
            (Some(0), ""),
            (Some(0), "10.10.10.0/24 dev cynic-mesh0 scope link\n"),
        ]);

        let report = apply_network_mesh_target_transaction_with_runner(
            "/virtual/bundle",
            "mesh-route-install",
            true,
            &harness,
            &runner,
            plan,
        );

        assert_eq!(report.status, "completed");
        assert!(report.mutates);
        assert_eq!(report.proof_refs.len(), 3);
        assert!(report.rollback.executions.is_empty());
        assert_eq!(runner.calls.lock().expect("calls").len(), 3);
    }

    #[test]
    fn target_transaction_rolls_back_when_poststate_is_not_proven() {
        let plan = fake_apply_plan();
        let harness = closed_local_lab_context();
        let runner = scripted_runner([
            (Some(0), ""),
            (Some(0), ""),
            (Some(0), ""),
            (Some(0), ""),
            (Some(0), ""),
        ]);

        let report = apply_network_mesh_target_transaction_with_runner(
            "/virtual/bundle",
            "mesh-route-install",
            true,
            &harness,
            &runner,
            plan,
        );

        assert_eq!(report.status, "rolled_back");
        assert_eq!(report.rollback.executions.len(), 1);
        assert_eq!(report.rollback_verification.executions.len(), 1);
        assert_eq!(report.proof_refs.len(), 5);
        assert_eq!(runner.calls.lock().expect("calls").len(), 5);
    }

    #[test]
    fn target_transaction_rolls_back_an_attempted_route_after_apply_failure() {
        let plan = fake_apply_plan();
        let harness = closed_local_lab_context();
        let runner = scripted_runner([(Some(0), ""), (Some(1), ""), (Some(0), ""), (Some(0), "")]);

        let report = apply_network_mesh_target_transaction_with_runner(
            "/virtual/bundle",
            "mesh-route-install",
            true,
            &harness,
            &runner,
            plan,
        );

        assert_eq!(report.status, "rolled_back");
        assert_eq!(report.apply_phase.executions.len(), 1);
        assert_eq!(report.rollback.executions.len(), 1);
        assert_eq!(runner.calls.lock().expect("calls").len(), 4);
    }

    #[test]
    fn target_transaction_reports_unrecoverable_rollback_failure() {
        let plan = fake_apply_plan();
        let harness = closed_local_lab_context();
        let runner = scripted_runner([(Some(0), ""), (Some(0), ""), (Some(0), ""), (Some(1), "")]);

        let report = apply_network_mesh_target_transaction_with_runner(
            "/virtual/bundle",
            "mesh-route-install",
            true,
            &harness,
            &runner,
            plan,
        );

        assert_eq!(report.status, "rollback_failed");
        assert!(
            report
                .violations
                .iter()
                .any(|violation| violation.code == "route_rollback_failed")
        );
        assert!(report.rollback_verification.executions.is_empty());
        assert_eq!(runner.calls.lock().expect("calls").len(), 4);
    }

    #[test]
    fn target_transaction_never_replaces_a_preexisting_route() {
        let plan = fake_apply_plan();
        let harness = closed_local_lab_context();
        let runner = scripted_runner([(Some(0), "10.10.10.0/24 dev eth0\n")]);

        let report = apply_network_mesh_target_transaction_with_runner(
            "/virtual/bundle",
            "mesh-route-install",
            true,
            &harness,
            &runner,
            plan,
        );

        assert_eq!(report.status, "preflight_refused");
        assert!(!report.mutates);
        assert!(report.apply_phase.executions.is_empty());
        assert_eq!(runner.calls.lock().expect("calls").len(), 1);
    }

    #[test]
    fn target_transaction_without_apply_never_reaches_the_transport() {
        let plan = fake_apply_plan();
        let harness = closed_local_lab_context();
        let runner = scripted_runner([]);

        let report = apply_network_mesh_target_transaction_with_runner(
            "/virtual/bundle",
            "mesh-route-install",
            false,
            &harness,
            &runner,
            plan,
        );

        assert_eq!(report.status, "refused");
        assert!(!report.mutates);
        assert!(
            report
                .violations
                .iter()
                .any(|violation| violation.code == "apply_flag_required")
        );
        assert!(runner.calls.lock().expect("calls").is_empty());
    }

    #[test]
    fn target_transaction_allowlist_contains_only_exact_route_transaction_argv() {
        let plan = fake_apply_plan();

        let allowed = target_transaction_argv(&plan, "mesh-route-install");

        assert_eq!(allowed.len(), 3);
        assert!(
            allowed
                .iter()
                .all(|argv| argv.first().is_some_and(|bin| bin == "/portable/bin/ip"))
        );
        assert!(target_transaction_argv(&plan, "mesh-dns-install").is_empty());
    }

    fn fake_apply_plan() -> NetworkMeshApplyPlanReport {
        build_network_mesh_apply_plan_with_reader_and_context(
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
        .expect("apply plan")
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
