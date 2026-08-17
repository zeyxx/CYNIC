//! Typed WireGuard peer handshake observation for the L0 watchdog.
//! Reads `wg show <iface> latest-handshakes` via the canonical capture_command port.
//! Does NOT mutate state — mutation belongs to node_network_recovery_uc.

use std::time::{SystemTime, UNIX_EPOCH};

use cynic_host::process_observation::capture_command;
use cynic_host::runtime_layout::current_runtime_layout;
use serde::Serialize;

pub const PEER_HANDSHAKE_SCHEMA: &str = "cynic.wg_peer_handshake_observation.v1";

// ---------------------------------------------------------------------------
// Output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
pub struct WireguardHandshakeObservation {
    pub schema: &'static str,
    pub interface: String,
    pub observed_at_unix_s: u64,
    pub peers: Vec<PeerHandshakeStatus>,
    /// Number of peers whose handshake age exceeds `stale_threshold_s`.
    pub stale_count: usize,
    /// Number of peers whose handshake age exceeds `dead_threshold_s`.
    pub dead_count: usize,
    pub ok_count: usize,
    /// Peers that have never completed a handshake (`last_handshake_unix_s == 0`).
    pub unknown_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct PeerHandshakeStatus {
    /// First 16 chars of the public key — enough to identify in logs without
    /// leaking the full key into observation JSON.
    pub public_key_prefix: String,
    pub last_handshake_unix_s: u64,
    pub age_s: u64,
    pub state: HandshakeState,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HandshakeState {
    Ok,
    Stale,
    Dead,
    NeverSeen,
}

// ---------------------------------------------------------------------------
// Port trait — enables unit tests without a real `wg` binary
// ---------------------------------------------------------------------------

pub trait PeerHandshakePort {
    fn observe(
        &self,
        interface: &str,
        stale_s: u64,
        dead_s: u64,
    ) -> WireguardHandshakeObservation;
}

/// Production implementation: shells out to the `wg` binary resolved from the
/// current runtime layout.
#[derive(Debug, Clone, Copy, Default)]
pub struct LocalPeerHandshakeObserver;

impl PeerHandshakePort for LocalPeerHandshakeObserver {
    fn observe(
        &self,
        interface: &str,
        stale_s: u64,
        dead_s: u64,
    ) -> WireguardHandshakeObservation {
        observe_peer_handshakes(interface, stale_s, dead_s)
    }
}

// ---------------------------------------------------------------------------
// Core logic
// ---------------------------------------------------------------------------

/// Observe the latest WireGuard handshake timestamps for all peers on
/// `interface`.  Any error (wg not installed, interface missing, etc.) produces
/// an observation with zero peers rather than propagating a hard failure —
/// the watchdog must keep running even on nodes that have no WireGuard.
pub fn observe_peer_handshakes(
    interface: &str,
    stale_threshold_s: u64,
    dead_threshold_s: u64,
) -> WireguardHandshakeObservation {
    let now_unix_s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let wg_bin = current_runtime_layout()
        .capabilities()
        .wireguard_tool
        .command;

    let raw = match capture_command(&wg_bin, &["show", interface, "latest-handshakes"]) {
        Ok(capture) if capture.success => capture.stdout,
        Ok(capture) => {
            tracing::debug!(
                interface,
                stderr = %capture.stderr,
                "wg show latest-handshakes exited non-zero; reporting zero peers"
            );
            String::new()
        }
        Err(err) => {
            tracing::debug!(
                interface,
                error = %err,
                "wg show latest-handshakes unavailable; reporting zero peers"
            );
            String::new()
        }
    };

    let peers = parse_handshake_output(&raw, now_unix_s, stale_threshold_s, dead_threshold_s);

    let stale_count = peers.iter().filter(|p| p.state == HandshakeState::Stale).count();
    let dead_count = peers.iter().filter(|p| p.state == HandshakeState::Dead).count();
    let ok_count = peers.iter().filter(|p| p.state == HandshakeState::Ok).count();
    let unknown_count = peers.iter().filter(|p| p.state == HandshakeState::NeverSeen).count();

    WireguardHandshakeObservation {
        schema: PEER_HANDSHAKE_SCHEMA,
        interface: interface.to_string(),
        observed_at_unix_s: now_unix_s,
        peers,
        stale_count,
        dead_count,
        ok_count,
        unknown_count,
    }
}

/// Parse the tab-separated output of `wg show <iface> latest-handshakes`.
///
/// Each line has the form:
/// ```text
/// <base64-pubkey>\t<unix-timestamp-seconds>
/// ```
/// A timestamp of `0` means the peer has never completed a handshake.
fn parse_handshake_output(
    raw: &str,
    now_unix_s: u64,
    stale_threshold_s: u64,
    dead_threshold_s: u64,
) -> Vec<PeerHandshakeStatus> {
    raw.lines()
        .filter_map(|line| {
            let mut parts = line.splitn(2, '\t');
            let pubkey = parts.next()?.trim();
            let ts_str = parts.next()?.trim();

            if pubkey.is_empty() {
                return None;
            }

            let last_handshake_unix_s: u64 = ts_str.parse().ok()?;

            let (age_s, state) = if last_handshake_unix_s == 0 {
                (0, HandshakeState::NeverSeen)
            } else {
                let age = now_unix_s.saturating_sub(last_handshake_unix_s);
                let state = if age >= dead_threshold_s {
                    HandshakeState::Dead
                } else if age >= stale_threshold_s {
                    HandshakeState::Stale
                } else {
                    HandshakeState::Ok
                };
                (age, state)
            };

            // Truncate to first 16 chars to avoid logging full public keys.
            let public_key_prefix = pubkey.chars().take(16).collect();

            Some(PeerHandshakeStatus {
                public_key_prefix,
                last_handshake_unix_s,
                age_s,
                state,
            })
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Unit tests — no real `wg` binary required
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_now() -> u64 {
        1_000_000u64 // fixed epoch for deterministic tests
    }

    fn parse(raw: &str, stale_s: u64, dead_s: u64) -> Vec<PeerHandshakeStatus> {
        parse_handshake_output(raw, mk_now(), stale_s, dead_s)
    }

    #[test]
    fn never_seen_when_ts_zero() {
        let out = "Kar1lDbVWLydccWH\t0\n";
        let peers = parse(out, 150, 200);
        assert_eq!(peers.len(), 1);
        assert_eq!(peers[0].state, HandshakeState::NeverSeen);
        assert_eq!(peers[0].age_s, 0);
    }

    #[test]
    fn ok_when_recently_handshook() {
        // 30 seconds ago
        let ts = mk_now() - 30;
        let raw = format!("Kar1lDbVWLydccWHsTNQp/NggDHzzpVX\t{ts}\n");
        let peers = parse(&raw, 150, 200);
        assert_eq!(peers[0].state, HandshakeState::Ok);
        assert_eq!(peers[0].age_s, 30);
    }

    #[test]
    fn stale_when_between_thresholds() {
        let ts = mk_now() - 160; // > 150, < 200
        let raw = format!("AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\t{ts}\n");
        let peers = parse(&raw, 150, 200);
        assert_eq!(peers[0].state, HandshakeState::Stale);
    }

    #[test]
    fn dead_when_above_dead_threshold() {
        let ts = mk_now() - 250; // > 200
        let raw = format!("BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=\t{ts}\n");
        let peers = parse(&raw, 150, 200);
        assert_eq!(peers[0].state, HandshakeState::Dead);
    }

    #[test]
    fn public_key_prefix_truncated_to_16() {
        let ts = mk_now() - 10;
        let raw = format!("Kar1lDbVWLydccWHsTNQp/NggDHzzpVX\t{ts}\n");
        let peers = parse(&raw, 150, 200);
        assert_eq!(peers[0].public_key_prefix, "Kar1lDbVWLydccWH");
    }

    #[test]
    fn empty_output_yields_zero_peers() {
        assert!(parse("", 150, 200).is_empty());
    }

    #[test]
    fn counts_match_state_distribution() {
        let now = mk_now();
        let raw = format!(
            "AAAAAAAAAAAAAAAA\t0\n\
             BBBBBBBBBBBBBBBB\t{ok}\n\
             CCCCCCCCCCCCCCCC\t{stale}\n\
             DDDDDDDDDDDDDDDD\t{dead}\n",
            ok = now - 10,
            stale = now - 160,
            dead = now - 250,
        );
        let peers = parse(&raw, 150, 200);
        let obs = WireguardHandshakeObservation {
            schema: PEER_HANDSHAKE_SCHEMA,
            interface: "wg0".into(),
            observed_at_unix_s: now,
            stale_count: peers.iter().filter(|p| p.state == HandshakeState::Stale).count(),
            dead_count: peers.iter().filter(|p| p.state == HandshakeState::Dead).count(),
            ok_count: peers.iter().filter(|p| p.state == HandshakeState::Ok).count(),
            unknown_count: peers.iter().filter(|p| p.state == HandshakeState::NeverSeen).count(),
            peers,
        };
        assert_eq!(obs.unknown_count, 1);
        assert_eq!(obs.ok_count, 1);
        assert_eq!(obs.stale_count, 1);
        assert_eq!(obs.dead_count, 1);
    }

    /// Confirm the port trait is object-safe and can be used in tests via a
    /// stub without a real wg binary.
    #[test]
    fn stub_port_usable() {
        struct StubPort;
        impl PeerHandshakePort for StubPort {
            fn observe(&self, interface: &str, _stale_s: u64, _dead_s: u64) -> WireguardHandshakeObservation {
                WireguardHandshakeObservation {
                    schema: PEER_HANDSHAKE_SCHEMA,
                    interface: interface.to_string(),
                    observed_at_unix_s: 999,
                    peers: vec![],
                    stale_count: 0,
                    dead_count: 0,
                    ok_count: 0,
                    unknown_count: 0,
                }
            }
        }

        let obs = StubPort.observe("wg0", 150, 200);
        assert_eq!(obs.interface, "wg0");
        assert_eq!(obs.dead_count, 0);
    }
}
