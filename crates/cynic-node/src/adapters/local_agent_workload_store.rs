use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use cynic_protocol::digest_json;
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

    fn archive_terminal(
        &self,
        workload_id: &str,
        older_than_ms: u64,
        now_ms: u64,
    ) -> Result<Option<String>, AgentWorkloadStoreError> {
        let root = self.workload_root(workload_id)?;
        reject_non_directory(&root)?;
        let record = match read_latest(&root)? {
            Some(record) => record,
            None => return Ok(None),
        };
        if !record.record.state.terminal() {
            return Ok(None);
        }
        let age_ms = now_ms.saturating_sub(record.committed_at_ms);
        if age_ms < older_than_ms {
            return Ok(None);
        }
        let archive_root = self.root.join("archive");
        fs::create_dir_all(&archive_root)
            .map_err(|_| error("workload_store_archive_root_create_failed"))?;
        let archive_path = archive_root.join(workload_id);
        fs::rename(&root, &archive_path)
            .map_err(|_| error("workload_store_archive_move_failed"))?;
        if root.exists() {
            return Err(error("workload_store_archive_read_back_failed"));
        }
        Ok(Some(archive_path.to_string_lossy().into_owned()))
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
mod tests {
    use cynic_protocol::{
        AGENT_DELEGATION_ENVELOPE_SCHEMA, AgentBudget, AgentCapabilityGrant,
        AgentDelegationEnvelope, AgentEvidenceRef, EXECUTION_AUTHORIZATION_SCHEMA,
        ExecutionAuthorization, MutationContract, POLICY_ADMISSION_REPORT_SCHEMA,
        PolicyAdmissionReport, PolicyAuthority, PolicyDecision, PolicyIntentRef, PolicyScope,
        PolicySetRef, RequiredProof, digest_json,
    };
    use cynic_workload::{
        AGENT_WORKLOAD_ENVELOPE_SCHEMA, AGENT_WORKLOAD_RECORD_SCHEMA, AgentBoundedAction,
        AgentWorkloadEnvelope, AgentWorkloadRecord, AgentWorkloadState,
    };

    use super::*;

    fn valid_envelope(workload_id: &str) -> AgentWorkloadEnvelope {
        let action = AgentBoundedAction {
            argv: vec!["make".into(), "check".into()],
            working_directory: Some("/source".into()),
            input_digest: Some(format!("sha256:{}", "b".repeat(64))),
            mutates: true,
            requires_privilege: false,
            stop_condition: "stop on first non-zero exit".into(),
            rollback_intent: "discard isolated checkout".into(),
        };
        let plan_digest = digest_json(&action).expect("action digest");
        let mutation = MutationContract {
            kind: "repository_validation".into(),
            scope: PolicyScope {
                domain: "build".into(),
                target: "commit-a".into(),
            },
            mutates: true,
            requires_privilege: false,
            requires_explicit_apply: true,
            required_authority: PolicyAuthority::RuntimePrincipal,
            preconditions: vec!["source_commit_observed".into()],
            rollback_intent: "discard isolated checkout".into(),
            expected_proofs: vec![RequiredProof {
                gate: "canonical_gate_report".into(),
                subject: "commit-a".into(),
            }],
        };
        AgentWorkloadEnvelope {
            schema: AGENT_WORKLOAD_ENVELOPE_SCHEMA.into(),
            workload_id: workload_id.into(),
            idempotency_key: format!("idem-{workload_id}"),
            plan_digest: plan_digest.clone(),
            mandate_binding: None,
            input: None,
            resource_budget: None,
            action,
            delegation: AgentDelegationEnvelope {
                schema: AGENT_DELEGATION_ENVELOPE_SCHEMA.into(),
                delegation_id: format!("delegation-{workload_id}"),
                node_id: "test-node".into(),
                agent_id: "test-agent".into(),
                parent_agent_id: None,
                depth: 0,
                objective: "run test".into(),
                authority_snapshot: AgentEvidenceRef {
                    kind: "policy".into(),
                    subject: workload_id.into(),
                    digest: format!("sha256:{}", "c".repeat(64)),
                    lineage: None,
                },
                context_refs: vec![],
                capabilities: vec![AgentCapabilityGrant {
                    id: "repository_validation".into(),
                    mutates: true,
                }],
                budget: AgentBudget {
                    context_tokens: 1_000,
                    actions: 2,
                    children: 0,
                    wall_time_ms: 300_000,
                    cpu_ms: None,
                },
                expected_proofs: vec!["canonical_gate_report".into()],
            },
            parent_delegation: None,
            policy_admission: PolicyAdmissionReport {
                schema: POLICY_ADMISSION_REPORT_SCHEMA.into(),
                policy_set: PolicySetRef {
                    id: "test.policy".into(),
                    version: "v1".into(),
                    digest: format!("sha256:{}", "e".repeat(64)),
                },
                intent: PolicyIntentRef {
                    id: format!("intent-{workload_id}"),
                    kind: "repository_validation".into(),
                    actor: "test-agent".into(),
                    subject: "commit-a".into(),
                    digest: format!("sha256:{}", "f".repeat(64)),
                    plan_digest: plan_digest.clone(),
                },
                mutation,
                decision: PolicyDecision::Allow {
                    required_authority: PolicyAuthority::RuntimePrincipal,
                    required_proofs: vec![RequiredProof {
                        gate: "canonical_gate_report".into(),
                        subject: "commit-a".into(),
                    }],
                },
            },
            execution_authorization: ExecutionAuthorization {
                schema: EXECUTION_AUTHORIZATION_SCHEMA.into(),
                actor: "test-agent".into(),
                authority: PolicyAuthority::RuntimePrincipal,
                explicit_apply: true,
                satisfied_preconditions: vec!["source_commit_observed".into()],
            },
        }
    }

    fn terminal_record(
        workload_id: &str,
        state: AgentWorkloadState,
        committed_at_ms: u64,
    ) -> DurableAgentWorkloadRecord {
        let record = AgentWorkloadRecord {
            schema: AGENT_WORKLOAD_RECORD_SCHEMA.into(),
            envelope: valid_envelope(workload_id),
            state,
            claim: None,
            last_fencing_token: 0,
            cancellation_generation: 0,
            consumed_actions: 0,
            completion: None,
        };
        let record_digest = digest_json(&record).expect("record digest");
        DurableAgentWorkloadRecord {
            schema: DURABLE_AGENT_WORKLOAD_RECORD_SCHEMA.to_string(),
            revision: 1,
            previous_record_digest: None,
            record_digest,
            record,
            committed_at_ms,
        }
    }

    fn write_revision(root: &Path, revision: u64, record: &DurableAgentWorkloadRecord) {
        let path = root.join(revision_name(revision));
        let bytes = serde_json::to_vec_pretty(record).unwrap();
        fs::write(&path, &bytes).unwrap();
    }

    #[test]
    fn debug_validate_record_violations() {
        let record = terminal_record("test-workload", AgentWorkloadState::Completed, 1000);
        let bytes = serde_json::to_vec_pretty(&record).unwrap();
        let result: Result<DurableAgentWorkloadRecord, _> = serde_json::from_slice(&bytes);
        match result {
            Ok(deserialized) => {
                let violations = cynic_workload::validate_durable_agent_workload_record(&deserialized);
                panic!("deserialized OK, violations: {:?}", violations);
            }
            Err(e) => panic!("deserialization failed: {}", e),
        }
    }

    #[test]
    fn archive_terminal_moves_directory_when_terminal_and_old() {
        let tmp = std::env::temp_dir().join(format!("cynic-test-{}", std::process::id()));
        let store = LocalAgentWorkloadStore::new(&tmp);
        let workload_id = "test-workload";

        let root = tmp.join(workload_id);
        fs::create_dir_all(&root).unwrap();

        let record = terminal_record(workload_id, AgentWorkloadState::Completed, 1000);
        write_revision(&root, 1, &record);

        let archive_path = store.archive_terminal(workload_id, 500, 2000).unwrap();

        assert!(
            archive_path.is_some(),
            "expected Some(path) for terminal+old workload"
        );
        let path_str = archive_path.unwrap();
        assert!(
            path_str.contains("archive"),
            "path should contain archive dir"
        );
        assert!(
            Path::new(&path_str).exists(),
            "archive directory should exist"
        );
        assert!(!root.exists(), "original directory should be moved");

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn archive_terminal_returns_none_when_live() {
        let tmp = std::env::temp_dir().join(format!("cynic-test-{}", std::process::id()));
        let store = LocalAgentWorkloadStore::new(&tmp);
        let workload_id = "test-workload";

        let root = tmp.join(workload_id);
        fs::create_dir_all(&root).unwrap();

        let record = terminal_record(workload_id, AgentWorkloadState::Dispatched, 1000);
        write_revision(&root, 1, &record);

        let result = store.archive_terminal(workload_id, 500, 2000).unwrap();

        assert!(result.is_none(), "expected None for live workload");
        assert!(
            root.exists(),
            "directory should still exist for live workload"
        );

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn archive_terminal_returns_none_when_missing() {
        let tmp = std::env::temp_dir().join(format!("cynic-test-{}", std::process::id()));
        let store = LocalAgentWorkloadStore::new(&tmp);

        let result = store.archive_terminal("nonexistent", 500, 2000).unwrap();

        assert!(result.is_none(), "expected None for missing workload");

        fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn archive_terminal_returns_none_when_terminal_but_fresh() {
        let tmp = std::env::temp_dir().join(format!("cynic-test-{}", std::process::id()));
        let store = LocalAgentWorkloadStore::new(&tmp);
        let workload_id = "test-workload";

        let root = tmp.join(workload_id);
        fs::create_dir_all(&root).unwrap();

        let record = terminal_record(workload_id, AgentWorkloadState::Completed, 1500);
        write_revision(&root, 1, &record);

        let result = store.archive_terminal(workload_id, 500, 2000).unwrap();

        assert!(
            result.is_none(),
            "expected None for terminal but fresh workload"
        );
        assert!(
            root.exists(),
            "directory should still exist for fresh workload"
        );

        fs::remove_dir_all(&tmp).ok();
    }
}
