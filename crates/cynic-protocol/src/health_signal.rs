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
/// 001 SoftlockupPanicDisabled, 002 THPEnabledRisky, 003 THPDefragRisky,
/// 004 MemAvailBelowFloor, 005 ReclaimPressureAboveCeiling,
/// 006 KernelSoftlockupDetected, 007 KernelHungTaskDetected,
/// 008 OomDetected, 009 Unknown, 010 ObservationUnknown.
/// Each string is `CynicErrorCode::code()` — never a free literal.
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
    /// The registry declares something about a node that observation refutes.
    ///
    /// Distinct from `FleetDrift`, which compares two *status* strings for a
    /// node already agreed to exist. This variant carries the claim itself, so
    /// the reconciler can tell "declared role lives elsewhere" from "declared
    /// address answers nothing" without parsing prose.
    RegistrySubstrateDivergence {
        node_id: String,
        /// Which registry field diverges (`role_host`, `synapse_address`,
        /// `dns_record`, `service_binding`).
        claim_kind: String,
        /// What the registry asserts.
        declared: String,
        /// What the substrate reported when probed.
        observed: String,
        /// True when the registry itself marks the entry unproven
        /// (`status: candidate`). A candidate that never materialized is a
        /// stale intent, not a substrate fault.
        declared_as_candidate: bool,
    },
    /// One name resolves differently depending on which node asks.
    ///
    /// Carries every observed answer, not just the local one: a node cannot
    /// detect this class by looking at its own resolver.
    NameResolutionInconsistent {
        name: String,
        /// `(observer_node_id, resolved_value)` for each vantage point probed.
        answers: Vec<(String, String)>,
        /// Vantage points that could not resolve at all — unknown, not a
        /// disagreement.
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
    /// Ask the operator for explicit consent before any activation mutation.
    ///
    /// Activation is a bounded mutation: without `--yes` the reconciler must
    /// stop here rather than propose the mutation itself. This keeps consent a
    /// distinct decision from the action it authorizes.
    RequestActivationConsent {
        node_id: String,
        phase: String,
        /// Codes the operator must acknowledge, projected from the payload.
        blocking_codes: Vec<String>,
    },
    /// Yield a contested source scope so the holder can finish.
    ///
    /// A scope conflict is not retryable by waiting: the contender must release
    /// and pick another scope. This action names that, instead of leaving the
    /// collision to resolve itself through broken builds.
    YieldAgentScope {
        scope_path: String,
        contender_agent_id: String,
        holder_agent_id: String,
    },
    /// Renew a lapsed claim's TTL for an agent still holding its scope.
    ///
    /// The counterpart to `YieldAgentScope`: the claim was legitimate, only the
    /// heartbeat stopped, so renewal is the correct bounded action.
    RenewAgentClaim {
        claim_id: String,
        agent_id: String,
        scope_path: String,
    },
    /// Re-verify a drifted shared type against its current definition.
    ///
    /// Observation-and-verification only: repairing consumers is a code change
    /// that belongs to an agent under a claim, never an autonomic mutation.
    RevalidateSharedType {
        crate_name: String,
        type_name: String,
        divergent_symbols: Vec<String>,
    },
    /// Reconcile a registry claim with the substrate that refutes it.
    ///
    /// Deliberately NOT a mutation. Two safe repairs exist and only the
    /// operator knows which is intended: correct the registry to match reality,
    /// or materialize the declared state. Guessing would either erase a real
    /// plan or fabricate a node. The action therefore carries both candidate
    /// repairs and stops.
    ReconcileRegistryClaim {
        node_id: String,
        claim_kind: String,
        declared: String,
        observed: String,
        /// Repair that makes the registry match the substrate.
        align_registry_to_substrate: String,
        /// Repair that makes the substrate match the registry.
        materialize_declared_state: String,
    },
    /// Republish a name from one authoritative zone.
    ///
    /// Names it as a zone-authority repair rather than an /etc/hosts edit: the
    /// defect is that several nodes each own a private answer, so patching one
    /// more host would deepen it.
    UnifyNameAuthority {
        name: String,
        conflicting_answers: Vec<(String, String)>,
        /// The zone that should own the record once unified.
        authoritative_zone: String,
    },
    /// Restore fleet observation transport after a total blackout.
    ///
    /// Bounded to the observation plane: it repairs the ability to see, never
    /// the nodes themselves, since with zero evidence their state is unknown.
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
        HealthSignalPayload::ActivationBlocked {
            node_id,
            phase,
            blockers,
            consent_granted,
        } => {
            // Consent is a separate decision from the action it authorizes.
            // Without `--yes` the reconciler stops at requesting it; it must
            // never propose the mutation on the operator's behalf.
            let blocking_codes: Vec<String> =
                blockers.iter().map(|c| c.code().to_string()).collect();
            let codes_joined = blocking_codes.join(", ");
            // Retryability comes from the catalog, not from a local rule.
            let all_retryable = !blockers.is_empty() && blockers.iter().all(|c| c.retryable());
            let action = if *consent_granted {
                AutonomicActionKind::RecordObservationOnly {
                    reason: format!(
                        "activation of '{node_id}' consented but blocked by [{codes_joined}]"
                    ),
                }
            } else {
                AutonomicActionKind::RequestActivationConsent {
                    node_id: node_id.clone(),
                    phase: phase.clone(),
                    blocking_codes,
                }
            };
            Some(AutonomicRemediationPlan {
                schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
                plan_id: format!("remediation-activation-{node_id}-{now_unix_s}"),
                signal_id: signal.signal_id.clone(),
                generated_at_unix_s: now_unix_s,
                severity: signal.severity,
                // Requesting consent is actionable; a consented-but-blocked
                // activation is not, since the blockers remain.
                is_actionable: !*consent_granted,
                action,
                reasoning: format!(
                    "Activation phase '{phase}' for node '{node_id}' blocked by [{codes_joined}]"
                ),
                stop_condition: if all_retryable {
                    "stop when all blockers clear on re-observation".to_string()
                } else {
                    "stop until operator resolves non-retryable blockers".to_string()
                },
                rollback_intent: "leave node at its current activation phase".to_string(),
            })
        }
        HealthSignalPayload::AgentScopeConflict {
            scope_kind,
            scope_path,
            holder_agent_id,
            holder_mode,
            contender_agent_id,
            contender_mode,
        } => {
            // Cynic-mode claims preempt baseline (AGENTS.md conflict table);
            // otherwise first-wins and the contender yields.
            let contender_preempts = contender_mode == "cynic" && holder_mode == "baseline";
            let (yielding, retaining) = if contender_preempts {
                (holder_agent_id, contender_agent_id)
            } else {
                (contender_agent_id, holder_agent_id)
            };
            Some(AutonomicRemediationPlan {
                schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
                plan_id: format!("remediation-agent-scope-{now_unix_s}"),
                signal_id: signal.signal_id.clone(),
                generated_at_unix_s: now_unix_s,
                severity: SignalSeverity::Critical,
                is_actionable: true,
                action: AutonomicActionKind::YieldAgentScope {
                    scope_path: scope_path.clone(),
                    contender_agent_id: yielding.clone(),
                    holder_agent_id: retaining.clone(),
                },
                reasoning: format!(
                    "Agents '{holder_agent_id}' ({holder_mode}) and '{contender_agent_id}' ({contender_mode}) both claim {scope_kind} '{scope_path}' — '{yielding}' yields to '{retaining}'"
                ),
                stop_condition: "stop when only one active claim covers the scope".to_string(),
                rollback_intent: format!("re-claim '{scope_path}' after '{retaining}' releases"),
            })
        }
        HealthSignalPayload::AgentClaimExpired {
            claim_id,
            agent_id,
            scope_path,
            ttl_seconds,
            age_seconds,
        } => Some(AutonomicRemediationPlan {
            schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
            plan_id: format!("remediation-agent-claim-{claim_id}-{now_unix_s}"),
            signal_id: signal.signal_id.clone(),
            generated_at_unix_s: now_unix_s,
            severity: SignalSeverity::Warning,
            // Renewal is bounded and reversible, unlike yielding a scope.
            is_actionable: true,
            action: AutonomicActionKind::RenewAgentClaim {
                claim_id: claim_id.clone(),
                agent_id: agent_id.clone(),
                scope_path: scope_path.clone(),
            },
            reasoning: format!(
                "Claim '{claim_id}' held by '{agent_id}' on '{scope_path}' lapsed: age {age_seconds}s > TTL {ttl_seconds}s"
            ),
            stop_condition: "stop when heartbeat renews the claim TTL".to_string(),
            rollback_intent: format!(
                "release '{claim_id}' so another agent may preempt '{scope_path}'"
            ),
        }),
        HealthSignalPayload::SharedTypeDrift {
            crate_name,
            type_name,
            divergent_symbols,
            affected_consumers,
        } => Some(AutonomicRemediationPlan {
            schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
            plan_id: format!("remediation-type-drift-{type_name}-{now_unix_s}"),
            signal_id: signal.signal_id.clone(),
            generated_at_unix_s: now_unix_s,
            severity: SignalSeverity::Critical,
            // Deliberately NOT actionable: repairing consumers is a code change
            // that belongs to an agent working under a claim. An autonomic loop
            // that edits source would be an unbounded mutation.
            is_actionable: false,
            action: AutonomicActionKind::RevalidateSharedType {
                crate_name: crate_name.clone(),
                type_name: type_name.clone(),
                divergent_symbols: divergent_symbols.clone(),
            },
            reasoning: format!(
                "Shared type '{crate_name}::{type_name}' drifted on {} symbol(s), breaking {} consumer(s) — repair belongs to a claimed agent, not the autonomic loop",
                divergent_symbols.len(),
                affected_consumers.len()
            ),
            stop_condition: "stop when consumers compile against the current definition"
                .to_string(),
            rollback_intent: "revert the type change if consumers cannot be adapted".to_string(),
        }),
        HealthSignalPayload::FleetObservationBlackout {
            declared_nodes,
            observed_nodes,
            window_seconds,
        } => Some(AutonomicRemediationPlan {
            schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
            plan_id: format!("remediation-fleet-blackout-{now_unix_s}"),
            signal_id: signal.signal_id.clone(),
            generated_at_unix_s: now_unix_s,
            severity: SignalSeverity::Critical,
            is_actionable: true,
            action: AutonomicActionKind::RestoreFleetObservation {
                declared_nodes: *declared_nodes,
                window_seconds: *window_seconds,
            },
            reasoning: format!(
                "{observed_nodes}/{declared_nodes} declared nodes reported within {window_seconds}s — fleet state is unknown, not healthy; restoring the observation plane only"
            ),
            stop_condition: "stop when at least one node reports in-window".to_string(),
            rollback_intent: "retain existing observation transport if restore fails".to_string(),
        }),
        HealthSignalPayload::RegistrySubstrateDivergence {
            node_id,
            claim_kind,
            declared,
            observed,
            declared_as_candidate,
        } => Some(AutonomicRemediationPlan {
            schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
            plan_id: format!("remediation-registry-divergence-{now_unix_s}"),
            signal_id: signal.signal_id.clone(),
            generated_at_unix_s: now_unix_s,
            // A candidate that never materialized is stale intent, not an
            // incident: the registry already admits it is unproven.
            severity: if *declared_as_candidate {
                SignalSeverity::Warning
            } else {
                SignalSeverity::Critical
            },
            // Actionable as a decision to surface, never as a silent mutation:
            // both repairs are legitimate and only the operator knows which.
            is_actionable: false,
            action: AutonomicActionKind::ReconcileRegistryClaim {
                node_id: node_id.clone(),
                claim_kind: claim_kind.clone(),
                declared: declared.clone(),
                observed: observed.clone(),
                align_registry_to_substrate: format!(
                    "set {node_id}.{claim_kind} to the observed value {observed}"
                ),
                materialize_declared_state: format!(
                    "provision {declared} so {node_id}.{claim_kind} becomes true"
                ),
            },
            reasoning: format!(
                "registry declares {node_id}.{claim_kind}={declared} but observation reports {observed}\
                 {}; the registry is desired state and never proof, so the divergence is the finding",
                if *declared_as_candidate {
                    " (entry is marked candidate, so this is unmaterialized intent)"
                } else {
                    " (entry is not marked candidate, so one side is wrong)"
                }
            ),
            stop_condition: "stop until the operator picks which side to change".to_string(),
            rollback_intent: "no mutation proposed; nothing to roll back".to_string(),
        }),
        HealthSignalPayload::NameResolutionInconsistent {
            name,
            answers,
            unresolved_from,
        } => Some(AutonomicRemediationPlan {
            schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
            plan_id: format!("remediation-name-authority-{now_unix_s}"),
            signal_id: signal.signal_id.clone(),
            generated_at_unix_s: now_unix_s,
            severity: SignalSeverity::Critical,
            is_actionable: false,
            action: AutonomicActionKind::UnifyNameAuthority {
                name: name.clone(),
                conflicting_answers: answers.clone(),
                authoritative_zone: "synapse.cynic".to_string(),
            },
            reasoning: format!(
                "{name} resolves {} different ways across {} vantage point(s); \
                 {} could not resolve it at all. Each node holding a private \
                 answer means no single node can observe the fault, so patching \
                 one more /etc/hosts would deepen it",
                answers
                    .iter()
                    .map(|(_, value)| value.as_str())
                    .collect::<std::collections::BTreeSet<_>>()
                    .len(),
                answers.len(),
                unresolved_from.len()
            ),
            stop_condition: "stop when one zone owns the record and peers agree".to_string(),
            rollback_intent: "no mutation proposed; nothing to roll back".to_string(),
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
            divergent_symbols: vec!["target_profile".to_string(), "allowed_actions".to_string()],
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

    fn signal_with(payload: HealthSignalPayload, severity: SignalSeverity) -> HealthSignal {
        HealthSignal::new("cynic-node-100", severity, payload, 1_700_000_000)
    }

    /// Without consent the reconciler must request it, never propose the
    /// mutation itself. Consent is a separate decision from the action it
    /// authorizes — the `--yes` boundary lives in the plan, not in the caller.
    #[test]
    fn activation_without_consent_requests_it() {
        let signal = signal_with(
            HealthSignalPayload::ActivationBlocked {
                node_id: "cynic-builder".to_string(),
                phase: "takeover".to_string(),
                blockers: vec![CynicErrorCode::ActivationConsentMissing],
                consent_granted: false,
            },
            SignalSeverity::Warning,
        );

        let plan = reconcile_health_signal(&signal, 1_700_000_100).expect("plan");
        assert!(plan.is_actionable);
        match plan.action {
            AutonomicActionKind::RequestActivationConsent {
                node_id,
                phase,
                blocking_codes,
            } => {
                assert_eq!(node_id, "cynic-builder");
                assert_eq!(phase, "takeover");
                assert_eq!(blocking_codes, vec!["CYNIC-ACT-006"]);
            }
            other => panic!("expected RequestActivationConsent, got {other:?}"),
        }
    }

    /// With consent granted but blockers remaining, the plan degrades to
    /// observation. Consent authorizes a mutation; it does not clear blockers,
    /// so claiming actionability here would fabricate convergence.
    #[test]
    fn consented_but_blocked_activation_is_not_actionable() {
        let signal = signal_with(
            HealthSignalPayload::ActivationBlocked {
                node_id: "cynic-builder".to_string(),
                phase: "bootstrap".to_string(),
                blockers: vec![CynicErrorCode::ActivationPhaseAuthorityInsufficient],
                consent_granted: true,
            },
            SignalSeverity::Critical,
        );

        let plan = reconcile_health_signal(&signal, 1_700_000_100).expect("plan");
        assert!(!plan.is_actionable);
        assert!(matches!(
            plan.action,
            AutonomicActionKind::RecordObservationOnly { .. }
        ));
        // Non-retryable blocker → stop condition defers to the operator.
        assert!(plan.stop_condition.contains("non-retryable"));
    }

    /// The stop condition must derive retryability from the catalog, not from a
    /// rule re-implemented here (one canonical decision source per concern).
    #[test]
    fn activation_stop_condition_follows_catalog_retryability() {
        let signal = signal_with(
            HealthSignalPayload::ActivationBlocked {
                node_id: "n1".to_string(),
                phase: "advance".to_string(),
                // Both retryable per the catalog.
                blockers: vec![
                    CynicErrorCode::ActivationEpochUnobservable,
                    CynicErrorCode::ActivationBootstrapIncomplete,
                ],
                consent_granted: true,
            },
            SignalSeverity::Warning,
        );

        let plan = reconcile_health_signal(&signal, 1_700_000_100).expect("plan");
        assert!(plan.stop_condition.contains("re-observation"));
    }

    /// Baseline-vs-baseline conflict: first-wins, the contender yields.
    #[test]
    fn baseline_scope_conflict_makes_contender_yield() {
        let signal = signal_with(
            HealthSignalPayload::AgentScopeConflict {
                scope_kind: "crate".to_string(),
                scope_path: "crates/cynic-protocol".to_string(),
                holder_agent_id: "hermes:main".to_string(),
                holder_mode: "baseline".to_string(),
                contender_agent_id: "codex:worker".to_string(),
                contender_mode: "baseline".to_string(),
            },
            SignalSeverity::Critical,
        );

        let plan = reconcile_health_signal(&signal, 1_700_000_100).expect("plan");
        assert!(plan.is_actionable);
        match plan.action {
            AutonomicActionKind::YieldAgentScope {
                contender_agent_id,
                holder_agent_id,
                ..
            } => {
                assert_eq!(contender_agent_id, "codex:worker");
                assert_eq!(holder_agent_id, "hermes:main");
            }
            other => panic!("expected YieldAgentScope, got {other:?}"),
        }
    }

    /// Cynic mode preempts baseline (AGENTS.md conflict table): the *holder*
    /// yields even though it claimed first. Preemption direction is policy, not
    /// arrival order.
    #[test]
    fn cynic_mode_preempts_baseline_holder() {
        let signal = signal_with(
            HealthSignalPayload::AgentScopeConflict {
                scope_kind: "file".to_string(),
                scope_path: "crates/cynic-kernel/src/lib.rs".to_string(),
                holder_agent_id: "hermes:main".to_string(),
                holder_mode: "baseline".to_string(),
                contender_agent_id: "cynic:scheduler".to_string(),
                contender_mode: "cynic".to_string(),
            },
            SignalSeverity::Critical,
        );

        let plan = reconcile_health_signal(&signal, 1_700_000_100).expect("plan");
        match plan.action {
            AutonomicActionKind::YieldAgentScope {
                contender_agent_id,
                holder_agent_id,
                ..
            } => {
                // The baseline holder is the one yielding.
                assert_eq!(contender_agent_id, "hermes:main");
                assert_eq!(holder_agent_id, "cynic:scheduler");
            }
            other => panic!("expected YieldAgentScope, got {other:?}"),
        }
    }

    /// An expired claim is renewed, not yielded — the distinction Layer 2
    /// encoded must survive into the action.
    #[test]
    fn expired_claim_is_renewed_not_yielded() {
        let signal = signal_with(
            HealthSignalPayload::AgentClaimExpired {
                claim_id: "claim_001".to_string(),
                agent_id: "hermes:main".to_string(),
                scope_path: "crates/cynic-protocol".to_string(),
                ttl_seconds: 1800,
                age_seconds: 3600,
            },
            SignalSeverity::Warning,
        );

        let plan = reconcile_health_signal(&signal, 1_700_000_100).expect("plan");
        assert!(plan.is_actionable);
        assert!(matches!(
            plan.action,
            AutonomicActionKind::RenewAgentClaim { .. }
        ));
        assert!(!matches!(
            plan.action,
            AutonomicActionKind::YieldAgentScope { .. }
        ));
    }

    /// Type drift must NOT be autonomically actionable: repairing consumers is
    /// a source change owned by a claimed agent. An autonomic loop editing code
    /// would be an unbounded mutation, which the execution standard forbids.
    #[test]
    fn shared_type_drift_is_observation_not_autonomic_repair() {
        let signal = signal_with(
            HealthSignalPayload::SharedTypeDrift {
                crate_name: "cynic-protocol".to_string(),
                type_name: "BuildRecipe".to_string(),
                divergent_symbols: vec!["target_profile".to_string()],
                affected_consumers: vec!["build_workflow::tests".to_string()],
            },
            SignalSeverity::Critical,
        );

        let plan = reconcile_health_signal(&signal, 1_700_000_100).expect("plan");
        assert!(!plan.is_actionable);
        assert_eq!(plan.severity, SignalSeverity::Critical);
        assert!(matches!(
            plan.action,
            AutonomicActionKind::RevalidateSharedType { .. }
        ));
        assert!(plan.rollback_intent.contains("revert"));
    }

    /// A blackout repairs the observation plane only. With zero evidence the
    /// nodes' state is unknown, so acting on the nodes themselves would be
    /// acting without observation.
    #[test]
    fn fleet_blackout_restores_observation_plane_only() {
        let signal = signal_with(
            HealthSignalPayload::FleetObservationBlackout {
                declared_nodes: 3,
                observed_nodes: 0,
                window_seconds: 30,
            },
            SignalSeverity::Critical,
        );

        let plan = reconcile_health_signal(&signal, 1_700_000_100).expect("plan");
        assert!(plan.is_actionable);
        match plan.action {
            AutonomicActionKind::RestoreFleetObservation {
                declared_nodes,
                window_seconds,
            } => {
                assert_eq!(declared_nodes, 3);
                assert_eq!(window_seconds, 30);
            }
            other => panic!("expected RestoreFleetObservation, got {other:?}"),
        }
        assert!(plan.reasoning.contains("unknown, not healthy"));
        // No node eviction: that would act on unobserved substrate.
        assert!(!matches!(
            plan.action,
            AutonomicActionKind::EvictDegradedNode { .. }
        ));
    }

    /// Every plan must carry a stop condition and a rollback intent — the
    /// execution standard requires both to be separately inspectable.
    #[test]
    fn all_new_plans_carry_stop_and_rollback() {
        let payloads = vec![
            HealthSignalPayload::ActivationBlocked {
                node_id: "n".to_string(),
                phase: "advance".to_string(),
                blockers: vec![CynicErrorCode::ActivationReadBackAbsent],
                consent_granted: false,
            },
            HealthSignalPayload::AgentScopeConflict {
                scope_kind: "crate".to_string(),
                scope_path: "c".to_string(),
                holder_agent_id: "a".to_string(),
                holder_mode: "baseline".to_string(),
                contender_agent_id: "b".to_string(),
                contender_mode: "baseline".to_string(),
            },
            HealthSignalPayload::AgentClaimExpired {
                claim_id: "c1".to_string(),
                agent_id: "a".to_string(),
                scope_path: "c".to_string(),
                ttl_seconds: 10,
                age_seconds: 20,
            },
            HealthSignalPayload::SharedTypeDrift {
                crate_name: "c".to_string(),
                type_name: "T".to_string(),
                divergent_symbols: vec!["f".to_string()],
                affected_consumers: vec!["x".to_string()],
            },
            HealthSignalPayload::FleetObservationBlackout {
                declared_nodes: 1,
                observed_nodes: 0,
                window_seconds: 30,
            },
        ];

        for payload in payloads {
            let signal = signal_with(payload, SignalSeverity::Critical);
            let plan = reconcile_health_signal(&signal, 1_700_000_100)
                .unwrap_or_else(|| panic!("expected a plan for {:?}", signal.payload));
            assert!(!plan.stop_condition.is_empty(), "empty stop_condition");
            assert!(!plan.rollback_intent.is_empty(), "empty rollback_intent");
            assert_eq!(plan.signal_id, signal.signal_id);
            assert_eq!(plan.schema, AUTONOMIC_REMEDIATION_PLAN_SCHEMA);
        }
    }

    /// A registry claim the substrate refutes must never auto-mutate.
    ///
    /// Two repairs are legitimate — fix the registry, or provision the declared
    /// state — and only the operator knows which was intended. Guessing would
    /// either erase a real plan or fabricate a node, so the plan surfaces both
    /// and stops. This is the live case: the registry names kernel_primary at
    /// 10.10.10.100 while the kernel actually runs on cortex.
    #[test]
    fn registry_divergence_surfaces_both_repairs_and_mutates_nothing() {
        let payload = HealthSignalPayload::RegistrySubstrateDivergence {
            node_id: "local-kernel".to_string(),
            claim_kind: "role_host".to_string(),
            declared: "10.10.10.100".to_string(),
            observed: "kernel serving on cortex 192.168.100.2:3030".to_string(),
            declared_as_candidate: true,
        };
        assert_eq!(
            payload.error_codes(),
            vec![CynicErrorCode::FleetRegistrySubstrateDivergence]
        );

        let signal = signal_with(payload, SignalSeverity::Warning);
        let plan = reconcile_health_signal(&signal, 1_000).expect("divergence must reconcile");

        assert!(
            !plan.is_actionable,
            "a divergence must not be auto-applied: both repairs are valid"
        );
        assert_eq!(
            plan.rollback_intent,
            "no mutation proposed; nothing to roll back"
        );

        match &plan.action {
            AutonomicActionKind::ReconcileRegistryClaim {
                align_registry_to_substrate,
                materialize_declared_state,
                ..
            } => {
                assert!(!align_registry_to_substrate.trim().is_empty());
                assert!(!materialize_declared_state.trim().is_empty());
            }
            other => panic!("expected ReconcileRegistryClaim, got {other:?}"),
        }
    }

    /// A `candidate` entry is stale intent, not an incident.
    ///
    /// The registry already admits an unproven entry, so contradicting it is
    /// less severe than a claim asserted as fact. Without this distinction every
    /// unmaterialized plan would page as critical.
    #[test]
    fn candidate_divergence_is_warning_asserted_divergence_is_critical() {
        let make = |candidate: bool| {
            let signal = signal_with(
                HealthSignalPayload::RegistrySubstrateDivergence {
                    node_id: "n".to_string(),
                    claim_kind: "synapse_address".to_string(),
                    declared: "10.10.10.100".to_string(),
                    observed: "no answer".to_string(),
                    declared_as_candidate: candidate,
                },
                SignalSeverity::Warning,
            );
            reconcile_health_signal(&signal, 1).expect("plan").severity
        };
        assert_eq!(make(true), SignalSeverity::Warning);
        assert_eq!(make(false), SignalSeverity::Critical);
    }

    /// Inconsistent name resolution is repaired at the zone, not per host.
    ///
    /// The live case: `kernel.synapse.cynic` answers 127.0.0.1 on cortex,
    /// 192.168.100.2 on the builder, 10.10.10.100 in the registry. Vantage
    /// points that could not resolve stay `unresolved_from` — unknown, never
    /// counted as agreement.
    #[test]
    fn name_conflict_counts_distinct_answers_and_keeps_unresolved_unknown() {
        let payload = HealthSignalPayload::NameResolutionInconsistent {
            name: "kernel.synapse.cynic".to_string(),
            answers: vec![
                ("cortex".to_string(), "127.0.0.1".to_string()),
                ("cynic-builder".to_string(), "192.168.100.2".to_string()),
                ("registry".to_string(), "10.10.10.100".to_string()),
            ],
            unresolved_from: vec!["forgejo".to_string()],
        };
        assert_eq!(
            payload.error_codes(),
            vec![CynicErrorCode::FleetNameResolutionInconsistent]
        );

        let plan = reconcile_health_signal(&signal_with(payload, SignalSeverity::Critical), 7)
            .expect("plan");
        assert!(!plan.is_actionable, "zone repair needs operator intent");
        assert!(
            plan.reasoning.contains('3'),
            "must report 3 distinct answers, got: {}",
            plan.reasoning
        );
        match &plan.action {
            AutonomicActionKind::UnifyNameAuthority {
                conflicting_answers,
                authoritative_zone,
                ..
            } => {
                assert_eq!(conflicting_answers.len(), 3);
                assert!(!authoritative_zone.is_empty());
            }
            other => panic!("expected UnifyNameAuthority, got {other:?}"),
        }
    }

    /// The three fleet-divergence codes demand operator action, not a retry.
    ///
    /// Retrying a divergence re-reads the same contradiction forever. Marking
    /// them retryable would let a reconciler spin instead of surfacing.
    #[test]
    fn fleet_divergence_codes_are_not_retryable() {
        for code in [
            CynicErrorCode::FleetRegistrySubstrateDivergence,
            CynicErrorCode::FleetNameResolutionInconsistent,
            CynicErrorCode::FleetNodeUndiscoverable,
        ] {
            assert!(!code.retryable(), "{} must not be retryable", code.code());
            assert!(
                !code.operator_action().trim().is_empty(),
                "{} needs an operator action",
                code.code()
            );
        }
    }
}
