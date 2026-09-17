use std::process::ExitCode;

use cynic_host::adapters::http_freebox_probe_adapter::HttpFreeboxProbeAdapter;
use cynic_host::adapters::local_network::LocalNetworkAdapter;
use cynic_host::domain::network::freebox::FreeboxProbePort;
use cynic_node::NetworkCommands;
use cynic_node::usecases::network_watchdog_uc::{ServiceProbeSpec, execute_network_watchdog};
use cynic_node::usecases::notification_outbox_uc::InMemoryNotificationOutboxStore;
use cynic_node::watchdog::peer_handshake::LocalPeerHandshakeObserver;

use crate::cli_json::emit_pretty_json;
use crate::network_surface::NetworkCommand;
use crate::registry_path;
use crate::runtime_paths::RuntimePaths;
pub(crate) fn try_execute_base(
    command: NetworkCommand,
    runtime: &RuntimePaths,
) -> Result<ExitCode, NetworkCommand> {
    match command {
        NetworkCommand::Chain {
            registry,
            resolve_dns,
            check_provider_connectivity,
        } => Ok(cynic_node::execute_network_command(
            &NetworkCommands::ActivationChain {
                json: true,
                registry: registry_path(registry),
                resolve_dns,
                check_provider_connectivity,
                provider_hosts_path: runtime.provider_hosts_path.to_string_lossy().to_string(),
                hosts_path: runtime.dns_hosts_path.to_string_lossy().to_string(),
                corefile_path: runtime.dns_corefile_path.to_string_lossy().to_string(),
                service_path: runtime.dns_service_path.to_string_lossy().to_string(),
                resolved_dropin_path: runtime.resolved_dropin_path.to_string_lossy().to_string(),
            },
        )),
        NetworkCommand::Runtime {
            registry,
            resolve_dns,
            check_provider_connectivity,
        } => Ok(cynic_node::execute_network_command(
            &NetworkCommands::RuntimeStatus {
                json: true,
                registry: registry_path(registry),
                resolve_dns,
                check_provider_connectivity,
                provider_hosts_path: runtime.provider_hosts_path.to_string_lossy().to_string(),
                hosts_path: runtime.dns_hosts_path.to_string_lossy().to_string(),
                corefile_path: runtime.dns_corefile_path.to_string_lossy().to_string(),
                service_path: runtime.dns_service_path.to_string_lossy().to_string(),
                resolved_dropin_path: runtime.resolved_dropin_path.to_string_lossy().to_string(),
            },
        )),
        NetworkCommand::Proof {
            registry,
            resolve_dns,
            check_provider_connectivity,
        } => Ok(cynic_node::execute_network_command(
            &NetworkCommands::RuntimeProof {
                json: true,
                registry: registry_path(registry),
                resolve_dns,
                check_provider_connectivity,
                provider_hosts_path: runtime.provider_hosts_path.to_string_lossy().to_string(),
                hosts_path: runtime.dns_hosts_path.to_string_lossy().to_string(),
                corefile_path: runtime.dns_corefile_path.to_string_lossy().to_string(),
                service_path: runtime.dns_service_path.to_string_lossy().to_string(),
                resolved_dropin_path: runtime.resolved_dropin_path.to_string_lossy().to_string(),
            },
        )),
        NetworkCommand::Evidence { registry } => Ok(cynic_node::execute_network_command(
            &NetworkCommands::Evidence {
                json: true,
                registry: registry_path(registry),
            },
        )),
        NetworkCommand::Observe { json, freebox } => {
            let adapter = LocalNetworkAdapter;
            let report = adapter.observe_network();

            let freebox_obs = if let Some(endpoint) = freebox {
                let probe = HttpFreeboxProbeAdapter::default();
                probe.probe_freebox_lan(&endpoint).ok()
            } else {
                None
            };

            if json {
                let mut val = serde_json::to_value(&report).unwrap_or_default();
                if let Some(fb) = freebox_obs {
                    val["freebox_lan"] = serde_json::to_value(fb).unwrap_or_default();
                }
                emit_pretty_json(&val);
            } else {
                println!(
                    "Network Observation (4 Layers): Interfaces={}, Addresses={}, Routes={}",
                    report.interfaces.len(),
                    report.addresses.len(),
                    report.routes.len()
                );
                for iface in &report.interfaces {
                    println!(
                        "  L1 [{}] state={}, mtu={}, speed={:?}, mac={:?}",
                        iface.name, iface.state, iface.mtu, iface.speed_mbps, iface.mac
                    );
                }
                for addr in &report.addresses {
                    println!(
                        "  L2 [{}] {}/{} ({})",
                        addr.iface, addr.address, addr.prefix_len, addr.family
                    );
                }
                for u in &report.usage {
                    println!(
                        "  L3 [{}] Rx: {} bytes ({} drops), Tx: {} bytes ({} drops)",
                        u.iface, u.rx_bytes, u.rx_drop, u.tx_bytes, u.tx_drop
                    );
                }
                for p in &report.pressure {
                    println!(
                        "  L4 [{}] saturated={}, rx_drop={}, tx_drop={}",
                        p.iface, p.saturated, p.rx_drop, p.tx_drop
                    );
                }
                if let Some(fb) = freebox_obs {
                    println!(
                        "  Router [Freebox] reachable={}, model={:?}, version={:?}, domain={:?}",
                        fb.reachable, fb.box_model_name, fb.api_version, fb.api_domain
                    );
                }
            }
            Ok(ExitCode::SUCCESS)
        }
        NetworkCommand::Watchdog {
            json,
            interface,
            freebox,
        } => {
            let wg_observer = LocalPeerHandshakeObserver;
            let freebox_probe = HttpFreeboxProbeAdapter::default();
            let outbox_store = InMemoryNotificationOutboxStore::default();

            let services = vec![
                ServiceProbeSpec {
                    service: "forgejo".to_string(),
                    node: "forgejo".to_string(),
                    endpoint: "10.10.10.101:3000".to_string(),
                },
                ServiceProbeSpec {
                    service: "surrealdb".to_string(),
                    node: "cynic-lxc-surrealdb".to_string(),
                    endpoint: "10.10.10.103:8000".to_string(),
                },
                ServiceProbeSpec {
                    service: "kernel".to_string(),
                    node: "local-kernel".to_string(),
                    endpoint: "192.168.100.2:3030".to_string(),
                },
            ];

            let adapter = LocalNetworkAdapter;
            let pressure = adapter.observe_network().pressure;

            let now_unix_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            let report = match execute_network_watchdog(
                &interface,
                &wg_observer,
                &services,
                freebox.as_deref(),
                Some(&freebox_probe),
                pressure,
                &outbox_store,
                now_unix_ms,
            ) {
                Ok(rep) => rep,
                Err(error) => {
                    eprintln!("network watchdog execution failed: {error}");
                    return Ok(ExitCode::FAILURE);
                }
            };

            if json {
                emit_pretty_json(&report);
            } else {
                println!(
                    "Network Watchdog: status={}, alerts_enqueued={}, WireGuard peers ok={}/{}, dead={}",
                    report.status,
                    report.alerts_enqueued_count,
                    report.assessment.wireguard_mesh.ok_peers,
                    report.assessment.wireguard_mesh.total_peers,
                    report.assessment.wireguard_mesh.dead_peers
                );
                for alert in &report.assessment.alerts {
                    println!(
                        "  [{:?}] {}: {} -> {}",
                        alert.level, alert.component, alert.message, alert.recovery_hint
                    );
                }
            }

            if report.status == "critical" {
                Ok(ExitCode::FAILURE)
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        NetworkCommand::Sync {
            json,
            registry,
            node_id,
            interface,
            apply,
        } => {
            let report = match cynic_node::usecases::network_sync_uc::execute_network_sync(
                &registry, &node_id, &interface, apply,
            ) {
                Ok(r) => r,
                Err(error) => {
                    eprintln!("network sync failed: {error}");
                    return Ok(ExitCode::FAILURE);
                }
            };

            if json {
                emit_pretty_json(&report);
            } else {
                println!(
                    "Network Sync: status={}, apply={}, local_node={}, peers_synced={}, dns_records_synced={}",
                    report.status,
                    report.apply,
                    report.local_node_id,
                    report.peers_synced,
                    report.dns_records_synced
                );
                if !report.blockers.is_empty() {
                    println!("Blockers:");
                    for b in &report.blockers {
                        println!("  - {b}");
                    }
                }
            }

            if report.status == "degraded" {
                Ok(ExitCode::FAILURE)
            } else {
                Ok(ExitCode::SUCCESS)
            }
        }
        NetworkCommand::Audit { json, registry } => {
            let registry_path = registry.unwrap_or_else(|| "infra/registry.json".to_string());
            let report = crate::network_audit::build_network_audit(&registry_path);
            if json {
                crate::cli_json::emit_pretty_json(&report);
            } else {
                crate::network_audit::render_text(&report);
            }
            if report.status == "ok" {
                Ok(ExitCode::SUCCESS)
            } else {
                Ok(ExitCode::FAILURE)
            }
        }
        NetworkCommand::Peer { command } => Ok(cynic_node::execute_peer_command(&command)),
        NetworkCommand::Transport { command } => {
            Ok(crate::transport_surface::execute_transport(&command))
        }
        NetworkCommand::SecurityAudit {
            json: _,
            max_handshake_age_secs,
            registry,
        } => Ok(cynic_node::execute_network_command(
            &NetworkCommands::SecurityAudit {
                json: true,
                max_handshake_age_secs,
                registry: registry_path(registry),
            },
        )),
        NetworkCommand::ReconcileObserved {
            json: _,
            registry,
            timeout_ms,
            apply,
        } => Ok(cynic_node::execute_network_command(
            &NetworkCommands::ReconcileObserved {
                json: true,
                registry: registry_path(registry),
                timeout_ms,
                apply,
            },
        )),
        NetworkCommand::Firewall {
            json: _,
            registry,
            node_id,
            audit,
        } => Ok(cynic_node::execute_network_command(
            &NetworkCommands::FirewallPolicy {
                json: true,
                registry: registry_path(registry),
                node_id,
                audit,
            },
        )),
        other => Err(other),
    }
}
