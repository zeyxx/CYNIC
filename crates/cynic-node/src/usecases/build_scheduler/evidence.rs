use cynic_build::RELEASE_BINARY_ARTIFACT_READ_BACK_SCHEMA;

use std::{collections::BTreeMap, fs, path::Path};

use cynic_host::ports::evidence_store::ContentAddressedEvidenceStorePort;
use cynic_protocol::{AgentEvidenceRef, build_evidence_envelope};
use sha2::{Digest, Sha256};

use crate::ports::build_scheduler_port::{BuildExecutionReadBack, BuildWorkspaceReadBack};

pub(super) fn persist_success_proofs(
    node_id: &str,
    workload_id: &str,
    expected_proofs: &[String],
    workspace: &BuildWorkspaceReadBack,
    execution: &BuildExecutionReadBack,
    action: &[String],
    observed_at_ms: u64,
    provenance: &[String],
    store: &dyn ContentAddressedEvidenceStorePort,
) -> Result<Vec<AgentEvidenceRef>, String> {
    let mut refs = Vec::new();
    for proof in expected_proofs {
        let payload = match proof.as_str() {
            // Owned by release_publication::publish (signed basket plus
            // store read-back). Skipped here to avoid a double persist;
            // the ref joins through the publication path, not this loop.
            "signed_release_basket_read_back" => continue,
            "build_grant_read_back" => serde_json::json!({
                "schema": "cynic.build.grant_read_back.v1",
                "workload_id": workload_id,
                "source_commit": workspace.source_commit,
                "grant_provenance": provenance,
            }),
            "canonical_gate_report" => serde_json::json!({
                "schema": "cynic.build.canonical_gate_read_back.v1",
                "source_commit": workspace.source_commit,
                "argv": action,
                "exit_code": execution.exit_code,
                "stdout": execution.stdout,
                "stderr": execution.stderr,
            }),
            "source_commit_read_back" => serde_json::json!({
                "schema": "cynic.build.source_commit_read_back.v1",
                "workspace_root": workspace.workspace_root,
                "source_commit": workspace.source_commit,
            }),
            "build_runner_observation" => serde_json::json!({
                "schema": "cynic.build.runner_execution_read_back.v1",
                "node_id": node_id,
                "argv": action,
                "working_directory": workspace.workspace_root,
                "exit_code": execution.exit_code,
            }),
            "release_binary_artifact_read_back" => serde_json::json!({
                "schema": RELEASE_BINARY_ARTIFACT_READ_BACK_SCHEMA,
                "source_commit": workspace.source_commit,
                "artifacts": release_artifacts(&workspace.workspace_root)?,
            }),
            unsupported => return Err(format!("build_proof_kind_unsupported:{unsupported}")),
        };
        let envelope = build_evidence_envelope(
            &format!("build-workload:{workload_id}:{proof}"),
            &format!("cynic-node/build-scheduler:{node_id}"),
            u128::from(observed_at_ms),
            provenance.to_vec(),
            &payload,
        )?;
        store.persist(&envelope)?;
        refs.push(AgentEvidenceRef {
            kind: proof.clone(),
            subject: workload_id.to_string(),
            digest: envelope.digest,
            lineage: None,
        });
    }
    Ok(refs)
}

pub(super) fn persist_execution_failure(
    node_id: &str,
    workload_id: &str,
    execution: &BuildExecutionReadBack,
    observed_at_ms: u64,
    provenance: Vec<String>,
    store: &dyn ContentAddressedEvidenceStorePort,
) -> Result<AgentEvidenceRef, String> {
    let envelope = build_evidence_envelope(
        &format!("build-workload:{workload_id}:failed"),
        &format!("cynic-node/build-scheduler:{node_id}"),
        u128::from(observed_at_ms),
        provenance,
        execution,
    )?;
    store.persist(&envelope)?;
    Ok(AgentEvidenceRef {
        kind: "build_runner_observation".into(),
        subject: workload_id.into(),
        digest: envelope.digest,
        lineage: None,
    })
}

/// Captures ownership, source read-back and a command-output digest before the
/// workload's terminal transition. The terminal durable record retains this
/// reference, so it forms a content-addressed local history while kernel PoH
/// is unavailable; the `promotion` value prevents it being mistaken for one.
pub(super) fn persist_execution_provenance(
    node_id: &str,
    workload_id: &str,
    repository_id: &str,
    delegation_id: &str,
    agent_id: &str,
    lease_id: &str,
    fencing_token: u64,
    previous_workload_record_digest: &str,
    workspace: &BuildWorkspaceReadBack,
    execution: &BuildExecutionReadBack,
    action: &[String],
    observed_at_ms: u64,
    provenance: Vec<String>,
    store: &dyn ContentAddressedEvidenceStorePort,
) -> Result<AgentEvidenceRef, String> {
    let payload = serde_json::json!({
        "schema": "cynic.build.execution_provenance.v1",
        "ownership": {
            "node_id": node_id,
            "agent_id": agent_id,
            "delegation_id": delegation_id,
            "workload_id": workload_id,
            "lease_id": lease_id,
            "fencing_token": fencing_token,
        },
        "source": {
            "repository_id": repository_id,
            "workspace_root": workspace.workspace_root,
            "source_commit": workspace.source_commit,
        },
        "build": {
            "argv": action,
            "exit_code": execution.exit_code,
            "stdout_sha256": format!("sha256:{}", hex::encode(Sha256::digest(execution.stdout.as_bytes()))),
            "stderr_sha256": format!("sha256:{}", hex::encode(Sha256::digest(execution.stderr.as_bytes()))),
        },
        "previous_workload_record_digest": previous_workload_record_digest,
        "promotion": "forbidden_until_kernel_poh_read_back",
    });
    let envelope = build_evidence_envelope(
        &format!("build-workload:{workload_id}:execution-provenance"),
        &format!("cynic-node/build-scheduler:{node_id}"),
        u128::from(observed_at_ms),
        provenance,
        &payload,
    )?;
    store.persist(&envelope)?;
    Ok(AgentEvidenceRef {
        kind: "build_execution_provenance".into(),
        subject: workload_id.into(),
        digest: envelope.digest,
        lineage: None,
    })
}

fn release_artifacts(workspace: &Path) -> Result<BTreeMap<String, String>, String> {
    let mut artifacts = BTreeMap::new();
    for name in cynic_protocol::runtime_release_artifact_names()
        .map_err(|error| format!("release_runtime_basket_invalid:{error}"))?
    {
        let path = workspace.join("target/release").join(&name);
        let metadata =
            fs::symlink_metadata(&path).map_err(|_| format!("release_artifact_missing:{name}"))?;
        if metadata.file_type().is_symlink() || !metadata.is_file() || metadata.len() == 0 {
            return Err(format!("release_artifact_rejected:{name}"));
        }
        let bytes = fs::read(&path).map_err(|_| format!("release_artifact_unreadable:{name}"))?;
        artifacts.insert(
            name,
            format!("sha256:{}", hex::encode(Sha256::digest(bytes))),
        );
    }
    Ok(artifacts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Default)]
    struct MemoryEvidenceStore(std::sync::Mutex<Vec<serde_json::Value>>);

    impl ContentAddressedEvidenceStorePort for MemoryEvidenceStore {
        fn persist(&self, envelope: &cynic_protocol::EvidenceEnvelope) -> Result<String, String> {
            self.0.lock().unwrap().push(envelope.payload.clone());
            Ok("memory:evidence".to_string())
        }
    }

    #[test]
    fn grant_read_back_is_a_supported_completion_proof() {
        let workspace = BuildWorkspaceReadBack {
            workspace_root: std::path::PathBuf::from("/workspace"),
            source_commit: "a".repeat(40),
            created: true,
        };
        let execution = BuildExecutionReadBack {
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
            peak_memory_bytes: None,
        };
        let store = MemoryEvidenceStore::default();
        let refs = persist_success_proofs(
            "node-1",
            "workload-1",
            &[cynic_build::BUILD_GRANT_READ_BACK_PROOF.to_string()],
            &workspace,
            &execution,
            &["make".to_string(), "check".to_string()],
            1,
            &["grant:file".to_string()],
            &store,
        )
        .expect("grant read-back proof must be supported");
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].kind, cynic_build::BUILD_GRANT_READ_BACK_PROOF);
        assert_eq!(refs[0].subject, "workload-1");
        assert!(!refs[0].digest.is_empty());
        assert_eq!(store.0.lock().unwrap().len(), 1);
        assert_eq!(
            store.0.lock().unwrap()[0]["grant_provenance"][0],
            "grant:file"
        );
    }
}
