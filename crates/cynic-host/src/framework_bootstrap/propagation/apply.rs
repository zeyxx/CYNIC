use cynic_protocol::BUILD_RUNNER_OBSERVATION_SCHEMA;

use crate::update_lifecycle::ArgvRunner;

use super::types::{
    FrameworkPropagationExecution, FrameworkPropagationPlan, FrameworkPropagationReport,
};

pub fn apply_framework_propagation(
    plan: &FrameworkPropagationPlan,
    yes: bool,
    runner: &dyn ArgvRunner,
) -> FrameworkPropagationReport {
    if plan.status == "converged" {
        return report(plan, "converged", false, yes, Vec::new(), None, Vec::new());
    }

    let mut blockers = plan.blockers.clone();
    if plan.status != "planned_enrollment" {
        blockers.push("propagation_plan_not_admissible".to_string());
    }
    if !yes {
        blockers.push("explicit_yes_required".to_string());
    }
    if !blockers.is_empty() {
        blockers.sort();
        blockers.dedup();
        return report(plan, "blocked", false, yes, Vec::new(), None, blockers);
    }

    let mut executions = Vec::new();
    for operation in &plan.operations {
        let argv = operation.argv.clone();
        let output = runner.run(&argv.iter().map(String::as_str).collect::<Vec<_>>());
        let command_succeeded = output.success();
        let digest_matches = operation
            .expected_stdout_sha256
            .as_ref()
            .is_none_or(|expected| {
                output
                    .stdout
                    .split_whitespace()
                    .next()
                    .is_some_and(|observed| expected == &format!("sha256:{observed}"))
            });
        let principal_matches =
            operation.phase != "preflight" || output.stdout.trim() == plan.enrollment_principal;
        let success = command_succeeded && digest_matches && principal_matches;
        executions.push(FrameworkPropagationExecution {
            phase: operation.phase.to_string(),
            argv,
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            spawn_error: output.spawn_error,
            proof: success
                .then(|| operation.proof.map(str::to_string))
                .flatten(),
        });
        if !success {
            blockers.push(if command_succeeded && !principal_matches {
                "machine_enrollment_principal_changed_before_apply".to_string()
            } else if command_succeeded {
                "staged_seed_manifest_digest_mismatch".to_string()
            } else {
                format!("propagation_operation_failed:{}", operation.phase)
            });
            return report(plan, "blocked", true, yes, executions, None, blockers);
        }
    }

    let marker = read_back_json(&executions, "runtime_marker_read_back");
    let node = read_back_json(&executions, "node_observation_read_back");
    let marker_valid = marker.as_ref().is_some_and(|marker| {
        marker["schema"].as_str() == Some("cynic.runtime_seed.release_marker.v3")
            && marker["release_epoch"]
                .as_u64()
                .is_some_and(|epoch| epoch > 0)
            && marker["basket_digest"].as_str()
                == Some(plan.expected_runtime_basket_digest.as_str())
    });
    let node_valid = node.as_ref().is_some_and(|node| {
        node["schema"].as_str() == Some(BUILD_RUNNER_OBSERVATION_SCHEMA)
            && node["status"].as_str() == Some("observed")
            && node["node_id"].as_str() == Some(plan.node_id.as_str())
    });
    if marker_valid
        && let Some(execution) = executions
            .iter_mut()
            .find(|execution| execution.phase == "runtime_marker_read_back")
    {
        execution.proof = Some("runtime_seed_release_marker_read_back".to_string());
    }
    if node_valid
        && let Some(execution) = executions
            .iter_mut()
            .find(|execution| execution.phase == "node_observation_read_back")
    {
        execution.proof = Some("node_registry_observation_read_back".to_string());
    }
    if !marker_valid {
        blockers.push("runtime_seed_release_marker_read_back_invalid".to_string());
    }
    if !node_valid {
        blockers.push("node_registry_observation_read_back_invalid".to_string());
    }
    let read_back = Some(serde_json::json!({
        "runtime_marker": marker,
        "node_observation": node,
    }));
    report(
        plan,
        if blockers.is_empty() {
            "converged"
        } else {
            "blocked"
        },
        true,
        yes,
        executions,
        read_back,
        blockers,
    )
}

fn read_back_json(
    executions: &[FrameworkPropagationExecution],
    phase: &str,
) -> Option<serde_json::Value> {
    executions
        .iter()
        .find(|execution| execution.phase == phase)
        .and_then(|execution| serde_json::from_str(&execution.stdout).ok())
}

fn report(
    plan: &FrameworkPropagationPlan,
    status: &'static str,
    mutates: bool,
    requested_apply: bool,
    executions: Vec<FrameworkPropagationExecution>,
    read_back_evidence: Option<serde_json::Value>,
    blockers: Vec<String>,
) -> FrameworkPropagationReport {
    FrameworkPropagationReport {
        schema: "cynic.framework_bootstrap.propagation_report.v2",
        status,
        mutates,
        requested_apply,
        node_id: plan.node_id.clone(),
        role: plan.role.clone(),
        seed_id: plan.seed_id.clone(),
        decision: plan.decision.status,
        executions,
        read_back_evidence,
        blockers,
        rollback_intent: plan.rollback_intent,
    }
}
