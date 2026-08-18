//! Typed health signals, anomalies, and autonomic remediation contracts.
//!
//! Replaces brittle string heuristics with schema-validated signals
//! for closed-loop organism homeostasis.

use serde::{Deserialize, Serialize};

use crate::judgment::{CynefinDomain, DijkwLayer, SagePhase};
use crate::CynicJudgment;

pub const HEALTH_SIGNAL_SCHEMA: &str = "cynic.health_signal.v1";
pub const ANOMALY_SIGNAL_SCHEMA: &str = "cynic.anomaly_signal.v1";
pub const AUTONOMIC_REMEDIATION_PLAN_SCHEMA: &str = "cynic.autonomic_remediation_plan.v1";

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

impl AutonomicRemediationPlan {
    /// Convert this remediation plan into a [`CynicJudgment`] — the canonical
    /// observe→reconcile→act→read-back projection. The plan is a *Knowledge*
    /// layer judgment (a pure decision rule applied to observed Information),
    /// in the Act phase. Its Cynefin domain derives from the severity and
    /// actionability: Critical+actionable → Chaotic (act first), Warning →
    /// Complicated (known unknowns, record and monitor).
    pub fn to_judgment(&self, source_node_id: &str) -> CynicJudgment {
        let residual_unknowns = match &self.action {
            AutonomicActionKind::RecordObservationOnly { reason } => {
                vec![reason.clone()]
            }
            _ => Vec::new(),
        };

        let confidence = if self.severity == SignalSeverity::Critical && self.is_actionable {
            0.9
        } else if self.severity == SignalSeverity::Warning {
            0.6
        } else {
            0.4
        };

        let mut j = CynicJudgment::new(
            format!("health-{}", self.plan_id),
            DijkwLayer::Knowledge,
            SagePhase::Act,
            CynefinDomain::Disordered,
            self.generated_at_unix_s,
        );

        j.residual_unknowns = residual_unknowns;
        j.confidence = confidence;
        j.cynefin_domain = self.classify_domain(source_node_id);
        j
    }

    /// Pure domain classification from the plan's own shape. Critical and
    /// actionable → Chaotic (act→sense→respond). Warning with residual
    /// unknowns → Complicated. Otherwise → Complex.
    fn classify_domain(&self, source_node_id: &str) -> CynefinDomain {
        match (&self.severity, self.is_actionable) {
            (SignalSeverity::Critical, true) => CynefinDomain::Chaotic,
            (SignalSeverity::Warning, _) => CynefinDomain::Complicated,
            _ => {
                if !source_node_id.is_empty() {
                    CynefinDomain::Complicated
                } else {
                    CynefinDomain::Complex
                }
            }
        }
    }
}

/// Portable OS-substrate anomaly, matching the kernel's `KernelEvent::Anomaly`
/// shape so the host can publish substrate faults into the kernel anomaly stream
/// without depending on the kernel crate. Mined from the 2026-08-18 soft-lockup
/// hang, where CYNIC was blind to the OS substrate entirely.
pub const OS_HEALTH_ANOMALY_SCHEMA: &str = "cynic.os_health.anomaly.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsHealthAnomaly {
    pub schema: String,
    /// Sovereign CYNIC error code (e.g. `CYNIC-OSH-001`), when mapped.
    #[serde(default)]
    pub error_code: Option<String>,
    /// Stable kind, e.g. `softlockup_no_panic`.
    pub kind: String,
    pub severity: String, // "critical" | "warning" | "info"
    pub message: String,
    pub remediation: String,
}

impl OsHealthAnomaly {
    /// Convert into a kernel `KernelEvent::Anomaly` payload (kind/message/severity).
    /// The kernel consumes this on its SSE/anomaly stream once the host reports it.
    pub fn to_kernel_anomaly(&self) -> (String, String, String) {
        (
            self.kind.clone(),
            self.message.clone(),
            self.severity.clone(),
        )
    }

    /// Convert this OS anomaly into a [`CynicJudgment`]. The anomaly is an
    /// *Information* layer judgment (schema-validated, typed), in the Sense
    /// phase (it assembles observed substrate facts into a coherent finding).
    /// Its Cynefin domain derives from the severity string.
    pub fn to_judgment(&self, source_node_id: &str) -> CynicJudgment {
        let severity = self.severity.to_ascii_lowercase();
        let (cynefin, confidence) = match severity.as_str() {
            "critical" => (CynefinDomain::Complicated, 0.7),
            "warning" => (CynefinDomain::Complicated, 0.5),
            "info" => (CynefinDomain::Clear, 0.4),
            _ => (CynefinDomain::Disordered, 0.2),
        };

        let mut j = CynicJudgment::new(
            format!("os-anomaly-{}", self.kind),
            DijkwLayer::Information,
            SagePhase::Sense,
            cynefin,
            0,
        );
        j.residual_unknowns = if self.error_code.is_none() {
            vec![format!("os_anomaly_unmapped:{}", self.kind)]
        } else {
            Vec::new()
        };
        j.confidence = confidence;
        let _ = source_node_id;
        j
    }
}
