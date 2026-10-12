//! Node compute CLI surface — run hybrid workloads & enroll remote runners.

use clap::Subcommand;
use std::process::ExitCode;
use std::sync::Arc;
use cynic_lifecycle::LocalArgvRunner;
use cynic_protocol::compute::{
    ComputeRequirements, RemoteRunnerSpec, SubstrateKind, SubstrateObservation,
};
use cynic_protocol::agent_isolation::SandboxStrategy;
use cynic_sandbox::adapters::bwrap_substrate::LocalBwrapSubstrateAdapter;
use cynic_node::usecases::compute_provisioning_uc::{
    ComputeProvisioningIntent, execute_compute_provisioning,
};
use cynic_node::usecases::remote_runner_enrollment_uc::execute_remote_runner_enrollment;
use cynic_network::adapters::headscale::HeadscaleClient;
use crate::emit_json;

#[derive(Subcommand, Debug, Clone)]
pub enum NodeComputeCommand {
    /// Execute a compute workload through the optimal sovereign substrate.
    Run {
        #[arg(long)]
        workload: String,
        #[arg(long, default_value_t = 1)]
        vcpus: u32,
        #[arg(long, default_value_t = 1073741824)] // 1 GiB
        memory_bytes: u64,
        #[arg(long, default_value_t = false)]
        gpu: bool,
        #[arg(long, default_value_t = 30000)]
        timeout_ms: u64,
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
    /// Provision a single-use enrollment pre-auth key for an external GPU/CPU runner.
    Enroll {
        #[arg(long)]
        node_name: String,
        #[arg(long, default_value_t = 4)]
        vcpus: u32,
        #[arg(long, default_value_t = 8589934592)] // 8 GiB
        memory_bytes: u64,
        #[arg(long, default_value_t = false)]
        gpu: bool,
        #[arg(long)]
        gpu_model: Option<String>,
        #[arg(long)]
        pubkey: String,
    },
}

pub async fn dispatch(cmd: NodeComputeCommand) -> ExitCode {
    match cmd {
        NodeComputeCommand::Run {
            workload,
            vcpus,
            memory_bytes,
            gpu,
            timeout_ms,
            command,
        } => {
            if command.is_empty() {
                eprintln!("error: command to execute inside runner cannot be empty");
                return ExitCode::FAILURE;
            }

            let requirements = ComputeRequirements {
                vcpus,
                memory_bytes,
                disk_bytes: 10 * 1024 * 1024 * 1024,
                requires_gpu: gpu,
                preferred_strategy: SandboxStrategy::BwrapEphemeral,
                allowed_strategies: vec![
                    SandboxStrategy::BwrapEphemeral,
                    SandboxStrategy::LxcUnprivileged,
                ],
            };

            let intent = ComputeProvisioningIntent {
                workload_id: workload,
                requirements,
                command,
                timeout_ms,
            };

            let substrates = vec![SubstrateObservation {
                name: "local-node".to_string(),
                kind: SubstrateKind::LocalBwrap,
                available_vcpus: 16,
                available_memory_bytes: 32 * 1024 * 1024 * 1024,
                available_disk_bytes: 500 * 1024 * 1024 * 1024,
                has_gpu: false,
                supported_strategies: vec![SandboxStrategy::BwrapEphemeral],
            }];

            let runner = Arc::new(LocalArgvRunner);
            let substrate_port = LocalBwrapSubstrateAdapter::new(runner);

            match execute_compute_provisioning(&intent, &substrates, &substrate_port).await {
                Ok(report) => emit_json(&report),
                Err(e) => {
                    eprintln!("error: compute provisioning failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        NodeComputeCommand::Enroll {
            node_name,
            vcpus,
            memory_bytes,
            gpu,
            gpu_model,
            pubkey,
        } => {
            let spec = RemoteRunnerSpec {
                node_name,
                vcpus,
                memory_bytes,
                disk_bytes: 50 * 1024 * 1024 * 1024,
                has_gpu: gpu,
                gpu_model,
                wireguard_pubkey: pubkey,
            };

            let mesh_client = HeadscaleClient::from_environment_or_authority();
            match execute_remote_runner_enrollment(&spec, &mesh_client).await {
                Ok(report) => emit_json(&report),
                Err(e) => {
                    eprintln!("error: remote runner enrollment failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
