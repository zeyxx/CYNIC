use std::{
    fs,
    process::ExitCode,
    time::{SystemTime, UNIX_EPOCH},
};

use cynic_node::{
    local_agent_session::{LocalAgentIdentity, LocalAgentSessionClient, declared_kernel_socket},
    usecases::build_workload_observation_uc::observe_build_workload,
    usecases::build_workload_submission_uc::{
        blocked_build_workload_submission, plan_build_workload_submission,
        submit_build_workload_submission,
    },
    usecases::runtime_release_declaration_uc::{
        apply_runtime_release_declaration, plan_runtime_release_declaration,
        prepare_runtime_release_declaration,
    },
};
use serde_json::json;

use crate::cli_json::emit_pretty_json;

pub(super) fn submit(
    request_path: String,
    registry: String,
    workspace_root: String,
    apply: bool,
) -> ExitCode {
    let request = match fs::read_to_string(&request_path)
        .map_err(|error| format!("read workload request: {error}"))
        .and_then(|raw| {
            serde_json::from_str::<cynic_build::BuildAgentRequest>(&raw)
                .map_err(|error| format!("decode workload request: {error}"))
        }) {
        Ok(mut request) => {
            if !apply {
                request.mutation_authorized = false;
            }
            request
        }
        Err(error) => {
            return emit_pretty_json(
                &json!({"schema":"cynic.cli.build_workload_submission.v1","status":"blocked","mutates":false,"blockers":[error]}),
            );
        }
    };
    let submitted_at_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(u128::from(u64::MAX)) as u64;
    let plan = match plan_build_workload_submission(
        &request,
        &registry,
        &workspace_root,
        submitted_at_ms,
    ) {
        Ok(plan) => plan,
        Err(error) => {
            return emit_pretty_json(
                &json!({"schema":"cynic.cli.build_workload_submission.v1","status":"blocked","mutates":false,"request":request,"blockers":[error]}),
            );
        }
    };
    let Some(delivery) = plan.delivery else {
        return emit_pretty_json(
            &json!({"schema":"cynic.cli.build_workload_submission.v1","status":plan.admission.status,"mutates":false,"request":request,"admission":plan.admission,"next_action":"review the admission; repeat with --apply only when mutation is authorized"}),
        );
    };
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(_) => {
            return emit_pretty_json(
                &json!({"schema":"cynic.cli.build_workload_submission.v1","status":"blocked","mutates":false,"blockers":["local_async_runtime_unavailable"]}),
            );
        }
    };
    let result = runtime.block_on(async {
        let identity = LocalAgentIdentity::resolve("cli")?;
        let session =
            LocalAgentSessionClient::bootstrap(declared_kernel_socket(), &identity).await?;
        submit_build_workload_submission(&session, &delivery).await
    });
    match result {
        Ok(result) => {
            let read_back = result.into_read_back_report(delivery);
            emit_pretty_json(
                &json!({"schema":"cynic.cli.build_workload_submission.v1","status":read_back.status,"mutates":read_back.mutation_attempted,"request":request,"admission":plan.admission,"submission_read_back":read_back}),
            )
        }
        Err(blocker) => {
            let submission_read_back = blocked_build_workload_submission(delivery, blocker);
            emit_pretty_json(
                &json!({"schema":"cynic.cli.build_workload_submission.v1","status":submission_read_back.status,"mutates":submission_read_back.mutation_attempted,"request":request,"admission":plan.admission,"submission_read_back":submission_read_back}),
            )
        }
    }
}

pub(super) fn status(workload_id: String) -> ExitCode {
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(_) => {
            return emit_pretty_json(
                &json!({"schema":"cynic.cli.build_workload_observation.v1","status":"blocked","mutates":false,"workload_id":workload_id,"blockers":["local_async_runtime_unavailable"]}),
            );
        }
    };
    let result = runtime.block_on(async {
        let identity = LocalAgentIdentity::resolve("cli")?;
        let session =
            LocalAgentSessionClient::bootstrap(declared_kernel_socket(), &identity).await?;
        observe_build_workload(&session, &workload_id).await
    });
    match result {
        Ok(result) => emit_pretty_json(
            &json!({"schema":"cynic.cli.build_workload_observation.v1","status":if result.read_back_violations.is_empty(){"observed"}else{"blocked_delivery_read_back"},"mutates":false,"workload_id":workload_id,"delivery":result.delivery,"delivery_read_back":result.read_back.body,"delivery_read_back_violations":result.read_back_violations,"kernel_http_status":result.read_back.http_status}),
        ),
        Err(blocker) => emit_pretty_json(
            &json!({"schema":"cynic.cli.build_workload_observation.v1","status":"blocked_disconnected","mutates":false,"workload_id":workload_id,"blockers":[blocker]}),
        ),
    }
}

pub(super) fn runtime_release_declare(
    workload_id: String,
    desired_state_path: String,
    apply: bool,
) -> ExitCode {
    let desired_state = match fs::read_to_string(&desired_state_path)
        .map_err(|error| format!("read desired state: {error}"))
        .and_then(|raw| {
            serde_json::from_str::<cynic_protocol::NodeDesiredState>(&raw)
                .map_err(|error| format!("decode desired state: {error}"))
        }) {
        Ok(desired_state) => desired_state,
        Err(blocker) => {
            return emit_pretty_json(
                &json!({"schema":"cynic.cli.runtime_release_declaration.v1","status":"blocked","mutates":false,"blockers":[blocker]}),
            );
        }
    };
    let identity = match LocalAgentIdentity::resolve("cli") {
        Ok(identity) => identity,
        Err(blocker) => {
            return emit_pretty_json(
                &json!({"schema":"cynic.cli.runtime_release_declaration.v1","status":"blocked","mutates":false,"blockers":[blocker]}),
            );
        }
    };
    let declaration = match prepare_runtime_release_declaration(
        identity.agent_id.clone(),
        workload_id,
        desired_state,
    ) {
        Ok(declaration) => declaration,
        Err(blocker) => {
            return emit_pretty_json(
                &json!({"schema":"cynic.cli.runtime_release_declaration.v1","status":"blocked","mutates":false,"blockers":[blocker]}),
            );
        }
    };
    if !apply {
        return emit_pretty_json(&plan_runtime_release_declaration(declaration));
    }
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(_) => {
            return emit_pretty_json(
                &json!({"schema":"cynic.cli.runtime_release_declaration.v1","status":"blocked","mutates":false,"blockers":["local_async_runtime_unavailable"]}),
            );
        }
    };
    let result = runtime.block_on(async {
        let session =
            LocalAgentSessionClient::bootstrap(declared_kernel_socket(), &identity).await?;
        apply_runtime_release_declaration(&session, declaration).await
    });
    match result {
        Ok(report) => emit_pretty_json(&report),
        Err(blocker) => emit_pretty_json(
            &json!({"schema":"cynic.cli.runtime_release_declaration.v1","status":"blocked_disconnected","mutates":false,"blockers":[blocker]}),
        ),
    }
}
