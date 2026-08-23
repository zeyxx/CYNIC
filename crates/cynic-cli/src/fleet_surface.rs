use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Subcommand;

use cynic_host::peer_validator::PeerValidator;
use cynic_protocol::fleet_discovery::FleetNodeStatus;
use cynic_protocol::fleet_heartbeat::FleetHeartbeat;
use crate::fleet_report;
use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};

#[derive(Subcommand)]
pub(crate) enum FleetCommand {
    Report {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long)]
        proxmox_json: Option<String>,
        #[arg(long)]
        proxmox_list_text: Option<String>,
        #[arg(long)]
        watchdog_json: Option<String>,
    },
    /// Discover declared nodes and probe their services.
    Discover {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
    },
    /// Aggregate fleet health from heartbeats.
    Health {
        #[arg(long, default_value = "30")]
        heartbeat_timeout_s: u64,
        #[arg(long)]
        json: bool,
    },
    /// Observe disk usage across declared fleet nodes (SSH out, df -h).
    Disk {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        /// SSH target template (default "cynic@{ip}"); {ip} is substituted.
        #[arg(long, default_value = "cynic@{ip}")]
        ssh_target: String,
    },
    /// Emit a cryptographically signed heartbeat pulse.
    Pulse {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        cert_id: String,
        #[arg(long)]
        key_path: Option<String>,
        #[arg(long, default_value = "alive")]
        status: String,
        #[arg(long, default_value = "0")]
        uptime_s: u64,
        #[arg(long)]
        json: bool,
    },
}

fn current_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(crate) fn execute_fleet(command: FleetCommand) -> ExitCode {
    match command {
        FleetCommand::Report { json, registry, proxmox_json, proxmox_list_text, watchdog_json } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let runtime = resolve_runtime_paths(&RuntimeContext::from_env(registry));
            fleet_report::emit_result(fleet_report::emit_fleet_report(
                &runtime.registry_path,
                proxmox_json.as_deref(),
                proxmox_list_text.as_deref(),
                watchdog_json.as_deref(),
            ))
        }
        FleetCommand::Discover { json, registry } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry_path = resolve_runtime_paths(&RuntimeContext::from_env(registry)).registry_path;
            let adapter = cynic_host::fleet_discovery::FleetDiscoveryAdapter::new(
                registry_path.to_string_lossy().to_string(),
            );
            let report = adapter.discover();
            crate::cli_json::emit_pretty_json(&report)
        }
        FleetCommand::Health {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry_path =
                resolve_runtime_paths(&RuntimeContext::from_env(None)).registry_path;
            let adapter = cynic_host::fleet_health_observer::FleetHealthObserver::new(
                registry_path.to_string_lossy().to_string(),
                heartbeat_timeout_s,
            );
            let report = adapter.observe();
            crate::cli_json::emit_pretty_json(&report)
        }
        FleetCommand::Disk { json, registry, ssh_target } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry_path =
                resolve_runtime_paths(&RuntimeContext::from_env(registry)).registry_path;
            let adapter =
                cynic_host::fleet_disk_observer::FleetDiskObserver::with_runner(
                    registry_path.to_string_lossy().to_string(),
                    cynic_host::update_lifecycle::LocalArgvRunner,
                )
                .with_ssh_target(ssh_target);
            let report = adapter.observe();
            crate::cli_json::emit_pretty_json(&report)
        }
        FleetCommand::Pulse { node_id, cert_id, key_path, status, uptime_s, json } => {
            let key_p = key_path.map(PathBuf::from).unwrap_or_else(|| PathBuf::from("/etc/cynic/node-identity.key"));
            let private_key = match cynic_host::crypto::SecretStore::load_secret(&key_p) {
                Ok(k) => k,
                Err(e) => {
                    eprintln!("Failed to load node private key from {}: {e}", key_p.display());
                    return ExitCode::FAILURE;
                }
            };
            let node_status = match status.to_lowercase().as_str() {
                "alive" => FleetNodeStatus::Alive,
                "degraded" => FleetNodeStatus::Degraded,
                _ => FleetNodeStatus::Unreachable,
            };
            let now = current_unix_s();
            let hb = FleetHeartbeat {
                node_id,
                emitted_at_unix_s: now,
                self_status: node_status,
                uptime_s,
                unknowns: vec![],
            };
            let validator = PeerValidator::default();
            match validator.sign_heartbeat(&hb, &private_key, &cert_id) {
                Ok(signed) => {
                    if json {
                        crate::cli_json::emit_pretty_json(&signed)
                    } else {
                        println!("PULSE EMITTED: Node {} ({:?}) signed with cert {}", hb.node_id, hb.self_status, cert_id);
                        println!("Signature: {}", signed.signature_hex);
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("Failed to sign fleet pulse: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
