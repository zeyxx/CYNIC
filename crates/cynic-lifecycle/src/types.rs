//! Lifecycle types and report shapes (no I/O, no subprocess).

use serde::{Deserialize, Serialize};

use crate::harness_context::RuntimeHarnessContext;

use super::time::now_ms;

pub const UPDATE_LIFECYCLE_STATE_SCHEMA: &str = "cynic.update_lifecycle.state.v1";
pub const UPDATE_LIFECYCLE_STATUS_SCHEMA: &str = "cynic.update_lifecycle.status.v1";
pub const UPDATE_LIFECYCLE_PLAN_SCHEMA: &str = "cynic.update_lifecycle.plan.v1";
pub const UPDATE_LIFECYCLE_GATE_SCHEMA: &str = "cynic.update_lifecycle.gate.v1";
pub const UPDATE_LIFECYCLE_PROMOTE_SCHEMA: &str = "cynic.update_lifecycle.promote.v1";
pub const UPDATE_LIFECYCLE_OPS_SCHEMA: &str = "cynic.update_lifecycle.ops_report.v1";
pub const UPDATE_LIFECYCLE_REPAIR_SCHEMA: &str = "cynic.update_lifecycle.repair.v1";
pub const UPDATE_LIFECYCLE_APPLY_SCHEMA: &str = "cynic.update_lifecycle.apply.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LifecyclePhase {
    ClosedLocalLab,
    Conformance,
    Staging,
    Canary,
    Production,
}

impl LifecyclePhase {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ClosedLocalLab => "closed_local_lab",
            Self::Conformance => "conformance",
            Self::Staging => "staging",
            Self::Canary => "canary",
            Self::Production => "production",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "closed_local_lab" | "lab" => Some(Self::ClosedLocalLab),
            "conformance" => Some(Self::Conformance),
            "staging" => Some(Self::Staging),
            "canary" => Some(Self::Canary),
            "production" => Some(Self::Production),
            _ => None,
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Self::ClosedLocalLab => 0,
            Self::Conformance => 1,
            Self::Staging => 2,
            Self::Canary => 3,
            Self::Production => 4,
        }
    }

    pub fn next(&self) -> Option<Self> {
        match self {
            Self::ClosedLocalLab => Some(Self::Conformance),
            Self::Conformance => Some(Self::Staging),
            Self::Staging => Some(Self::Canary),
            Self::Canary => Some(Self::Production),
            Self::Production => None,
        }
    }

    pub fn progression() -> &'static [LifecyclePhase] {
        &[
            Self::ClosedLocalLab,
            Self::Conformance,
            Self::Staging,
            Self::Canary,
            Self::Production,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateLifecycleState {
    pub schema: String,
    pub current_phase: LifecyclePhase,
    pub last_gate_ok: Option<bool>,
    pub last_gate_at_unix_ms: Option<u64>,
    pub last_promote_at_unix_ms: Option<u64>,
    pub last_apply_at_unix_ms: Option<u64>,
    pub notes: Vec<String>,
    pub updated_at_unix_ms: u64,
}

impl Default for UpdateLifecycleState {
    fn default() -> Self {
        Self {
            schema: UPDATE_LIFECYCLE_STATE_SCHEMA.into(),
            current_phase: LifecyclePhase::ClosedLocalLab,
            last_gate_ok: None,
            last_gate_at_unix_ms: None,
            last_promote_at_unix_ms: None,
            last_apply_at_unix_ms: None,
            notes: vec!["default phase: closed_local_lab (not runtime authority)".into()],
            updated_at_unix_ms: now_ms(),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MutationClass {
    ObserveOnly,
    DepsScripted,
    InferenceRuntime,
    KernelRuntime,
    HostBootstrap,
    FleetNetwork,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GateCheck {
    pub id: &'static str,
    pub command: &'static str,
    pub purpose: &'static str,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PhasePolicy {
    pub phase: LifecyclePhase,
    pub harness: RuntimeHarnessContext,
    pub allowed_mutations: Vec<MutationClass>,
    pub gates: Vec<GateCheck>,
    pub promotion_gate: &'static str,
    pub rollback_intent: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct LifecycleStatusReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub state_path: String,
    pub state: UpdateLifecycleState,
    pub policy: PhasePolicy,
    pub next_phase: Option<&'static str>,
    pub can_promote: bool,
    pub promote_blocked_reason: Option<String>,
    pub progression: Vec<&'static str>,
    pub rules: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LifecyclePlanReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub phase: &'static str,
    pub policy: PhasePolicy,
    pub planned_actions: Vec<PlannedAction>,
    pub human: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PlannedAction {
    pub id: String,
    pub kind: &'static str,
    pub command: String,
    pub mutates: bool,
    pub mutation_class: Option<MutationClass>,
    pub requires_yes: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LifecycleGateReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub phase: &'static str,
    pub all_required_ok: bool,
    pub results: Vec<GateResult>,
    pub next: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GateResult {
    pub id: &'static str,
    pub command: &'static str,
    pub required: bool,
    pub ran: bool,
    pub ok: bool,
    pub exit_code: Option<i32>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LifecyclePromoteReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub from: &'static str,
    pub to: Option<&'static str>,
    pub mutates: bool,
    pub message: String,
    pub state: UpdateLifecycleState,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FindingSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OpsFinding {
    pub id: String,
    pub severity: FindingSeverity,
    pub surface: String,
    pub summary: String,
    pub evidence: String,
    pub repair_id: Option<String>,
    pub repair_command: Option<String>,
    pub phase_allows_repair: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct OpsReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub generated_at_unix_ms: u64,
    pub state_path: String,
    pub phase: &'static str,
    pub authority: String,
    pub black_box: bool,
    pub findings: Vec<OpsFinding>,
    pub finding_counts: FindingCounts,
    pub next: Vec<String>,
    pub rules: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FindingCounts {
    pub info: usize,
    pub warning: usize,
    pub error: usize,
    pub critical: usize,
    pub repairable: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepairAction {
    pub id: String,
    pub command: String,
    pub mutates: bool,
    pub mutation_class: MutationClass,
    pub finding_ids: Vec<String>,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepairReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub phase: &'static str,
    pub mutates: bool,
    pub actions: Vec<RepairAction>,
    pub results: Vec<RepairResult>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RepairResult {
    pub id: String,
    pub attempted: bool,
    pub ok: bool,
    pub exit_code: Option<i32>,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApplyReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub phase: &'static str,
    pub mutates: bool,
    pub actions: Vec<RepairAction>,
    pub results: Vec<RepairResult>,
    pub message: String,
}
