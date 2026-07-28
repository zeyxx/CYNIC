use std::path::Path;

use cynic_host::{
    adapters::local_evidence_store::LocalContentAddressedEvidenceStore,
    ports::evidence_store::ContentAddressedEvidenceStorePort,
};
use cynic_protocol::{
    LeaseGuard, RUNTIME_PROOF_BUNDLE_SCHEMA, RUNTIME_PROOF_SCHEMA, RuntimeProof,
    RuntimeProofBundle, RuntimeProofEvidence, SourceRepositoryBootstrapProofAuthority,
    SourceRepositoryBootstrapReport, SourceRepositoryBootstrapStatus, build_evidence_envelope,
};

pub fn persist_local_proof(
    mut report: SourceRepositoryBootstrapReport,
    state_root: &Path,
    observed_at_unix_ms: u128,
) -> SourceRepositoryBootstrapReport {
    let provenance = match report
        .after
        .as_ref()
        .and_then(|observation| observation.observed_revision.as_deref())
    {
        Some(revision) => vec![format!("git:{revision}")],
        None => {
            return mark_unknown(report, "source_bootstrap_proof_revision_missing");
        }
    };
    report = mark_unknown(report, "source_bootstrap_kernel_commit_missing");
    let evidence = match build_evidence_envelope(
        &format!(
            "source-repository:{}:{}",
            report.request.node_id, report.request.repository_id
        ),
        "cynic-node/source-bootstrap",
        observed_at_unix_ms,
        provenance,
        &report,
    ) {
        Ok(evidence) => evidence,
        Err(failure) => {
            return mark_unknown(report, format!("source_bootstrap_evidence_build:{failure}"));
        }
    };
    let store = LocalContentAddressedEvidenceStore::new(state_root.join("evidence"));
    match store.persist(&evidence) {
        Ok(local_ref) => {
            report.proof_ref = Some(evidence.digest.clone());
            report.proof_authority = SourceRepositoryBootstrapProofAuthority::LocalContentAddressed;
            report.evidence_refs.push(evidence.digest);
            report.evidence_refs.push(local_ref);
        }
        Err(failure) => {
            report = mark_unknown(
                report,
                format!("source_bootstrap_evidence_persist:{failure}"),
            );
        }
    }
    report
}

pub fn runtime_proof_bundle(
    report: &SourceRepositoryBootstrapReport,
    guard: &LeaseGuard,
    observed_at_unix_ms: u128,
) -> Result<RuntimeProofBundle, String> {
    let evidence_id = report
        .proof_ref
        .as_deref()
        .ok_or_else(|| "source_bootstrap_local_proof_missing".to_string())?;
    let detail = serde_json::to_string(report)
        .map_err(|failure| format!("source_bootstrap_report_serialize:{failure}"))?;
    Ok(RuntimeProofBundle {
        schema: RUNTIME_PROOF_BUNDLE_SCHEMA.to_string(),
        status: "satisfied".to_string(),
        proofs: vec![RuntimeProof {
            schema: RUNTIME_PROOF_SCHEMA.to_string(),
            gate: "source_repository_bootstrap_converged".to_string(),
            status: "satisfied".to_string(),
            producer: "cynic-node/source-bootstrap".to_string(),
            subject: guard.target.clone(),
            observed_at_unix_ms,
            evidence: vec![RuntimeProofEvidence {
                id: evidence_id.to_string(),
                status: "observed".to_string(),
                detail,
            }],
        }],
    })
}

pub fn mark_kernel_committed(
    mut report: SourceRepositoryBootstrapReport,
    proof_ref: String,
) -> SourceRepositoryBootstrapReport {
    report.status = SourceRepositoryBootstrapStatus::Completed;
    report.proof_ref = Some(proof_ref);
    report.proof_authority = SourceRepositoryBootstrapProofAuthority::KernelCommitted;
    report
        .unknowns
        .retain(|unknown| unknown != "source_bootstrap_kernel_commit_missing");
    report
}

pub fn mark_unknown(
    mut report: SourceRepositoryBootstrapReport,
    unknown: impl Into<String>,
) -> SourceRepositoryBootstrapReport {
    report.status = SourceRepositoryBootstrapStatus::Unknown;
    let unknown = unknown.into();
    if !report.unknowns.contains(&unknown) {
        report.unknowns.push(unknown);
    }
    report
}
