use crate::ports::RemoteExecutionPort;

use cynic_ssh::argv::{scp_argv, ssh_argv};

/// SSH argv adapter used by the current registered transport profile.
///
/// The framework policy never owns these flags; replacing SSH with an agent
/// bridge only requires another `RemoteExecutionPort` implementation.
#[derive(Debug, Default, Clone, Copy)]
pub struct SshRemoteExecutionAdapter;

pub fn remote_execution_adapter(
    transport_kind: &str,
) -> Result<Box<dyn RemoteExecutionPort>, String> {
    match transport_kind {
        "ssh_bootstrap" => Ok(Box::new(SshRemoteExecutionAdapter)),
        other => Err(format!("remote_execution_adapter_unavailable:{other}")),
    }
}

impl RemoteExecutionPort for SshRemoteExecutionAdapter {
    fn exec_argv(&self, target: &str, remote_argv: &[&str]) -> Result<Vec<String>, String> {
        if remote_argv.is_empty() || remote_argv.iter().any(|arg| arg.is_empty()) {
            return Err("remote_execution_argv_missing".to_string());
        }
        ssh_argv(target, remote_argv).map_err(|_| "remote_execution_target_invalid".to_string())
    }

    fn copy_argv(&self, source: &str, target: &str) -> Result<Vec<String>, String> {
        if source.is_empty() || source.starts_with('-') {
            return Err("remote_execution_copy_path_invalid".to_string());
        }
        scp_argv(source, target).map_err(|_| "remote_execution_copy_path_invalid".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adapter_constructs_bounded_argv() {
        let argv = SshRemoteExecutionAdapter
            .exec_argv("builder", &["id", "-un"])
            .unwrap();
        assert_eq!(argv.first().map(String::as_str), Some("ssh"));
        assert_eq!(argv.last().map(String::as_str), Some("-un"));
    }

    #[test]
    fn adapter_rejects_shell_like_target() {
        assert!(
            SshRemoteExecutionAdapter
                .exec_argv("builder;whoami", &["true"])
                .is_err()
        );
    }
}
