//! Agent host VM placement and conformance CLI surface.
//!
//! Thin projection only: parse CLI args → resolve observer via canonical adapter
//! → call domain use-case → emit JSON or human text.
//!
//! No secret reading, no endpoint defaulting, no Proxmox policy in this file.
//! All credential and endpoint resolution is delegated to
//! `cynic_node::adapters::proxmox_control` (the canonical authority).

use std::process::ExitCode;
use std::time::Duration;

use clap::Subcommand;
use cynic_host::agent_host::{
    AgentHostObservationPort, ProxmoxRestAgentHostObserver, ProxmoxSshAgentHostObserver,
    plan_agent_host_vm,
};
use cynic_lifecycle::LocalArgvRunner;
use cynic_node::adapters::proxmox_control::ProxmoxControlAdapter;
use cynic_protocol::AgentHostDesired;

use crate::cli_json::emit_pretty_json;

/// Resolve the hypervisor observer from the canonical Proxmox control adapter.
///
/// Priority:
/// 1. REST via `ProxmoxControlAdapter::from_env` (endpoint + token + CA from
///    registry/env/file — no secret or IP ever appears here).
/// 2. SSH breakglass via `ProxmoxSshAgentHostObserver` (uses the supplied
///    `ssh_target` from the CLI arg, not a hardcoded hostname).
///
/// Returns `blocked_*` error strings when neither path is configured.
fn resolve_agent_host_observer(
    ssh_target: &str,
) -> Result<Box<dyn AgentHostObservationPort>, String> {
    // 1. Try canonical REST path — all config comes from the adapter
    match ProxmoxControlAdapter::from_env(Duration::from_secs(5)) {
        Ok(adapter) => {
            let ca_pem: Option<Vec<u8>> = adapter
                .ca_file
                .as_deref()
                .and_then(|p| std::fs::read(p).ok());
            let (endpoint, token_id, token_secret) = adapter.connection_parts();
            let obs = ProxmoxRestAgentHostObserver::new(
                endpoint,
                token_id,
                token_secret,
                ca_pem.as_deref(),
            )?;
            return Ok(Box::new(obs));
        }
        Err(e) => {
            eprintln!("REST observer unavailable (reason: {e}), falling back to SSH breakglass");
        }
    }

    // 2. SSH breakglass — ssh_target is the only CLI-supplied parameter
    let obs = ProxmoxSshAgentHostObserver::new(&LocalArgvRunner, ssh_target)?;
    Ok(Box::new(obs))
}

#[derive(Subcommand)]
pub(crate) enum AgentHostCommand {
    /// Plan agent host VM placement and conformance on Proxmox
    Plan {
        /// Proxmox SSH target (used only when REST is unavailable)
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
        /// Declared storage pool on Proxmox (must match registry desired state)
        #[arg(long, default_value = "local-lvm")]
        storage_pool: String,
        /// Network bridge
        #[arg(long, default_value = "vmbr0")]
        bridge: String,
        /// Declared template volid (exact match, e.g. local:vztmpl/ubuntu-24.04-standard_24.04-2_amd64.tar.zst)
        #[arg(long)]
        template_spec: Option<String>,
        /// Print JSON output
        #[arg(long)]
        json: bool,
    },
    /// Observe Proxmox hypervisor preflight conditions
    Observe {
        /// Proxmox SSH target (used only when REST is unavailable)
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
            template_spec,
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
                network_ip: None,
                allowed_sandbox_strategies: vec![
                    cynic_protocol::SandboxStrategy::BwrapEphemeral,
                    cynic_protocol::SandboxStrategy::LxcUnprivileged,
                ],
                default_egress_policy: "default_deny".to_string(),
                cortex_autonomous: true,
                template_spec,
            };

            let observer = match resolve_agent_host_observer(&hypervisor) {
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

            let observer = match resolve_agent_host_observer(&hypervisor) {
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
                    "Hypervisor Preflight: reachable={}, version={:?}, mem_avail={} GiB, disk_avail={} GiB, vmid_{}_avail={}",
                    preflight.hypervisor_reachable,
                    preflight.hypervisor_version,
                    preflight.available_memory_bytes / (1024 * 1024 * 1024),
                    preflight.available_disk_bytes / (1024 * 1024 * 1024),
                    desired.vmid,
                    preflight.vmid_available
                );
                if !preflight.blockers.is_empty() {
                    println!("Blockers:");
                    for b in &preflight.blockers {
                        println!("  - {b}");
                    }
                }
            }

            if preflight.hypervisor_reachable {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    }
}
