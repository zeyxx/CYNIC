//! Typed, shell-free network-actuation policy shapes.
//!
//! Pure domain types and builders live in `cynic_network::actuator`.
//! This module re-exports them and keeps host execution adapters.

use std::collections::BTreeMap;

use cynic_network::actuator::*;
pub use cynic_network::actuator::*;
use cynic_protocol::{
    ExecutionAuthorization, PolicyAdmissionReport, PolicyAuthority,
};
use serde::{Deserialize, Serialize};

use crate::execution::{HostCommandExecution, command_to_argv};
use crate::runtime_harness::{
    RuntimeHarnessContext, RuntimeHarnessTargetAttestation,
    validate_runtime_harness_target_attestation,
};

pub trait NetworkActuationExecutor: Send + Sync {
    fn run_argv(&self, role: &str, argv: &[String]) -> HostCommandExecution;
}

pub trait NetworkActuationObserver: Send + Sync {
    fn observe(&self) -> Result<NetworkActuationObservation, String>;
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkActuationExecutionReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub plan_id: String,
    pub execution_count: usize,
    pub executions: Vec<HostCommandExecution>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkActuationProofReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub execution: NetworkActuationExecutionReport,
    pub postflight: Option<NetworkActuationObservation>,
    #[serde(default)]
    pub postflight_controls: BTreeMap<String, serde_json::Value>,
    pub proof_refs: Vec<String>,
    pub blockers: Vec<String>,
}

pub fn execute_admitted_network_actuation(
    plan: &NetworkActuationPlan,
    admission_blockers: Vec<String>,
    executor: &impl NetworkActuationExecutor,
) -> NetworkActuationExecutionReport {
    let mut blockers = admission_blockers;
    let mut executions = Vec::new();
    if blockers.is_empty() {
        for command in &plan.commands {
            let argv = match command_to_argv(command) {
                Ok(argv) => argv,
                Err(error) => {
                    blockers.push(format!("non_argv_command:{}:{error}", command.role));
                    break;
                }
            };
            let execution = executor.run_argv(&command.role, &argv);
            let succeeded = execution.exit_code == Some(0);
            executions.push(execution);
            if !succeeded {
                break;
            }
        }
    }
    let status = if !blockers.is_empty() {
        "blocked"
    } else if executions.len() == plan.commands.len()
        && executions
            .iter()
            .all(|execution| execution.exit_code == Some(0))
    {
        "completed"
    } else {
        "failed"
    };
    NetworkActuationExecutionReport {
        schema: "cynic.host.network_actuation_execution.v1",
        status,
        plan_id: plan.id.clone(),
        execution_count: executions.len(),
        executions,
        blockers,
    }
}

pub fn execute_network_actuation_with_readback(
    plan: &NetworkActuationPlan,
    admission_blockers: Vec<String>,
    executor: &impl NetworkActuationExecutor,
    observer: &impl NetworkActuationObserver,
) -> NetworkActuationProofReport {
    let execution = execute_admitted_network_actuation(plan, admission_blockers, executor);
    let mut blockers = execution.blockers.clone();
    let mut proof_refs = Vec::new();
    let postflight = if execution.status == "completed" {
        match observer.observe() {
            Ok(observation) => {
                if profile_satisfied(&observation, &plan.profile) {
                    proof_refs.push(format!("postflight:{}:{}", plan.target, plan.profile));
                } else {
                    blockers.push("postflight_desired_state_unproven".to_string());
                }
                Some(observation)
            }
            Err(error) => {
                blockers.push(format!("postflight_observation_failed:{error}"));
                None
            }
        }
    } else {
        None
    };
    let status = if execution.status == "completed" && blockers.is_empty() {
        "proved"
    } else if execution.status == "blocked" {
        "blocked"
    } else {
        "failed"
    };
    NetworkActuationProofReport {
        schema: "cynic.host.network_actuation_proof.v1",
        status,
        execution,
        postflight,
        postflight_controls: BTreeMap::new(),
        proof_refs,
        blockers,
    }
}

fn profile_satisfied(observation: &NetworkActuationObservation, profile: &str) -> bool {
    match profile {
        "provider-fallback-route" => observation.provider_fallback_active,
        "synapse-route" => observation.synapse_route_active,
        "wireguard-mesh" => observation.wireguard_active,
        "direct-management-link" => observation.management_link_active,
        _ => false,
    }
}

pub fn gate_network_actuation_harness(
    plan: &NetworkActuationPlan,
    context: &RuntimeHarnessContext,
    attestation: &RuntimeHarnessTargetAttestation,
) -> Vec<String> {
    let mut blockers = validate_runtime_harness_target_attestation(context, attestation)
        .into_iter()
        .map(|violation| violation.code.to_string())
        .collect::<Vec<_>>();
    if attestation.target_id != plan.target {
        blockers.push("harness_target_plan_mismatch".to_string());
    }
    if context.mutates_host_network {
        blockers.push("harness_may_mutate_operator_host_network".to_string());
    }
    blockers
}

#[cfg(test)]
mod tests {
    use super::{
        NetworkActuationAllowedProfile, NetworkActuationExecutor, NetworkActuationObservation,
        NetworkActuationObserver, NetworkActuationPlan, NetworkActuationPolicyDocument,
        NetworkActuationRuntime, NetworkActuationTarget, build_network_actuation_commands,
        build_network_actuation_policy, execute_admitted_network_actuation,
        execute_network_actuation_with_readback, gate_network_actuation_harness,
    };
    use crate::command_surface::argv_command;
    use crate::execution::HostCommandExecution;
    use crate::runtime_harness::{
        RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA, RuntimeHarnessKind,
        RuntimeHarnessTargetAttestation, closed_local_lab_context,
    };
    use cynic_protocol::RequiredProof;
    use std::collections::BTreeMap;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FailSecond(AtomicUsize);

    struct SuccessfulExecutor;
    struct ActiveSynapseObserver;

    impl NetworkActuationExecutor for FailSecond {
        fn run_argv(&self, role: &str, argv: &[String]) -> HostCommandExecution {
            let call = self.0.fetch_add(1, Ordering::SeqCst);
            HostCommandExecution {
                role: role.to_string(),
                argv: argv.to_vec(),
                exit_code: Some(if call == 1 { 1 } else { 0 }),
                stdout: String::new(),
                stderr: String::new(),
            }
        }
    }

    impl NetworkActuationExecutor for SuccessfulExecutor {
        fn run_argv(&self, role: &str, argv: &[String]) -> HostCommandExecution {
            HostCommandExecution {
                role: role.to_string(),
                argv: argv.to_vec(),
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
            }
        }
    }

    impl NetworkActuationObserver for ActiveSynapseObserver {
        fn observe(&self) -> Result<NetworkActuationObservation, String> {
            Ok(NetworkActuationObservation {
                provider_fallback_active: true,
                synapse_route_active: true,
                wireguard_active: true,
                management_link_active: false,
                target: NetworkActuationTarget {
                    provider_fallback_cidr: "10.0.0.0/24".to_string(),
                    provider_fallback_gateway: None,
                    service_cidr: "10.10.10.0/24".to_string(),
                    mesh_interface: "wg0".to_string(),
                    management_interface: None,
                    management_connection: None,
                    management_address: None,
                    management_gateway: None,
                },
            })
        }
    }

    #[test]
    fn policy_document_keeps_commands_as_host_argv_surfaces() {
        let policy = NetworkActuationPolicyDocument {
            schema: "cynic.network_actuator.policy.v1",
            status: "planned",
            executor_path: "/portable/bin/cynic".to_string(),
            sudoers_subject: "cynic",
            max_timeout_seconds: 60,
            allowed_profiles: BTreeMap::from([(
                "wireguard-mesh".to_string(),
                NetworkActuationAllowedProfile {
                    description: "activate mesh",
                    command_prefixes: vec![argv_command(
                        "inspect_wireguard",
                        vec!["/portable/bin/wg".to_string(), "show".to_string()],
                    )],
                },
            )]),
            forbidden_tokens: vec!["sh", "-c"],
        };

        assert_eq!(
            policy.allowed_profiles["wireguard-mesh"].command_prefixes[0].role,
            "inspect_wireguard"
        );
    }

    #[test]
    fn builder_uses_injected_capability_paths() {
        let runtime = NetworkActuationRuntime {
            executor_path: "/portable/bin/cynic".to_string(),
            sudoers_subject: "cynic",
            max_timeout_seconds: 60,
            service_manager_bin: "/portable/bin/systemctl".to_string(),
            wireguard_unit: "wg-quick@wg-portable".to_string(),
            route_bin: "/portable/bin/ip".to_string(),
            wireguard_bin: "/portable/bin/wg".to_string(),
            network_manager_bin: "/portable/bin/nmcli".to_string(),
        };
        let policy = build_network_actuation_policy(["wireguard-mesh", "unknown"], &runtime);

        assert_eq!(policy.allowed_profiles.len(), 1);
        assert_eq!(
            policy.allowed_profiles["wireguard-mesh"].command_prefixes[0].role,
            "start_service"
        );
    }

    #[test]
    fn command_builder_keeps_observed_topology_as_argv_data() {
        let runtime = NetworkActuationRuntime {
            executor_path: "/portable/bin/cynic".to_string(),
            sudoers_subject: "cynic",
            max_timeout_seconds: 60,
            service_manager_bin: "/portable/bin/systemctl".to_string(),
            wireguard_unit: "wg-quick@wg-portable".to_string(),
            route_bin: "/portable/bin/ip".to_string(),
            wireguard_bin: "/portable/bin/wg".to_string(),
            network_manager_bin: "/portable/bin/nmcli".to_string(),
        };
        let target = NetworkActuationTarget {
            provider_fallback_cidr: "10.0.0.0/24".to_string(),
            provider_fallback_gateway: Some("10.0.0.1".to_string()),
            service_cidr: "10.10.10.0/24".to_string(),
            mesh_interface: "wg-portable".to_string(),
            management_interface: None,
            management_connection: None,
            management_address: None,
            management_gateway: None,
        };
        let commands = build_network_actuation_commands("synapse-route", &target, &runtime)
            .expect("synapse route is supported");

        assert_eq!(commands[0].role, "mutate_route");
        assert_eq!(
            crate::command_surface::render_command(&commands[0]),
            "/portable/bin/ip route replace 10.10.10.0/24 dev wg-portable"
        );
    }

    #[test]
    fn direct_management_link_preserves_default_route_and_uses_injected_adapter() {
        let runtime = NetworkActuationRuntime {
            executor_path: "/portable/bin/cynic".to_string(),
            sudoers_subject: "cynic",
            max_timeout_seconds: 60,
            service_manager_bin: "/portable/bin/systemctl".to_string(),
            wireguard_unit: "wg-quick@wg-portable".to_string(),
            route_bin: "/portable/bin/ip".to_string(),
            wireguard_bin: "/portable/bin/wg".to_string(),
            network_manager_bin: "/portable/bin/nmcli".to_string(),
        };
        let target = NetworkActuationTarget {
            provider_fallback_cidr: "192.168.0.0/24".to_string(),
            provider_fallback_gateway: Some("192.168.0.254".to_string()),
            service_cidr: "10.10.10.0/24".to_string(),
            mesh_interface: "wg-portable".to_string(),
            management_interface: Some("eth-test".to_string()),
            management_connection: Some("cynic-management".to_string()),
            management_address: Some("192.168.100.2/24".to_string()),
            management_gateway: Some("192.168.100.1".to_string()),
        };

        let commands =
            build_network_actuation_commands("direct-management-link", &target, &runtime)
                .expect("complete topology admits the profile");
        let rendered = crate::command_surface::render_command(&commands[0]);

        assert!(rendered.starts_with("/portable/bin/nmcli connection modify"));
        assert!(rendered.contains("ipv4.never-default yes"));
        assert!(rendered.contains("10.10.10.0/24 192.168.100.1"));
        assert!(!rendered.contains("192.168.0.254"));
    }

    #[test]
    fn admitted_execution_is_fail_fast() {
        let plan = NetworkActuationPlan {
            schema: "cynic.host.network_actuation_plan.v1",
            id: "test-plan".to_string(),
            profile: "test".to_string(),
            target: "local-host".to_string(),
            commands: vec![
                argv_command("one", vec!["true".to_string()]),
                argv_command("two", vec!["false".to_string()]),
                argv_command("three", vec!["true".to_string()]),
            ],
            preconditions: Vec::new(),
            rollback_intent: "none needed for fixture".to_string(),
            expected_proofs: vec![RequiredProof {
                gate: "fixture".to_string(),
                subject: "local-host".to_string(),
            }],
        };
        let executor = FailSecond(AtomicUsize::new(0));
        let report = execute_admitted_network_actuation(&plan, Vec::new(), &executor);
        assert_eq!(report.status, "failed");
        assert_eq!(report.execution_count, 2);
        assert_eq!(executor.0.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn harness_gate_binds_plan_to_isolated_target() {
        let plan = NetworkActuationPlan {
            schema: "cynic.host.network_actuation_plan.v1",
            id: "lab-plan".to_string(),
            profile: "synapse-route".to_string(),
            target: "guest-1".to_string(),
            commands: vec![argv_command("probe", vec!["true".to_string()])],
            preconditions: Vec::new(),
            rollback_intent: "restore route".to_string(),
            expected_proofs: Vec::new(),
        };
        let context = closed_local_lab_context();
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let attestation = RuntimeHarnessTargetAttestation {
            schema: RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA.to_string(),
            harness_kind: RuntimeHarnessKind::ClosedLocalLab,
            observed_at_unix_ms: now,
            expires_at_unix_ms: now + 60_000,
            target_id: "guest-1".to_string(),
            provider: "qemu-libvirt".to_string(),
            transport: "qemu-guest-agent".to_string(),
            transport_target: "guest-1".to_string(),
            declared_isolation_boundary: context.isolation_boundary.to_string(),
            observed_target_boundary: "isolated-vm-network".to_string(),
            target_identity_verified: true,
            transport_verified: true,
            isolated_network_verified: true,
            operator_host_network_unchanged: true,
            proof_refs: vec!["proof:guest-1".to_string()],
        };
        assert!(gate_network_actuation_harness(&plan, &context, &attestation).is_empty());
    }

    #[test]
    fn completed_execution_requires_postflight_proof() {
        let plan = NetworkActuationPlan {
            schema: "cynic.host.network_actuation_plan.v1",
            id: "proof-plan".to_string(),
            profile: "synapse-route".to_string(),
            target: "guest-1".to_string(),
            commands: vec![argv_command("route", vec!["true".to_string()])],
            preconditions: Vec::new(),
            rollback_intent: "restore route".to_string(),
            expected_proofs: Vec::new(),
        };
        let report = execute_network_actuation_with_readback(
            &plan,
            Vec::new(),
            &SuccessfulExecutor,
            &ActiveSynapseObserver,
        );
        assert_eq!(report.status, "proved");
        assert_eq!(report.proof_refs, vec!["postflight:guest-1:synapse-route"]);
    }
}
