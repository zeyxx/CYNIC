//! Read-only projection: which build_runner node should own the next admitted
//! build, with explicit graceful-degradation reporting.
//!
//! This is a projection over the canonical `select_build_runner` reconcile in
//! `cynic_node::domain::fleet_truth`. It never mutates; it observes the
//! registry + observed fleet resources, derives the announced (preferred)
//! runner, and reports the selection + degradation reason for any agent that
//! asks. The actual build dispatch still happens through the node scheduler.

use std::process::ExitCode;

use serde_json::json;

use cynic_node::domain::fleet_truth::{self, SelectedBuildRunner};

use crate::fleet_report;
use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};

/// Derive the announced (preferred) build_runner.
///
/// Explicit `--preferred` wins. Otherwise the preferred runner is the first
/// declared `build_runner` with `status == "active"`; if none is active, the
/// first declared `build_runner` (so the reconcile can still explain why it is
/// unavailable rather than failing blind).
fn derive_preferred(
    explicit: Option<String>,
    declared: &[cynic_node::domain::fleet_truth::Node],
) -> String {
    if let Some(value) = explicit.filter(|v| !v.is_empty()) {
        return value;
    }
    declared
        .iter()
        .filter(|n| n.role == "build_runner")
        .find(|n| n.status == "active")
        .or_else(|| declared.iter().find(|n| n.role == "build_runner"))
        .map(|n| n.name.clone())
        .unwrap_or_else(|| "none".to_string())
}

pub(crate) fn emit_build_runner_selection(
    preferred: Option<String>,
    allow_local_fallback: bool,
    registry: Option<String>,
) -> ExitCode {
    let runtime = resolve_runtime_paths(&RuntimeContext::from_env(registry));
    let (payload, should_fail) = match fleet_report::emit_fleet_report(
        &runtime.registry_path,
        None,
        None,
        None,
    ) {
        Ok((fleet_payload, _)) => {
            let declared: Vec<cynic_node::domain::fleet_truth::Node> =
                serde_json::from_value(json!(fleet_payload.get("declared").cloned().unwrap_or(
                    serde_json::Value::Null
                )))
                .unwrap_or_default();
            let effective: Vec<cynic_node::domain::fleet_truth::EffectiveNode> =
                serde_json::from_value(json!(fleet_payload.get("effective").cloned().unwrap_or(
                    serde_json::Value::Null
                )))
                .unwrap_or_default();

            let preferred_name = derive_preferred(preferred, &declared);
            let selection = fleet_truth::select_build_runner(
                &preferred_name,
                &declared,
                &effective,
                allow_local_fallback,
            );
            let report = build_selection_report(&selection, &declared, &effective);
            (report, selection.degraded && selection.node.is_none())
        }
        Err(error) => {
            let report = json!({
                "schema": "cynic.build_runner_selection.v1",
                "status": "blocked",
                "error": error,
            });
            (report, true)
        }
    };

    match serde_json::to_string_pretty(&payload) {
        Ok(text) => {
            println!("{text}");
            if should_fail {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        Err(error) => crate::fail(error.to_string()),
    }
}

fn build_selection_report(
    selection: &SelectedBuildRunner,
    declared: &[cynic_node::domain::fleet_truth::Node],
    effective: &[cynic_node::domain::fleet_truth::EffectiveNode],
) -> serde_json::Value {
    let healthy_runners: Vec<String> = declared
        .iter()
        .filter(|n| n.role == "build_runner")
        .filter(|n| {
            effective
                .iter()
                .find(|e| e.name == n.name)
                .map(|e| e.effective_status == "active")
                .unwrap_or(false)
        })
        .map(|n| n.name.clone())
        .collect();

    json!({
        "schema": "cynic.build_runner_selection.v1",
        "status": if selection.node.is_some() { "selected" } else { "blocked" },
        "selected": selection.node,
        "degraded": selection.degraded,
        "reason": selection.reason,
        "preferred": selection.preferred,
        "healthy_build_runners": healthy_runners,
        "allow_local_fallback": selection.node.is_none() && selection.reason == "degraded_local",
    })
}
