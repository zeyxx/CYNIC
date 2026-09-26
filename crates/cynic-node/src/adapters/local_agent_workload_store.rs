use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use cynic_node_protocol::digest_json;
use cynic_workload::{
    AgentWorkloadRecord, DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA, DurableAgentWorkloadRecord,
    admit_aggregate_sibling_budget, validate_durable_agent_workload_record,
};

use crate::ports::agent_workload_store_port::{AgentWorkloadStoreError, AgentWorkloadStorePort};

#[derive(Debug, Clone)]
pub struct LocalAgentWorkloadStore {
    root: PathBuf,
}

impl LocalAgentWorkloadStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn workload_root(&self, workload_id: &str) -> Result<PathBuf, AgentWorkloadStoreError> {
        validate_id(workload_id)?;
        Ok(self.root.join(workload_id))
    }

    pub fn durable_path(
        &self,
        durable: &DurableAgentWorkloadRecord,
    ) -> Result<PathBuf, AgentWorkloadStoreError> {
        let path = self
            .workload_root(&durable.record.envelope.workload_id)?
            .join(revision_name(durable.revision));
        match read_revision(&path)? {
            Some(read_back) if read_back == *durable => Ok(path),
            _ => Err(error("workload_store_durable_path_read_back_mismatch")),
        }
    }
}

impl AgentWorkloadStorePort for LocalAgentWorkloadStore {
    fn read(
        &self,
        workload_id: &str,
    ) -> Result<Option<DurableAgentWorkloadRecord>, AgentWorkloadStoreError> {
        read_latest(&self.workload_root(workload_id)?)
    }

    fn active_idempotency_keys(&self) -> Result<Vec<String>, AgentWorkloadStoreError> {
        let mut keys = self
            .active_records()?
            .into_iter()
            .map(|record| record.envelope.idempotency_key)
            .collect::<Vec<_>>();
        keys.sort();
        keys.dedup();
        Ok(keys)
    }

    fn active_records(&self) -> Result<Vec<AgentWorkloadRecord>, AgentWorkloadStoreError> {
        let mut records = self.durable_records()?;
        records.retain(|record| !record.record.state.terminal());
        Ok(records.into_iter().map(|record| record.record).collect())
    }

    fn durable_records(&self) -> Result<Vec<DurableAgentWorkloadRecord>, AgentWorkloadStoreError> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Vec::new());
            }
            Err(_) => return Err(error("workload_store_root_unobservable")),
        };
        let mut records = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|_| error("workload_store_entry_unobservable"))?;
            if entry
                .file_type()
                .map_err(|_| error("workload_store_entry_type_unknown"))?
                .is_dir()
            {
                if let Some(record) = read_latest(&entry.path())? {
                    records.push(record);
                }
            }
        }
        records.sort_by(|left, right| {
            left.record
                .envelope
                .workload_id
                .cmp(&right.record.envelope.workload_id)
        });
        Ok(records)
    }

    fn compare_and_swap(
        &self,
        workload_id: &str,
        expected_revision: Option<u64>,
        next: &AgentWorkloadRecord,
        committed_at_ms: u64,
    ) -> Result<DurableAgentWorkloadRecord, AgentWorkloadStoreError> {
        if next.envelope.workload_id != workload_id {
            return Err(error("workload_store_identity_mismatch"));
        }
        fs::create_dir_all(&self.root).map_err(|_| error("workload_store_root_create_failed"))?;
        reject_non_directory(&self.root)?;
        let global_lock = open_lock(&self.root.join(".store.lock"))?;
        lock_exclusive(&global_lock)?;
        let root = self.workload_root(workload_id)?;
        let previous = read_latest(&root)?;
        let observed_revision = previous.as_ref().map(|record| record.revision);
        if observed_revision != expected_revision {
            return Err(AgentWorkloadStoreError::new(
                "workload_store_revision_conflict",
                format!("expected revision {expected_revision:?}, observed {observed_revision:?}"),
            ));
        }
        if expected_revision.is_none() {
            let active = self.active_records()?;
            if active
                .iter()
                .any(|record| record.envelope.idempotency_key == next.envelope.idempotency_key)
            {
                return Err(error("workload_store_duplicate_idempotency_key"));
            }
            admit_aggregate_sibling_budget(&next.envelope, &active)
                .map_err(|code| AgentWorkloadStoreError::new(&code, &code))?;
        }
        fs::create_dir(&root)
            .or_else(|failure| {
                if failure.kind() == std::io::ErrorKind::AlreadyExists {
                    Ok(())
                } else {
                    Err(failure)
                }
            })
            .map_err(|_| error("workload_store_workload_root_create_failed"))?;
        reject_non_directory(&root)?;
        let lock = open_lock(&root.join(".lock"))?;
        lock_exclusive(&lock)?;
        let revision = expected_revision
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| error("workload_store_revision_overflow"))?;
        let durable = DurableAgentWorkloadRecord {
            schema: DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
            revision,
            previous_record_digest: previous.map(|record| record.record_digest),
            record_digest: digest_json(next).map_err(|_| error("workload_store_digest_failed"))?,
            record: next.clone(),
            committed_at_ms,
        };
        if !validate_durable_agent_workload_record(&durable).is_empty() {
            return Err(error("workload_store_record_rejected"));
        }
        persist_revision(&root, &durable)?;
        let read_back = read_revision(&root.join(revision_name(revision)))?
            .ok_or_else(|| error("workload_store_read_back_missing"))?;
        if read_back != durable {
            return Err(error("workload_store_read_back_mismatch"));
        }
        let mut capabilities = vec!["durable_agent_workload_store"];
        if expected_revision.is_none() {
            capabilities.push("aggregate_sibling_budget_accounting");
        }
        if next.claim.is_some() {
            capabilities.push("lease_fenced_claim_and_recovery");
        }
        if next.completion.is_some() {
            capabilities.push("completion_evidence_persistence");
        }
        let evidence_ref = format!(
            "agent-workload-store:{}:{}",
            workload_id, read_back.record_digest
        );
        if let Err(proof_error) =
            crate::adapters::agent_execution_runtime_observer::persist_capabilities(
                &capabilities,
                &[evidence_ref],
            )
        {
            tracing::debug!(
                workload_id,
                error = %proof_error,
                "agent workload store proof publication unavailable"
            );
        }
        Ok(read_back)
    }
}

fn read_latest(root: &Path) -> Result<Option<DurableAgentWorkloadRecord>, AgentWorkloadStoreError> {
    match fs::symlink_metadata(root) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            return Err(error("workload_store_root_rejected"));
        }
        Ok(_) => {}
        Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(error("workload_store_root_unobservable")),
    }
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(error("workload_store_root_unobservable")),
    };
    let mut revisions = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|_| error("workload_store_entry_unobservable"))?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if let Some(revision) = parse_revision_name(name) {
            revisions.push((revision, entry.path()));
        }
    }
    revisions.sort_by_key(|(revision, _)| *revision);
    let mut previous: Option<DurableAgentWorkloadRecord> = None;
    for (expected_revision, path) in revisions {
        let current =
            read_revision(&path)?.ok_or_else(|| error("workload_store_revision_disappeared"))?;
        let sequential = previous.as_ref().map_or(expected_revision == 1, |record| {
            expected_revision == record.revision.saturating_add(1)
        });
        if !sequential
            || current.revision != expected_revision
            || current.previous_record_digest
                != previous.as_ref().map(|record| record.record_digest.clone())
        {
            return Err(error("workload_store_chain_invalid"));
        }
        previous = Some(current);
    }
    Ok(previous)
}

fn persist_revision(
    root: &Path,
    durable: &DurableAgentWorkloadRecord,
) -> Result<(), AgentWorkloadStoreError> {
    let bytes =
        serde_json::to_vec_pretty(durable).map_err(|_| error("workload_store_encode_failed"))?;
    let final_path = root.join(revision_name(durable.revision));
    let temp_path = root.join(format!(
        ".revision-{:020}.{}.tmp",
        durable.revision,
        std::process::id()
    ));
    write_new(&temp_path, &bytes)?;
    if fs::hard_link(&temp_path, &final_path).is_err() {
        let _ = fs::remove_file(&temp_path);
        return Err(error("workload_store_publish_failed"));
    }
    fs::remove_file(&temp_path).map_err(|_| error("workload_store_temp_cleanup_failed"))?;
    File::open(root)
        .and_then(|directory| directory.sync_all())
        .map_err(|_| error("workload_store_directory_sync_failed"))
}

fn read_revision(
    path: &Path,
) -> Result<Option<DurableAgentWorkloadRecord>, AgentWorkloadStoreError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(error("workload_store_record_unobservable")),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(error("workload_store_record_path_rejected"));
    }
    let bytes = fs::read(path).map_err(|_| error("workload_store_record_read_failed"))?;
    let record: DurableAgentWorkloadRecord =
        serde_json::from_slice(&bytes).map_err(|_| error("workload_store_record_invalid"))?;
    if !validate_durable_agent_workload_record(&record).is_empty() {
        return Err(error("workload_store_record_rejected"));
    }
    Ok(Some(record))
}

fn revision_name(revision: u64) -> String {
    format!("revision-{revision:020}.json")
}

fn parse_revision_name(name: &str) -> Option<u64> {
    name.strip_prefix("revision-")?
        .strip_suffix(".json")?
        .parse()
        .ok()
}

fn validate_id(value: &str) -> Result<(), AgentWorkloadStoreError> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(error("workload_store_identifier_invalid"));
    }
    Ok(())
}

fn reject_non_directory(path: &Path) -> Result<(), AgentWorkloadStoreError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| error("workload_store_root_unobservable"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(error("workload_store_root_rejected"));
    }
    Ok(())
}

fn open_lock(path: &Path) -> Result<File, AgentWorkloadStoreError> {
    let mut options = OpenOptions::new();
    options.read(true).write(true).create(true);
    #[cfg(unix)]
    options
        .mode(0o640)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
    let file = options
        .open(path)
        .map_err(|_| error("workload_store_lock_open_failed"))?;
    if !file
        .metadata()
        .map_err(|_| error("workload_store_lock_unobservable"))?
        .is_file()
    {
        return Err(error("workload_store_lock_rejected"));
    }
    Ok(file)
}

#[cfg(unix)]
fn lock_exclusive(file: &File) -> Result<(), AgentWorkloadStoreError> {
    file.lock().map_err(|_| error("workload_store_lock_failed"))
}

#[cfg(not(unix))]
fn lock_exclusive(_file: &File) -> Result<(), AgentWorkloadStoreError> {
    Err(error("workload_store_lock_adapter_missing"))
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), AgentWorkloadStoreError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o640);
    let mut file = options
        .open(path)
        .map_err(|_| error("workload_store_temp_create_failed"))?;
    file.write_all(bytes)
        .map_err(|_| error("workload_store_temp_write_failed"))?;
    file.sync_all()
        .map_err(|_| error("workload_store_temp_sync_failed"))
}

fn error(code: &str) -> AgentWorkloadStoreError {
    AgentWorkloadStoreError::new(code, code)
}

#[cfg(test)]
mod tests;
