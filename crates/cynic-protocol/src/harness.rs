use serde::{Deserialize, Serialize};

pub const HARNESS_SCENARIO_SCHEMA: &str = "cynic.build.harness_scenario.v1";
pub const HARNESS_CELL_REPORT_SCHEMA: &str = "cynic.build.harness_cell_report.v1";
pub const HARNESS_MATRIX_SCHEMA: &str = "cynic.build.harness_matrix.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessScenarioId {
    ClosedLabVm,
    LxcCloneConformance,
    BuilderStagingObserve,
    BuilderCanaryApply,
    FailureRecovery,
    ProofReplay,
}

impl HarnessScenarioId {
    pub const ALL: [HarnessScenarioId; 6] = [
        HarnessScenarioId::ClosedLabVm,
        HarnessScenarioId::LxcCloneConformance,
        HarnessScenarioId::BuilderStagingObserve,
        HarnessScenarioId::BuilderCanaryApply,
        HarnessScenarioId::FailureRecovery,
        HarnessScenarioId::ProofReplay,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::ClosedLabVm => "closed_lab_vm",
            Self::LxcCloneConformance => "lxc_clone_conformance",
            Self::BuilderStagingObserve => "builder_staging_observe",
            Self::BuilderCanaryApply => "builder_canary_apply",
            Self::FailureRecovery => "failure_recovery",
            Self::ProofReplay => "proof_replay",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessAuthority {
    ConformanceGate,
    StagingObservation,
    ExplicitCanary,
    ReadOnlyProjection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessStatus {
    Planned,
    Blocked,
    Unknown,
    Admitted,
    Claimed,
    Running,
    Completed,
    Failed,
    Cancelled,
    Stale,
}

impl HarnessStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Completed | Self::Failed | Self::Cancelled | Self::Stale)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessAction {
    Observe,
    Clone,
    Destroy,
    DispatchCheck,
    InjectFault,
    Replay,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessRollback {
    None,
    DestroyGuest,
    TeardownClone,
    PurgeWorkspace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessScenario {
    #[serde(default = "default_scenario_schema")]
    pub schema: String,
    pub id: HarnessScenarioId,
    pub authority: HarnessAuthority,
    pub target_role: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub prerequisites: Vec<String>,
    #[serde(default)]
    pub allowed_actions: Vec<HarnessAction>,
    #[serde(default = "default_rollback")]
    pub rollback: HarnessRollback,
    #[serde(default)]
    pub required_proofs: Vec<String>,
    #[serde(default)]
    pub explicit_apply_required: bool,
}

fn default_scenario_schema() -> String {
    HARNESS_SCENARIO_SCHEMA.to_string()
}

fn default_true() -> bool {
    true
}

fn default_rollback() -> HarnessRollback {
    HarnessRollback::None
}

impl HarnessScenario {
    pub fn closed_lab_vm() -> Self {
        Self {
            schema: HARNESS_SCENARIO_SCHEMA.to_string(),
            id: HarnessScenarioId::ClosedLabVm,
            authority: HarnessAuthority::ConformanceGate,
            target_role: "build_runner".to_string(),
            enabled: true,
            prerequisites: vec!["libvirt_socket_accessible".to_string()],
            allowed_actions: vec![HarnessAction::Observe, HarnessAction::Clone, HarnessAction::Destroy],
            rollback: HarnessRollback::DestroyGuest,
            required_proofs: vec!["target_identity".to_string(), "scheduler_observation".to_string()],
            explicit_apply_required: false,
        }
    }

    pub fn lxc_clone_conformance() -> Self {
        Self {
            schema: HARNESS_SCENARIO_SCHEMA.to_string(),
            id: HarnessScenarioId::LxcCloneConformance,
            authority: HarnessAuthority::ConformanceGate,
            target_role: "build_runner".to_string(),
            enabled: true,
            prerequisites: vec!["proxmox_provider_control_ready".to_string()],
            allowed_actions: vec![HarnessAction::Observe, HarnessAction::Clone, HarnessAction::Destroy],
            rollback: HarnessRollback::TeardownClone,
            required_proofs: vec!["clone_identity".to_string(), "isolated_network".to_string()],
            explicit_apply_required: false,
        }
    }

    pub fn builder_staging_observe() -> Self {
        Self {
            schema: HARNESS_SCENARIO_SCHEMA.to_string(),
            id: HarnessScenarioId::BuilderStagingObserve,
            authority: HarnessAuthority::StagingObservation,
            target_role: "build_runner".to_string(),
            enabled: true,
            prerequisites: vec!["node_identity_attested".to_string()],
            allowed_actions: vec![HarnessAction::Observe],
            rollback: HarnessRollback::None,
            required_proofs: vec!["builder_identity".to_string(), "service_state".to_string()],
            explicit_apply_required: false,
        }
    }

    pub fn builder_canary_apply() -> Self {
        Self {
            schema: HARNESS_SCENARIO_SCHEMA.to_string(),
            id: HarnessScenarioId::BuilderCanaryApply,
            authority: HarnessAuthority::ExplicitCanary,
            target_role: "build_runner".to_string(),
            enabled: true,
            prerequisites: vec!["staging_proof_completed".to_string()],
            allowed_actions: vec![HarnessAction::Observe, HarnessAction::DispatchCheck],
            rollback: HarnessRollback::PurgeWorkspace,
            required_proofs: vec!["signed_grant".to_string(), "workspace_commit".to_string(), "durable_completion".to_string()],
            explicit_apply_required: true,
        }
    }

    pub fn failure_recovery() -> Self {
        Self {
            schema: HARNESS_SCENARIO_SCHEMA.to_string(),
            id: HarnessScenarioId::FailureRecovery,
            authority: HarnessAuthority::ConformanceGate,
            target_role: "build_runner".to_string(),
            enabled: true,
            prerequisites: vec!["conformance_harness_ready".to_string()],
            allowed_actions: vec![HarnessAction::Observe, HarnessAction::InjectFault],
            rollback: HarnessRollback::None,
            required_proofs: vec!["durable_cancellation".to_string(), "terminal_fault_state".to_string()],
            explicit_apply_required: false,
        }
    }

    pub fn proof_replay() -> Self {
        Self {
            schema: HARNESS_SCENARIO_SCHEMA.to_string(),
            id: HarnessScenarioId::ProofReplay,
            authority: HarnessAuthority::ReadOnlyProjection,
            target_role: "build_runner".to_string(),
            enabled: true,
            prerequisites: vec!["retained_proof_present".to_string()],
            allowed_actions: vec![HarnessAction::Observe, HarnessAction::Replay],
            rollback: HarnessRollback::None,
            required_proofs: vec!["digest_equality".to_string()],
            explicit_apply_required: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessEvidenceKind {
    Observation,
    Execution,
    Teardown,
    Fault,
    Replay,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HarnessEvidenceStatus {
    Present,
    Completed,
    Failed,
    Cancelled,
    Stale,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessEvidence {
    pub kind: HarnessEvidenceKind,
    pub status: HarnessEvidenceStatus,
    pub reference: String,
    #[serde(default)]
    pub digest: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessObservation {
    pub scenario_id: Option<HarnessScenarioId>,
    #[serde(default)]
    pub target_observed: bool,
    #[serde(default)]
    pub target_identity: Option<String>,
    #[serde(default)]
    pub service_state: Option<String>,
    #[serde(default)]
    pub evidences: Vec<HarnessEvidence>,
    #[serde(default)]
    pub staging_proof_present: bool,
    #[serde(default)]
    pub staging_proof_completed: bool,
    #[serde(default)]
    pub unknowns: Vec<String>,
    #[serde(default)]
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessCellReport {
    pub schema: String,
    pub scenario_id: HarnessScenarioId,
    pub authority: HarnessAuthority,
    pub status: HarnessStatus,
    pub action: Option<HarnessAction>,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarnessMatrixReport {
    pub schema: String,
    pub cells: Vec<HarnessCellReport>,
    pub status: HarnessStatus,
    pub mutates: bool,
    pub can_apply: bool,
    pub blockers: Vec<String>,
    pub residual_unknowns: Vec<String>,
}

pub fn validate_harness_scenario(scenario: &HarnessScenario) -> Result<(), String> {
    if scenario.target_role.trim().is_empty() {
        return Err("harness_scenario_target_role_empty".to_string());
    }
    if scenario.allowed_actions.is_empty() {
        return Err("harness_scenario_allowed_actions_empty".to_string());
    }
    if scenario.authority == HarnessAuthority::ReadOnlyProjection
        && scenario.allowed_actions.iter().any(|action| !matches!(action, HarnessAction::Observe | HarnessAction::Replay))
    {
        return Err("harness_read_only_projection_mutation_forbidden".to_string());
    }
    if scenario.id == HarnessScenarioId::BuilderStagingObserve
        && scenario.allowed_actions.iter().any(|action| *action != HarnessAction::Observe)
    {
        return Err("harness_staging_observe_mutation_forbidden".to_string());
    }
    if scenario.id == HarnessScenarioId::BuilderCanaryApply && !scenario.explicit_apply_required {
        return Err("harness_canary_requires_explicit_apply".to_string());
    }
    if scenario.id == HarnessScenarioId::BuilderCanaryApply && scenario.rollback == HarnessRollback::None {
        return Err("harness_scenario_rollback_missing".to_string());
    }
    if scenario.id == HarnessScenarioId::LxcCloneConformance && scenario.rollback == HarnessRollback::None {
        return Err("harness_scenario_rollback_missing".to_string());
    }
    if scenario.id == HarnessScenarioId::ClosedLabVm && scenario.rollback == HarnessRollback::None {
        return Err("harness_scenario_rollback_missing".to_string());
    }
    Ok(())
}

pub fn reduce_harness_evidence(evidences: &[HarnessEvidence]) -> (HarnessStatus, Vec<String>) {
    if evidences.is_empty() {
        return (HarnessStatus::Unknown, Vec::new());
    }
    let evidence_refs = evidences.iter().map(|e| e.reference.clone()).collect::<Vec<_>>();
    let has_failed = evidences.iter().any(|e| e.status == HarnessEvidenceStatus::Failed);
    if has_failed {
        return (HarnessStatus::Failed, evidence_refs);
    }
    let has_cancelled = evidences.iter().any(|e| e.status == HarnessEvidenceStatus::Cancelled);
    if has_cancelled {
        return (HarnessStatus::Cancelled, evidence_refs);
    }
    let has_stale = evidences.iter().any(|e| e.status == HarnessEvidenceStatus::Stale);
    if has_stale {
        return (HarnessStatus::Stale, evidence_refs);
    }
    let all_completed = evidences.iter().all(|e| matches!(e.status, HarnessEvidenceStatus::Completed | HarnessEvidenceStatus::Present));
    if all_completed {
        return (HarnessStatus::Completed, evidence_refs);
    }
    (HarnessStatus::Running, evidence_refs)
}

pub fn reconcile_harness_scenario(
    scenario: &HarnessScenario,
    observation: &HarnessObservation,
) -> HarnessCellReport {
    let mut blockers = observation.blockers.clone();
    let mut unknowns = observation.unknowns.clone();

    if !scenario.enabled {
        return HarnessCellReport {
            schema: HARNESS_CELL_REPORT_SCHEMA.to_string(),
            scenario_id: scenario.id,
            authority: scenario.authority,
            status: HarnessStatus::Blocked,
            action: None,
            evidence_refs: Vec::new(),
            blockers: vec!["scenario_disabled_by_policy".to_string()],
            unknowns,
        };
    }

    if scenario.id == HarnessScenarioId::BuilderCanaryApply && !observation.staging_proof_completed {
        blockers.push("staging_proof_missing".to_string());
    }

    blockers.sort();
    blockers.dedup();
    unknowns.sort();
    unknowns.dedup();

    let (evidence_status, evidence_refs) = reduce_harness_evidence(&observation.evidences);

    let status = if !blockers.is_empty() {
        HarnessStatus::Blocked
    } else if evidence_status.is_terminal() {
        evidence_status
    } else if !unknowns.is_empty() || !observation.target_observed {
        HarnessStatus::Unknown
    } else if evidence_status == HarnessStatus::Running {
        HarnessStatus::Running
    } else {
        HarnessStatus::Planned
    };

    let action = match scenario.id {
        HarnessScenarioId::ClosedLabVm => Some(HarnessAction::Observe),
        HarnessScenarioId::LxcCloneConformance => Some(HarnessAction::Observe),
        HarnessScenarioId::BuilderStagingObserve => Some(HarnessAction::Observe),
        HarnessScenarioId::BuilderCanaryApply => {
            if status == HarnessStatus::Completed {
                None
            } else {
                Some(HarnessAction::DispatchCheck)
            }
        }
        HarnessScenarioId::FailureRecovery => Some(HarnessAction::Observe),
        HarnessScenarioId::ProofReplay => Some(HarnessAction::Replay),
    };

    HarnessCellReport {
        schema: HARNESS_CELL_REPORT_SCHEMA.to_string(),
        scenario_id: scenario.id,
        authority: scenario.authority,
        status,
        action,
        evidence_refs,
        blockers,
        unknowns,
    }
}

pub fn parse_harness_scenarios(value: &serde_json::Value) -> Result<Vec<HarnessScenario>, String> {
    let array = value
        .get("harness_scenarios")
        .or_else(|| value.get("scenarios"))
        .or_else(|| value.as_array().map(|_| value))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "harness_scenarios_array_missing".to_string())?;

    let mut scenarios = Vec::new();
    let mut seen_ids = std::collections::BTreeSet::new();

    for item in array {
        let scenario: HarnessScenario = serde_json::from_value(item.clone())
            .map_err(|err| format!("invalid_harness_scenario_json:{err}"))?;
        validate_harness_scenario(&scenario)?;
        if !seen_ids.insert(scenario.id) {
            return Err("harness_scenario_id_duplicate".to_string());
        }
        scenarios.push(scenario);
    }

    Ok(scenarios)
}

pub fn reconcile_harness_matrix(
    scenarios: &[HarnessScenario],
    observations: &[HarnessObservation],
) -> HarnessMatrixReport {
    let mut cells = Vec::new();
    let mut all_blockers = Vec::new();
    let mut all_unknowns = Vec::new();

    for scenario in scenarios {
        let empty_obs = HarnessObservation {
            scenario_id: Some(scenario.id),
            ..Default::default()
        };
        let observation = observations
            .iter()
            .find(|o| o.scenario_id == Some(scenario.id))
            .unwrap_or(&empty_obs);
        let cell = reconcile_harness_scenario(scenario, observation);
        all_blockers.extend(cell.blockers.iter().cloned());
        all_unknowns.extend(cell.unknowns.iter().cloned());
        cells.push(cell);
    }

    all_blockers.sort();
    all_blockers.dedup();
    all_unknowns.sort();
    all_unknowns.dedup();

    let matrix_status = if cells.is_empty() {
        HarnessStatus::Unknown
    } else if cells.iter().any(|c| c.status == HarnessStatus::Failed) {
        HarnessStatus::Failed
    } else if cells.iter().any(|c| c.status == HarnessStatus::Blocked) {
        HarnessStatus::Blocked
    } else if cells.iter().all(|c| c.status == HarnessStatus::Completed) {
        HarnessStatus::Completed
    } else if cells.iter().any(|c| c.status == HarnessStatus::Running) {
        HarnessStatus::Running
    } else if cells.iter().any(|c| c.status == HarnessStatus::Unknown) {
        HarnessStatus::Unknown
    } else {
        HarnessStatus::Planned
    };

    HarnessMatrixReport {
        schema: HARNESS_MATRIX_SCHEMA.to_string(),
        cells,
        status: matrix_status,
        mutates: false,
        can_apply: false,
        blockers: all_blockers,
        residual_unknowns: all_unknowns,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canary_requires_completed_staging_evidence() {
        let scenario = HarnessScenario::builder_canary_apply();
        let report = reconcile_harness_scenario(&scenario, &HarnessObservation::default());
        assert_eq!(report.status, HarnessStatus::Blocked);
        assert!(report.blockers.contains(&"staging_proof_missing".to_string()));
    }

    #[test]
    fn desired_state_cannot_promote_unknown_proof() {
        let scenario = HarnessScenario::closed_lab_vm();
        let report = reconcile_harness_scenario(&scenario, &HarnessObservation::default());
        assert_eq!(report.status, HarnessStatus::Unknown);
    }

    #[test]
    fn terminal_failure_preserves_evidence_reference() {
        let (status, refs) = reduce_harness_evidence(&[HarnessEvidence {
            kind: HarnessEvidenceKind::Execution,
            status: HarnessEvidenceStatus::Failed,
            reference: "completion:sha256:deadbeef".to_string(),
            digest: None,
            detail: None,
        }]);
        assert_eq!(status, HarnessStatus::Failed);
        assert_eq!(refs, vec!["completion:sha256:deadbeef".to_string()]);
    }

    #[test]
    fn terminal_cancellation_preserves_evidence_reference() {
        let (status, refs) = reduce_harness_evidence(&[HarnessEvidence {
            kind: HarnessEvidenceKind::Execution,
            status: HarnessEvidenceStatus::Cancelled,
            reference: "cancellation:workload-1".to_string(),
            digest: None,
            detail: None,
        }]);
        assert_eq!(status, HarnessStatus::Cancelled);
        assert_eq!(refs, vec!["cancellation:workload-1".to_string()]);
    }

    #[test]
    fn scenario_validation_enforces_invariants() {
        let mut invalid_role = HarnessScenario::closed_lab_vm();
        invalid_role.target_role = "".to_string();
        assert_eq!(
            validate_harness_scenario(&invalid_role).unwrap_err(),
            "harness_scenario_target_role_empty"
        );

        let mut read_only_mutation = HarnessScenario::proof_replay();
        read_only_mutation.allowed_actions = vec![HarnessAction::DispatchCheck];
        assert_eq!(
            validate_harness_scenario(&read_only_mutation).unwrap_err(),
            "harness_read_only_projection_mutation_forbidden"
        );

        let mut canary_no_apply = HarnessScenario::builder_canary_apply();
        canary_no_apply.explicit_apply_required = false;
        assert_eq!(
            validate_harness_scenario(&canary_no_apply).unwrap_err(),
            "harness_canary_requires_explicit_apply"
        );

        let mut canary_no_rollback = HarnessScenario::builder_canary_apply();
        canary_no_rollback.rollback = HarnessRollback::None;
        assert_eq!(
            validate_harness_scenario(&canary_no_rollback).unwrap_err(),
            "harness_scenario_rollback_missing"
        );
    }

    #[test]
    fn matrix_reconciliation_reduces_composite_status() {
        let scenarios = vec![
            HarnessScenario::closed_lab_vm(),
            HarnessScenario::builder_staging_observe(),
        ];
        let observations = vec![
            HarnessObservation {
                scenario_id: Some(HarnessScenarioId::ClosedLabVm),
                target_observed: true,
                evidences: vec![HarnessEvidence {
                    kind: HarnessEvidenceKind::Observation,
                    status: HarnessEvidenceStatus::Completed,
                    reference: "evidence:vm".to_string(),
                    digest: None,
                    detail: None,
                }],
                ..Default::default()
            },
            HarnessObservation {
                scenario_id: Some(HarnessScenarioId::BuilderStagingObserve),
                target_observed: true,
                evidences: vec![HarnessEvidence {
                    kind: HarnessEvidenceKind::Observation,
                    status: HarnessEvidenceStatus::Completed,
                    reference: "evidence:staging".to_string(),
                    digest: None,
                    detail: None,
                }],
                ..Default::default()
            },
        ];
        let matrix = reconcile_harness_matrix(&scenarios, &observations);
        assert_eq!(matrix.status, HarnessStatus::Completed);
        assert_eq!(matrix.cells.len(), 2);
    }

    #[test]
    fn parse_catalog_from_json_fixture() {
        let raw = serde_json::json!({
            "harness_scenarios": [
                {
                    "schema": "cynic.build.harness_scenario.v1",
                    "id": "closed_lab_vm",
                    "authority": "conformance_gate",
                    "target_role": "build_runner",
                    "enabled": true,
                    "allowed_actions": ["observe", "clone", "destroy"],
                    "rollback": "destroy_guest"
                },
                {
                    "schema": "cynic.build.harness_scenario.v1",
                    "id": "builder_staging_observe",
                    "authority": "staging_observation",
                    "target_role": "build_runner",
                    "enabled": true,
                    "allowed_actions": ["observe"],
                    "rollback": "none"
                }
            ]
        });
        let scenarios = parse_harness_scenarios(&raw).expect("parsed scenarios");
        assert_eq!(scenarios.len(), 2);
        assert_eq!(scenarios[0].id, HarnessScenarioId::ClosedLabVm);
        assert_eq!(scenarios[1].id, HarnessScenarioId::BuilderStagingObserve);
    }

    #[test]
    fn parse_catalog_rejects_duplicate_scenario_id() {
        let raw = serde_json::json!({
            "harness_scenarios": [
                {
                    "schema": "cynic.build.harness_scenario.v1",
                    "id": "closed_lab_vm",
                    "authority": "conformance_gate",
                    "target_role": "build_runner",
                    "enabled": true,
                    "allowed_actions": ["observe"],
                    "rollback": "destroy_guest"
                },
                {
                    "schema": "cynic.build.harness_scenario.v1",
                    "id": "closed_lab_vm",
                    "authority": "conformance_gate",
                    "target_role": "build_runner",
                    "enabled": true,
                    "allowed_actions": ["observe"],
                    "rollback": "destroy_guest"
                }
            ]
        });
        assert_eq!(
            parse_harness_scenarios(&raw).unwrap_err(),
            "harness_scenario_id_duplicate"
        );
    }
}
