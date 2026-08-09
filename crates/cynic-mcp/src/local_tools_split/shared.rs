//! Shared result/event types for local MCP tool adapters.

/// Result of a git operation.
#[derive(Debug, serde::Serialize)]
pub struct GitResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Result of a Docker CLI operation.
#[derive(Debug, serde::Serialize)]
pub struct DockerResult {
    pub success: bool,
    pub command: String,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}

#[derive(Debug)]
pub(crate) struct DockerCmdResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
}
