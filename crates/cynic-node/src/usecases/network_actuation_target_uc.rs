//! Target-bound orchestration for host-owned network actuation.
//!
//! This use case composes policy admission, explicit authorization, harness
//! attestation, QEMU guest-agent transport, and typed postflight observation.
//! Target discovery remains outside this boundary: callers must inject the
//! selected target attestation explicitly.

use std::time::Duration;

use cynic_host::execution::command_to_argv;
use cynic_host::network_actuation::{
    NetworkActuationProofReport, NetworkActuationRequest, execute_network_actuation_with_readback,
    gate_network_actuation_harness, gate_network_actuation_request,
};
use cynic_host::runtime_harness::{RuntimeHarnessContext, RuntimeHarnessTargetAttestation};
<<<<<<< HEAD
use cynic_lifecycle::ArgvRunner;
=======
use cynic_host::update_lifecycle::ArgvRunner;
>>>>>>> github/main

use crate::adapters::qemu_guest_agent_network_runner::{
    QemuGuestAgentNetworkObserver, QemuGuestAgentNetworkRunner,
};

#[derive(Debug, Clone)]
pub struct QemuNetworkActuationConfig {
    pub virsh_bin: String,
    pub observation_argv: Vec<String>,
    pub timeout_seconds: u64,
    pub max_status_polls: usize,
    pub poll_interval: Duration,
}

pub fn execute_qemu_network_actuation<R: ArgvRunner + Send + Sync>(
    request: &NetworkActuationRequest,
    context: RuntimeHarnessContext,
    attestation: RuntimeHarnessTargetAttestation,
    runner: R,
    config: QemuNetworkActuationConfig,
) -> Result<NetworkActuationProofReport, String> {
    let mut blockers = gate_network_actuation_request(request);
    blockers.extend(gate_network_actuation_harness(
        &request.plan,
        &context,
        &attestation,
    ));

    let mut allowed_argv = request
        .plan
        .commands
        .iter()
        .map(command_to_argv)
        .collect::<Result<Vec<_>, _>>()?;
    allowed_argv.push(config.observation_argv.clone());

    let transport = QemuGuestAgentNetworkRunner::new(
        runner,
        context,
        attestation,
        config.virsh_bin,
        allowed_argv,
        config.timeout_seconds,
        config.max_status_polls,
        config.poll_interval,
    )?;
    let observer = QemuGuestAgentNetworkObserver::new(&transport, config.observation_argv);

    Ok(execute_network_actuation_with_readback(
        &request.plan,
        blockers,
        &transport,
        &observer,
    ))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    use cynic_host::command_surface::argv_command;
    use cynic_host::network_actuation::{
        NETWORK_ACTUATION_REQUEST_SCHEMA, NetworkActuationPlan, NetworkActuationRequest,
        admit_network_actuation_plan,
    };
    use cynic_host::runtime_harness::{
        RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA, RuntimeHarnessKind,
        RuntimeHarnessTargetAttestation, closed_local_lab_context,
    };
<<<<<<< HEAD
    use cynic_lifecycle::{ArgvOutput, ArgvRunner};
=======
    use cynic_host::update_lifecycle::{ArgvOutput, ArgvRunner};
>>>>>>> github/main
    use cynic_protocol::{ExecutionAuthorization, PolicyAuthority};

    use super::{QemuNetworkActuationConfig, execute_qemu_network_actuation};

    struct RefusingRunner(AtomicUsize);

    impl ArgvRunner for RefusingRunner {
        fn run(&self, _argv: &[&str]) -> ArgvOutput {
            self.0.fetch_add(1, Ordering::SeqCst);
            panic!("transport must not run for a mismatched target")
        }
    }

    #[test]
    fn target_mismatch_is_blocked_before_qemu_transport() {
        let plan = NetworkActuationPlan {
            schema: "cynic.host.network_actuation_plan.v1",
            id: "plan:guest-a".to_string(),
            profile: "synapse-route".to_string(),
            target: "guest-a".to_string(),
            commands: vec![argv_command("route", vec!["/portable/ip".to_string()])],
            preconditions: Vec::new(),
            rollback_intent: "restore prior route".to_string(),
            expected_proofs: Vec::new(),
        };
        let admission = admit_network_actuation_plan(&plan, "operator");
        let request = NetworkActuationRequest {
            schema: NETWORK_ACTUATION_REQUEST_SCHEMA.to_string(),
            plan,
            admission,
            authorization: ExecutionAuthorization::new(
                "operator",
                PolicyAuthority::HumanOperator,
                true,
                Vec::new(),
            ),
        };
        let context = closed_local_lab_context();
        let now = chrono::Utc::now().timestamp_millis() as u64;
        let attestation = RuntimeHarnessTargetAttestation {
            schema: RUNTIME_HARNESS_TARGET_ATTESTATION_SCHEMA.to_string(),
            harness_kind: RuntimeHarnessKind::ClosedLocalLab,
            observed_at_unix_ms: now,
            expires_at_unix_ms: now + 60_000,
            target_id: "guest-b".to_string(),
            provider: "qemu-libvirt".to_string(),
            transport: "qemu-guest-agent".to_string(),
            transport_target: "guest-b".to_string(),
            declared_isolation_boundary: context.isolation_boundary.to_string(),
            observed_target_boundary: "libvirt-network:cynic-lab-isolated".to_string(),
            target_identity_verified: true,
            transport_verified: true,
            isolated_network_verified: true,
            operator_host_network_unchanged: true,
            proof_refs: vec!["proof:guest-b".to_string()],
        };
        let runner = RefusingRunner(AtomicUsize::new(0));

        let report = execute_qemu_network_actuation(
            &request,
            context,
            attestation,
            runner,
            QemuNetworkActuationConfig {
                virsh_bin: "virsh".to_string(),
                observation_argv: vec!["/portable/cynic-node".to_string()],
                timeout_seconds: 5,
                max_status_polls: 2,
                poll_interval: Duration::ZERO,
            },
        )
        .expect("blocked proof report");

        assert_eq!(report.status, "blocked");
        assert!(
            report
                .blockers
                .contains(&"harness_target_plan_mismatch".to_string())
        );
        assert_eq!(report.execution.execution_count, 0);
    }
}
