//! Pure reconciliation functions for health signals into remediation plans.

use super::types::*;

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
                plan_id: format!("remediation-{service}-{now_unix_s}"),
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
        HealthSignalPayload::ServiceCrashLoop {
            service,
            consecutive_restarts,
            window_seconds,
        } => Some(AutonomicRemediationPlan {
            schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
            plan_id: format!("remediation-{service}-crashloop-{now_unix_s}"),
            signal_id: signal.signal_id.clone(),
            generated_at_unix_s: now_unix_s,
            severity: SignalSeverity::Critical,
            is_actionable: true,
            action: AutonomicActionKind::RestartService {
                service: service.clone(),
            },
            reasoning: format!(
                "Service '{service}' crashed {consecutive_restarts} times within {window_seconds}s — restarting unit"
            ),
            stop_condition: "stop on unit active or exit 0".to_string(),
            rollback_intent: format!("leave service {service} inactive if restart fails"),
        }),
        HealthSignalPayload::WireguardPeerDead {
            interface,
            peer_public_key_prefix,
            age_seconds,
            dead_threshold_seconds,
        } => Some(AutonomicRemediationPlan {
            schema: AUTONOMIC_REMEDIATION_PLAN_SCHEMA.to_string(),
            plan_id: format!("remediation-wg-dead-{now_unix_s}"),
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
            plan_id: format!("remediation-wg-stale-{now_unix_s}"),
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
            plan_id: format!("remediation-organ-{now_unix_s}"),
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
            plan_id: format!("remediation-metabolism-{now_unix_s}"),
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
            plan_id: format!("remediation-fleet-{now_unix_s}"),
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
            plan_id: format!("remediation-storage-{now_unix_s}"),
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
            let blocking_codes: Vec<String> =
                blockers.iter().map(|c| c.code().to_string()).collect();
            let codes_joined = blocking_codes.join(", ");
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
            severity: if *declared_as_candidate {
                SignalSeverity::Warning
            } else {
                SignalSeverity::Critical
            },
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
