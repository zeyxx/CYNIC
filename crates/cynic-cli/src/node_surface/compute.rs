//! Node compute CLI surface — run hybrid workloads & enroll remote runners.

use crate::emit_json;
use clap::Subcommand;
use cynic_lifecycle::LocalArgvRunner;
use cynic_network::adapters::headscale::{HeadscaleClient, HeadscaleConfig};
use cynic_node::usecases::compute_provisioning_uc::{
    ComputeProvisioningIntent, execute_compute_provisioning,
};
use cynic_node::usecases::remote_runner_enrollment_uc::execute_remote_runner_enrollment;
use cynic_protocol::agent_isolation::SandboxStrategy;
use cynic_protocol::compute::{
    ComputeRequirements, RemoteRunnerSpec, SubstrateKind, SubstrateObservation,
};
use cynic_protocol::provider::{
    LXC_PROVISIONING_SCHEMA, LxcProvisioningSpec, NodeSubstrateAction, NodeSubstrateControlPort,
    NodeSubstrateDesiredState, NodeSubstratePower, NodeSubstrateProvisioningSpec,
};
use cynic_proxmox::adapters::ProxmoxSshSubstrateAdapter;
use cynic_sandbox::adapters::bwrap_substrate::LocalBwrapSubstrateAdapter;
use std::process::ExitCode;
use std::sync::Arc;

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
    /// Provision a Proxmox LXC container through the sovereign substrate port.
    /// Emits the pure plan by default; --apply executes it over SSH.
    Provision {
        #[arg(long)]
        vmid: u32,
        #[arg(long)]
        hostname: String,
        #[arg(long)]
        template_volid: String,
        #[arg(long, default_value = "local-lvm")]
        storage_pool: String,
        #[arg(long, default_value_t = 16)]
        disk_gib: u32,
        #[arg(long, default_value_t = 2)]
        cores: u32,
        #[arg(long, default_value_t = 2048)]
        memory_mib: u32,
        #[arg(long, default_value_t = 1024)]
        swap_mib: u32,
        #[arg(long, default_value = "vmbr0")]
        bridge: String,
        #[arg(long)]
        ip_cidr: Option<String>,
        #[arg(long)]
        gateway: Option<String>,
        #[arg(long)]
        nameserver: Option<String>,
        /// LXC feature flags (repeatable, e.g. --features nesting=1).
        #[arg(long)]
        features: Vec<String>,
        #[arg(long, default_value_t = true)]
        unprivileged: bool,
        #[arg(long, default_value_t = true)]
        onboot: bool,
        /// Hypervisor SSH target resolved from the registry build-runner role
        /// when omitted (never a hardcoded default).
        #[arg(long)]
        hypervisor: Option<String>,
        #[arg(long, default_value_t = false)]
        apply: bool,
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
                Ok(report) => emit_json(&report, false),
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

            let base_url = std::env::var("HEADSCALE_URL")
                .unwrap_or_else(|_| "http://10.10.10.101:8080".to_string());
            let config = HeadscaleConfig::new(base_url, "headscale-api-key", "HEADSCALE_API_KEY");
            let mesh_client = match HeadscaleClient::new(config) {
                Ok(client) => client,
                Err(e) => {
                    eprintln!("error: failed to initialize headscale client: {e:?}");
                    return ExitCode::FAILURE;
                }
            };

            match execute_remote_runner_enrollment(&spec, &mesh_client) {
                Ok(report) => emit_json(&report, false),
                Err(e) => {
                    eprintln!("error: remote runner enrollment failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        NodeComputeCommand::Provision {
            vmid,
            hostname,
            template_volid,
            storage_pool,
            disk_gib,
            cores,
            memory_mib,
            swap_mib,
            bridge,
            ip_cidr,
            gateway,
            nameserver,
            features,
            unprivileged,
            onboot,
            hypervisor,
            apply,
        } => {
            let lxc_spec = LxcProvisioningSpec {
                schema: LXC_PROVISIONING_SCHEMA.to_string(),
                hostname: hostname.clone(),
                template_volid,
                storage_pool,
                disk_gib,
                cores,
                memory_mib,
                swap_mib,
                bridge,
                ip_cidr,
                gateway,
                nameserver,
                unprivileged,
                onboot,
                features,
            };
            let desired = NodeSubstrateDesiredState {
                node_id: hostname.clone(),
                provider: Some("proxmox".to_string()),
                resource_id: Some(format!("lxc/{vmid}")),
                desired_power: NodeSubstratePower::Running,
                provisioning: Some(NodeSubstrateProvisioningSpec {
                    schema: "cynic.node_substrate.provisioning.v1".to_string(),
                    image_ref: String::new(),
                    image_digest: String::new(),
                    runtime_seed_ref: String::new(),
                    runtime_seed_digest: String::new(),
                    network_profile_ref: String::new(),
                    lxc: Some(lxc_spec),
                }),
            };

            // Default path: emit the pure plan without touching the hypervisor.
            if !apply {
                let plan = cynic_proxmox::lxc_provisioning::plan_lxc_provisioning(
                    vmid,
                    desired
                        .provisioning
                        .as_ref()
                        .and_then(|p| p.lxc.as_ref())
                        .expect("lxc spec present"),
                    &crate::node_surface::compute::clear_lxc_preflight(),
                );
                emit_json(&plan, false);
                return if plan.status == "planned" {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                };
            }

            // Apply path: execute the planned argv over SSH through the
            // canonical substrate control port. The hypervisor SSH target is
            // operator-supplied (no hardcoded IP).
            let Some(hypervisor) = hypervisor else {
                eprintln!(
                    "error: --apply requires --hypervisor <ssh-target> (no hardcoded default)"
                );
                return ExitCode::FAILURE;
            };
            let runner = LocalArgvRunner;
            let adapter = match ProxmoxSshSubstrateAdapter::new(&runner, hypervisor) {
                Ok(adapter) => adapter,
                Err(e) => {
                    eprintln!("error: invalid hypervisor target: {e}");
                    return ExitCode::FAILURE;
                }
            };
            match adapter.apply_node_substrate_action(&desired, &NodeSubstrateAction::Provision) {
                Ok(()) => {
                    emit_json(
                        &serde_json::json!({
                            "schema": "cynic.node.compute.provision.v1",
                            "status": "provisioned",
                            "vmid": vmid,
                            "hostname": hostname,
                        }),
                        false,
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("error: lxc provisioning failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}

/// Clear preflight used by the plan-only path (no hypervisor contact).
/// WHY: the pure planner needs a preflight report to emit argv; the plan-only
/// CLI path assumes the operator pre-verified reachability, resources, and
/// template presence (the --apply path executes real preflight implicitly).
fn clear_lxc_preflight() -> cynic_proxmox::lxc_provisioning::LxcPreflightReport {
    cynic_proxmox::lxc_provisioning::LxcPreflightReport {
        schema: "cynic.proxmox.lxc_preflight.v1".to_string(),
        hypervisor_reachable: true,
        available_memory_bytes: u64::MAX / 2,
        available_disk_bytes: u64::MAX / 2,
        vmid_available: true,
        vmid_existing_status: None,
        template_present: true,
        storage_pool_available: true,
        network_bridge_present: true,
        blockers: Vec::new(),
    }
}
