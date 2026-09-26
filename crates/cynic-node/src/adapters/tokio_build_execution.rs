use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::process::Command;

use crate::ports::build_scheduler_port::{
    BuildExecutionError, BuildExecutionPort, BuildExecutionReadBack, BuildExecutionRequest,
    BuildSandboxMode,
};
use cynic_sandbox::cargo::build_cargo_sandbox_argv;

#[derive(Debug, Clone, Copy, Default)]
pub struct TokioBuildExecution;

impl BuildExecutionPort for TokioBuildExecution {
    fn execute<'a>(
        &'a self,
        request: &'a BuildExecutionRequest,
        cancellation: &'a tokio_util::sync::CancellationToken,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<BuildExecutionReadBack, BuildExecutionError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(async move {
            if cancellation.is_cancelled() {
                return Err(BuildExecutionError::Cancelled);
            }
            let (program, arguments) = request.argv.split_first().ok_or_else(|| {
                BuildExecutionError::Invalid("build_execution_argv_missing".into())
            })?;
            if !request.working_directory.is_absolute() {
                return Err(BuildExecutionError::Invalid(
                    "build_execution_working_directory_not_absolute".into(),
                ));
            }
            // Resolve the launch plan. Sandboxed builds wrap the admitted
            // argv with bwrap and pass the bounded environment exclusively
            // via --setenv; the host environment never leaks inside.
            // Bare-metal (Off) preserves the historical direct spawn.
            let (program, arguments, inherit_host_env, observe_memory) = match &request.sandbox {
                BuildSandboxMode::Off => (program.clone(), arguments.to_vec(), true, true),
                BuildSandboxMode::Bwrap(spec) => {
                    if let Err(error) = tokio::fs::create_dir_all(&spec.target_directory).await {
                        return Err(BuildExecutionError::Invalid(format!(
                            "build_sandbox_target_dir_unwritable:{error}"
                        )));
                    }
                    let passthrough = request
                        .environment
                        .iter()
                        .map(|(key, value)| (key.clone(), value.clone()))
                        .collect::<Vec<_>>();
                    let sandbox_argv = build_cargo_sandbox_argv(spec, &request.argv, &passthrough)
                        .map_err(BuildExecutionError::Invalid)?;
                    let (sandbox_program, sandbox_arguments) =
                        sandbox_argv.split_first().ok_or_else(|| {
                            BuildExecutionError::Invalid("build_sandbox_argv_missing".into())
                        })?;
                    (
                        sandbox_program.clone(),
                        sandbox_arguments.to_vec(),
                        false,
                        false,
                    )
                }
            };
            let mut command = Command::new(&program);
            command
                .args(&arguments)
                .current_dir(&request.working_directory)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);
            if inherit_host_env {
                command.envs(&request.environment);
            }
            let child = command
                .spawn()
                .map_err(|error| BuildExecutionError::Spawn(error.to_string()))?;
            let child_pid = child.id();
            let output = child.wait_with_output();
            tokio::pin!(output);
            let deadline = Instant::now() + request.timeout;
            let mut next_sample = Instant::now();
            let mut peak_memory_bytes = None;

            loop {
                tokio::select! {
                    result = &mut output => {
                        let result = result.map_err(|error| BuildExecutionError::Spawn(error.to_string()))?;
                        return Ok(BuildExecutionReadBack {
                            exit_code: result.status.code(),
                            stdout: String::from_utf8_lossy(&result.stdout).into_owned(),
                            stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
                            peak_memory_bytes,
                        });
                    }
                    () = cancellation.cancelled() => return Err(BuildExecutionError::Cancelled),
                    () = tokio::time::sleep_until(tokio::time::Instant::from_std(next_sample)) => {
                        // Sandboxed builds report no peak: the supervisor PID
                        // is bwrap itself, not the confined compiler. The
                        // scheduler records an explicit unknown instead.
                        if observe_memory {
                            if let Some(pid) = child_pid {
                                peak_memory_bytes = max_memory_sample(peak_memory_bytes, read_peak_rss_bytes(pid));
                            }
                        }
                        if Instant::now() >= deadline {
                            return Err(BuildExecutionError::Timeout);
                        }
                        next_sample = Instant::now() + Duration::from_millis(25);
                    }
                    () = tokio::time::sleep_until(tokio::time::Instant::from_std(deadline)) => {
                        return Err(BuildExecutionError::Timeout);
                    }
                }
            }
        })
    }
}

fn max_memory_sample(current: Option<u64>, sample: Option<u64>) -> Option<u64> {
    match (current, sample) {
        (Some(current), Some(sample)) => Some(current.max(sample)),
        (None, Some(sample)) => Some(sample),
        (current, None) => current,
    }
}

fn read_peak_rss_bytes(pid: u32) -> Option<u64> {
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    let kilobytes = status.lines().find_map(|line| {
        let (key, value) = line.split_once(':')?;
        (key == "VmHWM").then_some(value.trim().strip_suffix(" kB")?.parse::<u64>().ok()?)
    })?;
    kilobytes.checked_mul(1024)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn execution_read_back_contains_child_peak_memory_when_proc_is_observable() {
        let request = BuildExecutionRequest {
            argv: vec!["/bin/sleep".into(), "0.08".into()],
            working_directory: std::path::PathBuf::from("/"),
            environment: Default::default(),
            timeout: Duration::from_secs(2),
            sandbox: BuildSandboxMode::Off,
        };
        let cancellation = tokio_util::sync::CancellationToken::new();
        let read_back = TokioBuildExecution
            .execute(&request, &cancellation)
            .await
            .expect("child execution");

        assert_eq!(read_back.exit_code, Some(0));
        assert!(read_back.peak_memory_bytes.is_some());
    }

    #[test]
    fn memory_samples_keep_the_highest_observed_value() {
        assert_eq!(max_memory_sample(None, Some(10)), Some(10));
        assert_eq!(max_memory_sample(Some(20), Some(10)), Some(20));
        assert_eq!(max_memory_sample(Some(10), None), Some(10));
    }

    #[test]
    fn missing_process_memory_is_explicitly_unknown() {
        assert_eq!(read_peak_rss_bytes(u32::MAX), None);
    }

    #[tokio::test]
    async fn sandboxed_execution_fails_closed_when_helper_is_missing() {
        use cynic_sandbox::cargo::CargoSandboxSpec;
        let request = BuildExecutionRequest {
            argv: vec!["cargo".into(), "--version".into()],
            working_directory: std::path::PathBuf::from("/tmp"),
            environment: Default::default(),
            timeout: Duration::from_secs(10),
            sandbox: BuildSandboxMode::Bwrap(CargoSandboxSpec {
                bwrap_bin: "/nonexistent/bwrap".to_string(),
                working_directory: std::path::PathBuf::from("/tmp"),
                target_directory: std::path::PathBuf::from("/tmp/sandbox-target-missing"),
                cargo_home: std::path::PathBuf::from("/tmp"),
                rustup_home: std::path::PathBuf::from("/tmp"),
                extra_ro_binds: vec![],
            }),
        };
        let cancellation = tokio_util::sync::CancellationToken::new();
        let error = TokioBuildExecution
            .execute(&request, &cancellation)
            .await
            .expect_err("missing bwrap helper must fail closed");
        assert!(
            matches!(error, BuildExecutionError::Spawn(_)),
            "expected spawn failure, got {error:?}"
        );
    }

    /// Live sandbox proof. Runs only when `CYNIC_BWRAP_BIN` points at a real
    /// bwrap binary; otherwise reports the skip explicitly. On the builder:
    /// `CYNIC_BWRAP_BIN=/tmp/bwrap-pkg/root/usr/bin/bwrap cargo test ...`.
    #[tokio::test]
    async fn sandboxed_cargo_check_converges_without_network() {
        use cynic_sandbox::cargo::CargoSandboxSpec;
        let bwrap_bin = std::env::var("CYNIC_BWRAP_BIN").unwrap_or_default();
        if bwrap_bin.trim().is_empty() || !std::path::Path::new(&bwrap_bin).is_file() {
            eprintln!("SKIPPED sandboxed_cargo_check: CYNIC_BWRAP_BIN is not a file");
            return;
        }
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("workspace root above crates/<pkg>")
            .to_path_buf();
        let target_directory = std::env::temp_dir().join("cynic-sandbox-target-live");
        let mut spec =
            CargoSandboxSpec::resolve_from_host_env(workspace_root.clone(), Some(target_directory))
                .expect("live sandbox proof needs PATH plus absolute workspace");
        spec.bwrap_bin = bwrap_bin;
        let request = BuildExecutionRequest {
            argv: vec![
                "cargo".into(),
                "check".into(),
                "-p".into(),
                "cynic-protocol".into(),
                "--lib".into(),
                "--frozen".into(),
            ],
            working_directory: workspace_root,
            environment: [
                ("CARGO_INCREMENTAL".to_string(), "0".to_string()),
                ("CARGO_BUILD_JOBS".to_string(), "1".to_string()),
            ]
            .into_iter()
            .collect(),
            timeout: Duration::from_secs(420),
            sandbox: BuildSandboxMode::Bwrap(spec),
        };
        let cancellation = tokio_util::sync::CancellationToken::new();
        let read_back = TokioBuildExecution
            .execute(&request, &cancellation)
            .await
            .expect("sandboxed cargo check must converge");
        assert_eq!(read_back.exit_code, Some(0));
        // Supervisor-RSS is not build-RSS: honestly unobserved.
        assert_eq!(read_back.peak_memory_bytes, None);
    }
}
