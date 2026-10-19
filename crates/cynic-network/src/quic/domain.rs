//! Pure domain types and predicates for CYNIC QUIC multiplexed communications (RFC 9000).

use serde::{Deserialize, Serialize};

/// Canonical ALPN identifier for CYNIC agent multiplexed transport.
pub const CYNIC_ALPN_V1: &[u8] = b"cynic-agent/1";

/// Maximum chunk size for artifact transmission over QUIC streams (64 KiB).
pub const ARTIFACT_CHUNK_SIZE_BYTES: usize = 65_536;

/// Role of the node in the QUIC transport session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuicRole {
    Server,
    Client,
}

/// Logical channel multiplexed over QUIC streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamChannel {
    /// Bidirectional, high-priority, zero-wait control channel (claims, tasks, leases).
    Control,
    /// Unidirectional high-frequency telemetry channel (load, pressure, heartbeats).
    Telemetry,
    /// Unidirectional, chunked binary artifact transfer channel (binaries, snapshots).
    Artifact,
}

/// Directionality of the QUIC stream according to RFC 9000.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StreamDirection {
    Bidirectional,
    Unidirectional,
}

impl StreamChannel {
    /// Returns the mandated RFC 9000 stream direction for this channel.
    #[must_use]
    pub const fn mandated_direction(&self) -> StreamDirection {
        match self {
            Self::Control => StreamDirection::Bidirectional,
            Self::Telemetry | Self::Artifact => StreamDirection::Unidirectional,
        }
    }

    /// Whether a packet drop on this channel may block others (always false in QUIC).
    #[must_use]
    pub const fn head_of_line_blocking(&self) -> bool {
        false
    }
}

/// Transport parameters negotiated during QUIC handshake.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuicTransportParameters {
    pub max_idle_timeout_ms: u64,
    pub initial_max_streams_bidi: u64,
    pub initial_max_streams_uni: u64,
    pub max_data_bytes: u64,
    pub enable_active_migration: bool,
}

impl Default for QuicTransportParameters {
    fn default() -> Self {
        Self {
            max_idle_timeout_ms: 30_000,
            initial_max_streams_bidi: 64,
            initial_max_streams_uni: 128,
            max_data_bytes: 10 * 1024 * 1024, // 10 MiB
            enable_active_migration: true,
        }
    }
}

/// Outcome of evaluating a peer's transport parameters.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TransportAdmissionVerdict {
    pub accepted: bool,
    pub reason: String,
}

/// Pure predicate evaluating transport parameters admission.
#[must_use]
pub fn decide_transport_admission(params: &QuicTransportParameters) -> TransportAdmissionVerdict {
    if params.max_idle_timeout_ms < 5_000 {
        return TransportAdmissionVerdict {
            accepted: false,
            reason: "idle timeout below minimum viable keepalive (5000ms)".to_string(),
        };
    }
    if params.initial_max_streams_bidi == 0 {
        return TransportAdmissionVerdict {
            accepted: false,
            reason: "peer must allow at least one bidirectional stream for control plane"
                .to_string(),
        };
    }
    TransportAdmissionVerdict {
        accepted: true,
        reason: "parameters comply with CYNIC agent mesh profile".to_string(),
    }
}

/// An observed connection migration event (RFC 9000 section 9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConnectionMigrationEvent {
    pub connection_id_hex: String,
    pub previous_remote_addr: String,
    pub new_remote_addr: String,
    pub validated: bool,
}
