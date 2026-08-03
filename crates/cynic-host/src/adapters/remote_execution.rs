use crate::ports::RemoteExecutionPort;

/// SSH argv adapter used by the current registered transport profile.
///
/// The framework policy never owns these flags; replacing SSH with an agent
/// bridge only requires another `RemoteExecutionPort` implementation.
#[derive(Debug, Default, Clone, Copy)]
pub struct SshRemoteExecutionAdapter;

impl RemoteExecutionPort for SshRemoteExecutionAdapter {
    fn exec_argv(&self, target: &str, remote_argv: &[&str]) -> Result<Vec<String>, String> {
        validate_target(target)?;
        if remote_argv.is_empty() || remote_argv.iter().any(|arg| arg.is_empty()) {
            return Err("remote_execution_argv_missing".to_string());
        }
        let mut argv = vec![
            "ssh".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            "-o".to_string(),
            "ConnectTimeout=10".to_string(),
            target.to_string(),
        ];
        argv.extend(remote_argv.iter().map(|argument| (*argument).to_string()));
        Ok(argv)
    }

    fn copy_argv(&self, source: &str, target: &str) -> Result<Vec<String>, String> {
        validate_target(target)?;
        if source.is_empty() || source.starts_with('-') || target.contains(char::is_whitespace) {
            return Err("remote_execution_copy_path_invalid".to_string());
        }
        Ok(vec![
            "scp".to_string(),
            "-r".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            "-o".to_string(),
            "ConnectTimeout=10".to_string(),
            source.to_string(),
            target.to_string(),
        ])
    }
}

fn validate_target(target: &str) -> Result<(), String> {
    if target.trim().is_empty()
        || target.starts_with('-')
        || target.contains(char::is_whitespace)
        || target
            .chars()
            .any(|character| ";|&$()<>".contains(character))
    {
        return Err("remote_execution_target_invalid".to_string());
    }
    Ok(())
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
