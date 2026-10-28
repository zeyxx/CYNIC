//! Generic SSH remote executor.
//!
//! Carries a [`RemoteCommand`] to a fixed SSH endpoint by rendering the
//! typed argv through [`crate::argv::ssh_argv`] and running it through an
//! [`ArgvRunner`]. No shell composition: the endpoint and argv are validated
//! before they cross the transport boundary.

use cynic_protocol::argv::{ArgvOutput, ArgvRunner};

use crate::argv::ssh_argv;
use crate::command::{RemoteCommand, RemoteExecutorPort, RemoteOutcome};

/// Runs remote commands over SSH against a single endpoint.
pub struct SshRemoteExecutor<R: ArgvRunner> {
    endpoint: String,
    runner: R,
}

impl<R: ArgvRunner> SshRemoteExecutor<R> {
    /// Create an executor bound to `endpoint`, using `runner` to spawn argv.
    pub fn new(endpoint: impl Into<String>, runner: R) -> Self {
        Self {
            endpoint: endpoint.into(),
            runner,
        }
    }

    /// The endpoint this executor targets.
    pub fn endpoint(&self) -> &str {
        &self.endpoint
    }
}

impl<R: ArgvRunner> RemoteExecutorPort for SshRemoteExecutor<R> {
    fn run(&self, target: &str, command: &RemoteCommand) -> RemoteOutcome {
        let endpoint = if self.endpoint.trim().is_empty() {
            target
        } else {
            self.endpoint.as_str()
        };

        let argv = match ssh_argv(endpoint, &command.argv) {
            Ok(argv) => argv,
            Err(error) => {
                return RemoteOutcome {
                    exit_code: 255,
                    stdout: String::new(),
                    stderr: error,
                    transport_failed: true,
                };
            }
        };

        let borrowed: Vec<&str> = argv.iter().map(|value| value.as_str()).collect();
        let output: ArgvOutput = self.runner.run(&borrowed);

        if let Some(error) = output.spawn_error {
            return RemoteOutcome {
                exit_code: 255,
                stdout: output.stdout,
                stderr: error,
                transport_failed: true,
            };
        }

        let exit_code = output.exit_code.unwrap_or(255);
        let transport_failed = exit_code == 255;
        RemoteOutcome {
            exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            transport_failed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::argv::ArgvOutput;
    use std::sync::Arc;

    struct FakeRunner {
        output: ArgvOutput,
    }

    impl ArgvRunner for FakeRunner {
        fn run(&self, _argv: &[&str]) -> ArgvOutput {
            self.output.clone()
        }
    }

    #[test]
    fn runs_command_and_reports_outcome() {
        let runner = FakeRunner {
            output: ArgvOutput {
                exit_code: Some(0),
                stdout: "ok".into(),
                stderr: String::new(),
                spawn_error: None,
            },
        };
        let exec = SshRemoteExecutor::new("host", runner);
        let cmd = RemoteCommand {
            argv: vec!["true".into()],
        };
        let out = exec.run("ignored", &cmd);
        assert!(out.success());
        assert_eq!(out.stdout, "ok");
    }

    #[test]
    fn rejects_malicious_endpoint() {
        let runner = FakeRunner {
            output: ArgvOutput {
                exit_code: Some(0),
                stdout: String::new(),
                stderr: String::new(),
                spawn_error: None,
            },
        };
        let exec = SshRemoteExecutor::new("-oProxyCommand=evil", runner);
        let cmd = RemoteCommand {
            argv: vec!["true".into()],
        };
        let out = exec.run("host", &cmd);
        assert!(!out.success());
        assert!(out.transport_failed);
    }

    #[test]
    fn arc_runner_works() {
        let runner = Arc::new(FakeRunner {
            output: ArgvOutput {
                exit_code: Some(0),
                stdout: "hi".into(),
                stderr: String::new(),
                spawn_error: None,
            },
        });
        let exec = SshRemoteExecutor::new("host", runner);
        let cmd = RemoteCommand {
            argv: vec!["true".into()],
        };
        assert!(exec.run("host", &cmd).success());
    }
}
