//! Proxmox LXC container implementation of the canonical `ComputeSubstratePort`.

use cynic_process::{ArgvOutput, ArgvRunner};
use cynic_protocol::agent_isolation::SandboxStrategy;
use cynic_protocol::compute::{
    CommandExecutionResult, ComputeRequirements, ComputeSubstratePort, RunnerAllocation,
    RunnerStatus, SubstrateKind,
};
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Instant;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProxmoxLxcSubstrateError {
    #[error("invalid ssh target: {0}")]
    InvalidSshTarget(String),
    #[error("unsupported compute requirements: {0}")]
    UnsupportedRequirements(String),
    #[error("lxc allocation failed: {0}")]
    AllocationFailed(String),
    #[error("lxc execution failed: {0}")]
    ExecutionFailed(String),
    #[error("lxc cleanup failed: {0}")]
    CleanupFailed(String),
}

/// Remote Proxmox LXC runner adapter realizing `ComputeSubstratePort`.
pub struct ProxmoxLxcSubstrateAdapter<R: ArgvRunner + Send + Sync> {
    runner: Arc<R>,
    ssh_target: String,
    template_vmid: u32,
    vmid_counter: AtomicU32,
}

impl<R: ArgvRunner + Send + Sync> ProxmoxLxcSubstrateAdapter<R> {
    pub fn new(
        runner: Arc<R>,
        ssh_target: impl Into<String>,
        template_vmid: u32,
        base_vmid: u32,
    ) -> Result<Self, ProxmoxLxcSubstrateError> {
        let ssh_target = ssh_target.into();
        if ssh_target.trim().is_empty() || ssh_target.starts_with('-') {
            return Err(ProxmoxLxcSubstrateError::InvalidSshTarget(
                "ssh target is empty or starts with dash".to_string(),
            ));
        }

        Ok(Self {
            runner,
            ssh_target,
            template_vmid,
            vmid_counter: AtomicU32::new(base_vmid),
        })
    }

    fn run_ssh(&self, cmd: &[String]) -> Result<ArgvOutput, ProxmoxLxcSubstrateError> {
        let mut argv = vec![
            "ssh".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            self.ssh_target.clone(),
            "sudo".to_string(),
            "-n".to_string(),
        ];
        argv.extend(cmd.iter().cloned());
        let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
        let output = self.runner.run(&refs);
        if let Some(err) = output.spawn_error {
            return Err(ProxmoxLxcSubstrateError::ExecutionFailed(err));
        }
        Ok(output)
    }
}

impl<R: ArgvRunner + Send + Sync + 'static> ComputeSubstratePort for ProxmoxLxcSubstrateAdapter<R> {
    type Error = ProxmoxLxcSubstrateError;

    async fn allocate_runner(
        &self,
        workload_id: &str,
        requirements: &ComputeRequirements,
    ) -> Result<RunnerAllocation, Self::Error> {
        if !requirements
            .allowed_strategies
            .contains(&SandboxStrategy::LxcUnprivileged)
        {
            return Err(ProxmoxLxcSubstrateError::UnsupportedRequirements(
                "workload does not allow LxcUnprivileged strategy".to_string(),
            ));
        }

        let new_vmid = self.vmid_counter.fetch_add(1, Ordering::SeqCst);

        // 1. Clone template
        let clone_cmd = vec![
            "/usr/bin/pct".to_string(),
            "clone".to_string(),
            self.template_vmid.to_string(),
            new_vmid.to_string(),
            "--full".to_string(),
            "0".to_string(),
        ];
        let clone_out = self.run_ssh(&clone_cmd)?;
        if !clone_out.success() {
            return Err(ProxmoxLxcSubstrateError::AllocationFailed(format!(
                "pct clone failed: {}",
                clone_out.stderr
            )));
        }

        // 2. Set resource limits (cores, RAM)
        let memory_mb = requirements.memory_bytes / (1024 * 1024);
        let set_cmd = vec![
            "/usr/bin/pct".to_string(),
            "set".to_string(),
            new_vmid.to_string(),
            "-cores".to_string(),
            requirements.vcpus.to_string(),
            "-memory".to_string(),
            memory_mb.max(512).to_string(),
        ];
        let set_out = self.run_ssh(&set_cmd)?;
        if !set_out.success() {
            return Err(ProxmoxLxcSubstrateError::AllocationFailed(format!(
                "pct set resource limits failed: {}",
                set_out.stderr
            )));
        }

        // 3. Start container
        let start_cmd = vec![
            "/usr/bin/pct".to_string(),
            "start".to_string(),
            new_vmid.to_string(),
        ];
        let start_out = self.run_ssh(&start_cmd)?;
        if !start_out.success() {
            return Err(ProxmoxLxcSubstrateError::AllocationFailed(format!(
                "pct start failed: {}",
                start_out.stderr
            )));
        }

        Ok(RunnerAllocation {
            allocation_id: format!("lxc-{new_vmid}"),
            workload_id: workload_id.to_string(),
            substrate: SubstrateKind::ProxmoxLxc,
            strategy: SandboxStrategy::LxcUnprivileged,
            status: RunnerStatus::Running,
            network_endpoint: Some(format!("lxc://{}:{}", self.ssh_target, new_vmid)),
        })
    }

    async fn execute_command(
        &self,
        allocation_id: &str,
        argv: &[String],
        _timeout_ms: u64,
    ) -> Result<CommandExecutionResult, Self::Error> {
        let vmid_str = allocation_id.trim_start_matches("lxc-");
        let started = Instant::now();

        let mut exec_cmd = vec![
            "/usr/bin/pct".to_string(),
            "exec".to_string(),
            vmid_str.to_string(),
            "--".to_string(),
        ];
        exec_cmd.extend(argv.iter().cloned());

        let out = self.run_ssh(&exec_cmd)?;
        let duration_ms = started.elapsed().as_millis() as u64;

        Ok(CommandExecutionResult {
            exit_code: out.exit_code.unwrap_or(-1),
            stdout: out.stdout,
            stderr: out.stderr,
            duration_ms,
        })
    }

    async fn release_runner(&self, allocation_id: &str) -> Result<(), Self::Error> {
        let vmid_str = allocation_id.trim_start_matches("lxc-");

        // Stop container
        let stop_cmd = vec![
            "/usr/bin/pct".to_string(),
            "stop".to_string(),
            vmid_str.to_string(),
        ];
        let _ = self.run_ssh(&stop_cmd);

        // Destroy container
        let destroy_cmd = vec![
            "/usr/bin/pct".to_string(),
            "destroy".to_string(),
            vmid_str.to_string(),
            "--purge".to_string(),
            "1".to_string(),
        ];
        let out = self.run_ssh(&destroy_cmd)?;
        if !out.success() {
            return Err(ProxmoxLxcSubstrateError::CleanupFailed(format!(
                "pct destroy failed: {}",
                out.stderr
            )));
        }

        Ok(())
    }
}
