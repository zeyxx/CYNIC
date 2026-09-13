//! Pure network parsers (no I/O). Each takes a captured substrate string and
//! returns typed layers of `cynic_protocol::network`. Tests use deterministic
//! fixtures, not host reads — the layer owns the parser, not the substrate.

use crate::types::{InterfacePressure, InterfaceUsage, NetworkAddress, NetworkRoute};

/// Parse a `/proc/net/dev` snapshot into L3 usage counters.
///
/// `/proc/net/dev` emits 8 rx columns (bytes, packets, errs, drop, fifo,
/// frame, compressed, multicast) followed by 8 tx columns (bytes, packets,
/// errs, drop, fifo, colls, carrier, compressed) — 16 numeric fields total.
/// The tx offset is therefore a stable constant of **8** on standard kernels.
const TX_BASE: usize = 8;
const MIN_COLS: usize = TX_BASE + 8;

pub fn parse_usage_from_dev(text: &str) -> Vec<InterfaceUsage> {
    let mut out = Vec::new();
    for line in text
        .lines()
        .skip_while(|l| l.contains("Inter") || l.contains("face"))
    {
        let line = line.trim();
        if line.is_empty() || !line.contains(':') {
            continue;
        }
        let (name, rest) = line.split_once(':').unwrap_or((line, ""));
        let name = name.trim();
        if name.is_empty() || name == "lo" {
            continue;
        }
        let cols: Vec<u64> = rest
            .split_whitespace()
            .map(|c| c.parse::<u64>().unwrap_or(0))
            .collect();
        if cols.len() < MIN_COLS {
            continue;
        }
        out.push(InterfaceUsage {
            iface: name.to_string(),
            rx_bytes: cols.get(0).copied().unwrap_or(0),
            rx_packets: cols.get(1).copied().unwrap_or(0),
            rx_errs: cols.get(2).copied().unwrap_or(0),
            rx_drop: cols.get(3).copied().unwrap_or(0),
            tx_bytes: cols.get(TX_BASE).copied().unwrap_or(0),
            tx_packets: cols.get(TX_BASE + 1).copied().unwrap_or(0),
            tx_errs: cols.get(TX_BASE + 2).copied().unwrap_or(0),
            tx_drop: cols.get(TX_BASE + 3).copied().unwrap_or(0),
        });
    }
    out
}

/// Parse `ip -j addr` JSON into L2 address bindings.
pub fn parse_addr_from_json(json: &str) -> Vec<NetworkAddress> {
    let parsed: Option<serde_json::Value> = serde_json::from_str(json).ok();
    let mut out = Vec::new();
    let arr = match parsed.as_ref().and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return out,
    };
    for iface in arr {
        let name = iface
            .get("ifname")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if let Some(addrs) = iface.get("addr_info").and_then(|v| v.as_array()) {
            for a in addrs {
                let family = a
                    .get("family")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let address = a
                    .get("local")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let prefix = a.get("prefixlen").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                if address.is_empty() {
                    continue;
                }
                out.push(NetworkAddress {
                    iface: name.clone(),
                    family,
                    address,
                    prefix_len: prefix,
                });
            }
        }
    }
    out
}

/// Parse `ip -j route` JSON into L2 routing entries.
pub fn parse_route_from_json(json: &str) -> Vec<NetworkRoute> {
    let parsed: Option<serde_json::Value> = serde_json::from_str(json).ok();
    let mut out = Vec::new();
    let arr = match parsed.as_ref().and_then(|v| v.as_array()) {
        Some(a) => a,
        None => return out,
    };
    for r in arr {
        let dst = r
            .get("dst")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let gw = r
            .get("gateway")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let device = r
            .get("dev")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let protocol = r
            .get("protocol")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        let metric = r.get("metric").and_then(|v| v.as_u64()).map(|n| n as u32);
        let scope = r
            .get("scope")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());
        if device.is_empty() {
            continue;
        }
        out.push(NetworkRoute {
            dst,
            gateway: gw,
            device,
            protocol,
            metric,
            scope,
        });
    }
    out
}

/// Derive L4 pressure signalling from usage counters.
///
/// Drops/errors are the substrate-level backpressure signal (ring overflow,
/// socket queue saturation, CRC/frame errors). A non-zero drop/error count
/// sets `saturated = true`.
pub fn parse_pressure_from_dev(usage: &[InterfaceUsage]) -> Vec<InterfacePressure> {
    usage
        .iter()
        .map(|u| {
            let saturated = u.rx_drop > 0 || u.tx_drop > 0 || u.rx_errs > 0 || u.tx_errs > 0;
            InterfacePressure {
                iface: u.iface.clone(),
                rx_drop: u.rx_drop,
                tx_drop: u.tx_drop,
                rx_errs: u.rx_errs,
                tx_errs: u.tx_errs,
                queue_length: None,
                saturated,
            }
        })
        .collect()
}

/// Extract interface names observed from `/proc/net/dev`.
pub fn parse_ifname_from_dev(text: &str) -> Vec<String> {
    parse_usage_from_dev(text)
        .into_iter()
        .map(|u| u.iface)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // /proc/net/dev snapshot: header has 2 lines, lo is loopback (excluded),
    // eth0 has 8 rx + 8 tx columns = 16 fields (real /proc/net/dev layout).
    const DEV_SNAPSHOT: &str = "Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
  eth0:12345678 9001 2 1 0 0 0 0 8765432 11002 3 0 0 0 0 0
    lo:  1024    16 0 0 0    0 0 0 0 0      1024    16 0 0 0 0 0 0
";

    #[test]
    fn parse_usage_skips_lo_and_header() {
        let usage = parse_usage_from_dev(DEV_SNAPSHOT);
        assert_eq!(usage.len(), 1, "lo must be excluded");
        assert_eq!(usage[0].iface, "eth0");
    }

    #[test]
    fn parse_usage_counters_are_correct() {
        let usage = parse_usage_from_dev(DEV_SNAPSHOT);
        let eth = &usage[0];
        assert_eq!(eth.iface, "eth0");
        assert_eq!(eth.rx_bytes, 12345678);
        assert_eq!(eth.rx_packets, 9001);
        assert_eq!(eth.rx_errs, 2);
        assert_eq!(eth.rx_drop, 1);
        assert_eq!(eth.tx_bytes, 8765432);
        assert_eq!(eth.tx_packets, 11002);
        assert_eq!(eth.tx_errs, 3);
        assert_eq!(eth.tx_drop, 0);
    }

    #[test]
    fn parse_pressure_flags_drops_and_errs() {
        let usage = parse_usage_from_dev(DEV_SNAPSHOT);
        let p = parse_pressure_from_dev(&usage);
        assert_eq!(p.len(), 1);
        assert!(
            p[0].saturated,
            "rx_drop=1 and rx_errs=2 must flag saturation"
        );
        assert_eq!(p[0].rx_drop, 1);
        assert_eq!(p[0].rx_errs, 2);
        assert_eq!(p[0].tx_errs, 3);
        assert_eq!(p[0].queue_length, None);
    }

    #[test]
    fn parse_pressure_clean_when_no_drops() {
        let clean = InterfaceUsage {
            iface: "enp3s0".into(),
            rx_bytes: 1,
            rx_packets: 1,
            rx_errs: 0,
            rx_drop: 0,
            tx_bytes: 1,
            tx_packets: 1,
            tx_errs: 0,
            tx_drop: 0,
        };
        let p = parse_pressure_from_dev(&[clean]);
        assert!(!p[0].saturated);
    }

    #[test]
    fn parse_route_from_json_real_route() {
        let routes = parse_route_from_json(
            r#"[
            {"dst":"10.20.0.0/24","gateway":null,"dev":"wg0","protocol":"kernel","metric":512,"scope":"link"},
            {"dst":null,"gateway":"10.0.0.1","dev":"eth0","protocol":"dhcp","metric":1024,"scope":"global"}
        ]"#,
        );
        assert_eq!(routes.len(), 2);
        assert_eq!(&routes[0].dst, "10.20.0.0/24");
        assert_eq!(&routes[0].device, "wg0");
        assert_eq!(routes[1].dst, "default");
        assert_eq!(routes[1].gateway.as_deref(), Some("10.0.0.1"));
        assert_eq!(routes[1].metric, Some(1024));
    }

    #[test]
    fn parse_addr_from_json_ipv4_and_ipv6() {
        let addrs = parse_addr_from_json(
            r#"[
            {"ifname":"eth0","addr_info":[{"family":"inet","local":"10.0.0.5","prefixlen":24},
                                           {"family":"inet6","local":"fe80::1","prefixlen":64}]}
        ]"#,
        );
        assert_eq!(addrs.len(), 2);
        assert_eq!(&addrs[0].address, "10.0.0.5");
        assert_eq!(addrs[0].prefix_len, 24);
        assert_eq!(&addrs[1].address, "fe80::1");
        assert_eq!(addrs[1].prefix_len, 64);
    }

    #[test]
    fn parse_ifname_lists_observed() {
        let names = parse_ifname_from_dev(DEV_SNAPSHOT);
        assert_eq!(names, vec!["eth0".to_string()]);
    }

    #[test]
    fn empty_dev_yields_no_usage() {
        assert!(parse_usage_from_dev("").is_empty());
    }
}
