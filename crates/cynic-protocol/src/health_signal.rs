//! Typed health signals, anomalies, and autonomic remediation contracts.
//!
//! Replaces brittle string heuristics with typed, schema-validated signals
//! for closed-loop organism homeostasis.

use serde::{Deserialize, Serialize};

pub const HEALTH_SIGNAL_SCHEMA: &str = "cynic.health_signal.v1";
pub const ANOMALY_SIGNAL_SCHEMA: &str = "cynic.anomaly_signal.v1";
pub const AUTONOMIC_REMEDIATION_PLAN_SCHEMA: &str = "cynic.autonomic_remediation_plan.v1";

/// Portable OS-substrate health anomaly, transport-neutral.
///
/// Mirrors the host-side `cynic_host::os_health` findings but carries only the
/// fields federation consumers need (severity + sovereign error code), so the
/// protocol crate stays free of host sysfs/proc dependencies. This is the
/// first-class data type the kernel anomaly stream and the node build-node
/// resolver consume.
///
/// Codes follow the sovereign `CYNIC-OSH-*` catalog (error_catalog):
/// 001 SoftlockupPanicDisabled, 002 THPDefragAggressive, 003 MemAvailBelowFloor,
/// 004 ReclaimPressureAboveCeiling, 005 KernelSoftlockupDetected,
/// 006 HungTaskDetected, 007 OomDetected, 008 ObservationUnknown.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsHealthAnomaly {
    pub severity: String,
    pub error_code: Option<String>,
    pub detail: String,
    pub observed_at_unix_ms: u128,
}

/// Fold a list of `OsHealthAnomaly` into a compact, dependency-light health
/// view (critical/warning flags + dedup codes). Used by the build-node
/// resolver to decide whether a candidate is dispatchable.
pub fn from_os_health_anomalies(anomalies: &[OsHealthAnomaly]) -> NodeHealthFold {
    let mut fold = NodeHealthFold::default();
    for a in anomalies {
        if a.severity.eq_ignore_ascii_case("critical") {
            fold.critical_active = true;
        } else if a.severity.eq_ignore_ascii_case("warning") {
            fold.warning_active = true;
        }
        if let Some(code) = &a.error_code {
            if !fold.active_osha_codes.contains(code) {
                fold.active_osha_codes.push(code.clone());
            }
        }
    }
    fold
}

/// Compact health fold derived from `OsHealthAnomaly` list.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NodeHealthFold {
    pub critical_active: bool,
    pub warning_active: bool,
    pub active_osha_codes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SignalSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "domain", content = "detail", rename_all = "snake_case")]
pub enum HealthSignalPayload {
    InferenceCrashLoop {
        service: String,
        failure_rate_percent: f64,
        consecutive_failures: u32,
    },
    OrganSilence {
        organ_id: String,
        silent_duration_seconds: u64,
    },
    MetabolismAnomaly {
        metric: String,
        observed_value: f64,
        threshold_value: f64,
    },
    FleetDrift {
        node_id: String,
        expected_status: String,
        observed_status: String,
    },
    StorageLatency {
        operation: String,
        latency_ms: u64,
        threshold_ms: u64,
    },
    WireguardPeerStale {
        interface: String,
        peer_public_key_prefix: String,
        age_seconds: u64,
        stale_threshold_seconds: u64,
    },
    WireguardPeerDead {
        interface: String,
        peer_public_key_prefix: String,
        age_seconds: u64,
        dead_threshold_seconds: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthSignal {
    pub schema: String,
    pub signal_id: String,
    pub source_node_id: String,
    pub observed_at_unix_s: u64,
    pub severity: SignalSeverity,
    pub payload: HealthSignalPayload,
}

impl HealthSignal {
    pub fn new(
        source_node_id: impl Into<String>,
        severity: SignalSeverity,
        payload: HealthSignalPayload,
        observed_at_unix_s: u64,
    ) -> Self {
        let source_str: String = source_node_id.into();
        let prefix = if source_str.is_empty() {
            "local"
        } else {
            source_str.as_str()
        };
        let signal_id = format!("sig-{prefix}-{observed_at_unix_s}");
        Self {
            schema: HEALTH_SIGNAL_SCHEMA.to_string(),
            signal_id,
            source_node_id: source_str,
            observed_at_unix_s,
            severity,
            payload,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AutonomicActionKind {
    DisableService {
        service: String,
    },
    RestartService {
        service: String,
    },
    RefreshWireguardEndpoint {
        interface: String,
        peer_pubkey_prefix: String,
    },
    PingWireguardPeer {
        interface: String,
        peer_overlay_ip: String,
    },
    SwitchToFallbackRoute {
        cidr: String,
    },
    EvictDegradedNode {
        node_id: String,
    },
    RecordObservationOnly {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AutonomicRemediationPlan {
    pub schema: String,
    pub plan_id: String,
    pub signal_id: String,
    pub generated_at_unix_s: u64,
    pub severity: SignalSeverity,
    pub is_actionable: bool,
    pub action: AutonomicActionKind,
    pub reasoning: String,
    pub stop_condition: String,
    pub rollback_intent: String,
}

/// Pure rule engine: maps a typed HealthSignal into an AutonomicRemediationPlan
pub fn reconcile_health_signal(
    signal: &HealthSignal,
    now_unix_s: u64,
) -> Option<AutonomicRemediationPlan> {
    match &signal.payload {
        HealthSignalPayload::InferenceCrashLoop {
            service,
            failure_rate_percent,
            consecutive_failures,
        } if *failure_rate_percent >= 99.0 || *consecutive_failures >= 5 => {
            Some(AutonomicRemediationPlan {
                schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
                plan_id: format!("remediation-{}-{}", service, now_unix_s),
                signal_id: signal.signal_id.clone(),
                generated_at_unix_s: now_unix_s,
                severity: SignalSeverity::Critical,
                is_actionable: true,
                action: AutonomicActionKind::RestartService {
                    service: service.clone(),
                },
                reasoning: format!(
                    "Service '{service}' failure rate {failure_rate_percent:.1}% ({consecutive_failures} consecutive failures) — restarting unit"
                ),
                stop_condition: "stop on unit active or exit 0".to_string(),
                rollback_intent: format!("leave service {service} inactive if restart fails"),
            })
        }
        HealthSignalPayload::WireguardPeerDead {
            interface,
            peer_public_key_prefix,
            age_seconds,
            dead_threshold_seconds,
        } => Some(AutonomicRemediationPlan {
            schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
            plan_id: format!("remediation-wg-dead-{}", now_unix_s),
            signal_id: signal.signal_id.clone(),
            generated_at_unix_s: now_unix_s,
            severity: SignalSeverity::Critical,
            is_actionable: true,
            action: AutonomicActionKind::RefreshWireguardEndpoint {
                interface: interface.clone(),
                peer_pubkey_prefix: peer_public_key_prefix.clone(),
            },
            reasoning: format!(
                "WireGuard peer '{peer_public_key_prefix}' on '{interface}' inactive for {age_seconds}s (threshold {dead_threshold_seconds}s) — refreshing endpoint and keepalive"
            ),
            stop_condition: "handshake renewed within 30s".to_string(),
            rollback_intent: "retain existing endpoint if refresh fails".to_string(),
        }),
        HealthSignalPayload::WireguardPeerStale {
            interface,
            peer_public_key_prefix,
            age_seconds,
            stale_threshold_seconds,
        } => Some(AutonomicRemediationPlan {
            schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
            plan_id: format!("remediation-wg-stale-{}", now_unix_s),
            signal_id: signal.signal_id.clone(),
            generated_at_unix_s: now_unix_s,
            severity: SignalSeverity::Warning,
            is_actionable: false,
            action: AutonomicActionKind::RecordObservationOnly {
                reason: format!(
                    "WireGuard peer '{peer_public_key_prefix}' handshake age {age_seconds}s > {stale_threshold_seconds}s"
                ),
            },
            reasoning: "Peer handshake approaching stale threshold — monitoring for recovery"
                .to_string(),
            stop_condition: "handshake renewed".to_string(),
            rollback_intent: "none".to_string(),
        }),
        HealthSignalPayload::OrganSilence {
            organ_id,
            silent_duration_seconds,
        } => Some(AutonomicRemediationPlan {
            schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
            plan_id: format!("remediation-organ-{}", now_unix_s),
            signal_id: signal.signal_id.clone(),
            generated_at_unix_s: now_unix_s,
            severity: SignalSeverity::Warning,
            is_actionable: false,
            action: AutonomicActionKind::RecordObservationOnly {
                reason: format!("Organ '{organ_id}' silent for {silent_duration_seconds}s"),
            },
            reasoning: format!(
                "Organ '{organ_id}' stopped emitting observations — check cron/logs"
            ),
            stop_condition: "organ emits observation".to_string(),
            rollback_intent: "none".to_string(),
        }),
        HealthSignalPayload::MetabolismAnomaly {
            metric,
            observed_value,
            threshold_value,
        } => Some(AutonomicRemediationPlan {
            schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
            plan_id: format!("remediation-metabolism-{}", now_unix_s),
            signal_id: signal.signal_id.clone(),
            generated_at_unix_s: now_unix_s,
            severity: SignalSeverity::Warning,
            is_actionable: false,
            action: AutonomicActionKind::RecordObservationOnly {
                reason: format!(
                    "Metric '{metric}'={observed_value:.2} below threshold {threshold_value:.2}"
                ),
            },
            reasoning: format!("Metabolic pressure anomaly on metric '{metric}'"),
            stop_condition: "metric recovers above threshold".to_string(),
            rollback_intent: "none".to_string(),
        }),
        HealthSignalPayload::FleetDrift {
            node_id,
            expected_status,
            observed_status,
        } => Some(AutonomicRemediationPlan {
            schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
            plan_id: format!("remediation-fleet-{}", now_unix_s),
            signal_id: signal.signal_id.clone(),
            generated_at_unix_s: now_unix_s,
            severity: SignalSeverity::Warning,
            is_actionable: false,
            action: AutonomicActionKind::RecordObservationOnly {
                reason: format!(
                    "Node '{node_id}' status drift: expected={expected_status}, observed={observed_status}"
                ),
            },
            reasoning: format!("Fleet topology drift observed for node '{node_id}'"),
            stop_condition: "node reaches expected status".to_string(),
            rollback_intent: "none".to_string(),
        }),
        HealthSignalPayload::StorageLatency {
            operation,
            latency_ms,
            threshold_ms,
        } => Some(AutonomicRemediationPlan {
            schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
            plan_id: format!("remediation-storage-{}", now_unix_s),
            signal_id: signal.signal_id.clone(),
            generated_at_unix_s: now_unix_s,
            severity: SignalSeverity::Warning,
            is_actionable: false,
            action: AutonomicActionKind::RecordObservationOnly {
                reason: format!(
                    "Storage '{operation}' latency {latency_ms}ms > threshold {threshold_ms}ms"
                ),
            },
            reasoning: "Storage latency degradation detected — monitor DB health".to_string(),
            stop_condition: "latency below threshold".to_string(),
            rollback_intent: "none".to_string(),
        }),
        _ => None,
    }
}

/// Topology drift event: emitted when observed operational reality diverges
/// from the declared desired topology (registry seed / role assignment).
///
/// Per the CYNIC coherence rule "reality primes, drift is data", an
/// unreachable builder or an OS-critical node is NOT a crash — it is a
/// first-class signal the federation reconciles (e.g. failover to a standby,
/// or emit a `cynic.topology_drift.v1` health signal).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopologyDriftSignal {
    pub node_id: String,
    pub desired_role: String,
    pub observed_role: String,
    pub reachable: bool,
    pub os_health_critical: bool,
    pub drift_kind: String,
}

impl TopologyDriftSignal {
    /// Map the drift to a kernel anomaly (schema `cynic.anomaly_signal.v1`
    /// compatible). Severity is `critical` when the node is unreachable or
    /// OS-critical, otherwise `warning`.
    pub fn to_kernel_anomaly(&self) -> (String, String, String) {
        let severity = if !self.reachable || self.os_health_critical {
            "critical"
        } else {
            "warning"
        };
        let message = format!(
            "topology drift node={} desired={} observed={} reachable={} drift={}",
            self.node_id, self.desired_role, self.observed_role, self.reachable, self.drift_kind
        );
        ("topology_drift".to_string(), message, severity.to_string())
    }

    /// Map the drift to a CynicJudgment projection (Information/Sense family).
    pub fn to_judgment(&self) -> CynicJudgment {
        let (_, message, severity) = self.to_kernel_anomaly();
        let quality = if severity == "critical" {
            JudgmentQuality::Refuted
        } else {
            JudgmentQuality::Sense
        };
        CynicJudgment {
            schema: JUDGMENT_SCHEMA.to_string(),
            judgment_id: format!("jdg-drift-{}", self.node_id),
            observed_at_unix_ms: now_unix_ms(),
            subject: format!("topology:{}", self.node_id),
            predicate: "drift_from_desired".to_string(),
            quality,
            confidence: 0.9,
            reasoning: message,
            evidence_refs: vec![],
            unknowns: vec!["residual_mesh_state".to_string()],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_drift_unreachable_is_critical() {
        let d = TopologyDriftSignal {
            node_id: "cynic-builder".to_string(),
            desired_role: "build_runner_primary".to_string(),
            observed_role: "build_runner_primary".to_string(),
            reachable: false,
            os_health_critical: false,
            drift_kind: "mesh_unreachable".to_string(),
        };
        let (kind, _msg, sev) = d.to_kernel_anomaly();
        assert_eq!(kind, "topology_drift");
        assert_eq!(sev, "critical");
    }

    #[test]
    fn topology_drift_os_critical_is_critical() {
        let d = TopologyDriftSignal {
            node_id: "cynic-fix".to_string(),
            desired_role: "build_runner_primary".to_string(),
            observed_role: "build_runner_primary".to_string(),
            reachable: true,
            os_health_critical: true,
            drift_kind: "os_health_critical".to_string(),
        };
        let (_k, _m, sev) = d.to_kernel_anomaly();
        assert_eq!(sev, "critical");
    }

    #[test]
    fn topology_drift_reachable_no_critical_is_warning() {
        let d = TopologyDriftSignal {
            node_id: "cynic-b".to_string(),
            desired_role: "build_runner_primary".to_string(),
            observed_role: "build_runner_secondary".to_string(),
            reachable: true,
            os_health_critical: false,
            drift_kind: "role_mismatch".to_string(),
        };
        let (_k, _m, sev) = d.to_kernel_anomaly();
        assert_eq!(sev, "warning");
    }
}
