use std::fs;

use cynic_lifecycle::ArgvRunner;
use cynic_protocol::AgentEvidenceRef;
use cynic_workload::{
    AGENT_WORKSPACE_LIFECYCLE_OBSERVATION_SCHEMA, AGENT_WORKSPACE_OBSERVATION_SCHEMA,
    AgentWorkloadState, AgentWorkspaceClaimObservation, AgentWorkspaceLifecycleObservation,
    AgentWorkspaceLifecycleRecord, AgentWorkspaceObservation, agent_workspace_target,
};

use crate::ports::agent_workspace_port::{
    AgentWorkspaceLayout, AgentWorkspacePortError, AgentWorkspaceTerminalObservationPort,
};

#[derive(Debug)]
pub struct LocalAgentWorkspaceLifecycleObserver<'a, R> {
    runner: &'a R,
    git_bin: &'a str,
}

impl<'a, R> LocalAgentWorkspaceLifecycleObserver<'a, R> {
    pub fn new(runner: &'a R, git_bin: &'a str) -> Self {
        Self { runner, git_bin }
    }
}

impl<R: ArgvRunner> AgentWorkspaceTerminalObservationPort
    for LocalAgentWorkspaceLifecycleObserver<'_, R>
{
    fn observe_terminal(
        &self,
        layout: &AgentWorkspaceLayout,
        record: &AgentWorkspaceLifecycleRecord,
        repository_claims: Vec<AgentWorkspaceClaimObservation>,
    ) -> Result<AgentWorkspaceLifecycleObservation, AgentWorkspacePortError> {
        let target = agent_workspace_target(&record.repository_id, &record.workspace_id);
        let collection_claim = repository_claims
            .iter()
            .find(|claim| claim.guard.target == target)
            .cloned();
        if !layout.workspace_root.exists() {
            return Ok(observation(
                layout,
                record,
                repository_claims,
                collection_claim,
                false,
                None,
                None,
            ));
        }
        let metadata = fs::symlink_metadata(&layout.workspace_root).map_err(|failure| {
            error(
                "workspace_lifecycle_observer_path_unobservable",
                format!("cannot observe workspace root: {failure}"),
            )
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(error(
                "workspace_lifecycle_observer_path_rejected",
                "workspace root is a symlink or not a directory",
            ));
        }
        let workspace = layout.workspace_root.to_str().ok_or_else(|| {
            error(
                "workspace_lifecycle_observer_path_not_utf8",
                "workspace root cannot be represented as UTF-8 argv",
            )
        })?;
        let head = self
            .runner
            .run(&[self.git_bin, "-C", workspace, "rev-parse", "HEAD"]);
        let head_commit = head.success().then(|| head.stdout.trim().to_string());
        let status = self.runner.run(&[
            self.git_bin,
            "-C",
            workspace,
            "status",
            "--porcelain",
            "--untracked-files=all",
        ]);
        let workspace_clean = status.success().then(|| status.stdout.trim().is_empty());
        Ok(observation(
            layout,
            record,
            repository_claims,
            collection_claim,
            true,
            head_commit,
            workspace_clean,
        ))
    }
}

#[allow(clippy::too_many_arguments)]
fn observation(
    layout: &AgentWorkspaceLayout,
    record: &AgentWorkspaceLifecycleRecord,
    repository_claims: Vec<AgentWorkspaceClaimObservation>,
    collection_claim: Option<AgentWorkspaceClaimObservation>,
    exists: bool,
    head_commit: Option<String>,
    workspace_clean: Option<bool>,
) -> AgentWorkspaceLifecycleObservation {
    AgentWorkspaceLifecycleObservation {
        schema: AGENT_WORKSPACE_LIFECYCLE_OBSERVATION_SCHEMA.to_string(),
        workload_id: record.workload_id.clone(),
        workload_state: AgentWorkloadState::Completed,
        workload_claim: None,
        workspace: AgentWorkspaceObservation {
            schema: AGENT_WORKSPACE_OBSERVATION_SCHEMA.to_string(),
            repository_id: record.repository_id.clone(),
            workspace_id: record.workspace_id.clone(),
            node_id: record.node_id.clone(),
            exists,
            isolated: layout.dedicated,
            head_commit,
            path_preimages: Vec::new(),
            repository_claims,
            evidence_refs: vec![AgentEvidenceRef {
                kind: "workspace_layout".to_string(),
                subject: format!("{}:{}", record.repository_id, record.workspace_id),
                digest: layout.evidence_digest.clone(),
                lineage: None,
            }],
        },
        completion_evidence: Some(record.completion_evidence.clone()),
        workspace_clean,
        retention_until_ms: Some(record.retention_until_ms),
        protected_roots: record.protected_roots.clone(),
        collection_claim,
    }
}

fn error(code: impl Into<String>, message: impl Into<String>) -> AgentWorkspacePortError {
    AgentWorkspacePortError::new(code, message)
}
