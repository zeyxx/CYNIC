use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use cynic_protocol::{
    AGENT_EXECUTION_RUNTIME_OBSERVATION_SCHEMA, AgentExecutionRuntimeObservation,
};
use sha2::{Digest, Sha256};

pub const DEFAULT_AGENT_EXECUTION_RUNTIME_PROOF: &str =
    "/var/lib/cynic/agent-scheduler/runtime-proof.json";
const AGENT_EXECUTION_RUNTIME_RECEIPT_SCHEMA: &str = "cynic.agent_execution_runtime_receipt.v1";
static PUBLISH_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct AgentExecutionRuntimeReceipt {
    schema: String,
    observed_at_unix_ms: u128,
    observed_capabilities: Vec<String>,
    evidence_refs: Vec<String>,
}

/// Read the node-owned execution proof supplied by the deployed scheduler.
/// An absent or malformed proof never promotes the runtime to readiness.
pub fn observe_from_env() -> AgentExecutionRuntimeObservation {
    observe_from_path(&configured_path())
}

fn observe_from_path(path: &Path) -> AgentExecutionRuntimeObservation {
    let mut observation = match reduce_receipts(path) {
        Ok(observation) => observation,
        Err(error) => {
            let mut observation = AgentExecutionRuntimeObservation::unobserved();
            observation.unknowns.push(error);
            return observation;
        }
    };
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error)
            if error.kind() == std::io::ErrorKind::NotFound && receipt_root(path).is_dir() =>
        {
            return observation;
        }
        Err(error) => {
            let mut observation = AgentExecutionRuntimeObservation::unobserved();
            observation.unknowns.push(format!(
                "node_owned_agent_execution_proof_read_failed:{error}"
            ));
            return observation;
        }
    };
    let digest = format!("sha256:{}", hex::encode(Sha256::digest(bytes)));
    observation
        .evidence_refs
        .push(format!("agent-execution-runtime-proof:{digest}"));
    observation.evidence_refs.sort();
    observation.evidence_refs.dedup();
    observation
}

/// Merge a capability proof produced by a node-owned runtime transition into
/// the configured observation file and read it back exactly. This is the only
/// writer for the scheduler proof surface; MCP remains read-only.
pub fn persist_capabilities(
    capabilities: &[&str],
    evidence_refs: &[String],
) -> Result<AgentExecutionRuntimeObservation, String> {
    let path = configured_path();
    persist_capabilities_at(&path, capabilities, evidence_refs)
}

pub fn persist_capabilities_at(
    path: &Path,
    capabilities: &[&str],
    evidence_refs: &[String],
) -> Result<AgentExecutionRuntimeObservation, String> {
    if fs::symlink_metadata(&path)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err("node_owned_agent_execution_proof_path_symlink".to_string());
    }
    append_receipt(path, capabilities, evidence_refs)?;
    let reduced = reduce_receipts(path)?;
    write_snapshot(path, &reduced)?;
    let read_back = observe_from_path(path);
    if !capabilities.iter().all(|capability| {
        read_back
            .observed_capabilities
            .iter()
            .any(|observed| observed == capability)
    }) {
        return Err("node_owned_agent_execution_proof_read_back_missing".to_string());
    }
    Ok(read_back)
}

pub fn observe_at(path: &Path) -> AgentExecutionRuntimeObservation {
    observe_from_path(path)
}

fn append_receipt(
    path: &Path,
    capabilities: &[&str],
    evidence_refs: &[String],
) -> Result<(), String> {
    let mut receipt = AgentExecutionRuntimeReceipt {
        schema: AGENT_EXECUTION_RUNTIME_RECEIPT_SCHEMA.to_string(),
        observed_at_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
        observed_capabilities: capabilities
            .iter()
            .map(|value| (*value).to_string())
            .collect(),
        evidence_refs: evidence_refs.to_vec(),
    };
    receipt.observed_capabilities.sort();
    receipt.observed_capabilities.dedup();
    receipt.evidence_refs.sort();
    receipt.evidence_refs.dedup();
    let bytes = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("node_owned_agent_execution_receipt_encode_failed:{error}"))?;
    let digest = hex::encode(Sha256::digest(&bytes));
    let root = receipt_root(path);
    if fs::symlink_metadata(&root)
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false)
    {
        return Err("node_owned_agent_execution_receipt_root_symlink".to_string());
    }
    fs::create_dir_all(&root)
        .map_err(|error| format!("node_owned_agent_execution_receipt_parent_failed:{error}"))?;
    let final_path = root.join(format!("receipt-{digest}.json"));
    let sequence = PUBLISH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = root.join(format!(".receipt-{}-{sequence}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("node_owned_agent_execution_receipt_write_failed:{error}"))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("node_owned_agent_execution_receipt_sync_failed:{error}"))?;
    match fs::hard_link(&temporary, &final_path) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            let _ = fs::remove_file(&temporary);
            return Err(format!(
                "node_owned_agent_execution_receipt_publish_failed:{error}"
            ));
        }
    }
    fs::remove_file(&temporary)
        .map_err(|error| format!("node_owned_agent_execution_receipt_cleanup_failed:{error}"))?;
    File::open(&root)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| {
            format!("node_owned_agent_execution_receipt_directory_sync_failed:{error}")
        })
}

fn reduce_receipts(path: &Path) -> Result<AgentExecutionRuntimeObservation, String> {
    let mut observation = read_snapshot(path)?.unwrap_or_else(empty_observation);
    let root = receipt_root(path);
    let entries = match fs::read_dir(&root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(observation),
        Err(error) => {
            return Err(format!(
                "node_owned_agent_execution_receipt_read_failed:{error}"
            ));
        }
    };
    let mut paths = entries
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("node_owned_agent_execution_receipt_read_failed:{error}"))?;
    paths.sort();
    for receipt_path in paths {
        let metadata = fs::symlink_metadata(&receipt_path).map_err(|error| {
            format!("node_owned_agent_execution_receipt_metadata_failed:{error}")
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_file() {
            return Err("node_owned_agent_execution_receipt_path_unsafe".to_string());
        }
        let bytes = fs::read(&receipt_path)
            .map_err(|error| format!("node_owned_agent_execution_receipt_read_failed:{error}"))?;
        let receipt: AgentExecutionRuntimeReceipt = serde_json::from_slice(&bytes)
            .map_err(|error| format!("node_owned_agent_execution_receipt_decode_failed:{error}"))?;
        if receipt.schema != AGENT_EXECUTION_RUNTIME_RECEIPT_SCHEMA {
            return Err("node_owned_agent_execution_receipt_schema_invalid".to_string());
        }
        observation
            .observed_capabilities
            .extend(receipt.observed_capabilities);
        observation.evidence_refs.extend(receipt.evidence_refs);
        observation.evidence_refs.push(format!(
            "agent-execution-runtime-receipt:sha256:{}",
            hex::encode(Sha256::digest(bytes))
        ));
    }
    observation.observed_capabilities.sort();
    observation.observed_capabilities.dedup();
    observation.evidence_refs.sort();
    observation.evidence_refs.dedup();
    Ok(observation)
}

fn read_snapshot(path: &Path) -> Result<Option<AgentExecutionRuntimeObservation>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "node_owned_agent_execution_proof_read_failed:{error}"
            ));
        }
    };
    let observation: AgentExecutionRuntimeObservation = serde_json::from_slice(&bytes)
        .map_err(|error| format!("node_owned_agent_execution_proof_decode_failed:{error}"))?;
    if observation.schema != AGENT_EXECUTION_RUNTIME_OBSERVATION_SCHEMA {
        return Err("node_owned_agent_execution_proof_schema_invalid".to_string());
    }
    Ok(Some(observation))
}

fn write_snapshot(
    path: &Path,
    observation: &AgentExecutionRuntimeObservation,
) -> Result<(), String> {
    let bytes = serde_json::to_vec_pretty(observation)
        .map_err(|error| format!("node_owned_agent_execution_proof_encode_failed:{error}"))?;
    let parent = path
        .parent()
        .ok_or_else(|| "node_owned_agent_execution_proof_parent_missing".to_string())?;
    fs::create_dir_all(parent)
        .map_err(|error| format!("node_owned_agent_execution_proof_parent_failed:{error}"))?;
    let sequence = PUBLISH_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let temporary = path.with_extension(format!("proof-{}-{sequence}.tmp", std::process::id()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|error| format!("node_owned_agent_execution_proof_write_failed:{error}"))?;
    file.write_all(&bytes)
        .and_then(|_| file.sync_all())
        .map_err(|error| format!("node_owned_agent_execution_proof_sync_failed:{error}"))?;
    fs::rename(&temporary, path)
        .map_err(|error| format!("node_owned_agent_execution_proof_publish_failed:{error}"))?;
    File::open(parent)
        .and_then(|directory| directory.sync_all())
        .map_err(|error| format!("node_owned_agent_execution_proof_directory_sync_failed:{error}"))
}

fn receipt_root(path: &Path) -> PathBuf {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("runtime-proof");
    path.with_file_name(format!("{name}.receipts"))
}

fn empty_observation() -> AgentExecutionRuntimeObservation {
    AgentExecutionRuntimeObservation {
        schema: AGENT_EXECUTION_RUNTIME_OBSERVATION_SCHEMA.to_string(),
        observed_capabilities: Vec::new(),
        evidence_refs: Vec::new(),
        unknowns: Vec::new(),
    }
}

fn configured_path() -> PathBuf {
    std::env::var_os("CYNIC_AGENT_EXECUTION_RUNTIME_PROOF")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_AGENT_EXECUTION_RUNTIME_PROOF))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Barrier};

    use super::*;

    #[test]
    fn concurrent_transition_receipts_do_not_lose_capabilities() {
        let temp = tempfile::tempdir().expect("temporary proof root");
        let proof_path = Arc::new(temp.path().join("runtime-proof.json"));
        let barrier = Arc::new(Barrier::new(2));
        let first_path = Arc::clone(&proof_path);
        let first_barrier = Arc::clone(&barrier);
        let first = std::thread::spawn(move || {
            first_barrier.wait();
            persist_capabilities_at(
                &first_path,
                &["scheduler_claim_renewal"],
                &["proof:renewal".to_string()],
            )
        });
        let second_path = Arc::clone(&proof_path);
        let second = std::thread::spawn(move || {
            barrier.wait();
            persist_capabilities_at(
                &second_path,
                &["scheduler_cancellation_delivery"],
                &["proof:cancellation".to_string()],
            )
        });

        first
            .join()
            .expect("first writer thread")
            .expect("first receipt");
        second
            .join()
            .expect("second writer thread")
            .expect("second receipt");

        let read_back = observe_from_path(&proof_path);
        assert!(
            read_back
                .observed_capabilities
                .contains(&"scheduler_claim_renewal".to_string())
        );
        assert!(
            read_back
                .observed_capabilities
                .contains(&"scheduler_cancellation_delivery".to_string())
        );
        assert!(
            read_back
                .evidence_refs
                .contains(&"proof:renewal".to_string())
        );
        assert!(
            read_back
                .evidence_refs
                .contains(&"proof:cancellation".to_string())
        );
    }

    #[test]
    fn receipt_recovers_observation_when_crash_prevents_snapshot_publication() {
        let temp = tempfile::tempdir().expect("temporary proof root");
        let proof_path = temp.path().join("runtime-proof.json");

        append_receipt(
            &proof_path,
            &["scheduler_crash_recovery"],
            &["proof:crash-recovery".to_string()],
        )
        .expect("durable receipt");

        let read_back = observe_from_path(&proof_path);
        assert!(
            read_back
                .observed_capabilities
                .contains(&"scheduler_crash_recovery".to_string())
        );
        assert!(
            read_back
                .evidence_refs
                .contains(&"proof:crash-recovery".to_string())
        );
        assert!(read_back.unknowns.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn receipt_writer_rejects_a_symlinked_receipt_root() {
        use std::os::unix::fs::symlink;

        let temp = tempfile::tempdir().expect("temporary proof root");
        let proof_path = temp.path().join("runtime-proof.json");
        let redirected = temp.path().join("redirected");
        fs::create_dir(&redirected).expect("redirect target");
        symlink(&redirected, receipt_root(&proof_path)).expect("receipt root symlink");

        let error = append_receipt(
            &proof_path,
            &["scheduler_claim_renewal"],
            &["proof:renewal".to_string()],
        )
        .expect_err("symlinked receipt root must be rejected");

        assert_eq!(error, "node_owned_agent_execution_receipt_root_symlink");
    }
}
