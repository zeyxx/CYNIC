//! Typed health signals, anomalies, and autonomic remediation contracts.
//!
//! Replaces brittle string heuristics with typed, schema-validated signals
//! for closed-loop organism homeostasis.

use serde::{Deserialize, Serialize};

use crate::error_catalog::CynicErrorCode;

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

impl OsHealthAnomaly {
    /// Map to a kernel anomaly triple `(kind, message, severity)` compatible
    /// with the `cynic.anomaly_signal.v1` surface.
    pub fn to_kernel_anomaly(&self) -> (String, String, String) {
        let kind = self
            .error_code
            .as_deref()
            .unwrap_or("OS_HEALTH_ANOMALY")
            .to_string();
        (kind, self.detail.clone(), self.severity.clone())
    }
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

/// First-class signal describing the *kernel* single-point-of-failure.
///
/// The kernel is deployed centrally (operator machine + one dedicated VM), not
/// per-node. That makes the dedicated VM a genuine SPOF: if it dies, the whole
/// federation loses its cortex (coordination + event stream). This signal makes
/// that residual risk explicit and machine-readable instead of hidden behind a
/// "mesh is up" green light.
///
/// Design (A: node autonomy, D: kernel kept separate): nodes MUST keep working
/// in a degraded local mode when `kernel_reachable == false`. The kernel is a
/// coordination convenience, never a liveness dependency for local build/observe.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KernelSpofSignal {
    pub schema: &'static str,
    /// Where the kernel lives (operator_machine | dedicated_vm).
    pub kernel_location: String,
    /// True only when an automated failover target exists (2nd VM, helper).
    pub failover_available: bool,
    /// Observed kernel reachability from this node's perspective.
    pub kernel_reachable: bool,
    /// Seconds since last successful heartbeat, if known.
    pub last_heartbeat_age_s: Option<u64>,
    /// Residual risk statement — never empty; unknown is explicit data.
    pub residual_risk: String,
    /// Machine-readable severity for the autonomic loop.
    pub severity: String, // "info" | "warning" | "critical"
}

impl KernelSpofSignal {
    pub const SCHEMA: &'static str = "cynic.kernel_spof_signal.v1";

    /// Build a signal when no kernel failover target is configured (the common
    /// single-VM case). `kernel_reachable` and `last_heartbeat_age_s` come from
    /// the observer; everything else is the declared topology truth.
    pub fn single_vm(
        kernel_reachable: bool,
        last_heartbeat_age_s: Option<u64>,
    ) -> Self {
        let severity = if kernel_reachable {
            "info"
        } else {
            "critical"
        };
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
    /// A node activation could not converge. Blockers are typed
    /// `CynicErrorCode` variants (`CYNIC-ACT-*`), never free strings, so the
    /// reconciler decides on data instead of parsing prose.
    ActivationBlocked {
        node_id: String,
        /// Activation phase that produced the blockers (bootstrap | advance |
        /// takeover).
        phase: String,
        blockers: Vec<CynicErrorCode>,
        /// True when the operator gave explicit consent (`--yes`). Without it
        /// no bounded action may be proposed.
        consent_granted: bool,
    },
    /// Two agents hold overlapping claims on the same source scope.
    ///
    /// This is the emergence that produced the shared-type churn in this
    /// repository: agents mutate the same crate without a claim, discover the
    /// conflict only after causing it, and each "fix" reintroduces the next
    /// divergence. Making it a signal lets the harness observe collisions
    /// instead of inferring them from broken builds.
    AgentScopeConflict {
        /// Hierarchical scope (`fleet` | `node` | `crate` | `module` | `file`).
        scope_kind: String,
        scope_path: String,
        holder_agent_id: String,
        holder_mode: String,
        contender_agent_id: String,
        contender_mode: String,
    },
    /// An agent's claim TTL lapsed while it was still mutating.
    ///
    /// Distinct from a scope conflict: the claim was legitimate, the heartbeat
    /// stopped. Retryable by renewal, whereas a conflict requires a new scope.
    AgentClaimExpired {
        claim_id: String,
        agent_id: String,
        scope_path: String,
        ttl_seconds: u64,
        age_seconds: u64,
    },
    /// A shared type changed under a concurrent consumer.
    ///
    /// The observable symptom class behind the `BuildRecipe` /
    /// `NodeControlPlan` breakage: one agent adds or removes fields, other
    /// agents' construction sites stop compiling. Recorded as data so the
    /// pattern is learnable rather than rediscovered each session.
    SharedTypeDrift {
        crate_name: String,
        type_name: String,
        /// Symbols that no longer resolve or no longer match.
        divergent_symbols: Vec<String>,
        /// Consumer surfaces observed to break.
        affected_consumers: Vec<String>,
    },
    /// Declared nodes exist but none reported within the heartbeat window.
    ///
    /// Pairs with `FleetHealthAggregation::observation_blackout`: absence of
    /// evidence is reported as unknown, never as health.
    FleetObservationBlackout {
        declared_nodes: usize,
        observed_nodes: usize,
        window_seconds: u64,
    },
}

impl HealthSignalPayload {
    /// Sovereign error codes carried by this payload, when the domain has one.
    ///
    /// Lets a consumer project `CYNIC-*` codes without re-deriving policy from
    /// the payload shape — one canonical decision source per concern.
    pub fn error_codes(&self) -> Vec<CynicErrorCode> {
        match self {
            Self::ActivationBlocked { blockers, .. } => blockers.clone(),
            Self::AgentScopeConflict { .. } => vec![CynicErrorCode::AgentScopeConflict],
            Self::AgentClaimExpired { .. } => vec![CynicErrorCode::AgentClaimExpired],
            Self::SharedTypeDrift { .. } => vec![CynicErrorCode::AgentSharedTypeDrift],
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
            interface: _,
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

    /// Activation blockers must travel as typed codes, not prose.
    ///
    /// The regression this locks: `node_activation` previously carried
    /// `blockers: Vec<String>` and decided by `read_back_output.contains(..)`.
    /// A payload that owns `Vec<CynicErrorCode>` makes the blocker set
    /// machine-comparable and gives each blocker its catalogued operator action.
    #[test]
    fn activation_blocked_projects_typed_codes() {
        let payload = HealthSignalPayload::ActivationBlocked {
            node_id: "cynic-builder".to_string(),
            phase: "takeover".to_string(),
            blockers: vec![
                CynicErrorCode::ActivationTakeoverProcessInactive,
                CynicErrorCode::ActivationReadBackAbsent,
            ],
            consent_granted: false,
        };

        let codes = payload.error_codes();
        assert_eq!(codes.len(), 2);
        assert!(codes.contains(&CynicErrorCode::ActivationTakeoverProcessInactive));
        // Each blocker keeps its catalogued projection.
        assert_eq!(
            CynicErrorCode::ActivationReadBackAbsent.code(),
            "CYNIC-ACT-005"
        );
    }

    /// Scope conflict and claim expiry are different contracts and must not
    /// collapse into one code: a conflict needs a new scope, an expiry only
    /// needs renewal. Conflating them is what makes agent collisions look
    /// retryable when they are not.
    #[test]
    fn agent_emergences_map_to_distinct_codes() {
        let conflict = HealthSignalPayload::AgentScopeConflict {
            scope_kind: "crate".to_string(),
            scope_path: "crates/cynic-protocol".to_string(),
            holder_agent_id: "hermes:main".to_string(),
            holder_mode: "baseline".to_string(),
            contender_agent_id: "codex:worker".to_string(),
            contender_mode: "baseline".to_string(),
        };
        let expired = HealthSignalPayload::AgentClaimExpired {
            claim_id: "claim_001".to_string(),
            agent_id: "hermes:main".to_string(),
            scope_path: "crates/cynic-protocol".to_string(),
            ttl_seconds: 1800,
            age_seconds: 3600,
        };

        assert_eq!(
            conflict.error_codes(),
            vec![CynicErrorCode::AgentScopeConflict]
        );
        assert_eq!(
            expired.error_codes(),
            vec![CynicErrorCode::AgentClaimExpired]
        );
        assert_ne!(conflict.error_codes(), expired.error_codes());
        // Retryability differs — the whole reason they stay separate.
        assert!(CynicErrorCode::AgentClaimExpired.retryable());
        assert!(!CynicErrorCode::AgentScopeConflict.retryable());
    }

    /// Shared type drift is the emergence class behind this repository's
    /// recurring breakage (`BuildRecipe` fields, `NodeControlPlan::epoch`).
    /// Recording the divergent symbols and affected consumers as data is what
    /// makes the pattern learnable instead of rediscovered each session.
    #[test]
    fn shared_type_drift_retains_divergent_symbols() {
        let payload = HealthSignalPayload::SharedTypeDrift {
            crate_name: "cynic-protocol".to_string(),
            type_name: "BuildRecipe".to_string(),
            divergent_symbols: vec![
                "target_profile".to_string(),
                "allowed_actions".to_string(),
            ],
            affected_consumers: vec!["build_workflow::tests".to_string()],
        };

        assert_eq!(
            payload.error_codes(),
            vec![CynicErrorCode::AgentSharedTypeDrift]
        );
        match &payload {
            HealthSignalPayload::SharedTypeDrift {
                divergent_symbols,
                affected_consumers,
                ..
            } => {
                assert_eq!(divergent_symbols.len(), 2);
                assert_eq!(affected_consumers.len(), 1);
            }
            other => panic!("expected SharedTypeDrift, got {other:?}"),
        }
    }

    /// Payloads with no sovereign code must return empty, never a placeholder.
    /// Inventing a code for an unmodelled domain would fabricate authority.
    #[test]
    fn payloads_without_codes_return_empty() {
        let payload = HealthSignalPayload::StorageLatency {
            operation: "select".to_string(),
            latency_ms: 900,
            threshold_ms: 500,
        };
        assert!(payload.error_codes().is_empty());
    }

    /// The signal envelope must carry the new payloads unchanged through
    /// serde, since the harness persists them as evidence.
    #[test]
    fn new_payloads_round_trip_through_serde() {
        let signal = HealthSignal::new(
            "cynic-node-100",
            SignalSeverity::Critical,
            HealthSignalPayload::FleetObservationBlackout {
                declared_nodes: 3,
                observed_nodes: 0,
                window_seconds: 30,
            },
            1_700_000_000,
        );

        let json = serde_json::to_string(&signal).expect("serialize");
        let back: HealthSignal = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, signal);
        assert_eq!(
            back.payload.error_codes(),
            vec![CynicErrorCode::FleetObservationBlackout]
        );
    }
}
