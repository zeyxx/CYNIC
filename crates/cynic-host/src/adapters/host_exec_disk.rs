//! Bounded adapter for the injected `cynic-host-exec` authority.
//!
//! This is deliberately not a root or sudo variant: it invokes only the
//! installed host-exec binary with its fixed argv envelope and never composes a
//! shell command or substitutes another elevation mechanism.

use crate::domain::disk::{DiskCleanupResult, HostExecProof};
use crate::process_observation::{CommandCapture, capture_command};
use crate::runtime_layout::current_runtime_layout;

const OUTPUT_TAIL_BYTES: usize = 8 * 1024;

#[derive(Debug, Clone, Copy)]
pub enum HostExecDiskPhase {
    Preflight,
    Apply,
}

impl HostExecDiskPhase {
    fn flag(self) -> &'static str {
        match self {
            Self::Preflight => "--preflight",
            Self::Apply => "--apply",
        }
    }
}

#[derive(Clone)]
pub struct HostExecDiskInvocation {
    pub result: Option<DiskCleanupResult>,
    pub proof: HostExecProof,
}

/// Injectable execution authority for the host-exec bridge.
#[derive(Debug, Clone)]
pub struct HostExecDiskAdapter {
    helper: String,
}

impl HostExecDiskAdapter {
    pub fn from_runtime_layout() -> Self {
        Self {
            helper: current_runtime_layout().host_exec_helper_path(),
        }
    }

    pub fn new(helper: impl Into<String>) -> Self {
        Self {
            helper: helper.into(),
        }
    }

    pub fn invoke(&self, phase: HostExecDiskPhase) -> HostExecDiskInvocation {
        let helper = self.helper.clone();
        let args = ["disk-clean", "--privileged-only", phase.flag()];
        let argv = vec![
            helper.clone(),
            args[0].to_string(),
            args[1].to_string(),
            args[2].to_string(),
        ];

        interpret(argv, capture_command(&helper, &args))
    }
}

fn interpret(argv: Vec<String>, output: Result<CommandCapture, String>) -> HostExecDiskInvocation {
    match output {
        Ok(output) => match serde_json::from_str::<DiskCleanupResult>(&output.stdout) {
            Ok(result) => HostExecDiskInvocation {
                proof: HostExecProof {
                    executor: "host-exec".to_string(),
                    argv,
                    status: if output.success {
                        "succeeded".to_string()
                    } else {
                        "failed".to_string()
                    },
                    exit_code: output.exit_code,
                    stdout_tail: tail(&output.stdout),
                    stderr_tail: tail(&output.stderr),
                    post_action_partitions: result.partitions_after.clone(),
                },
                result: Some(result),
            },
            Err(error) if output.success => HostExecDiskInvocation {
                result: None,
                proof: HostExecProof {
                    executor: "host-exec".to_string(),
                    argv,
                    status: "unknown".to_string(),
                    exit_code: output.exit_code,
                    stdout_tail: tail(&output.stdout),
                    stderr_tail: format!("invalid host-exec proof: {error}"),
                    post_action_partitions: Vec::new(),
                },
            },
            Err(_) => blocked(
                argv,
                output.exit_code,
                "host-exec rejected privileged cleanup",
                &output.stderr,
            ),
        },
        Err(error) => blocked(argv, None, "host-exec unavailable", &error),
    }
}

fn blocked(
    argv: Vec<String>,
    exit_code: Option<i32>,
    context: &str,
    stderr: &str,
) -> HostExecDiskInvocation {
    HostExecDiskInvocation {
        result: None,
        proof: HostExecProof {
            executor: "host-exec".to_string(),
            argv,
            status: "blocked".to_string(),
            exit_code,
            stdout_tail: String::new(),
            stderr_tail: format!("{context}: {}", tail(stderr)),
            post_action_partitions: Vec::new(),
        },
    }
}

fn tail(value: &str) -> String {
    let start = value.len().saturating_sub(OUTPUT_TAIL_BYTES);
    value[start..].to_string()
}

#[cfg(test)]
mod tests {
    use super::{interpret, tail};
    use crate::domain::disk::{DiskCleanupResult, DiskPartition};
    use crate::process_observation::CommandCapture;

    #[test]
    fn proof_output_is_bounded() {
        let value = "x".repeat(9 * 1024);
        assert_eq!(tail(&value).len(), 8 * 1024);
    }

    #[test]
    fn successful_host_exec_retains_read_back_proof() {
        let result = DiskCleanupResult {
            schema: "test".to_string(),
            timestamp: "now".to_string(),
            success: true,
            items_executed: Vec::new(),
            total_reclaimed_bytes: 0,
            total_reclaimed_human: "0B".to_string(),
            partitions_before: Vec::new(),
            partitions_after: vec![DiskPartition::default()],
            host_exec_proof: None,
        };
        let invocation = interpret(
            vec!["host-exec".to_string(), "disk-clean".to_string()],
            Ok(CommandCapture {
                success: true,
                exit_code: Some(0),
                stdout: serde_json::to_string(&result).unwrap(),
                stderr: String::new(),
            }),
        );
        assert_eq!(invocation.proof.status, "succeeded");
        assert_eq!(
            invocation.proof.post_action_partitions,
            result.partitions_after
        );
    }

    #[test]
    fn unavailable_or_refused_authority_is_blocked() {
        let unavailable = interpret(
            vec!["host-exec".to_string()],
            Err("not installed".to_string()),
        );
        assert_eq!(unavailable.proof.status, "blocked");
        let refused = interpret(
            vec!["host-exec".to_string()],
            Ok(CommandCapture {
                success: false,
                exit_code: Some(1),
                stdout: String::new(),
                stderr: "permission denied".to_string(),
            }),
        );
        assert_eq!(refused.proof.status, "blocked");
        assert!(refused.proof.stderr_tail.contains("permission denied"));
    }
}
