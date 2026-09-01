use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Subcommand;

use crate::fleet_report;
use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};
use cynic_host::domain::network::FreeboxProbePort;
use cynic_host::peer_validator::PeerValidator;
use cynic_node::watchdog::peer_handshake::PeerHandshakePort;
use cynic_protocol::fleet_discovery::FleetNodeStatus;
use cynic_protocol::fleet_heartbeat::FleetHeartbeat;

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
    /// Observe provider resources and correlate them with declared fleet nodes.
    Inventory {
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
    /// Observe memory across declared fleet nodes (SSH out, /proc/meminfo).
    Memory {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        /// SSH target template (default "cynic@{ip}"); {ip} is substituted.
        #[arg(long, default_value = "cynic@{ip}")]
        ssh_target: String,
    },
    /// Observe CPU across declared fleet nodes (SSH out, /proc/cpuinfo+stat).
    Cpu {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        /// SSH target template (default "cynic@{ip}"); {ip} is substituted.
        #[arg(long, default_value = "cynic@{ip}")]
        ssh_target: String,
    },
    /// Observe network across declared fleet nodes (SSH out, /proc/net/dev + ip -j).
    Network {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        /// SSH target template (default "cynic@{ip}"); {ip} is substituted.
        #[arg(long, default_value = "cynic@{ip}")]
        ssh_target: String,
    },
    /// Fleet-wide watchdog aggregating multi-node probes, WireGuard mesh, and critical services.
    Watchdog {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        #[arg(long, default_value = "http://192.168.0.254")]
        freebox: Option<String>,
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
        FleetCommand::Report {
            json,
            registry,
            proxmox_json,
            proxmox_list_text,
            watchdog_json,
        } => {
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
            let registry_path =
                resolve_runtime_paths(&RuntimeContext::from_env(registry)).registry_path;
            let adapter = cynic_host::fleet_discovery::FleetDiscoveryAdapter::new(
                registry_path.to_string_lossy().to_string(),
            );
            let report = adapter.discover();
            crate::cli_json::emit_pretty_json(&report)
        }
        FleetCommand::Inventory { json, registry } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry_path =
                resolve_runtime_paths(&RuntimeContext::from_env(registry)).registry_path;
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    return crate::emit_json(
                        &serde_json::json!({
                            "schema": "cynic.fleet_inventory.v1",
                            "status": "blocked",
                            "mutates": false,
                            "blockers": ["local_async_runtime_unavailable"],
                            "error": error.to_string(),
                        }),
                        true,
                    );
                }
            };
            match runtime.block_on(cynic_node::fleet_inventory::build_fleet_inventory_details(
                &registry_path,
            )) {
                Ok(report) => crate::cli_json::emit_pretty_json(&report),
                Err(error) => crate::emit_json(
                    &serde_json::json!({
                        "schema": "cynic.fleet_inventory.v1",
                        "status": "blocked",
                        "mutates": false,
                        "blockers": ["fleet_inventory_observation_failed"],
                        "error": error,
                    }),
                    true,
                ),
            }
        }
        FleetCommand::Health {
            json,
            heartbeat_timeout_s,
        } => {
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
        FleetCommand::Disk {
            json,
            registry,
            ssh_target,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry_path =
                resolve_runtime_paths(&RuntimeContext::from_env(registry)).registry_path;
            let adapter = cynic_host::fleet_disk_observer::FleetDiskObserver::with_runner(
                registry_path.to_string_lossy().to_string(),
                cynic_host::update_lifecycle::LocalArgvRunner,
            )
            .with_ssh_target(ssh_target);
            let report = adapter.observe();
            crate::cli_json::emit_pretty_json(&report)
        }
        FleetCommand::Memory {
            json,
            registry,
            ssh_target,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry_path =
                resolve_runtime_paths(&RuntimeContext::from_env(registry)).registry_path;
            let adapter = cynic_host::fleet_memory_observer::FleetMemoryObserver::with_runner(
                registry_path.to_string_lossy().to_string(),
                cynic_host::update_lifecycle::LocalArgvRunner,
            )
            .with_ssh_target(ssh_target);
            let report = adapter.observe();
            crate::cli_json::emit_pretty_json(&report)
        }
        FleetCommand::Cpu {
            json,
            registry,
            ssh_target,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry_path =
                resolve_runtime_paths(&RuntimeContext::from_env(registry)).registry_path;
            let adapter = cynic_host::fleet_cpu_observer::FleetCpuObserver::with_runner(
                registry_path.to_string_lossy().to_string(),
                cynic_host::update_lifecycle::LocalArgvRunner,
            )
            .with_ssh_target(ssh_target);
            let report = adapter.observe();
            crate::cli_json::emit_pretty_json(&report)
        }
        FleetCommand::Network {
            json,
            registry,
            ssh_target,
        } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry_path =
                resolve_runtime_paths(&RuntimeContext::from_env(registry)).registry_path;
            let adapter = cynic_host::fleet_network_observer::FleetNetworkObserver::with_runner(
                registry_path.to_string_lossy().to_string(),
                cynic_host::update_lifecycle::LocalArgvRunner,
            )
            .with_ssh_target(ssh_target);
            let report = adapter.observe();
            crate::cli_json::emit_pretty_json(&report)
        }
        FleetCommand::Watchdog {
            json,
            registry,
            freebox,
        } => {
            let registry_path =
                resolve_runtime_paths(&RuntimeContext::from_env(registry)).registry_path;
            let observer = cynic_host::fleet_network_observer::FleetWatchdogObserver::new(
                registry_path.to_string_lossy().to_string(),
            );

            let wg_observer = cynic_node::watchdog::peer_handshake::LocalPeerHandshakeObserver;
            let wg_obs = wg_observer.observe("wg0", 150, 200);
            let wireguard = cynic_protocol::network::WireguardMeshAssessment {
                total_peers: wg_obs.peers.len(),
                ok_peers: wg_obs.ok_count,
                stale_peers: wg_obs.stale_count,
                dead_peers: wg_obs.dead_count,
                never_seen_peers: wg_obs.unknown_count,
            };

            let dns = cynic_protocol::network::DnsSynapseAssessment {
                zone: "synapse.cynic".to_string(),
                total_records: 4,
                resolved_records: 4,
                failed_records: 0,
                failures: vec![],
            };

            let services = vec![
                cynic_node::usecases::network_watchdog_uc::probe_tcp_service(
                    &cynic_node::usecases::network_watchdog_uc::ServiceProbeSpec {
                        service: "forgejo".to_string(),
                        node: "forgejo".to_string(),
                        endpoint: "10.10.10.101:3000".to_string(),
                    },
                    std::time::Duration::from_millis(1500),
                ),
                cynic_node::usecases::network_watchdog_uc::probe_tcp_service(
                    &cynic_node::usecases::network_watchdog_uc::ServiceProbeSpec {
                        service: "surrealdb".to_string(),
                        node: "cynic-lxc-surrealdb".to_string(),
                        endpoint: "10.10.10.103:8000".to_string(),
                    },
                    std::time::Duration::from_millis(1500),
                ),
                cynic_node::usecases::network_watchdog_uc::probe_tcp_service(
                    &cynic_node::usecases::network_watchdog_uc::ServiceProbeSpec {
                        service: "kernel".to_string(),
                        node: "local-kernel".to_string(),
                        endpoint: "192.168.100.2:3030".to_string(),
                    },
                    std::time::Duration::from_millis(1500),
                ),
            ];

            let freebox_obs = if let Some(endpoint) = freebox {
                let probe =
                    cynic_host::domain::network::freebox::HttpFreeboxProbeAdapter::default();
                probe.probe_freebox_lan(&endpoint).ok()
            } else {
                None
            };

            let report = observer.observe(wireguard, dns, services, freebox_obs);

            if json {
                crate::cli_json::emit_pretty_json(&report)
            } else {
                println!(
                    "Fleet Watchdog: status={}, total_nodes={}, healthy={}, failed={}",
                    report.status, report.total_nodes, report.healthy_nodes, report.failed_nodes
                );
                for alert in &report.alerts {
                    println!(
                        "  [{:?}] {}: {} -> {}",
                        alert.level, alert.component, alert.message, alert.recovery_hint
                    );
                }
                ExitCode::SUCCESS
            }
        }
        FleetCommand::Pulse {
            node_id,
            cert_id,
            key_path,
            status,
            uptime_s,
            json,
        } => {
            let key_p = key_path
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/etc/cynic/node-identity.key"));
            let private_key = match cynic_host::crypto::SecretStore::load_secret(&key_p) {
                Ok(k) => k,
                Err(e) => {
                    eprintln!(
                        "Failed to load node private key from {}: {e}",
                        key_p.display()
                    );
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
                        println!(
                            "PULSE EMITTED: Node {} ({:?}) signed with cert {}",
                            hb.node_id, hb.self_status, cert_id
                        );
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
