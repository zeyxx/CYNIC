use std::process::ExitCode;

use cynic_host::kernel_proof_persistence::{
    CurlKernelProofPersistenceAdapter, KernelProofPersistencePort,
};
use cynic_lifecycle::LocalArgvRunner;
use cynic_protocol::RuntimeProofPersistenceRecord;

use crate::cli_json::emit_pretty_json;
use crate::fail;

pub(crate) fn execute_persistence(
    json: bool,
    proof_path: &str,
    endpoint: Option<&str>,
) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    let raw = match std::fs::read_to_string(proof_path) {
        Ok(raw) => raw,
        Err(error) => return fail(format!("failed to read persistence record: {error}")),
    };
    let record = match serde_json::from_str::<RuntimeProofPersistenceRecord>(&raw) {
        Ok(record) => record,
        Err(error) => return fail(format!("failed to parse persistence record: {error}")),
    };
    let endpoint = endpoint
        .map(str::to_string)
        .or_else(|| std::env::var("CYNIC_KERNEL_URL").ok())
        .unwrap_or_else(|| "http://127.0.0.1:8000".to_string());
    let token = std::env::var("CYNIC_KERNEL_API_KEY").ok();
    let runner = LocalArgvRunner;
    let adapter = CurlKernelProofPersistenceAdapter::new(&runner, endpoint, token);
    let report = adapter.persist(&record);
    emit_pretty_json(&report)
}
