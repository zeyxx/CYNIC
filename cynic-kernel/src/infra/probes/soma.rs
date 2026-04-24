//! SomaProbe — body awareness: listening ports, firewall state, wild binds.
//! Reads `ss -tlnp` / `ss -ulnp` (no root needed for user processes).
//! Reads `sudo nft list ruleset` (best-effort, requires sudoers entry).
//! R23: subprocess env is explicit — no inherited state.

use crate::domain::probe::{
    ListeningPort, ProbeDetails, ProbeError, ProbeResult, ProbeStatus, SomaDetails,
};
use async_trait::async_trait;
use std::time::{Duration, Instant};

/// Body awareness probe — scans ports and firewall every 60s.
#[derive(Debug, Default)]
pub struct SomaProbe;

impl SomaProbe {
    /// Parse `ss -tlnp` or `ss -ulnp` output into ListeningPort entries.
    fn parse_ss_output(output: &str, protocol: &str) -> Vec<ListeningPort> {
        let mut ports = Vec::new();
        for line in output.lines().skip(1) {
            // ss output columns: State Recv-Q Send-Q Local_Address:Port Peer_Address:Port Process
            let fields: Vec<&str> = line.split_whitespace().collect();
            if fields.len() < 5 {
                continue;
            }
            let local = fields[3];
            // Parse address:port — handle IPv6 brackets and %scope
            let (bind, port_str) = match local.rfind(':') {
                Some(i) => (&local[..i], &local[i + 1..]),
                None => continue,
            };
            let port: u16 = match port_str.parse() {
                Ok(p) => p,
                Err(_) => continue,
            };
            // Clean bind address: remove brackets, %scope
            let bind_clean = bind
                .trim_start_matches('[')
                .trim_end_matches(']')
                .split('%')
                .next()
                .unwrap_or(bind)
                .to_string();

            // Process name from the last field: users:(("name",pid=N,fd=N))
            let process = fields.get(5).and_then(|s| {
                s.split("((\"")
                    .nth(1)
                    .and_then(|p| p.split('"').next())
                    .map(|s| s.to_string())
            });

            ports.push(ListeningPort {
                port,
                protocol: protocol.to_string(),
                bind_address: bind_clean,
                process,
            });
        }
        ports
    }

    /// Ports that are legitimately wild-bound (0.0.0.0 / [::]) by design.
    /// WireGuard needs 0.0.0.0 for mesh. mDNS/DHCP are OS-level.
    const WILD_BIND_WHITELIST: &[u16] = &[
        41641, // WireGuard (Tailscale mesh)
        5353,  // mDNS (service discovery)
        67,    // DHCP (libvirt VM bridge)
    ];

    /// Detect wild binds — ports listening on 0.0.0.0 or [::], excluding whitelisted.
    fn find_wild_binds(ports: &[ListeningPort]) -> Vec<ListeningPort> {
        ports
            .iter()
            .filter(|p| {
                (p.bind_address == "0.0.0.0" || p.bind_address == "::" || p.bind_address == "*")
                    && !Self::WILD_BIND_WHITELIST.contains(&p.port)
            })
            .cloned()
            .collect()
    }

    /// Check firewall state via `sudo nft list ruleset`.
    /// Best-effort: returns (false, 0) if sudo not available or no rules.
    async fn check_firewall() -> (bool, u32) {
        // R23: explicit env, no inherited state
        let result = tokio::process::Command::new("sudo")
            .args(["nft", "list", "ruleset"])
            .env_clear()
            .env("PATH", "/usr/sbin:/usr/bin:/sbin:/bin")
            .env("HOME", std::env::var("HOME").unwrap_or_default())
            .output()
            .await;

        match result {
            Ok(output) if output.status.success() => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                // Count lines containing accept/drop/reject — nft comments
                // appear after the action, so ends_with doesn't work.
                let rule_count = stdout
                    .lines()
                    .filter(|l| {
                        let trimmed = l.trim();
                        trimmed.contains(" accept")
                            || trimmed.contains(" drop")
                            || trimmed.contains(" reject")
                    })
                    .count() as u32;
                (rule_count > 0, rule_count)
            }
            _ => (false, 0),
        }
    }
}

#[async_trait]
impl crate::domain::probe::Probe for SomaProbe {
    fn name(&self) -> &str {
        "soma"
    }

    fn interval(&self) -> Duration {
        // Must match other probes (30s) — ProbeScheduler replaces the
        // snapshot on each tick, so mismatched intervals drop probes from view.
        Duration::from_secs(30)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = Instant::now();

        // TCP ports — R23: explicit env
        let tcp_output = tokio::process::Command::new("ss")
            .args(["-tlnp"])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .output()
            .await
            .map_err(|e| ProbeError::Internal(format!("ss -tlnp: {e}")))?;

        let tcp_ports = Self::parse_ss_output(&String::from_utf8_lossy(&tcp_output.stdout), "tcp");

        // UDP ports
        let udp_output = tokio::process::Command::new("ss")
            .args(["-ulnp"])
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .output()
            .await
            .map_err(|e| ProbeError::Internal(format!("ss -ulnp: {e}")))?;

        let udp_ports = Self::parse_ss_output(&String::from_utf8_lossy(&udp_output.stdout), "udp");

        // Firewall state (best-effort, non-blocking with timeout)
        let (firewall_active, firewall_rule_count) =
            tokio::time::timeout(Duration::from_secs(5), Self::check_firewall())
                .await
                .unwrap_or_default();

        // Wild binds — processes listening on all interfaces
        let mut all_ports = tcp_ports.clone();
        all_ports.extend(udp_ports.clone());
        let wild_binds = Self::find_wild_binds(&all_ports);

        let duration_ms = start.elapsed().as_millis() as u64;
        let timestamp = chrono::Utc::now().to_rfc3339();

        // Status: degraded if wild binds exist or firewall absent
        let status = if !wild_binds.is_empty() || !firewall_active {
            ProbeStatus::Degraded
        } else {
            ProbeStatus::Ok
        };

        Ok(ProbeResult {
            name: "soma".to_string(),
            status,
            details: ProbeDetails::Soma(SomaDetails {
                tcp_ports,
                udp_ports,
                firewall_active,
                firewall_rule_count,
                wild_binds,
            }),
            duration_ms,
            timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ss_tcp_output() {
        let output = r#"State  Recv-Q Send-Q Local Address:Port  Peer Address:Port Process
LISTEN 0      4096   127.0.0.1:8000       0.0.0.0:*     users:(("surreal",pid=3199,fd=20))
LISTEN 0      512    10.0.0.1:8080         0.0.0.0:*     users:(("llama-server",pid=1211406,fd=10))
LISTEN 0      4096   0.0.0.0:8888         0.0.0.0:*     users:(("mitmdump",pid=548837,fd=11))
"#;
        let ports = SomaProbe::parse_ss_output(output, "tcp");
        assert_eq!(ports.len(), 3);
        assert_eq!(ports[0].port, 8000);
        assert_eq!(ports[0].bind_address, "127.0.0.1");
        assert_eq!(ports[0].process.as_deref(), Some("surreal"));
        assert_eq!(ports[2].port, 8888);
        assert_eq!(ports[2].bind_address, "0.0.0.0");
    }

    #[test]
    fn wild_binds_detected() {
        let ports = vec![
            ListeningPort {
                port: 8000,
                protocol: "tcp".into(),
                bind_address: "127.0.0.1".into(),
                process: Some("surreal".into()),
            },
            ListeningPort {
                port: 8888,
                protocol: "tcp".into(),
                bind_address: "0.0.0.0".into(),
                process: Some("mitmdump".into()),
            },
        ];
        let wild = SomaProbe::find_wild_binds(&ports);
        assert_eq!(wild.len(), 1);
        assert_eq!(wild[0].port, 8888);
    }

    #[test]
    fn no_wild_binds_when_scoped() {
        let ports = vec![ListeningPort {
            port: 3030,
            protocol: "tcp".into(),
            bind_address: "10.0.0.1".into(),
            process: Some("cynic-kernel".into()),
        }];
        assert!(SomaProbe::find_wild_binds(&ports).is_empty());
    }

    #[test]
    fn whitelisted_wild_binds_ignored() {
        let ports = vec![
            ListeningPort {
                port: 41641, // WireGuard — whitelisted
                protocol: "udp".into(),
                bind_address: "0.0.0.0".into(),
                process: None,
            },
            ListeningPort {
                port: 5353, // mDNS — whitelisted
                protocol: "udp".into(),
                bind_address: "::".into(),
                process: None,
            },
            ListeningPort {
                port: 9999, // unknown — NOT whitelisted
                protocol: "tcp".into(),
                bind_address: "0.0.0.0".into(),
                process: Some("rogue".into()),
            },
        ];
        let wild = SomaProbe::find_wild_binds(&ports);
        assert_eq!(wild.len(), 1);
        assert_eq!(wild[0].port, 9999);
    }
}
