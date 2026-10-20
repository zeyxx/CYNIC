//! Network Synchronization Use Case (Gap K1 / IaC convergence).
//!
//! Reconciles local WireGuard peers and service DNS table from the declarative
//! SSOT (`infra/registry.json`).
//!
//! Follows the CYNIC Coherence Pattern:
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

pub const NETWORK_SYNC_REPORT_SCHEMA: &str = "cynic.network.sync_report.v1";

/// The generated service hosts file placement when no override is supplied.
///
/// Named so the literal appears once. Every other write to this directory
/// (`cynic-host` bundle writers) composes its path from this constant instead
/// of re-declaring the placement, so two components cannot disagree about where
/// the hosts file lands.
pub const DEFAULT_NETWORK_SYNC_DNS_DIR: &str = "/etc/cynic/dns";

/// The generated service hosts file placement, overridable for tests and labs.
///
/// A placement is a coordination claim: the writer and the reader must name the
/// same path. The default follows the observed DNS provider context
/// ([`crate::dns_contract::service_dns_hosts_path`]), so the sync writes where
/// the bundle report says the hosts file lives — `/etc/cynic/dns` under
/// dnsmasq, `/etc/coredns` under CoreDNS. `with_dns_dir` is the only way to
/// move it elsewhere; [`DEFAULT_NETWORK_SYNC_DNS_DIR`] remains the fallback
/// when the context yields no parent directory.
#[derive(Debug, Clone)]
pub struct NetworkSyncDnsTarget {
    pub dns_dir: String,
}

impl NetworkSyncDnsTarget {
    pub fn new() -> Self {
        Self {
            dns_dir: default_dns_dir_from_provider_context(),
        }
    }

    pub fn with_dns_dir(dns_dir: impl Into<String>) -> Self {
        Self {
            dns_dir: dns_dir.into(),
        }
    }

    pub fn hosts_path(&self) -> PathBuf {
        PathBuf::from(&self.dns_dir).join(cynic_network::dns::SERVICE_DNS_HOSTS_FILENAME)
    }
}

impl Default for NetworkSyncDnsTarget {
    fn default() -> Self {
        Self::new()
    }
}

/// Resolve the default hosts directory from the observed DNS provider
/// context, so the sync and the bundle report never disagree about where
/// the hosts file lives. Falls back to [`DEFAULT_NETWORK_SYNC_DNS_DIR`]
/// when the context path has no parent.
fn default_dns_dir_from_provider_context() -> String {
    let hosts = crate::dns_contract::service_dns_hosts_path();
    std::path::Path::new(&hosts)
        .parent()
        .map(|parent| parent.to_string_lossy().to_string())
        .filter(|dir| !dir.is_empty())
        .unwrap_or_else(|| DEFAULT_NETWORK_SYNC_DNS_DIR.to_string())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WireguardPeerSpec {
    pub node_id: String,
    pub overlay_ip: String,
    pub public_key: String,
    pub endpoint: String,
    pub keepalive_secs: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DnsRecordSpec {
    pub ip: String,
    pub hostnames: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkSyncPlan {
    pub local_node_id: String,
    pub overlay_ip: String,
    pub wireguard_interface: String,
    pub declared_peers: Vec<WireguardPeerSpec>,
    pub dns_records: Vec<DnsRecordSpec>,
    pub peers_to_add: Vec<String>,
    pub peers_to_remove: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NetworkSyncReport {
    pub schema: String,
    pub status: String,
    pub apply: bool,
    pub local_node_id: String,
    pub peers_synced: usize,
    pub dns_records_synced: usize,
    pub active_handshakes_count: usize,
    pub blockers: Vec<String>,
}

/// Pure parser extracting WireGuard and DNS specs from a registry JSON Value.
pub fn parse_registry_network_specs(
    registry: &serde_json::Value,
    local_node_id: &str,
) -> Result<(Vec<WireguardPeerSpec>, Vec<DnsRecordSpec>), String> {
    let nodes = registry
        .pointer("/network_policy/wireguard/nodes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "missing_network_policy_wireguard_nodes".to_string())?;

    let mut peers = Vec::new();
    for node in nodes {
        let node_id = node
            .get("node_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if node_id.is_empty() || node_id == local_node_id {
            continue; // skip self
        }
        let overlay_ip = node
            .get("overlay_ip")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let public_key = node
            .get("public_key")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let endpoint = node
            .get("endpoint")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");

        if !overlay_ip.is_empty() && !public_key.is_empty() {
            peers.push(WireguardPeerSpec {
                node_id: node_id.to_string(),
                overlay_ip: overlay_ip.to_string(),
                public_key: public_key.to_string(),
                endpoint: endpoint.to_string(),
                keepalive_secs: 25,
            });
        }
    }

    // Parse DNS records from service reservations
    let reservations = registry
        .pointer("/network_policy/reservations")
        .and_then(serde_json::Value::as_array);

    let mut dns_records = Vec::new();
    if let Some(res_arr) = reservations {
        for res in res_arr {
            let ip = cynic_network::plane_vocabulary::plane_address(res, "service").unwrap_or("");
            let name = res
                .get("name")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            let mut hostnames = vec![name.to_string(), format!("{name}.service.cynic")];
            if let Some(dns_arr) = res.get("dns").and_then(serde_json::Value::as_array) {
                for d in dns_arr {
                    if let Some(s) = d.as_str() {
                        if !hostnames.contains(&s.to_string()) {
                            hostnames.push(s.to_string());
                        }
                    }
                }
            }
            if !ip.is_empty() {
                dns_records.push(DnsRecordSpec {
                    ip: ip.to_string(),
                    hostnames,
                });
            }
        }
    }

    Ok((peers, dns_records))
}

/// Execute network sync on the current host.
///
/// The generated hosts file goes to the observed provider context's hosts
/// directory (see [`NetworkSyncDnsTarget`]). Callers that must not write
/// there (tests, a lab) pass [`NetworkSyncDnsTarget::with_dns_dir`] through
/// [`execute_network_sync_with_target`]; the signature above is unchanged so no
/// existing caller breaks.
pub fn execute_network_sync(
    registry_path: &str,
    local_node_id: &str,
    iface: &str,
    apply: bool,
) -> Result<NetworkSyncReport, String> {
    execute_network_sync_with_target(registry_path, local_node_id, iface, apply, None)
}

/// [`execute_network_sync`] with an explicit hosts-file target.
///
/// `target: None` is the deployed placement; `Some(target)` redirects the write.
pub fn execute_network_sync_with_target(
    registry_path: &str,
    local_node_id: &str,
    iface: &str,
    apply: bool,
    target: Option<&NetworkSyncDnsTarget>,
) -> Result<NetworkSyncReport, String> {
    let reg_content =
        std::fs::read_to_string(registry_path).map_err(|e| format!("cannot_read_registry:{e}"))?;
    let registry: serde_json::Value =
        serde_json::from_str(&reg_content).map_err(|e| format!("invalid_registry_json:{e}"))?;
    let dns_target = target.cloned().unwrap_or_default();

    let (peers, dns_records) = parse_registry_network_specs(&registry, local_node_id)?;
    let mut blockers = Vec::new();
    let mut peers_synced = 0;
    let dns_records_synced = dns_records.len();

    if apply {
        // 1. Sync WireGuard peers dynamically
        for peer in &peers {
            let mut cmd = Command::new("sudo");
            cmd.args([
                "-n",
                "wg",
                "set",
                iface,
                "peer",
                &peer.public_key,
                "allowed-ips",
                &format!("{}/32", peer.overlay_ip),
                "persistent-keepalive",
                &peer.keepalive_secs.to_string(),
            ]);
            if !peer.endpoint.is_empty() {
                cmd.args(["endpoint", &peer.endpoint]);
            }
            match cmd.output() {
                Ok(out) if out.status.success() => {
                    peers_synced += 1;
                }
                Ok(out) => {
                    let err = String::from_utf8_lossy(&out.stderr);
                    blockers.push(format!("wg_set_peer_failed:{}:{err}", peer.node_id));
                }
                Err(e) => {
                    blockers.push(format!("wg_cmd_failed:{}:{e}", peer.node_id));
                }
            }
        }

        // 2. Render service hosts file into the declared target directory.
        let mut hosts_content = String::from("# AUTO-GENERATED BY CYNIC NETWORK SYNC\n");
        for record in &dns_records {
            hosts_content.push_str(&format!("{}\t{}\n", record.ip, record.hostnames.join(" ")));
        }
        let hosts_path = dns_target.hosts_path();
        if let Err(e) = std::fs::create_dir_all(&dns_target.dns_dir) {
            blockers.push(format!("create_dns_dir_failed:{}:{e}", dns_target.dns_dir));
        } else if let Err(e) = std::fs::write(&hosts_path, &hosts_content) {
            blockers.push(format!("write_service_hosts_failed:{e}"));
        }
    } else {
        peers_synced = peers.len();
    }

    let status = if !blockers.is_empty() {
        "degraded".to_string()
    } else if apply {
        "converged".to_string()
    } else {
        "dry_run".to_string()
    };

    Ok(NetworkSyncReport {
        schema: NETWORK_SYNC_REPORT_SCHEMA.to_string(),
        status,
        apply,
        local_node_id: local_node_id.to_string(),
        peers_synced,
        dns_records_synced,
        active_handshakes_count: peers_synced,
        blockers,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parse_registry_network_specs_extracts_peers_and_skips_self() {
        let reg = json!({
            "network_policy": {
                "wireguard": {
                    "nodes": [
                        {
                            "node_id": "cortex",
                            "overlay_ip": "10.20.0.3",
                            "public_key": "key-cortex",
                            "endpoint": "192.168.100.2:44196"
                        },
                        {
                            "node_id": "agent-host",
                            "overlay_ip": "10.20.0.6",
                            "public_key": "key-agent-host",
                            "endpoint": "10.10.10.102:44196"
                        }
                    ]
                },
                "reservations": [
                    {
                        "name": "agent-host",
                        "service": "10.10.10.102",
                        "dns": ["agent-host.service.cynic"]
                    }
                ]
            }
        });

        let (peers, dns) = parse_registry_network_specs(&reg, "cortex").unwrap();
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].node_id, "agent-host");
        assert_eq!(peers[0].overlay_ip, "10.20.0.6");
        assert_eq!(dns.len(), 1);
        assert_eq!(dns[0].ip, "10.10.10.102");
    }

    fn temp_dir(tag: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "cynic-network-sync-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).expect("create temp dir");
        path
    }

    fn registry_with_reservation(path: &std::path::Path) {
        std::fs::write(
            path,
            serde_json::json!({
                "network_policy": {
                    "wireguard": { "nodes": [] },
                    "reservations": [
                        { "name": "kernel", "service": "10.10.10.100", "dns": [] }
                    ]
                }
            })
            .to_string(),
        )
        .expect("write registry");
    }

    /// The default target must follow the observed provider context. The
    /// override is only allowed to move the write; it is not allowed to
    /// change where an unmodified caller writes.
    #[test]
    fn default_dns_target_follows_provider_context() {
        let context_hosts = crate::dns_contract::service_dns_hosts_path();
        let context_dir = std::path::Path::new(&context_hosts)
            .parent()
            .expect("context hosts path has a parent")
            .to_string_lossy()
            .to_string();
        assert_eq!(NetworkSyncDnsTarget::new().hosts_path(), PathBuf::from(&context_dir).join(cynic_network::dns::SERVICE_DNS_HOSTS_FILENAME));
        assert_eq!(NetworkSyncDnsTarget::default().dns_dir, context_dir);
    }

    /// The override is the whole point: a caller that must not write
    /// `/etc/cynic` gets the generated hosts file under its own directory, and
    /// the deployed default is not touched. `apply` is true so the write path is
    /// exercised; the WireGuard peer loop is empty because the registry declares
    /// no peers, so the test needs no `wg` binary and no root.
    #[test]
    fn dns_dir_override_redirects_generated_hosts_file() {
        let work = temp_dir("override");
        let registry = work.join("registry.json");
        registry_with_reservation(&registry);
        let dns_dir = work.join("custom-dns");

        let target = NetworkSyncDnsTarget::with_dns_dir(dns_dir.to_string_lossy().to_string());
        let report = execute_network_sync_with_target(
            &registry.to_string_lossy(),
            "cortex",
            "wg0",
            true,
            Some(&target),
        )
        .expect("network sync");

        let written = dns_dir.join(cynic_network::dns::SERVICE_DNS_HOSTS_FILENAME);
        assert!(written.is_file(), "hosts file must land in the override dir");
        let content = std::fs::read_to_string(&written).expect("read hosts");
        assert!(content.starts_with("# AUTO-GENERATED BY CYNIC NETWORK SYNC"));
        assert!(content.contains("10.10.10.100\tkernel kernel.service.cynic"));
        assert_eq!(report.status, "converged");
        assert!(report.blockers.is_empty());
        assert_eq!(report.dns_records_synced, 1);

        std::fs::remove_dir_all(&work).expect("cleanup");
    }
}
