//! WireGuard read-back adapter (L4) — observes `wg show <iface> dump` through
//! the injected [`ArgvRunner`] port and projects typed
//! [`TransportReadback`](cynic_protocol::TransportReadback) reports.
//!
//! No raw `std::process::Command` here: tests inject fixture runners, prod
//! injects [`LocalArgvRunner`]. The adapter is disposable; the typed proof is
//! durable. Authority: `protocols/machine-enrollment-federation.md` step 2.

use cynic_protocol::{
    TransportFleetReadback, TransportKind, TransportReadback, TransportState, parse_wg_dump,
};

use crate::update_lifecycle::ArgvRunner;

/// Declared desired peer of one node on the overlay (registry projection).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesiredWgPeer {
    pub node_id: String,
    /// Public key as declared by registry desired state (`wireguard.nodes`).
    pub public_key: String,
}

/// Read-only observer port implementation over an injected argv runner.
pub struct WgReadbackObserver<'a> {
    runner: &'a dyn ArgvRunner,
    /// Interface name; default `wg0` keeps the zero-config path honest while
    /// staying injectable for hosts that name it differently.
    iface: String,
    /// Max handshake age considered live (ms). Registry watchdog policy may
    /// inject its own value; this stays L4-local and never feeds L3 policy.
    max_handshake_age_ms: u64,
}

impl<'a> WgReadbackObserver<'a> {
    pub fn new(
        runner: &'a dyn ArgvRunner,
        iface: impl Into<String>,
        max_handshake_age_ms: u64,
    ) -> Self {
        WgReadbackObserver {
            runner,
            iface: iface.into(),
            max_handshake_age_ms,
        }
    }

    /// Observe declared peers against the live dump. Pure assembly over one
    /// argv execution + pure parsers; no mutation, no retries, no guessing:
    /// a peer absent from the dump yields `Unreachable{no_handshake_recorded}`.
    pub fn observe_peers(
        &self,
        now_unix_ms: u64,
        desired: &[DesiredWgPeer],
    ) -> TransportFleetReadback {
        let output = self.runner.run(&["wg", "show", &self.iface, "dump"]);
        let mut readbacks = Vec::with_capacity(desired.len());
        let peers = if output.success() {
            parse_wg_dump(&output.stdout)
        } else {
            Vec::new()
        };
        // Transport-level failure (spawn error / non-zero exit) degrades every
        // declared peer to Unreachable with the reason attached — explicit
        // unknown instead of fabricated health.
        let tool_failure = (!output.success()).then(|| {
            output
                .spawn_error
                .clone()
                .unwrap_or_else(|| format!("exit_code={:?}", output.exit_code))
        });
        for want in desired {
            let rb = match tool_failure.as_ref() {
                Some(reason) => TransportReadback {
                    schema: "cynic.transport_readback.v1".to_string(),
                    node_id: want.node_id.clone(),
                    kind: TransportKind::WireGuardPeer {
                        endpoint: String::new(),
                        allowed_ips: String::new(),
                    },
                    state: TransportState::Unreachable {
                        detail: format!("wg_tool_unavailable: {reason}"),
                    },
                    observed_at_unix_ms: now_unix_ms,
                    evidence_refs: vec![],
                },
                None => match peers.iter().find(|p| p.public_key == want.public_key) {
                    Some(peer) => {
                        let mut rb = cynic_protocol::readback_from_wg_peer(
                            &want.node_id,
                            peer,
                            now_unix_ms,
                            self.max_handshake_age_ms,
                        );
                        // Keep declared endpoint as evidence even when dump
                        // omits it (roaming peers).
                        if let TransportKind::WireGuardPeer {
                            endpoint: observed, ..
                        } = &mut rb.kind
                        {
                            if observed.is_empty() {
                                *observed = peer.endpoint.clone().unwrap_or_default();
                            }
                        }
                        rb
                    }
                    None => TransportReadback {
                        schema: "cynic.transport_readback.v1".to_string(),
                        node_id: want.node_id.clone(),
                        kind: TransportKind::WireGuardPeer {
                            endpoint: String::new(),
                            allowed_ips: String::new(),
                        },
                        state: TransportState::Unreachable {
                            detail: "peer_absent_from_dump".to_string(),
                        },
                        observed_at_unix_ms: now_unix_ms,
                        evidence_refs: vec![],
                    },
                },
            };
            readbacks.push(rb);
        }
        TransportFleetReadback {
            schema: "cynic.transport_fleet_readback.v1".to_string(),
            observed_at_unix_ms: now_unix_ms,
            readbacks,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::update_lifecycle::ArgvOutput;

    const DUMP: &str = "\
wg0\t0000\t51820\toff
kRs1uCQBwuBWJJP0wA0P4eHY3QKtpM/fEw6fdvPaEV8=\t10.10.10.119:44196\t10.20.0.5/32\t1724612345\t21600\t65400\t25
NwnCBCswWWXr89AUEi6wEX+O7PHbZYQV8/2RWz1cayo=\t10.10.10.101:44196\t10.20.0.2/32\t0\t0\t0\t25";

    struct FixtureRunner(ArgvOutput);

    impl ArgvRunner for FixtureRunner {
        fn run(&self, _argv: &[&str]) -> ArgvOutput {
            self.0.clone()
        }
    }

    fn desired() -> Vec<DesiredWgPeer> {
        vec![
            DesiredWgPeer {
                node_id: "cynic-builder".to_string(),
                public_key: "kRs1uCQBwuBWJJP0wA0P4eHY3QKtpM/fEw6fdvPaEV8=".to_string(),
            },
            DesiredWgPeer {
                node_id: "forgejo".to_string(),
                public_key: "NwnCBCswWWXr89AUEi6wEX+O7PHbZYQV8/2RWz1cayo=".to_string(),
            },
        ]
    }

    #[test]
    fn fresh_peer_healthy_never_handshook_unreachable() {
        let now = 1_724_612_345_000u64;
        let runner = FixtureRunner(ArgvOutput {
            exit_code: Some(0),
            stdout: DUMP.to_string(),
            stderr: String::new(),
            spawn_error: None,
        });
        let obs = WgReadbackObserver::new(&runner, "wg0", 150_000);
        let fleet = obs.observe_peers(now, &desired());
        assert_eq!(fleet.readbacks.len(), 2);
        assert!(fleet.readbacks[0].healthy(), "{:?}", fleet.readbacks[0]);
        assert!(!fleet.readbacks[1].healthy());
        assert_eq!(
            fleet.readbacks[1].state,
            TransportState::Unreachable {
                detail: "no_handshake_recorded".to_string()
            }
        );
        assert!(fleet.validate().is_empty(), "{:?}", fleet.validate());
    }

    #[test]
    fn wg_tool_failure_degrades_every_declared_peer_explicitly() {
        let runner = FixtureRunner(ArgvOutput {
            exit_code: None,
            stdout: String::new(),
            stderr: String::new(),
            spawn_error: Some("No such file or directory (os error 2)".to_string()),
        });
        let obs = WgReadbackObserver::new(&runner, "wg0", 150_000);
        let fleet = obs.observe_peers(1000, &desired());
        for rb in &fleet.readbacks {
            assert!(matches!(rb.state, TransportState::Unreachable { .. }));
            assert!(matches!(&rb.state, TransportState::Unreachable { detail }
                    if detail.starts_with("wg_tool_unavailable")),);
        }
    }

    #[test]
    fn stale_handshake_beyond_max_age_is_not_healthy() {
        // same dump but now far in the future: builder's 30s-old handshake ages past window
        let now = 1_724_612_345_000u64 + 400_000;
        let runner = FixtureRunner(ArgvOutput {
            exit_code: Some(0),
            stdout: DUMP.to_string(),
            stderr: String::new(),
            spawn_error: None,
        });
        let obs = WgReadbackObserver::new(&runner, "wg0", 150_000);
        let fleet = obs.observe_peers(now, &desired());
        assert!(!fleet.readbacks[0].healthy());
        assert!(matches!(
            &fleet.readbacks[0].state,
            TransportState::Unreachable { detail }
                if detail.starts_with("handshake_stale_age_ms")
        ));
    }

    #[test]
    fn peer_missing_from_dump_reports_explicit_absence() {
        let dump_one_peer = DUMP.split('\n').take(2).collect::<Vec<_>>().join("\n");
        let runner = FixtureRunner(ArgvOutput {
            exit_code: Some(0),
            stdout: dump_one_peer,
            stderr: String::new(),
            spawn_error: None,
        });
        let obs = WgReadbackObserver::new(&runner, "wg0", 150_000);
        let fleet = obs.observe_peers(1_724_612_345_000, &desired());
        assert_eq!(
            fleet.readbacks[1].state,
            TransportState::Unreachable {
                detail: "peer_absent_from_dump".to_string()
            }
        );
    }
}
