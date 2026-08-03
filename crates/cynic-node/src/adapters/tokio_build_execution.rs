use std::process::Stdio;

use tokio::process::Command;

use crate::ports::build_scheduler_port::{
    BuildExecutionError, BuildExecutionPort, BuildExecutionReadBack, BuildExecutionRequest,
};

#[derive(Debug, Clone, Copy, Default)]
pub struct TokioBuildExecution;

impl BuildExecutionPort for TokioBuildExecution {
    async fn execute(
        &self,
        request: &BuildExecutionRequest,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<BuildExecutionReadBack, BuildExecutionError> {
        if cancellation.is_cancelled() {
            return Err(BuildExecutionError::Cancelled);
        }
        let (program, arguments) = request
            .argv
            .split_first()
            .ok_or_else(|| BuildExecutionError::Invalid("build_execution_argv_missing".into()))?;
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
        let output = child.wait_with_output();
        tokio::pin!(output);
        tokio::select! {
            result = &mut output => {
                let result = result.map_err(|error| BuildExecutionError::Spawn(error.to_string()))?;
                Ok(BuildExecutionReadBack {
                    exit_code: result.status.code(),
                    stdout: String::from_utf8_lossy(&result.stdout).into_owned(),
                    stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
                })
            }
            () = cancellation.cancelled() => Err(BuildExecutionError::Cancelled),
            () = tokio::time::sleep(request.timeout) => Err(BuildExecutionError::Timeout),
        }
    }
}
