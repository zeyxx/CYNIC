//! Group-aware child spawn for CYNIC-owned processes.
//!
//! Uses `process-wrap` for cross-platform orphan safety:
//! - Unix: `ProcessGroup::leader()` + `KillOnDrop`
//! - Windows: `JobObject` + `KillOnDrop`
//!
//! Stdio is inherited (NOT piped) to avoid pipe-buffer deadlock.
//! Returns `Err` if `command` is empty.

use std::{collections::HashMap, ffi::OsStr, io};

use process_wrap::tokio::{ChildWrapper, CommandWrap};
use tokio::time::sleep;

use cynic_protocol::supervision::{RestartDecision, decide_restart};

/// Spawn a CYNIC-owned child process in its own group (where supported).
pub fn spawn_backend(
    command: &[impl AsRef<OsStr>],
    working_dir: Option<&str>,
    env: &HashMap<String, String>,
) -> io::Result<Box<dyn ChildWrapper>> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "command must be non-empty"))?;

    let mut cmd = CommandWrap::with_new(program, |c| {
        c.args(args)
            .stdin(std::process::Stdio::inherit())
            .stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
        if let Some(dir) = working_dir {
            c.current_dir(dir);
        }
        for (k, v) in env {
            c.env(k, v);
        }
    });

    #[cfg(unix)]
    {
        use process_wrap::tokio::{KillOnDrop, ProcessGroup};
        cmd.wrap(ProcessGroup::leader()).wrap(KillOnDrop);
    }
    #[cfg(windows)]
    {
        use process_wrap::tokio::{JobObject, KillOnDrop};
        cmd.wrap(JobObject).wrap(KillOnDrop);
    }

    cmd.spawn()
}

/// Try to spawn the backend under a declared restart policy.
pub async fn spawn_with_retries(
    command: &[impl AsRef<OsStr>],
    working_dir: Option<&str>,
    env: &HashMap<String, String>,
    policy: &cynic_protocol::supervision::RestartPolicy,
) -> io::Result<Box<dyn ChildWrapper>> {
    use std::time::Duration;
    let mut last_err = io::Error::other("no attempts made");
    let mut attempt: u32 = 0;
    loop {
        match spawn_backend(command, working_dir, env) {
            Ok(child) => return Ok(child),
            Err(e) => {
                last_err = e;
                match decide_restart(policy, attempt, 0, true) {
                    RestartDecision::Restart { delay_secs } => {
                        attempt = attempt.saturating_add(1);
                        sleep(Duration::from_secs(delay_secs)).await;
                    }
                    RestartDecision::GiveUp { .. } => break,
                    RestartDecision::NoRestart => break,
                }
            }
        }
    }
    Err(last_err)
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn spawn_and_wait() {
        let env: HashMap<String, String> = HashMap::new();
        let mut child = spawn_backend(&["echo", "hello"], None, &env).unwrap();
        let status = child.wait().await.unwrap();
        assert!(status.success(), "echo should exit 0, got {status}");
    }

    #[tokio::test]
    async fn spawn_rejects_empty_command() {
        let env: HashMap<String, String> = HashMap::new();
        let empty: &[&str] = &[];
        assert!(spawn_backend(empty, None, &env).is_err());
    }
}
