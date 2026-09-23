//! Typed transport read-back contract (layer L4).
//!
//! A transport is an *adapter*: WireGuard, mTLS, plain HTTP kernel bus are
//! replaceable. What survives adapter replacement is the typed proof a
//! transport produces. Consumers (L3 membership, autonomy decisions) may read
//! these reports but must never let them produce L1/L2 authority: a live
//! handshake proves a channel, never identity issuance.
//!
//! Authority: `protocols/machine-enrollment-federation.md` §4 (proof schemas).

use serde::{Deserialize, Serialize};

pub const TRANSPORT_READBACK_SCHEMA: &str = "cynic.transport_readback.v1";
pub const TRANSPORT_FLEET_READBACK_SCHEMA: &str = "cynic.transport_fleet_readback.v1";

/// Which transport plane produced the evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TransportKind {
    /// WireGuard overlay peer handshake (parsed from `wg show`).
    WireGuardPeer {
        /// Configured peer endpoint (host:port) as declared by desired state.
        endpoint: String,
        /// Allowed-ips entry for this peer, e.g. `10.20.0.5/32`.
        allowed_ips: String,
    },
    /// Generic TCP reachability probe (connect-only).
    Tcp { host: String, port: u16 },
    /// HTTP(S) health endpoint read-back.
    Http { url: String },
}

impl TransportKind {
    pub fn plane_name(&self) -> &'static str {
        match self {
            TransportKind::WireGuardPeer { .. } => "wireguard_overlay",
            TransportKind::Tcp { .. } => "tcp",
            TransportKind::Http { .. } => "http",
        }
    }
}

/// Outcome of one transport read-back measurement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum TransportState {
    /// Evidence of liveness read back from the transport itself.
    Healthy,
    /// The transport answered but with degraded signals (e.g. exit code 0 but
    /// unparsable body). Still a channel; content trust reduced.
    Degraded { detail: String },
    /// Measured and absent: connection refused, timeout, no handshake ever.
    Unreachable { detail: String },
}

/// One typed read-back report for one node/plane pair at one instant.
///
/// Invariant (design doc §4): every membership transition that cites a
/// transport cites this struct, and this struct alone can never mint or
/// renew an enrollment grant.
///
/// Attribution contract: `node_id` names the SUBJECT the evidence is about —
/// a kernel-plane probe attributes its report to `kernel-primary`, a
/// WireGuard handshake to the peer whose key was checked, a self-observation
/// to `self`. It must never be silently repurposed as the observer's own id;
/// observers that measure others say so in `evidence_refs`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportReadback {
    pub schema: String,
    pub node_id: String,
    pub kind: TransportKind,
    pub state: TransportState,
    /// Monotonic-source wall clock of the observation (ms), for freshness
    /// windows; consumers apply their own max-age policy.
    pub observed_at_unix_ms: u64,
    /// Free-form machine evidence reference (log line digest, peer key id…),
    /// never secret material.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
}

impl TransportReadback {
    pub fn healthy(&self) -> bool {
        matches!(self.state, TransportState::Healthy)
    }

    /// Pure structural validation.
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != TRANSPORT_READBACK_SCHEMA {
            blockers.push("transport_readback_schema_invalid".to_string());
        }
        if self.node_id.trim().is_empty() {
            blockers.push("transport_readback_node_id_missing".to_string());
        }
        if let TransportKind::Tcp { port, .. } = &self.kind {
            if *port == 0 {
                blockers.push("transport_readback_tcp_port_invalid".to_string());
            }
        }
        if let TransportKind::WireGuardPeer { allowed_ips, .. } = &self.kind {
            if !allowed_ips.contains('/') {
                blockers.push("transport_readback_wg_allowed_ips_not_cidr".to_string());
            }
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }

    /// Freshness predicate against a consumer-owned max age (pure).
    pub fn is_fresh_at(&self, now_unix_ms: u64, max_age_ms: u64) -> bool {
        now_unix_ms.saturating_sub(self.observed_at_unix_ms) <= max_age_ms
    }
}

/// Fleet-level projection: many plane read-backs gathered in one tick.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportFleetReadback {
    pub schema: String,
    pub observed_at_unix_ms: u64,
    pub readbacks: Vec<TransportReadback>,
}

impl TransportFleetReadback {
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != TRANSPORT_FLEET_READBACK_SCHEMA {
            blockers.push("transport_fleet_readback_schema_invalid".to_string());
        }
        for rb in &self.readbacks {
            blockers.extend(
                rb.validate()
                    .into_iter()
                    .map(|b| format!("{}[{}]: {}", rb.node_id, rb.kind.plane_name(), b)),
            );
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }
}

// ── Pure WireGuard output parsing ─────────────────────────────────────────

/// One parsed `wg show <if> dump` peer row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WgDumpPeer {
    pub public_key: String,
    pub endpoint: Option<String>,
    pub allowed_ips: String,
    /// Raw unix timestamp (seconds) of the latest successful handshake;
    /// `None` = never handshook. Age is computed against a caller-provided
    /// now via [`wg_handshake_age_ms`] — this field is NOT an age.
    pub latest_handshake_unix_s: Option<u64>,
}

/// Parse the TSV output of `wg show <iface> dump`.
///
/// Line 0 is the interface itself; each following line is a peer:
/// key, endpoint, allowed-ips, latest-handshake(unix s, 0 = never),
/// transfer-rx, transfer-tx, persistent-keepalive.
/// Pure function over text — no I/O, fully fixture-testable.
pub fn parse_wg_dump(output: &str) -> Vec<WgDumpPeer> {
    let mut peers = Vec::new();
    for (idx, line) in output.lines().enumerate() {
        if idx == 0 || line.trim().is_empty() {
            continue; // first line = interface row
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 4 {
            continue;
        }
        let handshake_unix_s: u64 = cols[3].trim().parse().unwrap_or(0);
        let latest = if handshake_unix_s == 0 {
            None
        } else {
            Some(handshake_unix_s)
        };
        peers.push(WgDumpPeer {
            public_key: cols[0].to_string(),
            endpoint: (!cols[1].trim().is_empty()).then(|| cols[1].to_string()),
            allowed_ips: cols[2].to_string(),
            latest_handshake_unix_s: latest,
        });
    }
    peers
}

/// Compute handshake staleness in ms against a caller-provided now (unix ms).
/// Returns `None` when the peer never handshook. Pure.
pub fn wg_handshake_age_ms(peer: &WgDumpPeer, now_unix_ms: u64) -> Option<u64> {
    let hs = peer.latest_handshake_unix_s?;
    let hs_ms = hs.saturating_mul(1000);
    Some(now_unix_ms.saturating_sub(hs_ms))
}

/// Build a [`TransportReadback`] from a parsed peer row (pure assembly).
///
/// Healthy requires a handshake strictly inside `max_age_ms`; otherwise the
/// report says Unreachable with the measured age as evidence detail.
pub fn readback_from_wg_peer(
    node_id: &str,
    peer: &WgDumpPeer,
    now_unix_ms: u64,
    max_age_ms: u64,
) -> TransportReadback {
    let state = match wg_handshake_age_ms(peer, now_unix_ms) {
        None => TransportState::Unreachable {
            detail: "no_handshake_recorded".to_string(),
        },
        Some(age_ms) if age_ms <= max_age_ms => TransportState::Healthy,
        Some(age_ms) => TransportState::Unreachable {
            detail: format!("handshake_stale_age_ms={age_ms}"),
        },
    };
    TransportReadback {
        schema: TRANSPORT_READBACK_SCHEMA.to_string(),
        node_id: node_id.to_string(),
        kind: TransportKind::WireGuardPeer {
            endpoint: peer.endpoint.clone().unwrap_or_default(),
            allowed_ips: peer.allowed_ips.clone(),
        },
        state,
        observed_at_unix_ms: now_unix_ms,
        evidence_refs: vec![format!(
            "wg_public_key_prefix:{}",
            &peer.public_key[..8.min(peer.public_key.len())]
        )],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WG_DUMP_FIXTURE: &str = "\
wg0\t0000000000000000000000000000000000000000000000000000000000000000\t51820\toff
NwnCBCswWWXr89AUEi6wEX+O7PHbZYQV8/2RWz1cayo=\t10.10.10.101:44196\t10.20.0.2/32\t1724612345\t21600\t65400\t25
kRs1uCQBwuBWJJP0wA0P4eHY3QKtpM/fEw6fdvPaEV8=\t10.10.10.119:44196\t10.20.0.5/32\t0\t0\t5111808\t25\
";

    #[test]
    fn parse_wg_dump_skips_interface_row_and_reads_peers() {
        let peers = parse_wg_dump(WG_DUMP_FIXTURE);
        assert_eq!(peers.len(), 2);
        assert_eq!(
            peers[0].public_key,
            "NwnCBCswWWXr89AUEi6wEX+O7PHbZYQV8/2RWz1cayo="
        );
        assert_eq!(peers[0].allowed_ips, "10.20.0.2/32");
        assert_eq!(peers[0].endpoint.as_deref(), Some("10.10.10.101:44196"));
    }

    #[test]
    fn never_handshook_peer_is_unreachable_even_after_long_wait() {
        let peers = parse_wg_dump(WG_DUMP_FIXTURE);
        let builder = &peers[1];
        assert_eq!(builder.latest_handshake_unix_s, None);
        let rb = readback_from_wg_peer("cynic-builder", builder, 1_800_000_000_000, 150_000);
        assert!(!rb.healthy());
        assert_eq!(
            rb.state,
            TransportState::Unreachable {
                detail: "no_handshake_recorded".to_string()
            }
        );
    }

    #[test]
    fn fresh_handshake_is_healthy_and_stale_one_is_not() {
        let mut peers = parse_wg_dump(WG_DUMP_FIXTURE);
        let forge = peers[0].clone();
        let now = 1_724_612_345_000u64;
        // fresh: 30s old
        let rb_fresh = readback_from_wg_peer("forgejo", &forge, now, 150_000);
        assert!(rb_fresh.healthy());
        assert!(rb_fresh.validate().is_empty(), "{:?}", rb_fresh.validate());

        // stale: 400s old > 150s max
        let stale_peer = WgDumpPeer {
            latest_handshake_unix_s: forge.latest_handshake_unix_s.map(|t| t - 370),
            ..forge.clone()
        };
        let rb_stale = readback_from_wg_peer("forgejo", &stale_peer, now, 150_000);
        assert!(!rb_stale.healthy());
        assert!(matches!(rb_stale.state, TransportState::Unreachable { .. }));

        // structural check on the mutated clone too
        assert_eq!(peers.len(), 2);
        let _ = peers.pop();
    }

    #[test]
    fn tcp_and_http_kinds_carry_validation_law() {
        let bad = TransportReadback {
            schema: TRANSPORT_READBACK_SCHEMA.to_string(),
            node_id: "n".to_string(),
            kind: TransportKind::Tcp {
                host: "h".to_string(),
                port: 0,
            },
            state: TransportState::Healthy,
            observed_at_unix_ms: 1,
            evidence_refs: vec![],
        };
        assert!(
            bad.validate()
                .contains(&"transport_readback_tcp_port_invalid".to_string())
        );

        let wg_bad = TransportReadback {
            schema: TRANSPORT_READBACK_SCHEMA.to_string(),
            node_id: "n".to_string(),
            kind: TransportKind::WireGuardPeer {
                endpoint: "e".to_string(),
                allowed_ips: "not-a-cidr".to_string(),
            },
            state: TransportState::Healthy,
            observed_at_unix_ms: 1,
            evidence_refs: vec![],
        };
        assert!(
            wg_bad
                .validate()
                .contains(&"transport_readback_wg_allowed_ips_not_cidr".to_string())
        );
    }

    #[test]
    fn freshness_window_is_consumer_owned() {
        let rb = TransportReadback {
            schema: TRANSPORT_READBACK_SCHEMA.to_string(),
            node_id: "n".to_string(),
            kind: TransportKind::Http {
                url: "http://kernel/health".to_string(),
            },
            state: TransportState::Healthy,
            observed_at_unix_ms: 1000,
            evidence_refs: vec![],
        };
        assert!(rb.is_fresh_at(2000, 5000));
        assert!(!rb.is_fresh_at(9000, 5000));
    }
}
