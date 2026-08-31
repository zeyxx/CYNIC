//! Typed health signals, anomalies, and autonomic remediation types.

use crate::error_catalog::CynicErrorCode;
use serde::{Deserialize, Serialize};

pub const HEALTH_SIGNAL_SCHEMA: &str = "cynic.health_signal.v1";
pub const ANOMALY_SIGNAL_SCHEMA: &str = "cynic.anomaly_signal.v1";
pub const AUTONOMIC_REMEDIATION_PLAN_SCHEMA: &str = "cynic.autonomic_remediation_plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OsHealthAnomaly {
    pub severity: String,
    pub error_code: Option<String>,
    pub detail: String,
    pub observed_at_unix_ms: u128,
}

impl OsHealthAnomaly {
    pub fn to_kernel_anomaly(&self) -> (String, String, String) {
        let kind = self
            .error_code
            .as_deref()
            .unwrap_or("OS_HEALTH_ANOMALY")
            .to_string();
        (kind, self.detail.clone(), self.severity.clone())
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct NodeHealthFold {
    pub critical_active: bool,
    pub warning_active: bool,
    pub active_osha_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSpofSignal {
    pub schema: &'static str,
    pub kernel_location: String,
    pub failover_available: bool,
    pub kernel_reachable: bool,
    pub last_heartbeat_age_s: Option<u64>,
    pub residual_risk: String,
    pub severity: String,
}

impl KernelSpofSignal {
    pub const SCHEMA: &'static str = "cynic.kernel_spof_signal.v1";

    pub fn single_vm(kernel_reachable: bool, last_heartbeat_age_s: Option<u64>) -> Self {
        let severity = if kernel_reachable { "info" } else { "critical" };
        let residual_risk = if kernel_reachable {
            "kernel reachable on dedicated VM; loss of that VM is a total-cortex SPOF with no automated failover"
                .to_string()
        } else {
            "kernel UNREACHABLE: federation cortex down; nodes must run in degraded local autonomy (no coordination/event stream)".to_string()
        };
        KernelSpofSignal {
            schema: Self::SCHEMA,
            kernel_location: "dedicated_vm".to_string(),
            failover_available: false,
            kernel_reachable,
            last_heartbeat_age_s,
            residual_risk,
            severity: severity.to_string(),
        }
    }
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
    RegistrySubstrateDivergence {
        node_id: String,
        claim_kind: String,
        declared: String,
        observed: String,
        declared_as_candidate: bool,
    },
    NameResolutionInconsistent {
        name: String,
        answers: Vec<(String, String)>,
        unresolved_from: Vec<String>,
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
    ActivationBlocked {
        node_id: String,
        phase: String,
        blockers: Vec<CynicErrorCode>,
        consent_granted: bool,
    },
    AgentScopeConflict {
        scope_kind: String,
        scope_path: String,
        holder_agent_id: String,
        holder_mode: String,
        contender_agent_id: String,
        contender_mode: String,
    },
    AgentClaimExpired {
        claim_id: String,
        agent_id: String,
        scope_path: String,
        ttl_seconds: u64,
        age_seconds: u64,
    },
    SharedTypeDrift {
        crate_name: String,
        type_name: String,
        divergent_symbols: Vec<String>,
        affected_consumers: Vec<String>,
    },
    FleetObservationBlackout {
        declared_nodes: usize,
        observed_nodes: usize,
        window_seconds: u64,
    },
}

impl HealthSignalPayload {
    pub fn error_codes(&self) -> Vec<CynicErrorCode> {
        match self {
            Self::ActivationBlocked { blockers, .. } => blockers.clone(),
            Self::AgentScopeConflict { .. } => vec![CynicErrorCode::AgentScopeConflict],
            Self::AgentClaimExpired { .. } => vec![CynicErrorCode::AgentClaimExpired],
            Self::SharedTypeDrift { .. } => vec![CynicErrorCode::AgentSharedTypeDrift],
            Self::RegistrySubstrateDivergence { .. } => {
                vec![CynicErrorCode::FleetRegistrySubstrateDivergence]
            }
            Self::NameResolutionInconsistent { .. } => {
                vec![CynicErrorCode::FleetNameResolutionInconsistent]
            }
            Self::FleetObservationBlackout { .. } => {
                vec![CynicErrorCode::FleetObservationBlackout]
            }
            _ => Vec::new(),
        }
    }
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
    RequestActivationConsent {
        node_id: String,
        phase: String,
        blocking_codes: Vec<String>,
    },
    YieldAgentScope {
        scope_path: String,
        contender_agent_id: String,
        holder_agent_id: String,
    },
    RenewAgentClaim {
        claim_id: String,
        agent_id: String,
        scope_path: String,
    },
    RevalidateSharedType {
        crate_name: String,
        type_name: String,
        divergent_symbols: Vec<String>,
    },
    ReconcileRegistryClaim {
        node_id: String,
        claim_kind: String,
        declared: String,
        observed: String,
        align_registry_to_substrate: String,
        materialize_declared_state: String,
    },
    UnifyNameAuthority {
        name: String,
        conflicting_answers: Vec<(String, String)>,
        authoritative_zone: String,
    },
    RestoreFleetObservation {
        declared_nodes: usize,
        window_seconds: u64,
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
