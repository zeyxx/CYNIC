use std::{
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use cynic_protocol::{
    AgentSchedulerJournalRecord, AgentSchedulerJournalStatus,
    validate_agent_scheduler_journal_record,
};

use crate::ports::{
    agent_scheduler_port::AgentSchedulerJournalPort, agent_workspace_port::AgentWorkspacePortError,
};

#[derive(Debug, Clone)]
pub struct LocalAgentSchedulerJournal {
    root: PathBuf,
}

impl LocalAgentSchedulerJournal {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn step_root(
        &self,
        node_id: &str,
        epoch: u64,
        step_id: &str,
    ) -> Result<PathBuf, AgentWorkspacePortError> {
        validate_id(node_id)?;
        validate_id(step_id)?;
        if epoch == 0 {
            return Err(error("scheduler_journal_epoch_invalid"));
        }
        Ok(self
            .root
            .join(node_id)
            .join(epoch.to_string())
            .join(step_id))
    }
}

impl AgentSchedulerJournalPort for LocalAgentSchedulerJournal {
    fn records(
        &self,
        node_id: &str,
        epoch: u64,
        step_id: &str,
    ) -> Result<Vec<AgentSchedulerJournalRecord>, AgentWorkspacePortError> {
        let root = self.step_root(node_id, epoch, step_id)?;
        let mut records = Vec::new();
        for name in ["plan.json", "result.json"] {
            if let Some(record) = read_record(&root.join(name))? {
                if record.node_id != node_id
                    || record.scheduler_epoch != epoch
                    || record.step_id != step_id
                {
                    return Err(error("scheduler_journal_record_identity_mismatch"));
                }
                records.push(record);
            }
        }
        Ok(records)
    }

    fn persist(
        &self,
        record: &AgentSchedulerJournalRecord,
    ) -> Result<AgentSchedulerJournalRecord, AgentWorkspacePortError> {
        if !validate_agent_scheduler_journal_record(record).is_empty() {
            return Err(error("scheduler_journal_record_rejected"));
        }
        let root = self.step_root(&record.node_id, record.scheduler_epoch, &record.step_id)?;
        fs::create_dir_all(&root).map_err(|_| error("scheduler_journal_root_create_failed"))?;
        reject_non_directory(&root)?;
        let name = if record.status == AgentSchedulerJournalStatus::Planned {
            "plan.json"
        } else {
            "result.json"
        };
        let final_path = root.join(name);
        if let Some(existing) = read_record(&final_path)? {
            return if equivalent(&existing, record) {
                Ok(existing)
            } else {
                Err(error("scheduler_journal_immutable_conflict"))
            };
        }
        let bytes = serde_json::to_vec_pretty(record)
            .map_err(|_| error("scheduler_journal_encode_failed"))?;
        let temp = root.join(format!(".{name}.{}.tmp", std::process::id()));
        write_new(&temp, &bytes)?;
        match fs::hard_link(&temp, &final_path) {
            Ok(()) => {}
            Err(failure) if failure.kind() == ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&temp);
                return match read_record(&final_path)? {
                    Some(existing) if equivalent(&existing, record) => Ok(existing),
                    Some(_) => Err(error("scheduler_journal_immutable_conflict")),
                    None => Err(error("scheduler_journal_race_unknown")),
                };
            }
            Err(_) => {
                let _ = fs::remove_file(&temp);
                return Err(error("scheduler_journal_publish_failed"));
            }
        }
        fs::remove_file(&temp).map_err(|_| error("scheduler_journal_temp_cleanup_failed"))?;
        File::open(&root)
            .and_then(|directory| directory.sync_all())
            .map_err(|_| error("scheduler_journal_directory_sync_failed"))?;
        read_record(&final_path)?.ok_or_else(|| error("scheduler_journal_read_back_missing"))
    }
}

fn equivalent(left: &AgentSchedulerJournalRecord, right: &AgentSchedulerJournalRecord) -> bool {
    left.schema == right.schema
        && left.node_id == right.node_id
        && left.scheduler_epoch == right.scheduler_epoch
        && left.step_id == right.step_id
        && left.plan_digest == right.plan_digest
        && left.plan == right.plan
        && left.status == right.status
        && left.evidence_refs == right.evidence_refs
        && left.unknowns == right.unknowns
}

fn read_record(
    path: &Path,
) -> Result<Option<AgentSchedulerJournalRecord>, AgentWorkspacePortError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(failure) if failure.kind() == ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(error("scheduler_journal_record_unobservable")),
    };
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return Err(error("scheduler_journal_record_path_rejected"));
    }
    let bytes = fs::read(path).map_err(|_| error("scheduler_journal_record_read_failed"))?;
    let record: AgentSchedulerJournalRecord =
        serde_json::from_slice(&bytes).map_err(|_| error("scheduler_journal_record_invalid"))?;
    if !validate_agent_scheduler_journal_record(&record).is_empty() {
        return Err(error("scheduler_journal_record_rejected"));
    }
    Ok(Some(record))
}

fn write_new(path: &Path, bytes: &[u8]) -> Result<(), AgentWorkspacePortError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o640);
    let mut file = options
        .open(path)
        .map_err(|_| error("scheduler_journal_temp_create_failed"))?;
    file.write_all(bytes)
        .map_err(|_| error("scheduler_journal_temp_write_failed"))?;
    file.sync_all()
        .map_err(|_| error("scheduler_journal_temp_sync_failed"))
}

fn validate_id(value: &str) -> Result<(), AgentWorkspacePortError> {
    if value.is_empty()
        || value.len() > 160
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(error("scheduler_journal_identifier_invalid"));
    }
    Ok(())
}

fn reject_non_directory(path: &Path) -> Result<(), AgentWorkspacePortError> {
    let metadata =
        fs::symlink_metadata(path).map_err(|_| error("scheduler_journal_root_unobservable"))?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(error("scheduler_journal_root_rejected"));
    }
    Ok(())
}

fn error(code: &str) -> AgentWorkspacePortError {
    AgentWorkspacePortError::new(code, code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::AGENT_SCHEDULER_JOURNAL_RECORD_SCHEMA;

    fn planned(recorded_at_ms: u64) -> AgentSchedulerJournalRecord {
        AgentSchedulerJournalRecord {
            schema: AGENT_SCHEDULER_JOURNAL_RECORD_SCHEMA.into(),
            node_id: "node-1".into(),
            scheduler_epoch: 1,
            step_id: "step-1".into(),
            plan_digest: format!("sha256:{}", "a".repeat(64)),
            plan: Some(AgentSchedulerStepPlan {
                schema: cynic_protocol::AGENT_SCHEDULER_STEP_PLAN_SCHEMA.into(),
                node_id: "node-1".into(),
                scheduler_epoch: 1,
                step_id: "step-1".into(),
                lifecycle: cynic_protocol::AgentWorkspaceLifecyclePlan {
                    schema: cynic_protocol::AGENT_WORKSPACE_LIFECYCLE_PLAN_SCHEMA.into(),
                    workload_id: "workload-1".into(),
                    repository_id: "cynic".into(),
                    workspace_id: "workspace-1".into(),
                    mutates: false,
                    decision: cynic_protocol::AgentWorkspaceLifecycleDecision::Keep {
                        reasons: vec!["test".into()],
                    },
                },
                decision: cynic_protocol::AgentSchedulerStepDecision::Idle {
                    reasons: vec!["test".into()],
                },
            }),
            status: AgentSchedulerJournalStatus::Planned,
            evidence_refs: vec![],
            unknowns: vec![],
            recorded_at_ms,
        }
    }

    #[test]
    fn retry_identity_ignores_only_observation_time() {
        assert!(equivalent(&planned(1), &planned(2)));
        let mut different = planned(2);
        different.plan_digest = format!("sha256:{}", "b".repeat(64));
        assert!(!equivalent(&planned(1), &different));
    }
}
