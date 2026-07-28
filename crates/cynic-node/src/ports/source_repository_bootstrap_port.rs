use cynic_protocol::{SourceRepositoryBootstrapObservation, SourceRepositoryBootstrapRequest};

use crate::ports::agent_workspace_port::AgentWorkspaceLayout;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceRepositoryBootstrapPortError {
    pub code: String,
    pub message: String,
}

impl SourceRepositoryBootstrapPortError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

pub trait SourceRepositoryBootstrapPort: Send + Sync {
    fn observe(
        &self,
        layout: &AgentWorkspaceLayout,
        request: &SourceRepositoryBootstrapRequest,
    ) -> Result<SourceRepositoryBootstrapObservation, SourceRepositoryBootstrapPortError>;

    fn apply(
        &self,
        layout: &AgentWorkspaceLayout,
        request: &SourceRepositoryBootstrapRequest,
    ) -> Result<(), SourceRepositoryBootstrapPortError>;
}
