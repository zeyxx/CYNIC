use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Component, Path, PathBuf},
};

use cynic_protocol::{
    AGENT_WORKSPACE_APPLY_REPORT_SCHEMA, AgentWorkspaceApplyReport, AgentWorkspaceApplyStatus,
    AgentWorkspaceDecision, AgentWorkspaceMutationKind, AgentWorkspaceReadBack,
    AgentWorkspaceReconciliation, AgentWorkspaceRequest,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::ports::agent_workspace_port::{AgentWorkspaceLayout, AgentWorkspacePortError};

#[derive(Debug, Serialize)]
pub(super) struct LocalTransactionJournal<'a> {
    pub schema: &'static str,
    pub workload_id: &'a str,
    pub fencing_token: u64,
    pub target: String,
    pub mutation: AgentWorkspaceMutationKind,
    pub expected_preimage_digest: &'a Option<String>,
    pub expected_result_digest: &'a Option<String>,
}

pub(super) fn admit_apply(
    layout: &AgentWorkspaceLayout,
    request: &AgentWorkspaceRequest,
    reconciliation: &AgentWorkspaceReconciliation,
) -> Result<(), AgentWorkspacePortError> {
    if !layout.dedicated {
        return Err(error(
            "workspace_apply_shared_layout",
            "source mutation requires a dedicated workspace",
        ));
    }
    if request.changes.len() != 1 {
        return Err(error(
            "workspace_apply_cardinality",
            "the first source transaction vertical admits exactly one path",
        ));
    }
    match &reconciliation.decision {
        AgentWorkspaceDecision::Ready {
            workspace_target,
            change_set_digest,
            ..
        } if reconciliation.workload_id == request.workload_id
            && workspace_target == &request.guard.target
            && change_set_digest == &request.change_set_digest =>
        {
            Ok(())
        }
        _ => Err(error(
            "workspace_apply_not_reconciled",
            "source mutation requires a matching ready reconciliation",
        )),
    }
}

pub(super) fn mutation_kind(
    preimage: Option<&String>,
    result: Option<&String>,
) -> Result<AgentWorkspaceMutationKind, AgentWorkspacePortError> {
    match (preimage, result) {
        (None, Some(_)) => Ok(AgentWorkspaceMutationKind::Create),
        (Some(_), Some(_)) => Ok(AgentWorkspaceMutationKind::Replace),
        (Some(_), None) => Ok(AgentWorkspaceMutationKind::Delete),
        (None, None) => Err(error(
            "workspace_apply_empty_change",
            "source change has neither preimage nor result",
        )),
    }
}

pub(super) fn report(
    request: &AgentWorkspaceRequest,
    mutation: AgentWorkspaceMutationKind,
    observed_preimage_digest: Option<String>,
    observed_result_digest: Option<String>,
    status: AgentWorkspaceApplyStatus,
) -> AgentWorkspaceApplyReport {
    let change = &request.changes[0];
    AgentWorkspaceApplyReport {
        schema: AGENT_WORKSPACE_APPLY_REPORT_SCHEMA.to_string(),
        workload_id: request.workload_id.clone(),
        repository_id: request.repository_id.clone(),
        workspace_id: request.workspace_id.clone(),
        change_set_digest: request.change_set_digest.clone(),
        fencing_token: request.guard.fencing_token,
        status,
        read_back: Some(AgentWorkspaceReadBack {
            path: change.path.clone(),
            mutation,
            expected_preimage_digest: change.preimage_digest.clone(),
            observed_preimage_digest,
            expected_result_digest: change.result_digest.clone(),
            observed_result_digest,
        }),
        violations: Vec::new(),
    }
}

pub(super) fn safe_change_target(
    workspace_root: &Path,
    relative: &str,
) -> Result<PathBuf, AgentWorkspacePortError> {
    let root_metadata = fs::symlink_metadata(workspace_root).map_err(|failure| {
        error(
            "workspace_apply_root_unavailable",
            format!("workspace root cannot be observed: {failure}"),
        )
    })?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(error(
            "workspace_apply_root_rejected",
            "workspace root is a symlink or not a directory",
        ));
    }
    let components = Path::new(relative)
        .components()
        .map(|component| match component {
            Component::Normal(value) => Ok(value),
            _ => Err(error(
                "workspace_apply_path_rejected",
                "source change path is not workspace-relative",
            )),
        })
        .collect::<Result<Vec<_>, _>>()?;
    if components.is_empty() {
        return Err(error(
            "workspace_apply_path_rejected",
            "source change path is empty",
        ));
    }
    let mut cursor = workspace_root.to_path_buf();
    for component in &components[..components.len() - 1] {
        cursor.push(component);
        let metadata = fs::symlink_metadata(&cursor).map_err(|failure| {
            error(
                "workspace_apply_parent_unavailable",
                format!("source path parent cannot be observed: {failure}"),
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(error(
                "workspace_apply_parent_rejected",
                "source path crosses a symlink or non-directory parent",
            ));
        }
    }
    Ok(workspace_root.join(relative))
}

pub(super) fn read_regular_file(path: &Path) -> Result<Option<Vec<u8>>, AgentWorkspacePortError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => Err(error(
            "workspace_apply_target_rejected",
            "source target is not a regular file",
        )),
        Ok(_) => fs::read(path).map(Some).map_err(|failure| {
            error(
                "workspace_apply_target_read_failed",
                format!("cannot read source target: {failure}"),
            )
        }),
        Err(failure) if failure.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(failure) => Err(error(
            "workspace_apply_target_observation_failed",
            format!("cannot observe source target: {failure}"),
        )),
    }
}

pub(super) fn replace_atomically(
    target: &Path,
    content: &[u8],
    transaction_id: &str,
) -> Result<(), AgentWorkspacePortError> {
    let parent = target.parent().ok_or_else(|| {
        error(
            "workspace_apply_parent_missing",
            "source target has no parent directory",
        )
    })?;
    let staging = parent.join(format!(".cynic-{transaction_id}.staging"));
    persist_new_file(&staging, content)?;
    fs::rename(&staging, target).map_err(|failure| {
        cleanup_file(Some(&staging));
        error(
            "workspace_atomic_replace_failed",
            format!("cannot atomically replace source target: {failure}"),
        )
    })
}

pub(super) fn persist_journal(
    path: &Path,
    journal: &LocalTransactionJournal<'_>,
) -> Result<(), AgentWorkspacePortError> {
    let bytes = serde_json::to_vec_pretty(journal).map_err(|failure| {
        error(
            "workspace_journal_not_serializable",
            format!("cannot serialize transaction journal: {failure}"),
        )
    })?;
    persist_new_file(path, &bytes)
}

pub(super) fn persist_new_file(path: &Path, content: &[u8]) -> Result<(), AgentWorkspacePortError> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|failure| {
            error(
                "workspace_transaction_file_failed",
                format!("cannot create transaction file: {failure}"),
            )
        })?;
    file.write_all(content)
        .and_then(|_| file.sync_all())
        .map_err(|failure| {
            error(
                "workspace_transaction_file_failed",
                format!("cannot persist transaction file: {failure}"),
            )
        })
}

pub(super) fn cleanup_file(path: Option<&Path>) {
    if let Some(path) = path {
        let _ = fs::remove_file(path);
    }
}

pub(super) fn content_digest(content: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(content)))
}

pub(super) fn safe_component(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

pub(super) fn error(
    code: impl Into<String>,
    message: impl Into<String>,
) -> AgentWorkspacePortError {
    AgentWorkspacePortError::new(code, message)
}
