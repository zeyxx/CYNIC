//! Status + plan (pure over state store; no subprocess).

use std::path::Path;

use super::policy::phase_policy;
use super::state_store::load_state;
use super::types::{
    LifecyclePhase, LifecyclePlanReport, LifecycleStatusReport, MutationClass, PlannedAction,
    UPDATE_LIFECYCLE_PLAN_SCHEMA, UPDATE_LIFECYCLE_STATUS_SCHEMA, UpdateLifecycleState,
};

pub fn status_report(state_path: &Path) -> LifecycleStatusReport {
    let state = load_state(state_path);
    let policy = phase_policy(&state.current_phase);
    let next = state.current_phase.next();
    let (can_promote, promote_blocked_reason) = promote_eligibility(&state);
    LifecycleStatusReport {
        schema: UPDATE_LIFECYCLE_STATUS_SCHEMA,
        status: "ok",
        state_path: state_path.display().to_string(),
        state,
        policy,
        next_phase: next.map(|p| p.as_str()),
        can_promote,
        promote_blocked_reason,
        progression: LifecyclePhase::progression()
            .iter()
            .map(|p| p.as_str())
            .collect(),
        rules: vec![
            "harness phases bound mutation blast radius; they are not capabilities",
            "promote requires last_gate_ok=true for current phase",
            "apply/promote always need human --yes (no silent auto-upgrade)",
            "lab must never become production authority",
            "canary/production may mutate host/network only after explicit authority",
            "CLI is thin; business logic lives in update_lifecycle modules",
            "subprocess execution only via ArgvRunner port",
        ],
    }
}

pub fn promote_eligibility(state: &UpdateLifecycleState) -> (bool, Option<String>) {
    if state.current_phase.next().is_none() {
        return (
            false,
            Some("already at production; start a new cycle from lab after major change".into()),
        );
    }
    match state.last_gate_ok {
        Some(true) => (true, None),
        Some(false) => (
            false,
            Some("last gate failed — re-run: cynic lifecycle gate".into()),
        ),
        None => (
            false,
            Some("no gate result yet — run: cynic lifecycle gate".into()),
        ),
    }
}

pub fn plan_report(state_path: &Path) -> LifecyclePlanReport {
    let state = load_state(state_path);
    let policy = phase_policy(&state.current_phase);
    let mut planned = Vec::new();

    for g in &policy.gates {
        planned.push(PlannedAction {
            id: format!("gate:{}", g.id),
            kind: "gate",
            command: g.command.into(),
            mutates: false,
            mutation_class: None,
            requires_yes: false,
        });
    }

    for m in &policy.allowed_mutations {
        match m {
            MutationClass::ObserveOnly => planned.push(PlannedAction {
                id: "observe".into(),
                kind: "observe",
                command: "cynic lifecycle report --json".into(),
                mutates: false,
                mutation_class: Some(m.clone()),
                requires_yes: false,
            }),
            MutationClass::DepsScripted => planned.push(PlannedAction {
                id: "deps_upgrade".into(),
                kind: "apply",
                command: "cynic deps upgrade --yes".into(),
                mutates: true,
                mutation_class: Some(m.clone()),
                requires_yes: true,
            }),
            MutationClass::InferenceRuntime => planned.push(PlannedAction {
                id: "inference_start".into(),
                kind: "apply",
                command: "cynic runtime inference-start <alias> --json".into(),
                mutates: true,
                mutation_class: Some(m.clone()),
                requires_yes: true,
            }),
            MutationClass::HostBootstrap => planned.push(PlannedAction {
                id: "host_bootstrap".into(),
                kind: "apply",
                command: "cynic bootstrap host apply --apply".into(),
                mutates: true,
                mutation_class: Some(m.clone()),
                requires_yes: true,
            }),
            MutationClass::FleetNetwork => planned.push(PlannedAction {
                id: "network_actuator".into(),
                kind: "apply",
                command: "cynic network actuator apply --profile <name> --apply".into(),
                mutates: true,
                mutation_class: Some(m.clone()),
                requires_yes: true,
            }),
        }
    }

    if let Some(next) = state.current_phase.next() {
        planned.push(PlannedAction {
            id: format!("promote_to_{}", next.as_str()),
            kind: "promote",
            command: format!("cynic lifecycle promote --to {} --yes", next.as_str()),
            mutates: true,
            mutation_class: None,
            requires_yes: true,
        });
    }

    LifecyclePlanReport {
        schema: UPDATE_LIFECYCLE_PLAN_SCHEMA,
        status: "ok",
        phase: state.current_phase.as_str(),
        policy,
        planned_actions: planned,
        human: vec![
            "cynic lifecycle report",
            "cynic lifecycle repair",
            "cynic lifecycle repair --yes",
            "cynic lifecycle gate --yes",
            "cynic lifecycle apply --yes",
            "cynic lifecycle promote --to <phase> --yes",
        ],
    }
}
