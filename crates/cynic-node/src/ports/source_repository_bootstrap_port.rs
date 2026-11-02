//! Source repository bootstrap port — re-exported from canonical owner cynic-vcs.
pub use cynic_vcs::ports::{SourceRepositoryBootstrapPort, SourceRepositoryBootstrapPortError, SourceRepositoryLayout};

impl From<&crate::ports::agent_workspace_port::AgentWorkspaceLayout> for SourceRepositoryLayout {
    fn from(layout: &crate::ports::agent_workspace_port::AgentWorkspaceLayout) -> Self {
        Self {
            repository_root: layout.repository_root.clone(),
            required_principal: layout.required_principal.clone(),
        }
    }
}
