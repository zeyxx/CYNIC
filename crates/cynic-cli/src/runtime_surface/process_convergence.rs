use std::path::PathBuf;
use std::process::ExitCode;

use cynic_host::adapters::{
    local_causal_epoch_index::LocalCausalEpochIndexStore,
    local_evidence_store::LocalContentAddressedEvidenceStore,
};
use cynic_host::process_convergence::project_process_convergence_with_freshness;
use cynic_protocol::{
    ConvergenceStatus, EVIDENCE_FRESHNESS_POLICY_SCHEMA, EvidenceFreshnessPolicy,
};

use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};
use crate::{emit_json, fail};

pub(crate) fn execute_process_convergence(
    json: bool,
    subject: &str,
    vertical: &str,
    history_limit: usize,
    index_root: Option<&str>,
    evidence_root: Option<&str>,
) -> ExitCode {
    if !json {
        return fail("process convergence projection requires --json");
    }
    if subject.trim().is_empty() || vertical.trim().is_empty() {
        return fail("process convergence projection requires --subject and --vertical");
    }

    let runtime = resolve_runtime_paths(&RuntimeContext::from_env(None));
    let index_root = index_root
        .map(PathBuf::from)
        .unwrap_or_else(|| runtime.state_root.join("process-convergence/index"));
    let evidence_root = evidence_root
        .map(PathBuf::from)
        .unwrap_or_else(|| runtime.state_root.join("evidence"));
    let index_store = LocalCausalEpochIndexStore::new(index_root);
    let evidence_store = LocalContentAddressedEvidenceStore::new(evidence_root);

    let freshness_policy = EvidenceFreshnessPolicy {
        schema: EVIDENCE_FRESHNESS_POLICY_SCHEMA.to_string(),
        max_age_ms: 300_000,
        allow_future_skew_ms: 5_000,
    };
    match project_process_convergence_with_freshness(
        subject,
        vertical,
        history_limit,
        &index_store,
        &evidence_store,
        Some(unix_now_ms()),
        Some(freshness_policy),
    ) {
        Ok(projection) => emit_json(
            &projection,
            matches!(
                projection.status,
                ConvergenceStatus::Blocked | ConvergenceStatus::Unknown
            ),
        ),
        Err(error) => fail(error),
    }
}

fn unix_now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}
