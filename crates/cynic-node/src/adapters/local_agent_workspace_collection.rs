use std::{fs, path::Path};

use cynic_lifecycle::ArgvRunner;
use cynic_workload::{
    AGENT_WORKSPACE_LIFECYCLE_PLAN_SCHEMA, AgentWorkspaceLifecycleDecision,
    AgentWorkspaceLifecyclePlan, agent_workspace_target,
};

use crate::ports::agent_workspace_port::{
    AgentWorkspaceCollectionPort, AgentWorkspaceCollectionReadBack, AgentWorkspaceLayout,
    AgentWorkspacePortError,
};

#[derive(Debug)]
pub struct LocalAgentWorkspaceCollectionAdapter<'a, R> {
    runner: &'a R,
    git_bin: &'a str,
    df_bin: &'a str,
}

impl<'a, R> LocalAgentWorkspaceCollectionAdapter<'a, R> {
    pub fn new(runner: &'a R, git_bin: &'a str, df_bin: &'a str) -> Self {
        Self {
            runner,
            git_bin,
            df_bin,
        }
    }
}

impl<R: ArgvRunner> AgentWorkspaceCollectionPort for LocalAgentWorkspaceCollectionAdapter<'_, R> {
    fn collect(
        &self,
        layout: &AgentWorkspaceLayout,
        plan: &AgentWorkspaceLifecyclePlan,
    ) -> Result<AgentWorkspaceCollectionReadBack, AgentWorkspacePortError> {
        if !layout.dedicated {
            return Err(error(
                "workspace_collection_shared_layout",
                "collection requires a dedicated agent workspace",
            ));
        }
        if plan.schema != AGENT_WORKSPACE_LIFECYCLE_PLAN_SCHEMA || !plan.mutates {
            return Err(error(
                "workspace_collection_plan_invalid",
                "collection requires a mutating canonical lifecycle plan",
            ));
        }
        let (target, guard, completion_evidence_digest) = match &plan.decision {
            AgentWorkspaceLifecycleDecision::Collect {
                target,
                guard,
                completion_evidence_digest,
            } => (target, guard, completion_evidence_digest),
            _ => {
                return Err(error(
                    "workspace_collection_plan_not_admitted",
                    "only a lifecycle Collect decision may reach the collection adapter",
                ));
            }
        };
        let expected_target = agent_workspace_target(&plan.repository_id, &plan.workspace_id);
        if target != &expected_target
            || guard.target != expected_target
            || guard.fencing_token == 0
            || guard.lease_id.trim().is_empty()
            || guard.agent_id.trim().is_empty()
        {
            return Err(error(
                "workspace_collection_guard_mismatch",
                "collection target and fenced guard must match the admitted workspace",
            ));
        }
        if !valid_sha256(completion_evidence_digest) {
            return Err(error(
                "workspace_collection_completion_digest_invalid",
                "collection requires the admitted content-addressed completion evidence",
            ));
        }

        let repository = canonical_directory(&layout.repository_root, "repository_root")?;
        let workspace = canonical_directory(&layout.workspace_root, "workspace_root")?;
        if repository == workspace {
            return Err(error(
                "workspace_collection_repository_root_forbidden",
                "the managed repository root can never be collected as an agent workspace",
            ));
        }
        let parent = workspace.parent().ok_or_else(|| {
            error(
                "workspace_collection_parent_missing",
                "workspace root has no parent directory",
            )
        })?;
        let repository_text = path_text(&repository, "repository_root")?;
        let workspace_text = path_text(&workspace, "workspace_root")?;
        let parent_text = path_text(parent, "workspace_parent")?;
        let clean = self.runner.run(&[
            self.git_bin,
            "-C",
            workspace_text,
            "status",
            "--porcelain",
            "--untracked-files=all",
        ]);
        if !clean.success() {
            return Err(error(
                "workspace_collection_cleanliness_unobservable",
                "workspace cleanliness could not be read back immediately before collection",
            ));
        }
        if !clean.stdout.trim().is_empty() {
            return Err(error(
                "workspace_collection_dirty",
                "workspace changed after lifecycle reconciliation and cannot be collected",
            ));
        }
        let available_before = observe_available_bytes(self.runner, self.df_bin, parent_text);

        let output = self.runner.run(&[
            self.git_bin,
            "-C",
            repository_text,
            "worktree",
            "remove",
            "--force",
            workspace_text,
        ]);
        if !output.success() {
            return Err(error(
                "workspace_collection_git_failed",
                format!(
                    "Git worktree removal failed: {}",
                    output
                        .spawn_error
                        .as_deref()
                        .unwrap_or(output.stderr.trim())
                ),
            ));
        }
        if layout.workspace_root.exists() {
            return Err(error(
                "workspace_collection_read_back_failed",
                "workspace still exists after the collection command",
            ));
        }
        let available_after = observe_available_bytes(self.runner, self.df_bin, parent_text);
        let reclaimed_bytes = available_before
            .zip(available_after)
            .map(|(before, after)| after.saturating_sub(before));

        Ok(AgentWorkspaceCollectionReadBack {
            workspace_existed_before: true,
            workspace_exists_after: false,
            reclaimed_bytes,
            completion_evidence_digest: completion_evidence_digest.clone(),
        })
    }
}

fn observe_available_bytes<R: ArgvRunner>(runner: &R, df_bin: &str, path: &str) -> Option<u64> {
    let output = runner.run(&[df_bin, "-B1", "--output=avail", path]);
    if !output.success() {
        return None;
    }
    output
        .stdout
        .lines()
        .rev()
        .find_map(|line| line.trim().parse::<u64>().ok())
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

fn canonical_directory(
    path: &Path,
    field: &str,
) -> Result<std::path::PathBuf, AgentWorkspacePortError> {
    let canonical = fs::canonicalize(path).map_err(|failure| {
        error(
            "workspace_collection_path_unavailable",
            format!("{field} cannot be canonicalized: {failure}"),
        )
    })?;
    if !canonical.is_dir() {
        return Err(error(
            "workspace_collection_path_rejected",
            format!("{field} is not a directory"),
        ));
    }
    Ok(canonical)
}

fn path_text<'a>(path: &'a Path, field: &str) -> Result<&'a str, AgentWorkspacePortError> {
    path.to_str().ok_or_else(|| {
        error(
            "workspace_collection_path_not_utf8",
            format!("{field} cannot be represented as UTF-8 argv"),
        )
    })
}

fn error(code: impl Into<String>, message: impl Into<String>) -> AgentWorkspacePortError {
    AgentWorkspacePortError::new(code, message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_lifecycle::MapArgvRunner;
    use cynic_workload::AGENT_WORKSPACE_LIFECYCLE_PLAN_SCHEMA;

    #[test]
    fn adapter_rejects_non_collect_decisions_before_io() {
        let runner = MapArgvRunner::default();
        let adapter = LocalAgentWorkspaceCollectionAdapter::new(&runner, "git", "df");
        let layout = AgentWorkspaceLayout {
            repository_root: "/does/not/matter".into(),
            workspace_root: "/does/not/matter".into(),
            dedicated: true,
            required_principal: "cynic".into(),
            evidence_digest: "sha256:test".into(),
            transport_uri: None,
        };
        let plan = AgentWorkspaceLifecyclePlan {
            schema: AGENT_WORKSPACE_LIFECYCLE_PLAN_SCHEMA.into(),
            workload_id: "workload-1".into(),
            repository_id: "cynic".into(),
            workspace_id: "workspace-1".into(),
            mutates: true,
            decision: AgentWorkspaceLifecycleDecision::Keep {
                reasons: vec!["protected".into()],
            },
        };
        assert_eq!(
            adapter.collect(&layout, &plan).unwrap_err().code,
            "workspace_collection_plan_not_admitted"
        );
    }
}
