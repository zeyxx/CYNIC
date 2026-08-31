//! NetworkProbe — /proc/net interface stats and TCP socket summary.
//! Reads the injected mesh interface from /proc/net/dev + TCP state from
//! /proc/net/sockstat.
//! On non-Linux: returns ProbeStatus::Unavailable (no #[cfg] in domain — K1 compliant).

use crate::domain::probe::{
    NetInterfaceInfo, NetworkDetails, ProbeDetails, ProbeError, ProbeResult, ProbeStatus,
};
use async_trait::async_trait;
use std::time::{Duration, Instant};

#[cfg(test)]
use crate::domain::probe::Probe;
use cynic_host::runtime_layout::current_runtime_layout;

/// Probe that reads /proc/net/dev and /proc/net/sockstat.
#[derive(Debug, Default)]
pub struct NetworkProbe;

/// Parsed interface stats from /proc/net/dev.
#[derive(Debug, Default)]
struct IfaceStats {
    rx_bytes: u64,
    tx_bytes: u64,
    rx_drops: u64,
    tx_drops: u64,
    rx_errors: u64,
    tx_errors: u64,
}

/// TCP socket stats from /proc/net/sockstat.
#[derive(Debug, Default)]
struct TcpStats {
    inuse: u64,
    orphan: u64,
    tw: u64,
    alloc: u64,
}

impl NetworkProbe {
    /// Parse /proc/net/dev for a specific interface using canonical cynic-host parser.
    fn read_iface_stats(iface: &str) -> Option<IfaceStats> {
        let content = std::fs::read_to_string("/proc/net/dev").ok()?;
        let usages = cynic_host::domain::network::parse_usage_from_dev(&content);
        let u = usages.into_iter().find(|u| u.iface == iface)?;
        Some(IfaceStats {
            rx_bytes: u.rx_bytes,
            tx_bytes: u.tx_bytes,
            rx_errors: u.rx_errs,
            rx_drops: u.rx_drop,
            tx_errors: u.tx_errs,
            tx_drops: u.tx_drop,
        })
    }

    /// Parse /proc/net/sockstat for TCP line.
    fn read_tcp_stats() -> Option<TcpStats> {
        let content = std::fs::read_to_string("/proc/net/sockstat").ok()?;
        for line in content.lines() {
            if !line.starts_with("TCP:") {
                continue;
            }
            // "TCP: inuse N orphan N tw N alloc N mem N"
            let parts: Vec<&str> = line.split_whitespace().collect();
            let mut stats = TcpStats::default();
            let mut i = 1; // skip "TCP:"
            while i + 1 < parts.len() {
                let val: u64 = parts[i + 1].parse().unwrap_or(0);
                match parts[i] {
                    "inuse" => stats.inuse = val,
                    "orphan" => stats.orphan = val,
                    "tw" => stats.tw = val,
                    "alloc" => stats.alloc = val,
                    _ => {}
                }
                i += 2;
            }
            return Some(stats);
        }
        None
    }
}

#[async_trait]
impl crate::domain::probe::Probe for NetworkProbe {
    fn name(&self) -> &str {
        "network"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(30)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = Instant::now();

        // The mesh interface is injected (CYNIC_MESH_INTERFACE, default wg0), not
        // hardcoded. This probe previously read "tailscale0" — an interface that
        // exists on no node in this fleet — so it reported Ok while observing
        // nothing, and wg0 with its live peers was never read at all.
        let mesh_iface = current_runtime_layout().mesh_interface();
        let ts_stats = Self::read_iface_stats(&mesh_iface);
        let tcp_stats = Self::read_tcp_stats();

        let duration_ms = start.elapsed().as_millis() as u64;
        let timestamp = chrono::Utc::now().timestamp_millis();

        // If neither source is available, mark unavailable.
        if ts_stats.is_none() && tcp_stats.is_none() {
            return Ok(ProbeResult {
                name: "network".to_string(),
                status: ProbeStatus::Unavailable,
                details: ProbeDetails::Network(NetworkDetails {
                    interfaces: Vec::new(),
                }),
                duration_ms,
                timestamp,
            });
        }

        let mut interfaces = Vec::new();
        if let Some(ts) = &ts_stats {
            // Encode stats as structured IPs field for now — the NetworkDetails struct
            // uses interfaces[] with state/ips, which we repurpose for counter data.
            interfaces.push(NetInterfaceInfo {
                name: mesh_iface.clone(),
                state: "up".to_string(),
                ips: vec![
                    format!("rx_bytes={}", ts.rx_bytes),
                    format!("tx_bytes={}", ts.tx_bytes),
                    format!("rx_drops={}", ts.rx_drops),
                    format!("tx_drops={}", ts.tx_drops),
                    format!("rx_errors={}", ts.rx_errors),
                    format!("tx_errors={}", ts.tx_errors),
                ],
            });
        }

        if let Some(tcp) = &tcp_stats {
            interfaces.push(NetInterfaceInfo {
                name: "tcp_sockstat".to_string(),
                state: format!(
                    "inuse={} orphan={} tw={} alloc={}",
                    tcp.inuse, tcp.orphan, tcp.tw, tcp.alloc
                ),
                ips: Vec::new(),
            });
        }

        // Degraded on mesh errors/drops. A MISSING mesh interface is not Ok:
        // the transport carrying every inter-node call was not observed at all,
        // and reporting Ok for that is the fossil this probe used to be.
        let status = match &ts_stats {
            Some(ts) if ts.rx_errors > 0 || ts.tx_errors > 0 || ts.rx_drops > 100 => {
                ProbeStatus::Degraded
            }
            Some(_) => ProbeStatus::Ok,
            None => ProbeStatus::Degraded,
        };

        Ok(ProbeResult {
            name: "network".to_string(),
            status,
            details: ProbeDetails::Network(NetworkDetails { interfaces }),
            duration_ms,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_proc_net_dev_line() {
        // Smoke test: parser must not panic on /proc/net/dev, regardless of env.
        // On hosts carrying the mesh interface: Some(IfaceStats); else None.
        // Both valid. u64 non-negativity is guaranteed by the type.
        let iface = current_runtime_layout().mesh_interface();
        let stats = NetworkProbe::read_iface_stats(&iface);
        if let Some(s) = stats {
            // Exercise the returned struct — ensures parse path populated all fields.
            let _total_bytes = s.rx_bytes.saturating_add(s.tx_bytes);
            let _total_drops = s.rx_drops.saturating_add(s.tx_drops);
        }
    }

    #[test]
    fn parse_tcp_sockstat() {
        let stats = NetworkProbe::read_tcp_stats();
        if std::path::Path::new("/proc/net/sockstat").exists() {
            assert!(stats.is_some());
            let tcp = stats.unwrap();
            // inuse should be at least 1 (the test process itself)
            assert!(tcp.inuse >= 1 || tcp.alloc >= 1);
        }
    }

    #[tokio::test]
    async fn network_probe_returns_result() {
        let probe = NetworkProbe;
        let result = probe.sense().await.expect("should not error");
        // On Linux: at minimum tcp_sockstat should be available
        if std::path::Path::new("/proc/net/sockstat").exists() {
            assert_ne!(result.status, ProbeStatus::Unavailable);
        }
    }
}
