use std::process::ExitCode;

<<<<<<< HEAD
use cynic_host::kernel_proof_persistence::{CurlKernelProofHistoryAdapter, KernelProofHistoryPort};
use cynic_host::runtime_network_preflight::observe_runtime_network_preflight;
use cynic_host::runtime_network_preflight::to_host_proof_submission;
use cynic_lifecycle::LocalArgvRunner;

use crate::emit_json;

pub(crate) fn execute_network_preflight(
    json: bool,
    hosts: &[String],
    persist: bool,
    endpoint: Option<&str>,
) -> ExitCode {
=======
use crate::emit_json;

pub(crate) fn execute_network_preflight(json: bool, hosts: &[String]) -> ExitCode {
>>>>>>> github/main
    if !json {
        return ExitCode::FAILURE;
    }

<<<<<<< HEAD
    let report = observe_runtime_network_preflight(hosts);

    if persist {
        let submission = to_host_proof_submission(&report);
        let endpoint = endpoint
            .map(str::to_string)
            .or_else(|| std::env::var("CYNIC_KERNEL_URL").ok())
            .unwrap_or_else(|| "http://127.0.0.1:8000".to_string());
        let token = cynic_secrets::env_var("kernel-api-key", "CYNIC_KERNEL_API_KEY");
        let runner = LocalArgvRunner;
        let adapter = CurlKernelProofHistoryAdapter::new(&runner, endpoint, token);
        let poh_report = adapter.persist_poh(&submission);
        emit_json(&poh_report, !poh_report.post_success)
    } else {
        emit_json(&report, report.status != "ready")
    }
=======
    let report = cynic_host::runtime_network_preflight::observe_runtime_network_preflight(hosts);
    emit_json(&report, report.status != "ready")
>>>>>>> github/main
}
