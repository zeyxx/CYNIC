use std::{
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

#[cfg(unix)]
use std::os::unix::fs::OpenOptionsExt;

use cynic_protocol::{AgentWorkspaceLifecycleRecord, validate_agent_workspace_lifecycle_record};

use crate::ports::agent_workspace_port::{
    AgentWorkspaceLifecycleStorePort, AgentWorkspacePortError,
};

static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone)]
pub struct LocalAgentWorkspaceLifecycleStore {
    root: PathBuf,
}

impl LocalAgentWorkspaceLifecycleStore {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    fn record_path(&self, workload_id: &str) -> Result<PathBuf, AgentWorkspacePortError> {
        if workload_id.is_empty()
            || workload_id.len() > 160
            || !workload_id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
        {
            return Err(error(
                "workspace_lifecycle_store_workload_id_invalid",
                "workload id is not safe for the node-owned lifecycle store",
            ));
        }
        Ok(self.root.join(format!("{workload_id}.json")))
    }
}

impl AgentWorkspaceLifecycleStorePort for LocalAgentWorkspaceLifecycleStore {
    fn load(
        &self,
        workload_id: &str,
    ) -> Result<Option<AgentWorkspaceLifecycleRecord>, AgentWorkspacePortError> {
        let path = self.record_path(workload_id)?;
        match fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                return Err(error(
                    "workspace_lifecycle_store_record_path_rejected",
                    "lifecycle record path is a symlink or not a regular file",
                ));
            }
            Ok(_) => {}
            Err(failure) if failure.kind() == ErrorKind::NotFound => return Ok(None),
            Err(failure) => {
                return Err(error(
                    "workspace_lifecycle_store_record_unobservable",
                    format!("cannot observe lifecycle record: {failure}"),
                ));
            }
        }
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(failure) => {
                return Err(error(
                    "workspace_lifecycle_store_read_failed",
                    format!("cannot read lifecycle record: {failure}"),
                ));
            }
        };
        let record: AgentWorkspaceLifecycleRecord =
            serde_json::from_slice(&bytes).map_err(|failure| {
                error(
                    "workspace_lifecycle_store_record_invalid",
                    format!("cannot decode lifecycle record: {failure}"),
                )
            })?;
        let violations = validate_agent_workspace_lifecycle_record(&record);
        if !violations.is_empty() || record.workload_id != workload_id {
            return Err(error(
                "workspace_lifecycle_store_record_rejected",
                format!("stored lifecycle record failed validation: {violations:?}"),
            ));
        }
        Ok(Some(record))
    }

    fn persist(
        &self,
        record: &AgentWorkspaceLifecycleRecord,
    ) -> Result<AgentWorkspaceLifecycleRecord, AgentWorkspacePortError> {
        let violations = validate_agent_workspace_lifecycle_record(record);
        if !violations.is_empty() {
            return Err(error(
                "workspace_lifecycle_store_record_rejected",
                format!("lifecycle record failed validation: {violations:?}"),
            ));
        }
        let final_path = self.record_path(&record.workload_id)?;
        fs::create_dir_all(&self.root).map_err(|failure| {
            error(
                "workspace_lifecycle_store_root_failed",
                format!("cannot create lifecycle store root: {failure}"),
            )
        })?;
        reject_symlink_or_non_directory(&self.root)?;
        if let Some(existing) = self.load(&record.workload_id)? {
            return if existing == *record {
                Ok(existing)
            } else {
                Err(error(
                    "workspace_lifecycle_store_immutable_conflict",
                    "a different lifecycle record already exists for this workload",
                ))
            };
        }

        let bytes = serde_json::to_vec_pretty(record).map_err(|failure| {
            error(
                "workspace_lifecycle_store_encode_failed",
                format!("cannot encode lifecycle record: {failure}"),
            )
        })?;
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp_path = self.root.join(format!(
            ".{}.{}.{}.tmp",
            record.workload_id,
            std::process::id(),
            sequence
        ));
        write_new_file(&temp_path, &bytes)?;
        match fs::hard_link(&temp_path, &final_path) {
            Ok(()) => {}
            Err(failure) if failure.kind() == ErrorKind::AlreadyExists => {
                let _ = fs::remove_file(&temp_path);
                return match self.load(&record.workload_id)? {
                    Some(existing) if existing == *record => Ok(existing),
                    Some(_) => Err(error(
                        "workspace_lifecycle_store_immutable_conflict",
                        "concurrent writer persisted a different lifecycle record",
                    )),
                    None => Err(error(
                        "workspace_lifecycle_store_race_unknown",
                        "record appeared concurrently but could not be read back",
                    )),
                };
            }
            Err(failure) => {
                let _ = fs::remove_file(&temp_path);
                return Err(error(
                    "workspace_lifecycle_store_commit_failed",
                    format!("cannot atomically publish lifecycle record: {failure}"),
                ));
            }
        }
        fs::remove_file(&temp_path).map_err(|failure| {
            error(
                "workspace_lifecycle_store_temp_cleanup_failed",
                format!("record published but temporary link remains: {failure}"),
            )
        })?;
        sync_directory(&self.root)?;
        self.load(&record.workload_id)?.ok_or_else(|| {
            error(
                "workspace_lifecycle_store_read_back_missing",
                "published lifecycle record was not readable",
            )
        })
    }
}

fn write_new_file(path: &Path, bytes: &[u8]) -> Result<(), AgentWorkspacePortError> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    options.mode(0o640);
    let mut file = options.open(path).map_err(|failure| {
        error(
            "workspace_lifecycle_store_temp_create_failed",
            format!("cannot create lifecycle temporary file: {failure}"),
        )
    })?;
    file.write_all(bytes).map_err(|failure| {
        error(
            "workspace_lifecycle_store_temp_write_failed",
            format!("cannot write lifecycle temporary file: {failure}"),
        )
    })?;
    file.sync_all().map_err(|failure| {
        error(
            "workspace_lifecycle_store_temp_sync_failed",
            format!("cannot sync lifecycle temporary file: {failure}"),
        )
    })
}

fn reject_symlink_or_non_directory(path: &Path) -> Result<(), AgentWorkspacePortError> {
    let metadata = fs::symlink_metadata(path).map_err(|failure| {
        error(
            "workspace_lifecycle_store_root_unobservable",
            format!("cannot observe lifecycle store root: {failure}"),
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(error(
            "workspace_lifecycle_store_root_rejected",
            "lifecycle store root is a symlink or not a directory",
        ));
    }
    Ok(())
}

fn sync_directory(path: &Path) -> Result<(), AgentWorkspacePortError> {
    File::open(path)
        .and_then(|directory| directory.sync_all())
        .map_err(|failure| {
            error(
                "workspace_lifecycle_store_directory_sync_failed",
                format!("cannot sync lifecycle store directory: {failure}"),
            )
        })
}

fn error(code: impl Into<String>, message: impl Into<String>) -> AgentWorkspacePortError {
    AgentWorkspacePortError::new(code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        AGENT_WORKSPACE_LIFECYCLE_RECORD_SCHEMA, AgentEvidenceRef, AgentWorkspaceRetentionRoot,
    };

    fn record() -> AgentWorkspaceLifecycleRecord {
        AgentWorkspaceLifecycleRecord {
            schema: AGENT_WORKSPACE_LIFECYCLE_RECORD_SCHEMA.into(),
            workload_id: "workload-1".into(),
            repository_id: "cynic".into(),
            workspace_id: "workspace-1".into(),
            node_id: "node-1".into(),
            completion_evidence: AgentEvidenceRef {
                kind: "agent_completion".into(),
                subject: "workload-1".into(),
                digest: format!("sha256:{}", "a".repeat(64)),
                lineage: None,
            },
            completed_at_ms: 100,
            retention_until_ms: 200,
            protected_roots: vec![AgentWorkspaceRetentionRoot {
                kind: "proof".into(),
                id: "proof-1".into(),
            }],
        }
    }

    #[test]
    fn store_is_append_once_idempotent_and_read_back() {
        let root = std::env::temp_dir().join(format!(
            "cynic-workspace-lifecycle-store-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let store = LocalAgentWorkspaceLifecycleStore::new(&root);
        let expected = record();
        assert_eq!(store.persist(&expected).unwrap(), expected);
        assert_eq!(store.persist(&expected).unwrap(), expected);
        let mut conflicting = expected.clone();
        conflicting.retention_until_ms += 1;
        assert_eq!(
            store.persist(&conflicting).unwrap_err().code,
            "workspace_lifecycle_store_immutable_conflict"
        );
        assert_eq!(store.load("workload-1").unwrap(), Some(expected));
        fs::remove_dir_all(&root).unwrap();
    }
}
