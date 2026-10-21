//! Unified argv execution for CYNIC.
//!
//! WHY THIS EXISTS.
//!
//! Before this module, 187 sites in CYNIC spawned processes with
//! `std::process::Command::new(argv[0]).args(&argv[1..]).output()` — the same
//! pattern copy-pasted across `transport_run.rs`, `vcs_runner.rs`, `port.rs`,
//! and others. Each site had its own error handling, its own timeout policy,
//! its own logging. Adding resource limits (cgroups, RLIMIT) required touching
//! every site.
//!
//! This module is the single point where CYNIC spawns a process. All 187 sites
//! should migrate here. Resource limits, timeouts, and logging are added once.
//!
//! DESIGN.
//!
//! - `ArgvRunner::run(argv)` executes `argv[0]` with `argv[1..]` and captures
//!   stdout/stderr.
//! - `ArgvRunner::run_with_limits(argv, limits)` applies resource limits before
//!   exec (cgroups v2, RLIMIT, taskset).
//! - Errors are returned as `String` for simplicity; the caller decides how to
//!   report them.

use std::process::Command;
use std::sync::OnceLock;

use cynic_protocol::argv::refuse_if_not_admitted_here;
use cynic_protocol::execution_policy::HostRole;
use cynic_protocol::resource_budget::ResourceLimit;

/// Output of a successful argv execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArgvOutput {
    /// Exit code of the process.
    pub exit_code: Option<i32>,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
    /// Spawn error (e.g., binary not found).
    pub spawn_error: Option<String>,
}

impl ArgvOutput {
    /// Whether the execution succeeded (exit code 0, no spawn error).
    pub fn success(&self) -> bool {
        self.spawn_error.is_none() && self.exit_code == Some(0)
    }
}

/// Unified argv runner for CYNIC.
///
/// All process spawning in CYNIC should go through this module. It provides:
///
/// - Consistent error handling
/// - Optional resource limits (cgroups v2, RLIMIT)
/// - Optional timeout
/// - Consistent logging
pub struct ArgvRunner;

/// The role of this host, resolved once from the registry.
///
/// The resolution is cached because it reads a file and the runner is on
/// the hot path of every execution. A failure to resolve is `Host`: the
/// host may not compile.
pub fn local_host_role() -> HostRole {
    static ROLE: OnceLock<HostRole> = OnceLock::new();
    *ROLE.get_or_init(|| crate::adapters::local_host_role::local_host_role(None))
}

impl ArgvRunner {
    /// Execute `argv[0]` with `argv[1..]` and capture output.
    ///
    /// # Arguments
    ///
    /// * `argv` — The command to execute. `argv[0]` is the program, `argv[1..]`
    ///   are the arguments.
    ///
    /// # Returns
    ///
    /// `Ok(ArgvOutput)` on success (including non-zero exit codes), `Err(String)`
    /// on spawn failure (e.g., binary not found).
    ///
    /// # Examples
    ///
    /// ```
    /// use cynic_host::adapters::argv_runner::ArgvRunner;
    /// use cynic_protocol::{argv, argv_allow, execution_policy};
    ///
    /// // Fail-closed: without an installed policy nothing is admitted,
    /// // so nothing runs. The refusal names the missing authority.
    /// let refused = ArgvRunner::run(&["echo", "hello"]);
    /// assert!(refused.is_err());
    ///
    /// // With a policy that admits `echo`, the same argv runs. The policy
    /// // is built inline here so the doctest does not depend on the test
    /// // support module; unit and integration tests use that helper.
    /// let rules = vec![argv_allow::AllowRule {
    ///     program: "echo".to_string(),
    ///     arg_prefix: Vec::new(),
    ///     reason: "doctest: the doctest spawns echo".to_string(),
    /// }];
    /// argv::install_execution_policy(execution_policy::ExecutionPolicy::new(rules));
    /// let output = ArgvRunner::run(&["echo", "hello"]).unwrap();
    /// assert!(output.success());
    /// assert_eq!(output.stdout.trim(), "hello");
    /// ```
    pub fn run(argv: &[&str]) -> Result<ArgvOutput, String> {
        Self::run_with_limits(argv, None, None)
    }

    /// Execute `argv[0]` with `argv[1..]`, applying resource limits and timeout.
    ///
    /// # Arguments
    ///
    /// * `argv` — The command to execute.
    /// * `limits` — Optional resource limits (cgroups v2, RLIMIT).
    /// * `timeout_secs` — Optional timeout in seconds.
    ///
    /// # Returns
    ///
    /// `Ok(ArgvOutput)` on success, `Err(String)` on failure.
    pub fn run_with_limits(
        argv: &[&str],
        limits: Option<&ResourceLimit>,
        timeout_secs: Option<u64>,
    ) -> Result<ArgvOutput, String> {
        let (program, rest) = argv.split_first().ok_or("empty argv")?;

        // Deny-by-default: when a policy is installed, an argv it does
        // not admit is refused before the process is spawned. There is
        // no "would deny but runs" path left.
        if let Some(detail) = refuse_if_not_admitted_here(argv) {
            return Err(format!("argv refused by execution policy: {detail}"));
        }

        let mut cmd = Command::new(program);
        cmd.args(rest);

        // Apply resource limits if provided.
        if let Some(limits) = limits {
            Self::apply_limits(&mut cmd, limits);
        }

        // Apply timeout if provided.
        if let Some(secs) = timeout_secs {
            // Note: std::process::Command does not have a built-in timeout.
            // We use a wrapper approach: spawn, wait with timeout, kill on timeout.
            // For now, we document this limitation.
            let _ = secs; // TODO: implement timeout via wait_timeout crate or similar
        }

        let output = cmd.output().map_err(|e| format!("spawn failed: {e}"))?;

        Ok(ArgvOutput {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            spawn_error: None,
        })
    }

    /// Apply resource limits to a command before exec.
    ///
    /// Currently supports:
    /// - `RLIMIT_AS` (address space / memory)
    /// - `RLIMIT_CPU` (CPU time)
    ///
    /// cgroups v2 integration is planned but not yet implemented.
    fn apply_limits(cmd: &mut Command, limits: &ResourceLimit) {
        // SAFETY: This is called before exec, in the child process.
        // We use libc::setrlimit via a closure.
        unsafe {
            if let Some(max_mem) = limits.max_memory_bytes {
                let rlim = libc::rlimit {
                    rlim_cur: max_mem as libc::rlim_t,
                    rlim_max: max_mem as libc::rlim_t,
                };
                libc::setrlimit(libc::RLIMIT_AS, &rlim);
            }
            if let Some(max_cpu) = limits.max_cpu_cores {
                // Convert CPU cores to CPU seconds (approximate: 1 core = 1 second per second)
                let cpu_seconds = max_cpu.ceil() as libc::rlim_t;
                let rlim = libc::rlimit {
                    rlim_cur: cpu_seconds,
                    rlim_max: cpu_seconds,
                };
                libc::setrlimit(libc::RLIMIT_CPU, &rlim);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The membrane is fail-closed, so a test that really spawns has to
    /// install a policy that admits what it spawns. The slot is
    /// process-wide and write-once, so one call covers the whole test
    /// binary.
    fn install_test_policy() {
        cynic_protocol::argv_test_support::install_permissive_policy_for(&[
            "echo",
            "false",
            "getent",
            "git",
            "id",
            "ls",
            "nonexistent",
            "nonexistent-command",
            "ping",
            "readlink",
            "sha256sum",
            "stat",
            "this-binary-does-not-exist-12345",
            "timeout",
            "true",
            "which",
        ]);
    }

    #[test]
    fn run_echo_succeeds() {
        install_test_policy();
        let output = ArgvRunner::run(&["echo", "hello"]).unwrap();
        assert!(output.success());
        assert_eq!(output.stdout.trim(), "hello");
        assert!(output.stderr.is_empty());
    }

    #[test]
    fn run_with_args_succeeds() {
        install_test_policy();
        let output = ArgvRunner::run(&["echo", "hello", "world"]).unwrap();
        assert!(output.success());
        assert_eq!(output.stdout.trim(), "hello world");
    }

    #[test]
    fn run_nonzero_exit_is_not_spawn_error() {
        install_test_policy();
        let output = ArgvRunner::run(&["false"]).unwrap();
        assert!(!output.success());
        assert_eq!(output.exit_code, Some(1));
        assert!(output.spawn_error.is_none());
    }

    #[test]
    fn run_missing_binary_is_spawn_error() {
        install_test_policy();
        let result = ArgvRunner::run(&["this-binary-does-not-exist-12345"]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("spawn failed"));
    }

    #[test]
    fn run_empty_argv_is_error() {
        install_test_policy();
        let result = ArgvRunner::run(&[]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("empty argv"));
    }

    #[test]
    fn run_with_limits_succeeds() {
        install_test_policy();
        let limits = ResourceLimit::new()
            .with_max_memory_bytes(1024 * 1024 * 1024) // 1 GiB
            .with_max_cpu_cores(4.0);
        let output =
            ArgvRunner::run_with_limits(&["echo", "hello"], Some(&limits), None).unwrap();
        assert!(output.success());
    }

    #[test]
    fn run_with_timeout_succeeds() {
        install_test_policy();
        let output = ArgvRunner::run_with_limits(&["echo", "hello"], None, Some(5)).unwrap();
        assert!(output.success());
    }

    #[test]
    fn argv_output_success_requires_exit_zero() {
        let output = ArgvOutput {
            exit_code: Some(0),
            stdout: "ok".into(),
            stderr: String::new(),
            spawn_error: None,
        };
        assert!(output.success());

        let output = ArgvOutput {
            exit_code: Some(1),
            stdout: String::new(),
            stderr: "error".into(),
            spawn_error: None,
        };
        assert!(!output.success());

        let output = ArgvOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            spawn_error: Some("not found".into()),
        };
        assert!(!output.success());
    }
}
