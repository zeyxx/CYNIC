use std::env;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;

pub(super) fn write_satisfied_host_identity_proof(path: &Path) -> Result<(), String> {
    let proof = json!({
        "schema": "cynic.runtime_proof_bundle.v1",
        "status": "satisfied",
        "proofs": [{
            "schema": "cynic.runtime_proof.v1",
            "gate": "host_identity_observed",
            "status": "satisfied",
            "producer": "runtime-validation",
            "subject": "lab-kernel",
            "observed_at_unix_ms": 1,
            "evidence": [{
                "id": "synthetic_validation",
                "status": "active",
                "detail": "validates proof consumption only"
            }]
        }]
    });
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&proof).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write host proof {}: {error}", path.display()))
}

pub(super) fn write_satisfied_host_and_network_proofs(path: &Path) -> Result<(), String> {
    let proof = json!({
        "schema": "cynic.runtime_proof_bundle.v1",
        "status": "satisfied",
        "proofs": [
            {
                "schema": "cynic.runtime_proof.v1",
                "gate": "host_identity_observed",
                "status": "satisfied",
                "producer": "runtime-validation",
                "subject": "lab-kernel",
                "observed_at_unix_ms": 1,
                "evidence": [{"id":"synthetic_host","status":"active","detail":"validates proof consumption only"}]
            },
            {
                "schema": "cynic.runtime_proof.v1",
                "gate": "network_runtime_observed",
                "status": "satisfied",
                "producer": "runtime-validation",
                "subject": "lab-kernel",
                "observed_at_unix_ms": 1,
                "evidence": [{"id":"synthetic_network","status":"mesh_active","detail":"validates proof consumption only"}]
            }
        ]
    });
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&proof).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("failed to write network proof {}: {error}", path.display()))
}

pub(super) fn write_satisfied_architecture_boundary_proof(path: &Path) -> Result<(), String> {
    let proof = json!({
        "schema": "cynic.runtime_proof_bundle.v1",
        "status": "satisfied",
        "proofs": [{
            "schema": "cynic.runtime_proof.v1",
            "gate": "architecture_boundary_verified",
            "status": "satisfied",
            "producer": "runtime-validation",
            "subject": "cynic-cli/tests/architecture_boundary.rs",
            "observed_at_unix_ms": 1,
            "evidence": [{"id":"synthetic_architecture_boundary","status":"active","detail":"validates metrics proof consumption only"}]
        }]
    });
    std::fs::write(
        path,
        serde_json::to_vec_pretty(&proof).map_err(|error| error.to_string())?,
    )
    .map_err(|error| {
        format!(
            "failed to write architecture proof {}: {error}",
            path.display()
        )
    })
}

pub(super) fn validation_temp_dir() -> Result<PathBuf, String> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_nanos();
    let path = env::temp_dir().join(format!(
        "cynic-runtime-validate-{}-{stamp}",
        std::process::id()
    ));
    std::fs::create_dir_all(&path).map_err(|error| error.to_string())?;
    Ok(path)
}
