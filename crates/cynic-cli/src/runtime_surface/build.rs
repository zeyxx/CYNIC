use std::process::ExitCode;

use cynic_host::build::{BuildRequest, run_build};
use cynic_host::build_evidence::{
    BuildLeaseVerificationPort, BuildRunnerObservationPort, CurlBuildLeaseVerificationAdapter,
    SshBuildRunnerObservationAdapter,
};
use cynic_host::registry::resolve_build_runner_transport;
use cynic_host::update_lifecycle::LocalArgvRunner;
use cynic_protocol::{
    LeaseGuard, SourceSnapshot, admit_build_input, admit_build_runner,
    compose_build_runner_envelope, plan_build_runner,
};

use crate::cli_json::emit_pretty_json;
use crate::fail;

pub(crate) fn execute_build(
    json: bool,
    source_snapshot_path: &str,
    registry_path: &str,
    kernel_endpoint: &str,
    agent_id: &str,
    lease_id: &str,
    fencing_token: u64,
    artifact_path: &str,
    apply: bool,
    command: &[String],
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let raw = match std::fs::read_to_string(source_snapshot_path) {
        Ok(raw) => raw,
        Err(error) => return fail(format!("failed to read source snapshot: {error}")),
    };
    let snapshot = match serde_json::from_str::<SourceSnapshot>(&raw) {
        Ok(snapshot) => snapshot,
        Err(error) => return fail(format!("failed to parse source snapshot: {error}")),
    };
    let transport = match resolve_build_runner_transport(Some(registry_path)) {
        Ok(transport) => transport,
        Err(error) => return fail(error),
    };
    let runner = LocalArgvRunner;
    let observation_adapter =
        match SshBuildRunnerObservationAdapter::new(&runner, &transport.ssh_target) {
            Ok(adapter) => adapter,
            Err(error) => return fail(error),
        };
    let runner_observation = match observation_adapter.observe() {
        Ok(observation) => observation,
        Err(error) => return fail(error),
    };
    let Some(source_commit) = snapshot.head_commit.clone() else {
        return fail("source snapshot has no HEAD commit".to_string());
    };
    let input = admit_build_input(&snapshot);
    let guard = LeaseGuard {
        agent_id: agent_id.to_string(),
        target: format!("build-runner:{}", transport.node_id),
        lease_id: lease_id.to_string(),
        fencing_token,
    };
    let token = std::env::var("CYNIC_KERNEL_API_KEY").ok();
    let lease_adapter =
        match CurlBuildLeaseVerificationAdapter::new(&runner, kernel_endpoint, token) {
            Ok(adapter) => adapter,
            Err(error) => return fail(error),
        };
    let lease_verification = lease_adapter.verify(&guard);
    let runner_envelope = compose_build_runner_envelope(
        &runner_observation,
        &lease_verification,
        source_commit.clone(),
        command.to_vec(),
        artifact_path,
        apply,
        current_unix_ms(),
        &transport.node_id,
        &transport.role,
    );
    let runner_admission = if apply {
        admit_build_runner(&runner_envelope)
    } else {
        plan_build_runner(&runner_envelope)
    };
    let request = BuildRequest {
        input,
        runner_envelope,
        runner_admission,
        source_commit,
        argv: command.to_vec(),
        artifact_path: artifact_path.to_string(),
        apply,
    };
    let report = run_build(&request, &runner);
    emit_pretty_json(&report)
}

fn current_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or_default()
}
