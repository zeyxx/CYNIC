//! Local network observation adapter (4-layer).
//!
//! Reads `/proc/net/dev` for L3 usage, `ip -j addr` for L2 addresses,
//! and `ip -j route` for L2 routes; L4 pressure is derived from the usage
//! counters (drops/errs). Physical interface metadata (L1) comes from
//! `ip -j link`. No substrate source is hardcoded in the domain layer.

use crate::domain::network::types::NetworkReport;
use crate::domain::network::{NetworkObservationPort, PhysicalInterface};
use crate::domain::network::{
    parse_addr_from_json, parse_ifname_from_dev, parse_pressure_from_dev, parse_route_from_json,
    parse_usage_from_dev,
};
use crate::process_observation::capture_command;
use std::collections::HashMap;

pub struct LocalNetworkAdapter;

impl LocalNetworkAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn observe_network(&self) -> NetworkReport {
        let mut report = NetworkReport::default();

        // L1 — physical interface metadata from `ip -j link`.
        match capture_command("ip", &["-j", "link"]) {
            Ok(out) => {
                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&out.stdout) {
                    if let Some(arr) = v.as_array() {
                        for iface in arr {
                            let name = iface
                                .get("ifname")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            if name.is_empty() {
                                continue;
                            }
                            report.interfaces.push(PhysicalInterface {
                                name,
                                mac: iface
                                    .get("address")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                                mtu: iface.get("mtu").and_then(|v| v.as_u64()).unwrap_or(0) as u32,
                                state: iface
                                    .get("operstate")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string(),
                                speed_mbps: iface
                                    .get("speed")
                                    .and_then(|v| v.as_i64())
                                    .and_then(|s| if s < 0 { None } else { Some(s as u32) }),
                                duplex: iface
                                    .get("duplex")
                                    .and_then(|v| v.as_str())
                                    .map(String::from),
                                driver: None,
                            });
                        }
                    }
                } else {
                    report
                        .unknowns
                        .push("ip -j link returned non-JSON".to_string());
                }
            }
            Err(_) => {
                report
                    .unknowns
                    .push("ip: command not found / not executed".to_string());
            }
        }

        // L2/L3 — addresses, routes, usage from /proc + ip.
        let dev = std::fs::read_to_string("/proc/net/dev").ok();
        if let Some(dev) = dev {
            let usage = parse_usage_from_dev(&dev);
            report.usage = usage.clone();
            report.pressure = parse_pressure_from_dev(&report.usage);
            let known: HashMap<&str, ()> = report
                .interfaces
                .iter()
                .map(|i| (i.name.as_str(), ()))
                .collect();
            for name in parse_ifname_from_dev(&dev) {
                if !known.contains_key(name.as_str()) {
                    report.unknowns.push(format!(
                        "interface {name} observed in /proc/net/dev but absent from ip -j link"
                    ));
                }
            }
        } else {
            report
                .unknowns
                .push("/proc/net/dev not readable".to_string());
        }

        if let Ok(out) = capture_command("ip", &["-j", "addr"]) {
            report.addresses = parse_addr_from_json(&out.stdout);
        }
        if let Ok(out) = capture_command("ip", &["-j", "route"]) {
            report.routes = parse_route_from_json(&out.stdout);
        }

        report
    }

    /// Collect factual live network telemetry (listening ports, WireGuard peers, dropped packets).
    pub fn observe_live_telemetry(
        &self,
        node_id: &str,
        wg_iface: Option<&str>,
    ) -> cynic_network::NetworkTelemetrySnapshot {
        use cynic_network::{
            NETWORK_TELEMETRY_SNAPSHOT_SCHEMA, NetworkTelemetrySnapshot,
            parse_dropped_packets_from_dev, parse_listening_ports_from_proc_tcp,
            parse_wireguard_telemetry,
        };

        // 1. Listening ports from /proc/net/tcp and /proc/net/tcp6
        let mut listening_ports_set = std::collections::BTreeSet::new();
        if let Ok(tcp4) = std::fs::read_to_string("/proc/net/tcp") {
            for p in parse_listening_ports_from_proc_tcp(&tcp4) {
                listening_ports_set.insert(p);
            }
        }
        if let Ok(tcp6) = std::fs::read_to_string("/proc/net/tcp6") {
            for p in parse_listening_ports_from_proc_tcp(&tcp6) {
                listening_ports_set.insert(p);
            }
        }
        let listening_ports: Vec<u16> = listening_ports_set.into_iter().collect();

        // 2. WireGuard peers from `wg show <iface> dump`
        let iface = wg_iface.unwrap_or("wg0");
        let wireguard_peers = match capture_command("wg", &["show", iface, "dump"]) {
            Ok(out) if out.success => parse_wireguard_telemetry(&out.stdout),
            _ => std::collections::BTreeMap::new(),
        };

        // 3. Dropped packets from /proc/net/dev
        let dropped_packets_count = std::fs::read_to_string("/proc/net/dev")
            .map(|dev| parse_dropped_packets_from_dev(&dev))
            .unwrap_or(0);

        let observed_at_unix_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        NetworkTelemetrySnapshot {
            schema: NETWORK_TELEMETRY_SNAPSHOT_SCHEMA.to_string(),
            node_id: node_id.to_string(),
            listening_ports,
            wireguard_peers,
            dropped_packets_count,
            observed_at_unix_ms,
        }
    }

    /// Read live nftables ruleset on host without modifying state.
    pub fn observe_nftables_ruleset(&self) -> Result<String, String> {
        match capture_command("nft", &["list", "ruleset"]) {
            Ok(out) if out.success => Ok(out.stdout),
            Ok(out) => Err(out.stderr),
            Err(e) => Err(e.to_string()),
        }
    }
}

impl Default for LocalNetworkAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkObservationPort for LocalNetworkAdapter {
    fn observe_network(&self) -> NetworkReport {
        // The inherent method already assembles a complete report with
        // residual unknowns; the port is the same observation, infallible.
        Self::observe_network(self)
    }
}

/// Local live network discovery probe adapter.
/// Uses standard non-blocking/timed TCP socket connect and `wg show` command capture.
#[derive(Debug, Default, Clone, Copy)]
pub struct LocalNetworkDiscoveryProbeAdapter;

impl cynic_network::discovery::NetworkDiscoveryProbePort for LocalNetworkDiscoveryProbeAdapter {
    fn probe_tcp(&self, host: &str, port: u16, timeout_ms: u64) -> Result<u64, String> {
        let addr = format!("{}:{}", host, port);
        let socket_addr: std::net::SocketAddr = addr
            .parse()
            .or_else(|_| {
                use std::net::ToSocketAddrs;
                addr.to_socket_addrs()
                    .map_err(|e| e.to_string())?
                    .next()
                    .ok_or_else(|| "no socket address resolved".to_string())
            })
            .map_err(|e| format!("invalid address {}: {}", addr, e))?;

        let start = std::time::Instant::now();
        std::net::TcpStream::connect_timeout(
            &socket_addr,
            std::time::Duration::from_millis(timeout_ms),
        )
        .map_err(|e| e.to_string())?;
        Ok(start.elapsed().as_millis() as u64)
    }

    fn probe_wireguard_handshakes(
        &self,
        iface: &str,
    ) -> Result<std::collections::BTreeMap<String, u64>, String> {
        match capture_command("wg", &["show", iface, "dump"]) {
            Ok(out) if out.success => {
                let tele = cynic_network::parse_wireguard_telemetry(&out.stdout);
                let map = tele
                    .into_iter()
                    .filter_map(|(k, v)| v.last_handshake_unix_ms.map(|hs| (k, hs)))
                    .collect();
                Ok(map)
            }
            Ok(out) => Err(out.stderr),
            Err(e) => Err(e.to_string()),
        }
    }
}
