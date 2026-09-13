//! Phase policy tables (pure). Gate *commands* are declared argv strings;
//! execution is the runner port's job.

use crate::harness_context::{RuntimeHarnessContext, RuntimeHarnessKind};
use cynic_protocol::HarnessAuthority;

use super::types::{GateCheck, LifecyclePhase, MutationClass, PhasePolicy};

impl LifecyclePhase {
    pub fn harness(&self) -> RuntimeHarnessContext {
        let (kind, role, authority, boundary, scope, mutates_net) = match self {
            Self::ClosedLocalLab => (
                RuntimeHarnessKind::BareMetalDirect,
                "local_lab",
                HarnessAuthority::TestProjection,
                "host_process",
                "workspace",
                false,
            ),
            Self::Conformance => (
                RuntimeHarnessKind::IsolatedNamespace,
                "conformance_runner",
                HarnessAuthority::ConformanceGate,
                "sandbox_namespace",
                "ephemeral",
                false,
            ),
            Self::Staging => (
                RuntimeHarnessKind::RootlessContainer,
                "staging_supervisor",
                HarnessAuthority::ReleaseCandidate,
                "rootless_container",
                "cluster",
                true,
            ),
            Self::Canary => (
                RuntimeHarnessKind::RootlessContainer,
                "canary_supervisor",
                HarnessAuthority::FleetCanary,
                "rootless_container",
                "fleet_canary",
                true,
            ),
            Self::Production => (
                RuntimeHarnessKind::BareMetalDirect,
                "sovereign_operator",
                HarnessAuthority::RuntimeAuthority,
                "host_system",
                "global_sovereign",
                true,
            ),
        };

        RuntimeHarnessContext {
            schema: crate::harness_context::RUNTIME_HARNESS_CONTEXT_SCHEMA,
            kind,
            role,
            authority,
            isolation_boundary: boundary,
            target_scope: scope,
            mutates_host_network: mutates_net,
        }
    }
}

pub fn phase_policy(phase: &LifecyclePhase) -> PhasePolicy {
    let harness = phase.harness();
    match phase {
        LifecyclePhase::ClosedLocalLab => PhasePolicy {
            phase: phase.clone(),
            harness,
            allowed_mutations: vec![
                MutationClass::ObserveOnly,
                MutationClass::DepsScripted,
                MutationClass::KernelRuntime,
            ],
            gates: vec![
                GateCheck {
                    id: "runtime_validate",
                    command: "cynic runtime validate --json",
                    purpose: "portable validation suite; blocked is honest",
                    required: true,
                },
                GateCheck {
                    id: "deps_observe",
                    command: "cynic deps --json",
                    purpose: "ecosystem pins vs observed PATH/principal",
                    required: true,
                },
                GateCheck {
                    id: "harness_lab",
                    command: "cynic runtime harness --json --kind closed_local_lab",
                    purpose: "confirm lab membrane is selected",
                    required: true,
                },
            ],
            promotion_gate: "lab evidence complete without promoting lab to runtime authority",
            rollback_intent: "discard lab artifacts; no production rollback needed",
        },
        LifecyclePhase::Conformance => PhasePolicy {
            phase: phase.clone(),
            harness,
            allowed_mutations: vec![
                MutationClass::ObserveOnly,
                MutationClass::DepsScripted,
                MutationClass::KernelRuntime,
            ],
            gates: vec![
                GateCheck {
                    id: "conformance",
                    command: "cynic runtime conformance --json",
                    purpose: "node satisfies execution/bootstrap/security contracts",
                    required: true,
                },
                GateCheck {
                    id: "metrics",
                    command: "cynic runtime metrics --json",
                    purpose: "metric targets and residual risk budget",
                    required: true,
                },
                GateCheck {
                    id: "surface_audit",
                    command: "cynic dev surface-audit --json",
                    purpose: "canonical CLI surfaces green",
                    required: true,
                },
            ],
            promotion_gate: "conformance + metrics pass on candidate node",
            rollback_intent: "do not promote; re-run conformance after fix",
        },
        LifecyclePhase::Staging => PhasePolicy {
            phase: phase.clone(),
            harness,
            allowed_mutations: vec![
                MutationClass::ObserveOnly,
                MutationClass::DepsScripted,
                MutationClass::InferenceRuntime,
                MutationClass::KernelRuntime,
                MutationClass::HostBootstrap,
            ],
            gates: vec![
                GateCheck {
                    id: "conformance_repeat",
                    command: "cynic runtime conformance --json",
                    purpose: "staging node still qualifies",
                    required: true,
                },
                GateCheck {
                    id: "inference_config",
                    command: "cynic runtime inference --json",
                    purpose: "inference SSOT present for chat/runtime",
                    required: true,
                },
                GateCheck {
                    id: "network_preflight",
                    command: "cynic runtime network-preflight --json",
                    purpose: "resolver/DNS preflight before release",
                    required: false,
                },
            ],
            promotion_gate: "staging proofs held under production-like isolation",
            rollback_intent: "revert staging node image/config; no fleet blast radius",
        },
        LifecyclePhase::Canary => PhasePolicy {
            phase: phase.clone(),
            harness,
            allowed_mutations: vec![
                MutationClass::ObserveOnly,
                MutationClass::DepsScripted,
                MutationClass::InferenceRuntime,
                MutationClass::KernelRuntime,
                MutationClass::HostBootstrap,
                MutationClass::FleetNetwork,
            ],
            gates: vec![
                GateCheck {
                    id: "fleet_report",
                    command: "cynic fleet report --json",
                    purpose: "canary subset health observable",
                    required: true,
                },
                GateCheck {
                    id: "metrics_canary",
                    command: "cynic runtime metrics --json",
                    purpose: "canary metrics within residual risk budget",
                    required: true,
                },
                GateCheck {
                    id: "deps_canary",
                    command: "cynic deps --json",
                    purpose: "deps not blocking required pins on canary",
                    required: true,
                },
            ],
            promotion_gate: "canary subset stable with rollback path proven",
            rollback_intent: "roll back canary nodes only; preserve production fleet",
        },
        LifecyclePhase::Production => PhasePolicy {
            phase: phase.clone(),
            harness,
            allowed_mutations: vec![
                MutationClass::ObserveOnly,
                MutationClass::DepsScripted,
                MutationClass::InferenceRuntime,
                MutationClass::KernelRuntime,
                MutationClass::HostBootstrap,
                MutationClass::FleetNetwork,
            ],
            gates: vec![
                GateCheck {
                    id: "runtime_report",
                    command: "cynic runtime report --json",
                    purpose: "persistent runtime phase + inference summary",
                    required: true,
                },
                GateCheck {
                    id: "metrics_prod",
                    command: "cynic runtime metrics --json",
                    purpose: "SLO/residual risk under production authority",
                    required: true,
                },
                GateCheck {
                    id: "surface_audit_prod",
                    command: "cynic dev surface-audit --json",
                    purpose: "canonical surfaces still green after rollout",
                    required: true,
                },
            ],
            promotion_gate: "already at production authority; further changes require new lifecycle cycle",
            rollback_intent: "controlled rollback via previous artifact + proof bundle",
        },
    }
}
