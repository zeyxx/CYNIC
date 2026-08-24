//! Local network observation adapter (4-layer).
//!
//! Reads `/proc/net/dev` for L3 usage, `ip -j addr` for L2 addresses,
//! and `ip -j route` for L2 routes; L4 pressure is derived from the usage
//! counters (drops/errs). Physical interface metadata (L1) comes from
//! `ip -j link`. No substrate source is hardcoded in the domain layer.

use crate::domain::network::types::NetworkReport;
use crate::domain::network::{parse_addr_from_json, parse_ifname_from_dev, parse_pressure_from_dev, parse_route_from_json, parse_usage_from_dev};
use crate::domain::network::{NetworkObservationPort, PhysicalInterface};
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
                            let name = iface.get("ifname").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            if name.is_empty() {
                                continue;
                            }
                            report.interfaces.push(PhysicalInterface {
                                name,
                                mac: iface.get("address").and_then(|v| v.as_str()).map(String::from),
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
                                duplex: iface.get("duplex").and_then(|v| v.as_str()).map(String::from),
                                driver: None,
                            });
                        }
                    }
                } else {
                    report.unknowns.push("ip -j link returned non-JSON".to_string());
                }
            }
            Err(_) => {
                report.unknowns.push("ip: command not found / not executed".to_string());
            }
        }

        // L2/L3 — addresses, routes, usage from /proc + ip.
        let dev = std::fs::read_to_string("/proc/net/dev").ok();
        if let Some(dev) = dev {
            let usage = parse_usage_from_dev(&dev);
            report.usage = usage.clone();
            report.pressure = parse_pressure_from_dev(&report.usage);
            let known: HashMap<&str, ()> =
                report.interfaces.iter().map(|i| (i.name.as_str(), ())).collect();
            for name in parse_ifname_from_dev(&dev) {
                if !known.contains_key(name.as_str()) {
                    report.unknowns.push(format!(
                        "interface {name} observed in /proc/net/dev but absent from ip -j link"
                    ));
                }
            }
        } else {
            report.unknowns.push("/proc/net/dev not readable".to_string());
        }

        if let Ok(out) = capture_command("ip", &["-j", "addr"]) {
            report.addresses = parse_addr_from_json(&out.stdout);
        }
        if let Ok(out) = capture_command("ip", &["-j", "route"]) {
            report.routes = parse_route_from_json(&out.stdout);
        }

        report
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
