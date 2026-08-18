use std::process::ExitCode;

use cynic_host::{
    adapters::agent_access::observe_linux_uid,
    agent_runtime::{
        AgentRuntimeOwnershipState, classify_runtime_ownership, observe_linux_process_identity,
    },
    runtime_authority::default_runtime_authority_paths,
};
use cynic_inference::{
    HermesGatewayService, HermesGatewayState, HermesRunExecutor, read_hermes_api_key,
};

use crate::emit_json;

pub(crate) fn execute_hermes_run(json: bool, run_id: &str) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let service = HermesGatewayService::from_process();
    let gateway = match service.status() {
        Ok(status) => status,
        Err(error) => return emit_error(&format!("hermes_supervisor_observation:{error}")),
    };
    let principal = default_runtime_authority_paths().principal.runtime_user;
    let process = match (gateway.pid, observe_linux_uid(&principal)) {
        (Some(pid), Ok(uid)) => Some(observe_linux_process_identity(pid, uid)),
        (Some(_), Err(_)) => None,
        (None, _) => None,
    };
    let ownership = classify_runtime_ownership(process.as_ref());
    if matches!(ownership.state, AgentRuntimeOwnershipState::OperatorLocal) {
        emit_json(
            &serde_json::json!({
                "status":"blocked",
                "reason":"cynic_hermes_operator_local_adoption_required",
                "gateway":gateway,
                "process":process,
                "ownership":ownership,
                "adoption_plan":{
                    "mode":"side_by_side_proof_before_cutover",
                    "mutates_host":false,
                    "steps":[
                        "observe_operator_local_runtime",
                        "declare_cynic_owned_runtime_desired_state",
                        "prove_cynic_owned_supervisor_and_process_identity",
                        "obtain_read_back_before_any_cutover"
                    ]
                }
            }),
            false,
        );
        return ExitCode::FAILURE;
    }
    if !matches!(ownership.state, AgentRuntimeOwnershipState::RuntimeOwned) {
        emit_json(
            &serde_json::json!({
                "status":"blocked",
                "reason":"cynic_hermes_process_identity_not_ready",
                "gateway":gateway,
                "process":process,
                "ownership":ownership,
            }),
            false,
        );
        return ExitCode::FAILURE;
    }
    if gateway.state != HermesGatewayState::Active || !gateway.health_ok || !gateway.capabilities_ok
    {
        emit_json(
            &serde_json::json!({
                "status":"blocked",
                "reason":"cynic_hermes_supervisor_not_ready",
                "gateway":gateway,
                "process":process,
                "ownership":ownership,
            }),
            false,
        );
        return ExitCode::FAILURE;
    }
    let Some(api_key) = read_hermes_api_key() else {
        return emit_error("hermes_api_key_unavailable");
    };
    let executor = match HermesRunExecutor::new(
        gateway.endpoint,
        api_key,
        std::time::Duration::from_secs(1),
    ) {
        Ok(executor) => executor,
        Err(error) => return emit_error(&format!("hermes_client:{error}")),
    };
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => return emit_error(&format!("tokio_runtime:{error}")),
    };
    let observation = runtime.block_on(executor.observe_run(run_id));
    let success = observation.error.is_none();
    emit_json(&observation, false);
    if success {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn emit_error(error: &str) -> ExitCode {
    emit_json(
        &serde_json::json!({"status":"blocked","error":error}),
        false,
    );
    ExitCode::FAILURE
}
