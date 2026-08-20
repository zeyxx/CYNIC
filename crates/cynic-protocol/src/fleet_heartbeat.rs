//! Fleet heartbeat & health aggregation — the living layer of the network.
//!
//! Existing code covers one-shot discovery (fleet_discovery.rs) and policy
//! shapes (network_actuation.rs). This module adds the continuous pulse:
//! each node emits heartbeats, the aggregator assembles a fleet-wide health
//! view, and partition detection emerges from the gaps.
//!
//! Pattern: desired state (registry) → observe (heartbeats) → reconcile
//! (aggregate health) → bounded action (alert/failover) → read-back proof.

use serde::{Deserialize, Serialize};

use crate::error_catalog::CynicErrorCode;
pub use crate::fleet_discovery::FleetNodeStatus;

/// A single heartbeat emitted by a node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetHeartbeat {
    pub node_id: String,
    pub emitted_at_unix_s: u64,
    /// Node's self-assessed status.
    pub self_status: FleetNodeStatus,
    /// Node's uptime in seconds.
    pub uptime_s: u64,
    /// Residual unknowns the node could not observe.
    pub unknowns: Vec<String>,
}

pub const SIGNED_FLEET_HEARTBEAT_SCHEMA: &str = "cynic.signed_fleet_heartbeat.v1";
pub const AUTHENTICATED_FLEET_HEALTH_AGGREGATION_SCHEMA: &str =
    "cynic.authenticated_fleet_health_aggregation.v1";

/// A cryptographically signed heartbeat envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedFleetHeartbeat {
    pub schema: String,
    pub heartbeat: FleetHeartbeat,
    pub signer_node_id: String,
    pub certificate_id: String,
    pub signature_hex: String,
}

#[derive(Serialize)]
struct SignedFleetHeartbeatSigningPayload<'a> {
    schema: &'a str,
    heartbeat: &'a FleetHeartbeat,
    signer_node_id: &'a str,
    certificate_id: &'a str,
}

impl SignedFleetHeartbeat {
    pub fn new(
        heartbeat: FleetHeartbeat,
        signer_node_id: String,
        certificate_id: String,
        signature_hex: String,
    ) -> Self {
        Self {
            schema: SIGNED_FLEET_HEARTBEAT_SCHEMA.to_string(),
            heartbeat,
            signer_node_id,
            certificate_id,
            signature_hex,
        }
    }

    /// Canonical bytes signed by the node.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let payload = SignedFleetHeartbeatSigningPayload {
            schema: &self.schema,
            heartbeat: &self.heartbeat,
            signer_node_id: &self.signer_node_id,
            certificate_id: &self.certificate_id,
        };
        serde_json::to_vec(&payload).expect("serialize signed heartbeat payload")
    }

    /// Pure structural validation of the envelope.
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != SIGNED_FLEET_HEARTBEAT_SCHEMA {
            blockers.push("signed_heartbeat_schema_mismatch".to_string());
        }
        if self.signer_node_id.trim().is_empty() {
            blockers.push("signed_heartbeat_signer_node_id_empty".to_string());
        }
        if self.signer_node_id != self.heartbeat.node_id {
            blockers.push("signed_heartbeat_signer_mismatch".to_string());
        }
        if !self.certificate_id.starts_with("sha256:") {
            blockers.push("signed_heartbeat_certificate_id_invalid".to_string());
        }
        if self.signature_hex.len() != 128
            || !self.signature_hex.bytes().all(|b| b.is_ascii_hexdigit())
        {
            blockers.push("signed_heartbeat_signature_invalid".to_string());
        }
        blockers.sort();
        blockers.dedup();
        blockers
    }
}

/// Result of peer signature & certificate validation on an incoming heartbeat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HeartbeatValidationResult {
    Valid,
    InvalidSignature,
    UntrustedSigner,
    ExpiredCertificate,
    ClockSkewExceeded,
    NodeIdMismatch,
    MalformedPayload(String),
}

/// Health aggregation based solely on mathematically proved peer heartbeats.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthenticatedFleetHealthAggregation {
    pub schema: &'static str,
    pub aggregated_at_unix_s: u64,
    pub total_declared: usize,
    pub total_verified: usize,
    pub total_rejected: usize,
    pub alive: usize,
    pub degraded: usize,
    pub unreachable: usize,
    pub silent: Vec<String>,
    pub untrusted: Vec<String>,
    pub rejected_reasons: Vec<(String, HeartbeatValidationResult)>,
    pub verified_heartbeats: Vec<FleetHeartbeat>,
    pub partition_detected: bool,
    pub observation_blackout: bool,
    pub residual_unknowns: Vec<String>,
}

/// Pure decision: aggregate authenticated peer heartbeats into fleet health.
pub fn aggregate_authenticated_fleet_health(
    declared_node_ids: &[String],
    verified_heartbeats: &[FleetHeartbeat],
    rejected_heartbeats: &[(String, HeartbeatValidationResult)],
    now_unix_s: u64,
    heartbeat_timeout_s: u64,
) -> AuthenticatedFleetHealthAggregation {
    let mut alive = 0usize;
    let mut degraded = 0usize;
    let mut unreachable = 0usize;
    let mut silent = Vec::new();
    let mut untrusted = Vec::new();
    let mut recent_verified = Vec::new();

    for node_id in declared_node_ids {
        let latest = verified_heartbeats
            .iter()
            .filter(|h| &h.node_id == node_id)
            .max_by_key(|h| h.emitted_at_unix_s);

        match latest {
            Some(hb) if now_unix_s.saturating_sub(hb.emitted_at_unix_s) <= heartbeat_timeout_s => {
                recent_verified.push(hb.clone());
                match hb.self_status {
                    FleetNodeStatus::Alive => alive += 1,
                    FleetNodeStatus::Degraded => degraded += 1,
                    FleetNodeStatus::Unreachable => unreachable += 1,
                }
            }
            _ => {
                silent.push(node_id.clone());
                unreachable += 1;
            }
        }
    }

    for (node_id, _) in rejected_heartbeats {
        if !declared_node_ids.contains(node_id) {
            untrusted.push(node_id.clone());
        }
    }
    untrusted.sort();
    untrusted.dedup();

    let partition_detected = !silent.is_empty() && alive > 0;
    let observation_blackout = !declared_node_ids.is_empty() && recent_verified.is_empty();
    let mut residual_unknowns = Vec::new();
    if observation_blackout {
        residual_unknowns.push(CynicErrorCode::FleetObservationBlackout.code().to_string());
    }

    AuthenticatedFleetHealthAggregation {
        schema: AUTHENTICATED_FLEET_HEALTH_AGGREGATION_SCHEMA,
        aggregated_at_unix_s: now_unix_s,
        total_declared: declared_node_ids.len(),
        total_verified: recent_verified.len(),
        total_rejected: rejected_heartbeats.len(),
        alive,
        degraded,
        unreachable,
        silent,
        untrusted,
        rejected_reasons: rejected_heartbeats.to_vec(),
        verified_heartbeats: recent_verified,
        partition_detected,
        observation_blackout,
        residual_unknowns,
    }
}

/// Aggregated health view for the whole fleet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FleetHealthAggregation {
    pub schema: &'static str,
    pub aggregated_at_unix_s: u64,
    pub total_declared: usize,
    pub total_observed: usize,
    pub alive: usize,
    pub degraded: usize,
    pub unreachable: usize,
    /// Nodes that declared themselves but sent no heartbeat within the window.
    pub silent: Vec<String>,
    /// Nodes that sent a heartbeat but are not in the registry.
    pub unexpected: Vec<String>,
    /// Per-node latest heartbeat.
    pub heartbeats: Vec<FleetHeartbeat>,
    /// Partition detection: are there unreachable segments?
    ///
    /// True only for a *split* view: some declared nodes are silent while at
    /// least one is still alive. A total blackout is not a partition — see
    /// `observation_blackout`.
    pub partition_detected: bool,
    /// Total observation blackout: declared nodes exist but none reported
    /// within the window.
    ///
    /// This is deliberately distinct from `partition_detected`. With zero
    /// observations there is no evidence of a split *or* of health: the fleet
    /// state is unknown. Reporting `partition_detected: false` alone would let
    /// a full blackout read as a healthy fleet, which violates "unknown is
    /// data". Carries `CYNIC-FLT-001` in `residual_unknowns`.
    pub observation_blackout: bool,
    pub residual_unknowns: Vec<String>,
}

/// Pure decision: aggregate heartbeats into a fleet health view.
pub fn aggregate_fleet_health(
    declared_node_ids: &[String],
    heartbeats: &[FleetHeartbeat],
    now_unix_s: u64,
    heartbeat_timeout_s: u64,
) -> FleetHealthAggregation {
    let mut alive = 0usize;
    let mut degraded = 0usize;
    let mut unreachable = 0usize;
    let mut silent = Vec::new();
    let mut unexpected = Vec::new();
    let mut recent_heartbeats = Vec::new();

    // Check each declared node for a recent heartbeat.
    for node_id in declared_node_ids {
        let latest = heartbeats
            .iter()
            .filter(|h| &h.node_id == node_id)
            .max_by_key(|h| h.emitted_at_unix_s);

        match latest {
            Some(hb) if now_unix_s - hb.emitted_at_unix_s <= heartbeat_timeout_s => {
                recent_heartbeats.push(hb.clone());
                match hb.self_status {
                    FleetNodeStatus::Alive => alive += 1,
                    FleetNodeStatus::Degraded => degraded += 1,
                    FleetNodeStatus::Unreachable => unreachable += 1,
                }
            }
            _ => {
                silent.push(node_id.clone());
                unreachable += 1;
            }
        }
    }

    // Detect heartbeats from nodes not in the registry.
    for hb in heartbeats {
        if !declared_node_ids.contains(&hb.node_id) {
            unexpected.push(hb.node_id.clone());
        }
    }

    // Partition detection: if >0 declared nodes are silent AND at least one
    // node is alive, we may have a network partition.
    let partition_detected = !silent.is_empty() && alive > 0;

    // Total blackout: declared nodes exist but nothing was observed. This is
    // not a partition (no split is visible) and it is not health — it is an
    // absence of evidence, so it is reported as an explicit unknown.
    let observation_blackout = !declared_node_ids.is_empty() && recent_heartbeats.is_empty();
    let mut residual_unknowns = Vec::new();
    if observation_blackout {
        residual_unknowns.push(CynicErrorCode::FleetObservationBlackout.code().to_string());
    }

    FleetHealthAggregation {
        schema: "cynic.fleet_health_aggregation.v1",
        aggregated_at_unix_s: now_unix_s,
        total_declared: declared_node_ids.len(),
        total_observed: recent_heartbeats.len(),
        alive,
        degraded,
        unreachable,
        silent,
        unexpected,
        heartbeats: recent_heartbeats,
        partition_detected,
        observation_blackout,
        residual_unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, status: FleetNodeStatus, at: u64) -> FleetHeartbeat {
        FleetHeartbeat {
            node_id: id.into(),
            emitted_at_unix_s: at,
            self_status: status,
            uptime_s: 100,
            unknowns: vec![],
        }
    }

    #[test]
    fn all_healthy() {
        let declared = vec!["a".into(), "b".into()];
        let heartbeats = vec![
            node("a", FleetNodeStatus::Alive, 1000),
            node("b", FleetNodeStatus::Alive, 1000),
        ];
        let agg = aggregate_fleet_health(&declared, &heartbeats, 1000, 30);
        assert_eq!(agg.alive, 2);
        assert_eq!(agg.unreachable, 0);
        assert!(!agg.partition_detected);
    }

    #[test]
    fn silent_node_detected() {
        let declared = vec!["a".into(), "b".into()];
        let heartbeats = vec![node("a", FleetNodeStatus::Alive, 1000)];
        let agg = aggregate_fleet_health(&declared, &heartbeats, 1000, 30);
        assert_eq!(agg.alive, 1);
        assert_eq!(agg.silent, vec!["b"]);
        assert!(agg.partition_detected);
    }

    #[test]
    fn stale_heartbeat_treated_as_silent() {
        let declared = vec!["a".into()];
        let heartbeats = vec![node("a", FleetNodeStatus::Alive, 900)];
        let agg = aggregate_fleet_health(&declared, &heartbeats, 1000, 30);
        assert_eq!(agg.silent, vec!["a"]);
        // A single stale node is a total blackout, not a partition: no node is
        // alive, so there is no split view to detect. The state is unknown.
        assert!(!agg.partition_detected);
        assert!(agg.observation_blackout);
        assert!(agg.residual_unknowns.contains(&"CYNIC-FLT-001".to_string()));
    }

    /// Regression proof: a total blackout must not read as a healthy fleet.
    ///
    /// `partition_detected` requires `alive > 0` (a split view). With zero
    /// observations it is correctly false — but false alone is
    /// indistinguishable from "all good". `observation_blackout` keeps the
    /// absence of evidence as explicit data.
    #[test]
    fn total_blackout_is_unknown_not_healthy() {
        let declared = vec!["a".into(), "b".into()];
        let agg = aggregate_fleet_health(&declared, &[], 1000, 30);
        assert_eq!(agg.total_observed, 0);
        assert_eq!(agg.alive, 0);
        assert_eq!(agg.silent.len(), 2);
        assert!(!agg.partition_detected);
        assert!(agg.observation_blackout);
        assert!(agg.residual_unknowns.contains(&"CYNIC-FLT-001".to_string()));
    }

    /// A genuine split (some silent, some alive) is a partition and NOT a
    /// blackout — the two conditions must stay mutually exclusive.
    #[test]
    fn partition_is_not_blackout() {
        let declared = vec!["a".into(), "b".into()];
        let heartbeats = vec![node("a", FleetNodeStatus::Alive, 1000)];
        let agg = aggregate_fleet_health(&declared, &heartbeats, 1000, 30);
        assert!(agg.partition_detected);
        assert!(!agg.observation_blackout);
        assert!(agg.residual_unknowns.is_empty());
    }

    #[test]
    fn unexpected_node_flagged() {
        let declared = vec!["a".into()];
        let heartbeats = vec![
            node("a", FleetNodeStatus::Alive, 1000),
            node("rogue", FleetNodeStatus::Alive, 1000),
        ];
        let agg = aggregate_fleet_health(&declared, &heartbeats, 1000, 30);
        assert_eq!(agg.unexpected, vec!["rogue"]);
    }

    #[test]
    fn degraded_counts_separately() {
        let declared = vec!["a".into(), "b".into()];
        let heartbeats = vec![
            node("a", FleetNodeStatus::Alive, 1000),
            node("b", FleetNodeStatus::Degraded, 1000),
        ];
        let agg = aggregate_fleet_health(&declared, &heartbeats, 1000, 30);
        assert_eq!(agg.alive, 1);
        assert_eq!(agg.degraded, 1);
        assert!(!agg.partition_detected);
    }

    #[test]
    fn empty_fleet_is_unknown() {
        let agg = aggregate_fleet_health(&[], &[], 1000, 30);
        assert_eq!(agg.total_declared, 0);
        assert!(!agg.partition_detected);
    }

    #[test]
    fn signed_heartbeat_envelope_validation() {
        let hb = node("node-alpha", FleetNodeStatus::Alive, 1000);
        let signed = SignedFleetHeartbeat::new(
            hb.clone(),
            "node-alpha".to_string(),
            format!("sha256:{}", "a".repeat(64)),
            "b".repeat(128),
        );
        assert!(signed.validate().is_empty());

        let invalid_signer = SignedFleetHeartbeat::new(
            hb,
            "node-other".to_string(),
            "invalid-cert".to_string(),
            "too-short".to_string(),
        );
        let blockers = invalid_signer.validate();
        assert!(blockers.contains(&"signed_heartbeat_signer_mismatch".to_string()));
        assert!(blockers.contains(&"signed_heartbeat_certificate_id_invalid".to_string()));
        assert!(blockers.contains(&"signed_heartbeat_signature_invalid".to_string()));
    }

    #[test]
    fn authenticated_aggregation_only_counts_verified_heartbeats() {
        let declared = vec!["a".into(), "b".into()];
        let verified = vec![node("a", FleetNodeStatus::Alive, 1000)];
        let rejected = vec![("b".into(), HeartbeatValidationResult::InvalidSignature)];

        let agg = aggregate_authenticated_fleet_health(&declared, &verified, &rejected, 1000, 30);
        assert_eq!(agg.total_declared, 2);
        assert_eq!(agg.total_verified, 1);
        assert_eq!(agg.total_rejected, 1);
        assert_eq!(agg.alive, 1);
        assert_eq!(agg.unreachable, 1);
        assert_eq!(agg.silent, vec!["b"]);
        assert!(agg.partition_detected);
    }
}
