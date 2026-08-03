use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use tokio_util::sync::CancellationToken;

use crate::ports::agent_workspace_port::{AgentWorkspaceLayout, AgentWorkspacePortError};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BuildWorkspaceReadBack {
    pub workspace_root: PathBuf,
    pub source_commit: String,
    pub created: bool,
}

pub trait BuildWorkspacePort: Send + Sync {
    fn prepare(
        &self,
        layout: &AgentWorkspaceLayout,
        source_commit: &str,
    ) -> Result<BuildWorkspaceReadBack, AgentWorkspacePortError>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildExecutionRequest {
    pub argv: Vec<String>,
    pub working_directory: PathBuf,
    pub environment: BTreeMap<String, String>,
    pub timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct BuildExecutionReadBack {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BuildExecutionError {
    Invalid(String),
    Spawn(String),
    Timeout,
    Cancelled,
}

pub trait BuildExecutionPort: Send + Sync {
    async fn execute(
        &self,
        request: &BuildExecutionRequest,
        cancellation: &CancellationToken,
    ) -> Result<BuildExecutionReadBack, BuildExecutionError>;
}
