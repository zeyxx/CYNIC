use std::{fs, process::ExitCode};

use cynic_lifecycle::LocalArgvRunner;
use cynic_protocol::{RuntimeSeedObservation, RuntimeSeedRequest, reconcile_runtime_seed};

use crate::{
    host_exec_cli::{
        args::{flag_value, has_flag},
        io::emit_json,
    },
    runtime_seed::{
        LocalRuntimeSeedActivation, RuntimeSeedActivationPort, SshRuntimeSeedSignatureVerifier,
    },
};

pub(crate) fn execute_plan(args: &[String]) -> Result<ExitCode, String> {
    let request_path = flag_value(args, "--request")
        .ok_or_else(|| "runtime-seed-plan requires --request <request.json>".to_string())?;
    let observation_path = flag_value(args, "--observation")
        .ok_or_else(|| "runtime-seed-plan requires --observation <observation.json>".to_string())?;
    let request: RuntimeSeedRequest = read_json(&request_path)?;
    let observation: RuntimeSeedObservation = read_json(&observation_path)?;
    let plan = reconcile_runtime_seed(&request, &observation);
    emit_json(&plan, matches!(plan.status.as_str(), "blocked" | "unknown"))
}

pub(crate) fn execute_reconcile(args: &[String]) -> Result<ExitCode, String> {
    let request_path = flag_value(args, "--request")
        .ok_or_else(|| "runtime-seed-reconcile requires --request <request.json>".to_string())?;
    let request: RuntimeSeedRequest = read_json(&request_path)?;
    let verifier = SshRuntimeSeedSignatureVerifier;
    let runner = LocalArgvRunner;
    let activation = LocalRuntimeSeedActivation::new(&verifier, &runner);
    let before = activation.observe(&request);
    let plan = reconcile_runtime_seed(&request, &before);
    if !has_flag(args, "--apply") || plan.status != "planned" {
        return emit_json(&plan, matches!(plan.status.as_str(), "blocked" | "unknown"));
    }
    let report = activation.apply(&request);
    let evidence = activation.persist(&request, &report);
    let projection = serde_json::json!({
        "report": report,
        "evidence_path": evidence.as_ref().ok().map(|path| path.display().to_string()),
        "evidence_error": evidence.err(),
    });
    emit_json(&projection, projection["report"]["status"] != "proved")
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str) -> Result<T, String> {
    let bytes =
        fs::read(path).map_err(|error| format!("runtime seed input read failed: {error}"))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("runtime seed input decode failed: {error}"))
}
