use std::{fs, path::PathBuf};

use cynic_protocol::{
    AgentWorkspaceApplyReport, AgentWorkspaceApplyStatus, AgentWorkspaceMutationKind,
    AgentWorkspaceReconciliation, AgentWorkspaceRequest,
};

use super::local_agent_workspace_apply_support::{
    LocalTransactionJournal, admit_apply, cleanup_file, content_digest, error, mutation_kind,
    persist_journal, persist_new_file, read_regular_file, replace_atomically, report,
    safe_change_target, safe_component,
};
use crate::ports::{
    agent_workspace_port::{AgentWorkspaceLayout, AgentWorkspacePortError},
    source_change_transaction_port::{AgentSourceBlobStorePort, AgentWorkspaceApplyPort},
};

#[derive(Debug)]
pub struct LocalAgentWorkspaceApplyAdapter<'a, B> {
    blobs: &'a B,
}

impl<'a, B> LocalAgentWorkspaceApplyAdapter<'a, B> {
    pub fn new(blobs: &'a B) -> Self {
        Self { blobs }
    }
}

#[derive(Debug)]
pub struct LocalAgentWorkspaceApplyReceipt {
    report: AgentWorkspaceApplyReport,
    target: PathBuf,
    backup: Option<PathBuf>,
    journal: PathBuf,
    mutation: AgentWorkspaceMutationKind,
    preimage: Option<Vec<u8>>,
}

impl<B: AgentSourceBlobStorePort> AgentWorkspaceApplyPort
    for LocalAgentWorkspaceApplyAdapter<'_, B>
{
    type Receipt = LocalAgentWorkspaceApplyReceipt;

    fn apply(
        &self,
        layout: &AgentWorkspaceLayout,
        request: &AgentWorkspaceRequest,
        reconciliation: &AgentWorkspaceReconciliation,
    ) -> Result<Self::Receipt, AgentWorkspacePortError> {
        admit_apply(layout, request, reconciliation)?;
        let change = &request.changes[0];
        let target = safe_change_target(&layout.workspace_root, &change.path)?;
        let observed_preimage = read_regular_file(&target)?;
        let observed_preimage_digest = observed_preimage.as_deref().map(content_digest);
        if observed_preimage_digest != change.preimage_digest {
            return Err(error(
                "workspace_apply_preimage_drift",
                "source path changed after reconciliation and before apply",
            ));
        }
        let mutation = mutation_kind(
            change.preimage_digest.as_ref(),
            change.result_digest.as_ref(),
        )?;
        let result_content = change
            .result_digest
            .as_deref()
            .map(|digest| self.blobs.load(digest))
            .transpose()?;
        let transaction_dir = layout.workspace_root.join(".cynic-transactions");
        fs::create_dir_all(&transaction_dir).map_err(|failure| {
            error(
                "workspace_transaction_dir_failed",
                format!("cannot create transaction directory: {failure}"),
            )
        })?;
        let transaction_metadata = fs::symlink_metadata(&transaction_dir).map_err(|failure| {
            error(
                "workspace_transaction_dir_unobservable",
                format!("cannot observe transaction directory: {failure}"),
            )
        })?;
        if transaction_metadata.file_type().is_symlink() || !transaction_metadata.is_dir() {
            return Err(error(
                "workspace_transaction_dir_rejected",
                "transaction directory is a symlink or not a directory",
            ));
        }

        let transaction_id = format!(
            "{}-{}",
            safe_component(&request.workload_id),
            request.guard.fencing_token
        );
        let journal = transaction_dir.join(format!("{transaction_id}.json"));
        let backup = matches!(
            mutation,
            AgentWorkspaceMutationKind::Replace | AgentWorkspaceMutationKind::Delete
        )
        .then(|| transaction_dir.join(format!("{transaction_id}.backup")));
        persist_journal(
            &journal,
            &LocalTransactionJournal {
                schema: "cynic.local_source_transaction_journal.v1",
                workload_id: &request.workload_id,
                fencing_token: request.guard.fencing_token,
                target: target.display().to_string(),
                mutation,
                expected_preimage_digest: &change.preimage_digest,
                expected_result_digest: &change.result_digest,
            },
        )?;
        if let (Some(content), Some(path)) = (observed_preimage.as_deref(), backup.as_ref()) {
            persist_new_file(path, content)?;
        }

        let mutation_result = match mutation {
            AgentWorkspaceMutationKind::Create | AgentWorkspaceMutationKind::Replace => {
                replace_atomically(
                    &target,
                    result_content.as_deref().ok_or_else(|| {
                        error(
                            "workspace_result_blob_missing",
                            "create or replace requires a result blob",
                        )
                    })?,
                    &transaction_id,
                )
            }
            AgentWorkspaceMutationKind::Delete => fs::remove_file(&target).map_err(|failure| {
                error(
                    "workspace_delete_failed",
                    format!("cannot delete admitted source path: {failure}"),
                )
            }),
        };
        if let Err(failure) = mutation_result {
            cleanup_file(backup.as_deref());
            cleanup_file(Some(&journal));
            return Err(failure);
        }

        let observed_result = read_regular_file(&target)?;
        let observed_result_digest = observed_result.as_deref().map(content_digest);
        if observed_result_digest != change.result_digest {
            let receipt = LocalAgentWorkspaceApplyReceipt {
                report: report(
                    request,
                    mutation,
                    observed_preimage_digest,
                    observed_result_digest,
                    AgentWorkspaceApplyStatus::Failed,
                ),
                target,
                backup,
                journal,
                mutation,
                preimage: observed_preimage,
            };
            self.rollback(receipt)?;
            return Err(error(
                "workspace_apply_read_back_mismatch",
                "post-action source hash differs from the admitted result digest",
            ));
        }

        Ok(LocalAgentWorkspaceApplyReceipt {
            report: report(
                request,
                mutation,
                observed_preimage_digest,
                observed_result_digest,
                AgentWorkspaceApplyStatus::Applied,
            ),
            target,
            backup,
            journal,
            mutation,
            preimage: observed_preimage,
        })
    }

    fn report(receipt: &Self::Receipt) -> &AgentWorkspaceApplyReport {
        &receipt.report
    }

    fn rollback(&self, receipt: Self::Receipt) -> Result<(), AgentWorkspacePortError> {
        match receipt.mutation {
            AgentWorkspaceMutationKind::Create => {
                if receipt.target.exists() {
                    fs::remove_file(&receipt.target).map_err(|failure| {
                        error(
                            "workspace_rollback_failed",
                            format!("cannot remove created source path: {failure}"),
                        )
                    })?;
                }
            }
            AgentWorkspaceMutationKind::Replace | AgentWorkspaceMutationKind::Delete => {
                let content = receipt.preimage.as_deref().ok_or_else(|| {
                    error(
                        "workspace_rollback_preimage_missing",
                        "rollback preimage is unavailable",
                    )
                })?;
                replace_atomically(&receipt.target, content, "rollback")?;
            }
        }
        cleanup_file(receipt.backup.as_deref());
        cleanup_file(Some(&receipt.journal));
        Ok(())
    }

    fn commit(&self, receipt: Self::Receipt) -> Result<(), AgentWorkspacePortError> {
        cleanup_file(receipt.backup.as_deref());
        cleanup_file(Some(&receipt.journal));
        Ok(())
    }
}
