/// Portable remote execution command construction.
///
/// Policies name a target and a bounded remote argv; adapters decide whether
/// that target is reached through SSH, an agent bridge, or another transport.
pub trait RemoteExecutionPort: Send + Sync {
    fn exec_argv(&self, target: &str, remote_argv: &[&str]) -> Result<Vec<String>, String>;

    fn copy_argv(&self, source: &str, target: &str) -> Result<Vec<String>, String>;
}
