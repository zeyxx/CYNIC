//! Bubblewrap implementation of the canonical `ComputeSubstratePort`.

use cynic_lifecycle::{ArgvOutput, ArgvRunner};
use cynic_protocol::agent_isolation::SandboxStrategy;
use cynic_protocol::compute::{
    CommandExecutionResult, ComputeRequirements, ComputeSubstratePort, RunnerAllocation,
    RunnerStatus, SubstrateKind,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BwrapSubstrateError {
    #[error("unsupported compute requirements: {0}")]
    UnsupportedRequirements(String),
    #[error("allocation not found: {0}")]
    AllocationNotFound(String),
    #[error("execution failed: {0}")]
    ExecutionFailed(String),
    #[error("bwrap process error: {0}")]
    ProcessError(String),
}

/// Local Bubblewrap runner adapter realizing `ComputeSubstratePort`.
pub struct LocalBwrapSubstrateAdapter<R: ArgvRunner + Send + Sync> {
    runner: Arc<R>,
    alloc_counter: AtomicU64,
}

impl<R: ArgvRunner + Send + Sync> LocalBwrapSubstrateAdapter<R> {
    pub fn new(runner: Arc<R>) -> Self {
        Self {
            runner,
            alloc_counter: AtomicU64::new(1),
        }
    }

    fn generate_allocation_id(&self, workload_id: &str) -> String {
        let counter = self.alloc_counter.fetch_add(1, Ordering::SeqCst);
        format!("bwrap-{workload_id}-{counter:04}")
    }
}

impl<R: ArgvRunner + Send + Sync + 'static> ComputeSubstratePort for LocalBwrapSubstrateAdapter<R> {
    type Error = BwrapSubstrateError;

    async fn allocate_runner(
        &self,
        workload_id: &str,
        requirements: &ComputeRequirements,
    ) -> Result<RunnerAllocation, Self::Error> {
        if !requirements
            .allowed_strategies
            .contains(&SandboxStrategy::BwrapEphemeral)
        {
            return Err(BwrapSubstrateError::UnsupportedRequirements(
                "workload does not allow BwrapEphemeral isolation strategy".to_string(),
            ));
        }

        let allocation_id = self.generate_allocation_id(workload_id);
        Ok(RunnerAllocation {
            allocation_id,
            workload_id: workload_id.to_string(),
            substrate: SubstrateKind::LocalBwrap,
            strategy: SandboxStrategy::BwrapEphemeral,
            status: RunnerStatus::Allocated,
            network_endpoint: None,
        })
    }

    async fn execute_command(
        &self,
        _allocation_id: &str,
        argv: &[String],
        _timeout_ms: u64,
    ) -> Result<CommandExecutionResult, Self::Error> {
        let started = Instant::now();

        // Build typed bubblewrap command
        let mut bwrap_cmd = vec![
            "bwrap".to_string(),
            "--die-with-parent".to_string(),
            "--unshare-pid".to_string(),
            "--unshare-uts".to_string(),
            "--unshare-ipc".to_string(),
            "--unshare-net".to_string(),
            "--ro-bind".to_string(),
            "/usr".to_string(),
            "/usr".to_string(),
            "--ro-bind".to_string(),
            "/bin".to_string(),
            "/bin".to_string(),
            "--ro-bind".to_string(),
            "/lib".to_string(),
            "/lib".to_string(),
            "--ro-bind".to_string(),
            "/lib64".to_string(),
            "/lib64".to_string(),
            "--proc".to_string(),
            "/proc".to_string(),
            "--dev".to_string(),
            "/dev".to_string(),
            "--tmpfs".to_string(),
            "/tmp".to_string(),
            "--".to_string(),
        ];
        bwrap_cmd.extend(argv.iter().cloned());

        let cmd_refs: Vec<&str> = bwrap_cmd.iter().map(|s| s.as_str()).collect();
        let output: ArgvOutput = self.runner.run(&cmd_refs);

        let duration_ms = started.elapsed().as_millis() as u64;

        if let Some(err) = output.spawn_error {
            return Err(BwrapSubstrateError::ProcessError(err));
        }

        Ok(CommandExecutionResult {
            exit_code: output.exit_code.unwrap_or(-1),
            stdout: output.stdout,
            stderr: output.stderr,
            duration_ms,
        })
    }

    async fn release_runner(&self, _allocation_id: &str) -> Result<(), Self::Error> {
        // Ephemeral bwrap runs leave no persistent rootless state
        Ok(())
    }
}
