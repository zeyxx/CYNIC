use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::process::Command;

use crate::ports::build_scheduler_port::{
    BuildExecutionError, BuildExecutionPort, BuildExecutionReadBack, BuildExecutionRequest,
};

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
            let mut command = Command::new(program);
            command
                .args(arguments)
                .current_dir(&request.working_directory)
                .envs(&request.environment)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);
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
                        if let Some(pid) = child_pid {
                            peak_memory_bytes = max_memory_sample(peak_memory_bytes, read_peak_rss_bytes(pid));
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
}
