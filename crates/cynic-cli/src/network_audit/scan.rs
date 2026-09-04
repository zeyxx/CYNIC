use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use super::types::{
    DnsAudit, DnsRecordAudit, NetworkAuditAlert, NetworkAuditAlertLevel, NetworkAuditReport,
    NodeAudit, ServiceAudit, WireguardAudit, WireguardPeerAudit,
};

/// Raw registry structure (minimal — only what we audit).
#[derive(Debug, Deserialize)]
struct Registry {
    network_policy: Option<NetworkPolicy>,
    nodes: Vec<NodeEntry>,
}

#[derive(Debug, Deserialize)]
struct NetworkPolicy {
    reservations: Option<Vec<Reservation>>,
}

#[derive(Debug, Deserialize)]
struct Reservation {
    name: String,
    synapse: Option<String>,
    dns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct NodeEntry {
    id: String,
    name: Option<String>,
    role: Option<String>,
    network: Option<NetworkEntry>,
    services: Option<Vec<ServiceEntry>>,
}

#[derive(Debug, Deserialize)]
struct NetworkEntry {
    synapse: Option<String>,
    management: Option<String>,
    dns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct ServiceEntry {
    name: String,
    endpoint_host: Option<String>,
    network_plane: Option<String>,
    port: Option<u16>,
    scheme: Option<String>,
}

/// Build the network audit from declared registry + runtime probes.
pub fn build_network_audit(registry_path: &str) -> NetworkAuditReport {
    let now_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);

    let registry = match read_registry(registry_path) {
        Ok(r) => r,
        Err(e) => {
            return NetworkAuditReport {
                schema: super::types::NETWORK_AUDIT_SCHEMA.to_string(),
                status: "blocked".to_string(),
                mutates: false,
                total_nodes: 0,
                healthy_nodes: 0,
                degraded_nodes: 0,
                failed_nodes: 0,
                nodes: vec![],
                wireguard: WireguardAudit {
                    interface: "wg0".to_string(),
                    declared_peers: 0,
                    observed_peers: 0,
                    ok_peers: 0,
                    stale_peers: 0,
                    dead_peers: 0,
                    peers: vec![],
                },
                dns: DnsAudit {
                    zone: "synapse.cynic".to_string(),
                    total_records: 0,
                    resolved_records: 0,
                    failed_records: 0,
                    records: vec![],
                },
                alerts: vec![NetworkAuditAlert {
                    level: NetworkAuditAlertLevel::Critical,
                    component: "registry_read".to_string(),
                    message: format!("Failed to read registry: {e}"),
                    recovery_hint: format!("Check registry path: {registry_path}"),
                }],
                residual_unknowns: vec![],
                observed_at_unix_ms: now_unix_ms,
            };
        }
    };

    // Build lookup: node_name/IP -> FQDN from reservations
    let mut node_fqdn_map: HashMap<String, String> = HashMap::new();
    let mut node_ip_map: HashMap<String, String> = HashMap::new();
    if let Some(policy) = &registry.network_policy {
        if let Some(reservations) = &policy.reservations {
            for r in reservations {
                let name = &r.name;
                if let (Some(ip), Some(dns)) = (&r.synapse, &r.dns) {
                    if let Some(fqdn) = dns.first() {
                        node_fqdn_map.insert(name.clone(), fqdn.clone());
                        node_fqdn_map.insert(ip.clone(), fqdn.clone());
                        node_ip_map.insert(fqdn.clone(), ip.clone());
                    }
                }
            }
        }
    }

    // Build node audits
    let mut nodes = Vec::new();
    let mut healthy_nodes = 0usize;
    let mut degraded_nodes = 0usize;
    let mut failed_nodes = 0usize;
    let mut alerts: Vec<NetworkAuditAlert> = Vec::new();

    for entry in &registry.nodes {
        let node_name = entry.name.clone().unwrap_or_else(|| entry.id.clone());
        let declared_ip = entry.network.as_ref().and_then(|n| n.synapse.clone());

        // Determine FQDN for this service
        let fqdn = node_fqdn_map.get(&node_name).cloned().or_else(|| {
            declared_ip
                .as_ref()
                .and_then(|ip| node_fqdn_map.get(ip))
                .cloned()
        });

        // Build service audits
        let mut service_audits = Vec::new();
        if let Some(services) = &entry.services {
            for svc in services {
                let port = svc.port.unwrap_or(0);
                if port == 0 {
                    continue; // skip services with no port (e.g. process-only)
                }

                // Determine declared endpoint
                let declared_endpoint = match (&svc.network_plane, &svc.endpoint_host, &fqdn) {
                    (Some(plane), _, Some(f)) if plane == "synapse" => format!("{f}:{port}"),
                    (_, Some(host), _) => format!("{host}:{port}"),
                    (_, _, Some(f)) => format!("{f}:{port}"),
                    _ => continue,
                };

                // Probe TCP
                let endpoint_parts: Vec<&str> = declared_endpoint.rsplitn(2, ':').collect();
                if endpoint_parts.len() == 2 {
                    let host = endpoint_parts[1];
                    let port_str = endpoint_parts[0];
                    if let Ok(port_num) = port_str.parse::<u16>() {
                        let (reachable, latency, error) = probe_tcp(host, port_num);
                        service_audits.push(ServiceAudit {
                            service: svc.name.clone(),
                            node: entry.id.clone(),
                            declared_endpoint: declared_endpoint.clone(),
                            observed_endpoint: declared_endpoint.clone(),
                            reachable,
                            latency_ms: latency,
                            error,
                        });
                        if !reachable {
                            alerts.push(NetworkAuditAlert {
                                level: NetworkAuditAlertLevel::Critical,
                                component: format!("service:{}", svc.name),
                                message: format!(
                                    "Service '{}' on {} is unreachable at {}",
                                    svc.name, entry.id, declared_endpoint
                                ),
                                recovery_hint: format!(
                                    "Check service unit and firewall on {}",
                                    entry.id
                                ),
                            });
                        }
                    }
                }
            }
        }

        // Determine node outcome
        let outcome = if service_audits.is_empty() {
            "no_services"
        } else if service_audits.iter().all(|s| s.reachable) {
            healthy_nodes += 1;
            "healthy"
        } else if service_audits.iter().any(|s| s.reachable) {
            degraded_nodes += 1;
            "degraded"
        } else {
            failed_nodes += 1;
            "failed"
        };

        nodes.push(NodeAudit {
            node_id: entry.id.clone(),
            node_name,
            role: entry.role.clone().unwrap_or_default(),
            declared_ip: declared_ip.clone(),
            observed_ip: declared_ip,
            outcome: outcome.to_string(),
            services: service_audits,
            error: None,
        });
    }

    // Audit DNS
    let dns_audit = audit_dns(&node_fqdn_map, &node_ip_map);

    // Audit WireGuard (read from local wg show if available)
    let wireguard_audit = audit_wireguard();

    // Add DNS alerts
    if dns_audit.failed_records > 0 {
        alerts.push(NetworkAuditAlert {
            level: NetworkAuditAlertLevel::Critical,
            component: "dns_synapse".to_string(),
            message: format!(
                "DNS resolution failed for {} record(s) in zone '{}'",
                dns_audit.failed_records, dns_audit.zone
            ),
            recovery_hint: "Check PVE dnsmasq (10.10.10.1) and /etc/resolv.conf on nodes"
                .to_string(),
        });
    }

    let status = if failed_nodes > 0 || dns_audit.failed_records > 0 {
        "critical"
    } else if degraded_nodes > 0 {
        "degraded"
    } else {
        "ok"
    };

    NetworkAuditReport {
        schema: super::types::NETWORK_AUDIT_SCHEMA.to_string(),
        status: status.to_string(),
        mutates: false,
        total_nodes: nodes.len(),
        healthy_nodes,
        degraded_nodes,
        failed_nodes,
        nodes,
        wireguard: wireguard_audit,
        dns: dns_audit,
        alerts,
        residual_unknowns: vec![],
        observed_at_unix_ms: now_unix_ms,
    }
}

fn read_registry(path: &str) -> Result<Registry, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

fn probe_tcp(host: &str, port: u16) -> (bool, Option<u64>, Option<String>) {
    let start = std::time::Instant::now();
    match std::net::TcpStream::connect((host, port)) {
        Ok(_) => {
            let elapsed = start.elapsed().as_millis() as u64;
            (true, Some(elapsed), None)
        }
        Err(e) => (false, None, Some(e.to_string())),
    }
}

fn audit_dns(
    node_fqdn_map: &HashMap<String, String>,
    node_ip_map: &HashMap<String, String>,
) -> DnsAudit {
    let mut records = Vec::new();
    let mut resolved = 0usize;
    let mut failed = 0usize;

    for (name, declared_ip) in node_ip_map {
        // Try to resolve the FQDN
        let lookup_output = Command::new("getent").args(["hosts", name]).output();

        match lookup_output {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let mut found = false;
                for line in stdout.lines() {
                    if line.starts_with(declared_ip) {
                        found = true;
                        break;
                    }
                }
                if found {
                    resolved += 1;
                    records.push(DnsRecordAudit {
                        name: name.clone(),
                        declared_address: declared_ip.clone(),
                        observed_address: Some(declared_ip.clone()),
                        status: "ok".to_string(),
                    });
                } else {
                    failed += 1;
                    records.push(DnsRecordAudit {
                        name: name.clone(),
                        declared_address: declared_ip.clone(),
                        observed_address: Some(stdout.lines().next().unwrap_or("").to_string()),
                        status: "mismatch".to_string(),
                    });
                }
            }
            _ => {
                failed += 1;
                records.push(DnsRecordAudit {
                    name: name.clone(),
                    declared_address: declared_ip.clone(),
                    observed_address: None,
                    status: "unresolved".to_string(),
                });
            }
        }
    }

    DnsAudit {
        zone: "synapse.cynic".to_string(),
        total_records: records.len(),
        resolved_records: resolved,
        failed_records: failed,
        records,
    }
}

fn audit_wireguard() -> WireguardAudit {
    let output = Command::new("wg").args(&["show", "wg0"]).output();

    match output {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            parse_wg_show(&stdout)
        }
        _ => WireguardAudit {
            interface: "wg0".to_string(),
            declared_peers: 0,
            observed_peers: 0,
            ok_peers: 0,
            stale_peers: 0,
            dead_peers: 0,
            peers: vec![],
        },
    }
}

fn parse_wg_show(output: &str) -> WireguardAudit {
    let mut peers = Vec::new();
    let mut current_peer: Option<WireguardPeerAudit> = None;
    let mut ok = 0;
    let mut stale = 0;
    let mut dead = 0;

    for line in output.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("peer: ") {
            // Save previous peer
            if let Some(p) = current_peer.take() {
                match p.status.as_str() {
                    "ok" => ok += 1,
                    "stale" => stale += 1,
                    "dead" => dead += 1,
                    _ => {}
                }
                peers.push(p);
            }
            current_peer = Some(WireguardPeerAudit {
                public_key: rest.to_string(),
                declared_endpoint: String::new(),
                observed_endpoint: None,
                latest_handshake_s: None,
                status: "unknown".to_string(),
            });
        } else if let Some(ref mut p) = current_peer {
            if let Some(rest) = line.strip_prefix("endpoint: ") {
                p.declared_endpoint = rest.to_string();
                p.observed_endpoint = Some(rest.to_string());
            } else if let Some(rest) = line.strip_prefix("latest handshake: ") {
                // Parse "N seconds ago" or "N minutes, M seconds ago"
                let mut total_secs = 0u64;
                for part in rest.split(", ") {
                    if let Some(n) = part.split_whitespace().next() {
                        if let Ok(num) = n.parse::<u64>() {
                            if part.contains("minute") {
                                total_secs += num * 60;
                            } else if part.contains("second") {
                                total_secs += num;
                            }
                        }
                    }
                }
                p.latest_handshake_s = Some(total_secs);
                p.status = if total_secs < 150 {
                    "ok"
                } else if total_secs < 200 {
                    "stale"
                } else {
                    "dead"
                }
                .to_string();
            }
        }
    }

    // Don't forget the last peer
    if let Some(p) = current_peer {
        match p.status.as_str() {
            "ok" => ok += 1,
            "stale" => stale += 1,
            "dead" => dead += 1,
            _ => {}
        }
        peers.push(p);
    }

    WireguardAudit {
        interface: "wg0".to_string(),
        declared_peers: peers.len(),
        observed_peers: ok + stale + dead,
        ok_peers: ok,
        stale_peers: stale,
        dead_peers: dead,
        peers,
    }
}
