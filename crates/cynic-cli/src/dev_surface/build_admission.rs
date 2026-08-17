//! Non-secret build-admission readiness projection for operators.

use cynic_protocol::{CynicError, CynicErrorCode};
use serde::Serialize;
use std::process::Command;

#[derive(Debug, Serialize)]
pub(crate) struct BuildAdmissionReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mode: &'static str,
    pub node_id: String,
    pub source_commit: Option<String>,
    pub verifier: String,
    pub verifier_present: bool,
    pub grant_present: bool,
    pub workload_record_present: bool,
    pub authority_public_key_present: bool,
    pub bootstrap_receipt_present: bool,
    pub blockers: Vec<&'static str>,
    pub errors: Vec<CynicError>,
    pub next_action: &'static str,
}

pub(crate) fn observe() -> BuildAdmissionReport {
    let verifier = std::env::var("CYNIC_BUILD_ADMISSION_VERIFIER")
        .unwrap_or_else(|_| "cynic-node".to_string());
    let verifier_present = Command::new(&verifier)
        .arg("--help")
        .output()
        .is_ok_and(|output| output.status.success());
    let grant_present = non_empty_env("CYNIC_BUILD_ADMISSION_GRANT");
    let workload_record_present = non_empty_env("CYNIC_BUILD_WORKLOAD_RECORD");
    let authority_public_key_present = non_empty_env("CYNIC_BUILD_AUTHORITY_PUBLIC_KEY");
    let bootstrap_receipt_present = non_empty_env("CYNIC_BUILD_BOOTSTRAP_RECEIPT");
    let source_commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
        .filter(|commit| !commit.is_empty());
    let node_id = std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| {
            Command::new("hostname")
                .output()
                .ok()
                .filter(|output| output.status.success())
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
                .filter(|value| !value.is_empty())
        })
        .unwrap_or_else(|| "unknown".into());

    let mut blockers = Vec::new();
    let mut errors = Vec::new();
    if !verifier_present {
        blockers.push("admission_verifier_unavailable");
        errors.push(CynicError::new(
            CynicErrorCode::BuildAdmissionVerifierUnavailable,
        ));
    }
    let mode = if bootstrap_receipt_present {
        "bootstrap_receipt"
    } else if grant_present && workload_record_present && authority_public_key_present {
        "signed_grant"
    } else {
        "missing_authority_material"
    };
    if mode == "missing_authority_material" {
        blockers.push("signed_grant_or_bootstrap_receipt_missing");
        errors.push(CynicError::new(
            CynicErrorCode::BuildAdmissionAuthorityMaterialMissing,
        ));
    }
    if source_commit.is_none() {
        blockers.push("source_commit_unobservable");
        errors.push(CynicError::new(
            CynicErrorCode::BuildSourceCommitUnobservable,
        ));
    }
    if node_id == "unknown" {
        blockers.push("node_identity_unobservable");
        errors.push(CynicError::new(
            CynicErrorCode::BuildNodeIdentityUnobservable,
        ));
    }
    let ready = blockers.is_empty();
    BuildAdmissionReport {
        schema: "cynic.build_admission_readiness.v1",
        status: if ready { "ready" } else { "blocked" },
        mode,
        node_id,
        source_commit,
        verifier,
        verifier_present,
        grant_present,
        workload_record_present,
        authority_public_key_present,
        bootstrap_receipt_present,
        blockers,
        errors,
        next_action: if ready {
            "run make build or make build-release with the admitted environment"
        } else {
            "obtain a signed build grant/workload record/public key from the CYNIC admission authority, or a bootstrap receipt"
        },
    }
}

fn non_empty_env(name: &str) -> bool {
    std::env::var(name)
        .map(|value| !value.trim().is_empty())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_never_contains_secret_values() {
        let report = observe();
        let encoded = serde_json::to_string(&report).expect("report json");
        assert!(!encoded.contains("CYNIC_BUILD_ADMISSION_GRANT="));
        assert!(encoded.contains("cynic.build_admission_readiness.v1"));
    }
}
