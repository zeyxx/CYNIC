use std::path::{Component, Path, PathBuf};

use cynic_protocol::digest_json;

use super::{LocalRegistryAdapter, model::RegistrySourceWorkspaceLayout, node_name};
use crate::ports::agent_workspace_port::{
    AgentWorkspaceLayout, AgentWorkspaceLayoutPort, AgentWorkspacePortError,
};

#[derive(Debug, Clone)]
pub struct RegistryAgentWorkspaceLayoutAdapter {
    registry: LocalRegistryAdapter,
    runtime_state_root: PathBuf,
}

impl RegistryAgentWorkspaceLayoutAdapter {
    pub fn new(registry_path: impl Into<PathBuf>, runtime_state_root: impl Into<PathBuf>) -> Self {
        Self {
            registry: LocalRegistryAdapter::new(registry_path),
            runtime_state_root: runtime_state_root.into(),
        }
    }
}

impl AgentWorkspaceLayoutPort for RegistryAgentWorkspaceLayoutAdapter {
    fn resolve(
        &self,
        repository_id: &str,
        workspace_id: &str,
        node_id: &str,
    ) -> Result<AgentWorkspaceLayout, AgentWorkspacePortError> {
        validate_identifier(repository_id, "repository_id")?;
        validate_identifier(workspace_id, "workspace_id")?;
        validate_identifier(node_id, "node_id")?;

        let registry = self.registry.load_registry().map_err(|error| {
            AgentWorkspacePortError::new(
                "workspace_registry_unavailable",
                format!("cannot read runtime registry: {error}"),
            )
        })?;
        let node = registry
            .nodes
            .unwrap_or_default()
            .into_iter()
            .find(|node| node_name(node).as_deref() == Some(node_id))
            .ok_or_else(|| {
                AgentWorkspacePortError::new(
                    "workspace_node_not_declared",
                    format!("node '{node_id}' has no registry declaration"),
                )
            })?;
        let layout = node.source_workspace_layout.ok_or_else(|| {
            AgentWorkspacePortError::new(
                "workspace_layout_not_declared",
                format!("node '{node_id}' has no source workspace layout"),
            )
        })?;
        resolve_layout(
            &self.runtime_state_root,
            node_id,
            repository_id,
            workspace_id,
            &layout,
        )
    }
}

fn resolve_layout(
    runtime_state_root: &Path,
    node_id: &str,
    repository_id: &str,
    workspace_id: &str,
    layout: &RegistrySourceWorkspaceLayout,
) -> Result<AgentWorkspaceLayout, AgentWorkspacePortError> {
    let repository_store = safe_relative_path(&layout.repository_store, "repository_store")?;
    let workspace_store = safe_relative_path(&layout.workspace_store, "workspace_store")?;
    if layout.principal.trim().is_empty() {
        return Err(AgentWorkspacePortError::new(
            "workspace_principal_not_declared",
            format!("node '{node_id}' has no workspace principal"),
        ));
    }
    let repository = layout
        .repositories
        .iter()
        .find(|repository| repository.id == repository_id)
        .ok_or_else(|| {
            AgentWorkspacePortError::new(
                "workspace_repository_not_declared",
                format!("repository '{repository_id}' is not declared for node '{node_id}'"),
            )
        })?;
    let repository_path = safe_relative_path(&repository.relative_path, "repository_path")?;
    let evidence_digest = digest_json(&(
        node_id,
        repository_id,
        workspace_id,
        layout,
        runtime_state_root,
    ))
    .map_err(|error| {
        AgentWorkspacePortError::new(
            "workspace_layout_digest_failed",
            format!("workspace layout could not be canonicalized: {error}"),
        )
    })?;

    Ok(AgentWorkspaceLayout {
        repository_root: runtime_state_root
            .join(repository_store)
            .join(repository_path),
        workspace_root: runtime_state_root
            .join(workspace_store)
            .join(repository_id)
            .join(workspace_id),
        dedicated: layout.dedicated_worktrees,
        required_principal: layout.principal.clone(),
        evidence_digest,
        transport_uri: repository.transport_uri.clone(),
    })
}

fn validate_identifier(value: &str, field: &str) -> Result<(), AgentWorkspacePortError> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.contains('/')
        || value.contains('\\')
    {
        return Err(AgentWorkspacePortError::new(
            "workspace_identifier_rejected",
            format!("{field} is empty or contains path syntax"),
        ));
    }
    Ok(())
}

fn safe_relative_path(value: &str, field: &str) -> Result<PathBuf, AgentWorkspacePortError> {
    let path = Path::new(value);
    if value.is_empty()
        || path.is_absolute()
        || !path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
    {
        return Err(AgentWorkspacePortError::new(
            "workspace_layout_path_rejected",
            format!("{field} must be a non-empty portable relative path"),
        ));
    }
    Ok(path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;

    use super::*;

    fn registry(layout: serde_json::Value) -> (tempfile::TempDir, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let path = root.path().join("registry.json");
        fs::write(
            &path,
            serde_json::to_vec(&json!({
                "nodes": [{
                    "id": "builder-a",
                    "source_workspace_layout": layout
                }]
            }))
            .unwrap(),
        )
        .unwrap();
        (root, path)
    }

    #[test]
    fn resolves_portable_layout_below_injected_state_root() {
        let (root, registry) = registry(json!({
            "repository_store": "source/repositories",
            "workspace_store": "source/workspaces",
            "principal": "cynic",
            "dedicated_worktrees": true,
            "repositories": [{"id": "cynic", "relative_path": "CYNIC"}]
        }));
        let state = root.path().join("state");
        let adapter = RegistryAgentWorkspaceLayoutAdapter::new(registry, &state);

        let layout = adapter.resolve("cynic", "workload-1", "builder-a").unwrap();

        assert_eq!(
            layout.repository_root,
            state.join("source/repositories/CYNIC")
        );
        assert_eq!(
            layout.workspace_root,
            state.join("source/workspaces/cynic/workload-1")
        );
        assert!(layout.dedicated);
        assert!(layout.evidence_digest.starts_with("sha256:"));
    }

    #[test]
    fn rejects_machine_absolute_layout_declaration() {
        let (root, registry) = registry(json!({
            "repository_store": "/home/operator/repositories",
            "workspace_store": "source/workspaces",
            "principal": "cynic",
            "dedicated_worktrees": true,
            "repositories": [{"id": "cynic", "relative_path": "CYNIC"}]
        }));
        let adapter = RegistryAgentWorkspaceLayoutAdapter::new(registry, root.path());

        let error = adapter
            .resolve("cynic", "workload-1", "builder-a")
            .unwrap_err();
        assert_eq!(error.code, "workspace_layout_path_rejected");
    }
}
