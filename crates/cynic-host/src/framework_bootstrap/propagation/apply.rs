use crate::update_lifecycle::ArgvRunner;

use super::types::{
    FrameworkPropagationExecution, FrameworkPropagationPlan, FrameworkPropagationReport,
};

pub fn apply_framework_propagation(
    plan: &FrameworkPropagationPlan,
    yes: bool,
    runner: &dyn ArgvRunner,
) -> FrameworkPropagationReport {
    let mut blockers = plan.blockers.clone();
    if plan.status != "planned" {
        blockers.push("propagation_plan_not_admissible".to_string());
    }
    if !yes {
        blockers.push("explicit_yes_required".to_string());
    }
    if !blockers.is_empty() {
        blockers.sort();
        blockers.dedup();
        return report(plan, false, yes, Vec::new(), None, blockers);
    }

    let mut executions = Vec::new();
    for operation in &plan.operations {
        let argv = operation
            .argv
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let output = runner.run(&argv);
        let success = output.success();
        executions.push(FrameworkPropagationExecution {
            phase: operation.phase.to_string(),
            argv: operation.argv.clone(),
            exit_code: output.exit_code,
            stdout: output.stdout,
            stderr: output.stderr,
            spawn_error: output.spawn_error,
            proof: success
                .then(|| operation.proof.map(str::to_string))
                .flatten(),
        });
        if !success {
            blockers.push(format!("propagation_operation_failed:{}", operation.phase));
            return report(plan, true, yes, executions, None, blockers);
        }
    }

    let read_back = executions
        .iter()
        .find(|execution| execution.phase == "evidence_read_back")
        .and_then(|execution| serde_json::from_str::<serde_json::Value>(&execution.stdout).ok());
    let read_back_valid = read_back.as_ref().is_some_and(|evidence| {
        evidence["schema"].as_str() == Some("cynic.runtime.node_observation.v1")
            && evidence["status"].as_str() == Some("observed")
            && evidence["node_id"].as_str() == Some(plan.node_id.as_str())
    });
    if read_back_valid {
        if let Some(execution) = executions
            .iter_mut()
            .find(|execution| execution.phase == "evidence_read_back")
        {
            execution.proof = Some("node_registry_observation_read_back".to_string());
        }
    } else {
        blockers.push("node_registry_observation_read_back_invalid".to_string());
    }
    report(plan, true, yes, executions, read_back, blockers)
}

fn report(
    plan: &FrameworkPropagationPlan,
    mutates: bool,
    requested_apply: bool,
    executions: Vec<FrameworkPropagationExecution>,
    read_back_evidence: Option<serde_json::Value>,
    blockers: Vec<String>,
) -> FrameworkPropagationReport {
    FrameworkPropagationReport {
        schema: "cynic.framework_bootstrap.propagation_report.v1",
        status: if blockers.is_empty() {
            "applied"
        } else {
            "blocked"
        },
        mutates,
        requested_apply,
        node_id: plan.node_id.clone(),
        role: plan.role.clone(),
        seed_id: plan.seed_id.clone(),
        executions,
        read_back_evidence,
        blockers,
        rollback_intent: plan.rollback_intent,
    }
}
