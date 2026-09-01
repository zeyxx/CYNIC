//! Agent host VM placement and conformance CLI surface.

use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::agent_host::{
    AgentHostObservationPort, ProxmoxSshAgentHostObserver, plan_agent_host_vm,
};
use cynic_host::update_lifecycle::LocalArgvRunner;
use cynic_protocol::AgentHostDesired;

use crate::cli_json::emit_pretty_json;

#[derive(Subcommand)]
pub(crate) enum AgentHostCommand {
    /// Plan agent host VM placement and conformance on Proxmox
    Plan {
        /// Proxmox SSH target
        #[arg(long, default_value = "proxmox-forge")]
        hypervisor: String,
        /// VMID to allocate
        #[arg(long, default_value = "102")]
        vmid: u32,
        /// vCPUs allocated
        #[arg(long, default_value = "2")]
        vcpus: u32,
        /// Memory in MiB
        #[arg(long, default_value = "4096")]
        memory_mib: u64,
        /// Disk size in GiB
        #[arg(long, default_value = "32")]
        disk_gib: u64,
        /// Storage pool on Proxmox
        #[arg(long, default_value = "local-lvm")]
        storage_pool: String,
        /// Network bridge
        #[arg(long, default_value = "vmbr0")]
        bridge: String,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Observe Proxmox hypervisor preflight conditions
    Observe {
        /// Proxmox SSH target
        #[arg(long, default_value = "proxmox-forge")]
        hypervisor: String,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_agent_host(command: AgentHostCommand) -> ExitCode {
    match command {
        AgentHostCommand::Plan {
            hypervisor,
            vmid,
            vcpus,
            memory_mib,
            disk_gib,
            storage_pool,
            bridge,
            json,
        } => {
            let desired = AgentHostDesired {
                schema: cynic_protocol::AGENT_HOST_DESIRED_STATE_SCHEMA.to_string(),
                role: "agent_host".to_string(),
                harness: cynic_protocol::AgentHostHarness::Conformance,
                principal: "cynic".to_string(),
                host_node_id: "cynic-agent-host".to_string(),
                hypervisor_target: hypervisor.clone(),
                vmid,
                vcpus,
                memory_bytes: memory_mib * 1024 * 1024,
                disk_bytes: disk_gib * 1024 * 1024 * 1024,
                storage_pool,
                network_plane: "synapse".to_string(),
                network_bridge: bridge,
                network_ip: Some("10.10.10.102/24".to_string()),
                allowed_sandbox_strategies: vec![
                    cynic_protocol::SandboxStrategy::BwrapEphemeral,
                    cynic_protocol::SandboxStrategy::LxcUnprivileged,
                ],
                default_egress_policy: "default_deny".to_string(),
                cortex_autonomous: true,
            };

            let runner = LocalArgvRunner;
            let observer = match ProxmoxSshAgentHostObserver::new(&runner, hypervisor) {
                Ok(obs) => obs,
                Err(error) => {
                    eprintln!("failed to initialize hypervisor observer: {error}");
                    return ExitCode::FAILURE;
                }
            };

            let preflight = match observer.observe_agent_host_preflight(&desired) {
                Ok(p) => p,
                Err(error) => {
                    eprintln!("failed to observe hypervisor preflight: {error}");
                    return ExitCode::FAILURE;
                }
            };

            let plan = plan_agent_host_vm(&desired, &preflight);

            if json {
                emit_pretty_json(&plan);
            } else {
                println!(
                    "Agent Host VM Plan: status={}, VMID={}, vCPUs={}, RAM={} MiB, Disk={} GiB",
                    plan.status, desired.vmid, desired.vcpus, memory_mib, disk_gib
                );
                if !plan.blockers.is_empty() {
                    println!("Blockers:");
                    for blocker in &plan.blockers {
                        println!("  - {blocker}");
                    }
                }
                if !plan.actions.is_empty() {
                    println!("Planned Actions:");
                    for action in &plan.actions {
                        println!(
                            "  Step {}: {} -> {:?}",
                            action.step, action.purpose, action.argv
                        );
                    }
                }
            }

            if plan.status == "blocked" {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        AgentHostCommand::Observe { hypervisor, json } => {
            let desired = AgentHostDesired {
                hypervisor_target: hypervisor.clone(),
                ..Default::default()
            };

            let runner = LocalArgvRunner;
            let observer = match ProxmoxSshAgentHostObserver::new(&runner, hypervisor) {
                Ok(obs) => obs,
                Err(error) => {
                    eprintln!("failed to initialize hypervisor observer: {error}");
                    return ExitCode::FAILURE;
                }
            };

            let preflight = match observer.observe_agent_host_preflight(&desired) {
                Ok(p) => p,
                Err(error) => {
                    eprintln!("failed to observe hypervisor preflight: {error}");
                    return ExitCode::FAILURE;
                }
            };

            if json {
                emit_pretty_json(&preflight);
            } else {
                println!(
                    "Hypervisor Preflight: reachable={}, version={:?}, mem_avail={} GiB, disk_avail={} GiB, vmid_102_avail={}",
                    preflight.hypervisor_reachable,
                    preflight.hypervisor_version,
                    preflight.available_memory_bytes / (1024 * 1024 * 1024),
                    preflight.available_disk_bytes / (1024 * 1024 * 1024),
                    preflight.vmid_available
                );
            }

            if preflight.hypervisor_reachable {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    }
}
