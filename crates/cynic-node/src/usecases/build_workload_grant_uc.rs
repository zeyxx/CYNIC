use std::{fs, path::Path, process::ExitCode};

use cynic_build::{
    BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE,
    SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA,
    SignedBuildWorkloadGrant,
    build_workload_grant_signing_bytes,
    derive_build_workload_grant,
    validate_build_workload_grant,
    validate_signed_build_workload_grant,
};
use cynic_protocol::{
    DurableAgentWorkloadRecord,
};

use crate::adapters::ed25519_build_workload_grant::Ed25519BuildWorkloadGrantVerifier;
use crate::ports::build_workload_grant_port::{
    BuildWorkloadGrantSignerPort, BuildWorkloadGrantVerifierPort,
};

pub fn issue_signed_build_workload_grant(
    durable: &DurableAgentWorkloadRecord,
    issuer_node_id: &str,
    now_unix_s: u64,
    signer: &dyn BuildWorkloadGrantSignerPort,
) -> Result<SignedBuildWorkloadGrant, Vec<String>> {
    let grant = derive_build_workload_grant(durable, issuer_node_id, now_unix_s)?;
    let mut signed = SignedBuildWorkloadGrant {
        schema: SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA.to_string(),
        authority_key_id: signer.authority_key_id().to_string(),
        signature_namespace: BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE.to_string(),
        grant,
        signature_hex: String::new(),
    };
    let payload = build_workload_grant_signing_bytes(&signed)
        .map_err(|_| vec!["build_workload_grant_signing_payload_failed".to_string()])?;
    signed.signature_hex = signer
        .sign(&payload)
        .map_err(|_| vec!["build_workload_grant_machine_signing_failed".to_string()])?;
    let blockers = validate_signed_build_workload_grant(&signed);
    if blockers.is_empty() {
        Ok(signed)
    } else {
        Err(blockers)
    }
}

pub fn verify_signed_build_workload_grant(
    signed: &SignedBuildWorkloadGrant,
    durable: &DurableAgentWorkloadRecord,
    observed_node_id: &str,
    observed_source_commit: &str,
    requested_action: &str,
    now_unix_s: u64,
    verifier: &dyn BuildWorkloadGrantVerifierPort,
) -> Result<(), Vec<String>> {
    let mut blockers = validate_signed_build_workload_grant(signed);
    match build_workload_grant_signing_bytes(signed) {
        Ok(payload) => {
            if verifier
                .verify(&signed.authority_key_id, &payload, &signed.signature_hex)
                .is_err()
            {
                blockers.push("build_workload_grant_signature_unverified".to_string());
            }
        }
        Err(_) => blockers.push("build_workload_grant_signing_payload_failed".to_string()),
    }
    if let Err(mut claim_blockers) = validate_build_workload_grant(
        &signed.grant,
        durable,
        observed_node_id,
        observed_source_commit,
        requested_action,
        now_unix_s,
    ) {
        blockers.append(&mut claim_blockers);
    }
    blockers.sort();
    blockers.dedup();
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers)
    }
}

pub fn execute_build_workload_grant_verify(
    grant_path: &str,
    workload_record_path: &str,
    authority_public_key_path: &str,
    observed_node_id: &str,
    observed_source_commit: &str,
    requested_action: &str,
) -> ExitCode {
    let signed = match read_json::<SignedBuildWorkloadGrant>(grant_path, "signed grant") {
        Ok(signed) => signed,
        Err(blocker) => return emit_verification(None, vec![blocker]),
    };
    let durable = match read_json::<DurableAgentWorkloadRecord>(
        workload_record_path,
        "durable workload record",
    ) {
        Ok(durable) => durable,
        Err(blocker) => return emit_verification(Some(&signed.grant.grant_id), vec![blocker]),
    };
    let verifier = match Ed25519BuildWorkloadGrantVerifier::from_public_key_file(Path::new(
        authority_public_key_path,
    )) {
        Ok(verifier) => verifier,
        Err(_) => {
            return emit_verification(
                Some(&signed.grant.grant_id),
                vec!["build_workload_grant_trust_root_unavailable".to_string()],
            );
        }
    };
    let now_unix_s = chrono::Utc::now().timestamp().max(0) as u64;
    let blockers = verify_signed_build_workload_grant(
        &signed,
        &durable,
        observed_node_id,
        observed_source_commit,
        requested_action,
        now_unix_s,
        &verifier,
    )
    .err()
    .unwrap_or_default();
    emit_verification(Some(&signed.grant.grant_id), blockers)
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str, label: &str) -> Result<T, String> {
    let metadata = fs::symlink_metadata(path).map_err(|_| format!("{label}_unobservable"))?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(format!("{label}_path_rejected"));
    }
    let bytes = fs::read(path).map_err(|_| format!("{label}_read_failed"))?;
    serde_json::from_slice(&bytes).map_err(|_| format!("{label}_invalid"))
}

fn emit_verification(grant_id: Option<&str>, blockers: Vec<String>) -> ExitCode {
    let admitted = blockers.is_empty();
    let report = serde_json::json!({
        "schema": "cynic.node.build_workload_grant_verification.v1",
        "status": if admitted { "proved" } else { "blocked" },
        "grant_id": grant_id,
        "blockers": blockers,
        "secret_exposed": false
    });
    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(_) => {
            eprintln!("build workload grant verification report serialization failed");
            return ExitCode::FAILURE;
        }
    }
    if admitted {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
