use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use serde::Serialize;
use tokio::io::AsyncWriteExt;
use zeroize::Zeroize;

#[derive(Clone, PartialEq, Eq)]
pub struct ArgvExecutionRequest {
    pub argv: Vec<String>,
    pub current_dir: Option<PathBuf>,
    pub stdin: Option<Vec<u8>>,
    pub timeout: Duration,
}

impl std::fmt::Debug for ArgvExecutionRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ArgvExecutionRequest")
            .field("argv", &self.argv)
            .field("current_dir", &self.current_dir)
            .field(
                "stdin",
                &self
                    .stdin
                    .as_ref()
                    .map(|payload| format!("[REDACTED;{} bytes]", payload.len())),
            )
            .field("timeout", &self.timeout)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArgvExecutionOutput {
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub spawn_error: Option<String>,
    pub timed_out: bool,
}

impl ArgvExecutionOutput {
    pub fn success(&self) -> bool {
        self.spawn_error.is_none() && !self.timed_out && self.exit_code == Some(0)
    }
}

#[async_trait]
pub trait ArgvExecutionPort: Send + Sync {
    async fn execute(&self, request: ArgvExecutionRequest) -> ArgvExecutionOutput;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalArgvExecutionAdapter;

#[async_trait]
impl ArgvExecutionPort for LocalArgvExecutionAdapter {
    async fn execute(&self, request: ArgvExecutionRequest) -> ArgvExecutionOutput {
        let Some(program) = request.argv.first().cloned() else {
            return failure("empty argv");
        };
        if request.timeout.is_zero() {
            return failure("execution timeout must be positive");
        }
        let mut command = tokio::process::Command::new(&program);
        command
            .args(&request.argv[1..])
            .kill_on_drop(true)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        if let Some(current_dir) = &request.current_dir {
            command.current_dir(current_dir);
        }
        if request.stdin.is_some() {
            command.stdin(std::process::Stdio::piped());
        } else {
            command.stdin(std::process::Stdio::null());
        }
        let mut child = match command.spawn() {
            Ok(child) => child,
            Err(error) => return failure(&format!("spawn {program}: {error}")),
        };
        if let Some(mut stdin) = request.stdin {
            let write_result = match child.stdin.take() {
                Some(mut pipe) => pipe.write_all(&stdin).await,
                None => Err(std::io::Error::other("child stdin pipe is unavailable")),
            };
            stdin.zeroize();
            if let Err(error) = write_result {
                return failure(&format!("write stdin for {program}: {error}"));
            }
        }
        match tokio::time::timeout(request.timeout, child.wait_with_output()).await {
            Ok(Ok(output)) => ArgvExecutionOutput {
                exit_code: output.status.code(),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                spawn_error: None,
                timed_out: false,
            },
            Ok(Err(error)) => failure(&format!("wait for {program}: {error}")),
            Err(_) => ArgvExecutionOutput {
                exit_code: None,
                stdout: String::new(),
                stderr: format!(
                    "argv execution timed out after {} ms",
                    request.timeout.as_millis()
                ),
                spawn_error: None,
                timed_out: true,
            },
        }
    }
}

fn failure(error: &str) -> ArgvExecutionOutput {
    ArgvExecutionOutput {
        exit_code: None,
        stdout: String::new(),
        stderr: String::new(),
        spawn_error: Some(error.to_string()),
        timed_out: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_debug_redacts_stdin_payload() {
        let request = ArgvExecutionRequest {
            argv: vec!["tool".to_string()],
            current_dir: None,
            stdin: Some(b"secret-value".to_vec()),
            timeout: Duration::from_secs(1),
        };

        let debug = format!("{request:?}");
        assert!(!debug.contains("secret-value"));
        assert!(debug.contains("REDACTED"));
    }
}
