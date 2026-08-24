//! Remote network observation adapter (SSH).
//!
//! Drives the same pure parsers as `LocalNetworkAdapter` but over a remote
//! host via an injected `ArgvRunner`. Implements `NetworkObservationPort`.
//! No substrate path is hardcoded in the domain layer.

use crate::domain::network::types::NetworkReport;
use crate::domain::network::{
    NetworkObservationPort, PhysicalInterface, parse_addr_from_json, parse_pressure_from_dev,
    parse_route_from_json, parse_usage_from_dev,
};
use crate::update_lifecycle::ArgvRunner;
use serde_json::Value;

pub struct RemoteNetworkAdapter<R: ArgvRunner> {
    runner: R,
    ssh_target: String,
}

impl<R: ArgvRunner> RemoteNetworkAdapter<R> {
    /// Construct a remote adapter for `ssh_target` (e.g. `cynic@10.0.0.7`).
    /// Returns `Err` when the target is empty — the fleet observer treats that
    /// as a `skipped` node rather than fabricating a connection string.
    pub fn new(runner: R, ssh_target: &str) -> Result<Self, String> {
        if ssh_target.is_empty() {
            return Err("empty_ssh_target".to_string());
        }
        Ok(Self {
            runner,
            ssh_target: ssh_target.to_string(),
        })
    }

    /// Run `ip -j link` (or any single command) over SSH and return stdout.
    fn run_ssh_text(&self, remote_cmd: &str) -> Option<String> {
        let argv: Vec<String> = vec![
            "ssh".to_string(),
            self.ssh_target.clone(),
            remote_cmd.to_string(),
        ];
        let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let out = self.runner.run(&refs);
        if out.success() {
            Some(out.stdout)
        } else {
            None
        }
    }

    fn run_ssh_json(&self, args: &[&str]) -> Option<serde_json::Value> {
        let argv: Vec<String> = vec!["ssh".to_string(), self.ssh_target.clone()]
            .into_iter()
            .chain(args.iter().map(|s| s.to_string()))
            .collect();
        let refs: Vec<&str> = argv.iter().map(|s| s.as_str()).collect();
        let out = self.runner.run(&refs);
        if out.success() {
            serde_json::from_str::<serde_json::Value>(&out.stdout).ok()
        } else {
            None
        }
    }
}

impl<R: ArgvRunner + Clone> RemoteNetworkAdapter<R> {
    /// Inherent SSH observation: returns `Result` for transport-level
    /// hard failure; every partial failure lands in `report.unknowns`.
    pub fn observe_network(&self) -> Result<NetworkReport, String> {
        let mut report = NetworkReport::default();

        // L1 — physical interfaces from `ip -j link`. A transport-level
        // failure here means the host is unreachable: surface it as Err so
        // fleet observers can mark the node failed instead of empty-observed.
        let link = match self.run_ssh_json(&["ip", "-j", "link"]) {
            Some(v) => v,
            None => return Err("ssh_unreachable_or_ip_missing".to_string()),
        };
        if let Some(arr) = link.as_array() {
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
        } else {
            report
                .unknowns
                .push("remote ip -j link returned non-JSON".to_string());
        }

        // L2/L3 — usage from /proc/net/dev; addresses + routes from ip -j over SSH.
        if let Some(dev) = self.run_ssh_text("cat /proc/net/dev") {
            let usage = parse_usage_from_dev(&dev);
            report.usage = usage.clone();
            report.pressure = parse_pressure_from_dev(&report.usage);
        } else {
            report
                .unknowns
                .push("remote /proc/net/dev not readable".to_string());
        }

        if let Some(v) = self.run_ssh_json(&["ip", "-j", "addr"]) {
            let json_str = v.to_string();
            report.addresses = parse_addr_from_json(&json_str);
        }
        if let Some(v) = self.run_ssh_json(&["ip", "-j", "route"]) {
            let json_str = v.to_string();
            report.routes = parse_route_from_json(&json_str);
        }

        Ok(report)
    }
}

impl<R: ArgvRunner + Clone> NetworkObservationPort for RemoteNetworkAdapter<R> {
    fn observe_network(&self) -> NetworkReport {
        // Port view: infallible. Transport-level hard failure lands in
        // `report.unknowns` instead of fabricating an empty healthy report.
        match Self::observe_network(self) {
            Ok(report) => report,
            Err(error) => {
                let mut report = NetworkReport::default();
                report
                    .unknowns
                    .push(format!("remote observation failed: {error}"));
                report
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update_lifecycle::{ArgvOutput, MapArgvRunner};

    fn stub_ssh(stdout: &str) -> MapArgvRunner {
        MapArgvRunner {
            responses: vec![(
                vec!["ssh".to_string()],
                ArgvOutput {
                    stdout: stdout.to_string(),
                    exit_code: Some(0),
                    stderr: String::new(),
                    spawn_error: None,
                },
            )],
        }
    }

    #[test]
    fn remote_adapter_empty_target_is_err() {
        assert!(RemoteNetworkAdapter::new(stub_ssh(""), "").is_err());
    }

    #[test]
    fn remote_adapter_parses_addr_and_route() {
        let target = "cynic@10.0.0.7";
        let link_argv: Vec<String> = vec![
            "ssh".to_string(),
            target.to_string(),
            "ip".to_string(),
            "-j".to_string(),
            "link".to_string(),
        ];
        let dev_argv: Vec<String> = vec![
            "ssh".to_string(),
            target.to_string(),
            "cat".to_string(),
            "/proc/net/dev".to_string(),
        ];
        let addr_argv: Vec<String> = vec![
            "ssh".to_string(),
            target.to_string(),
            "ip".to_string(),
            "-j".to_string(),
            "addr".to_string(),
        ];
        let route_argv: Vec<String> = vec![
            "ssh".to_string(),
            target.to_string(),
            "ip".to_string(),
            "-j".to_string(),
            "route".to_string(),
        ];
        let ok = |out: &str| ArgvOutput {
            stdout: out.to_string(),
            exit_code: Some(0),
            stderr: String::new(),
            spawn_error: None,
        };
        let runner = MapArgvRunner {
            responses: vec![
                (
                    link_argv,
                    ok(
                        "[{\"ifname\":\"eth0\",\"address\":\"00:11:22:33:44:55\",\"operstate\":\"up\",\"mtu\":1500,\"speed\":1000,\"duplex\":\"full\"}]",
                    ),
                ),
                (dev_argv, ok("  eth0: 100 10 0 0 0 0 0 0 0 0 0 0 0 0 0 0\n")),
                (
                    addr_argv,
                    ok(
                        "[{\"ifname\":\"eth0\",\"addr_info\":[{\"family\":\"inet\",\"local\":\"10.0.0.7\",\"prefixlen\":24}]}]",
                    ),
                ),
                (
                    route_argv,
                    ok("[{\"dst\":\"default\",\"gateway\":\"10.0.0.1\",\"dev\":\"eth0\"}]"),
                ),
            ],
        };
        let adapter = RemoteNetworkAdapter::new(runner, target).unwrap();
        let report = adapter.observe_network();
        match report {
            Ok(r) => {
                assert_eq!(r.interfaces[0].name, "eth0");
                assert_eq!(r.interfaces[0].mac.as_deref(), Some("00:11:22:33:44:55"));
                assert!(!r.addresses.is_empty());
                assert_eq!(r.addresses[0].address, "10.0.0.7");
                assert_eq!(r.routes[0].dst, "default");
            }
            Err(e) => panic!("expected report, got {e}"),
        }
    }
}
