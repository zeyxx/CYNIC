//! Explicit host-side activation adapter for a proved standby takeover.
//!
//! The adapter is systemd-specific and disposable. The request/observation
//! contract remains in `cynic-protocol`; replacing systemd only replaces this
//! module. Every mutation is argv-based and followed by service, health, and
//! identity read-back.

use serde::Serialize;

use crate::update_lifecycle::ArgvRunner;

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeTakeoverActivationPlan {
    pub request: cynic_protocol::RuntimeTakeoverActivationRequest,
    pub stop_standby_argv: Vec<String>,
    pub start_active_argv: Vec<String>,
    pub status_active_argv: Vec<String>,
    pub health_argv: Vec<String>,
    pub identity_argv: Vec<String>,
    pub rollback_start_standby_argv: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeTakeoverActivationReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub plan: RuntimeTakeoverActivationPlan,
    pub observation: Option<cynic_protocol::RuntimeTakeoverActivationObservation>,
    pub decision: Option<cynic_protocol::RuntimeTakeoverActivationDecision>,
    pub blockers: Vec<String>,
}

pub fn plan_runtime_takeover_activation(
    request: cynic_protocol::RuntimeTakeoverActivationRequest,
    admission: &cynic_protocol::RuntimeTakeoverDecision,
) -> Result<RuntimeTakeoverActivationPlan, String> {
    if request.schema != cynic_protocol::RUNTIME_TAKEOVER_ACTIVATION_SCHEMA {
        return Err("takeover activation request schema is invalid".to_string());
    }
    let fields = [
        ("node_id", request.node_id.as_str()),
        ("admission_proof_ref", request.admission_proof_ref.as_str()),
        ("standby_unit", request.standby_unit.as_str()),
        ("active_unit", request.active_unit.as_str()),
        ("health_endpoint", request.health_endpoint.as_str()),
        ("identity_endpoint", request.identity_endpoint.as_str()),
    ];
    if let Some((name, _)) = fields.iter().find(|(_, value)| value.trim().is_empty()) {
        return Err(format!("takeover activation field is empty: {name}"));
    }
    if request.fencing_token == 0 || request.fencing_token <= request.prior_fencing_token {
        return Err("takeover activation fencing is not monotonic".to_string());
    }
    if !admission.promote || admission.fencing_token != request.fencing_token {
        return Err("takeover activation lacks matching promotable admission".to_string());
    }
    Ok(RuntimeTakeoverActivationPlan {
        stop_standby_argv: vec![
            "systemctl".to_string(),
            "stop".to_string(),
            request.standby_unit.clone(),
        ],
        start_active_argv: vec![
            "systemctl".to_string(),
            "start".to_string(),
            request.active_unit.clone(),
        ],
        status_active_argv: vec![
            "systemctl".to_string(),
            "is-active".to_string(),
            request.active_unit.clone(),
        ],
        health_argv: vec![
            "curl".to_string(),
            "--fail".to_string(),
            "--silent".to_string(),
            "--show-error".to_string(),
            "--max-time".to_string(),
            "10".to_string(),
            request.health_endpoint.clone(),
        ],
        identity_argv: vec![
            "curl".to_string(),
            "--fail".to_string(),
            "--silent".to_string(),
            "--show-error".to_string(),
            "--max-time".to_string(),
            "10".to_string(),
            request.identity_endpoint.clone(),
        ],
        rollback_start_standby_argv: vec![
            "systemctl".to_string(),
            "start".to_string(),
            request.standby_unit.clone(),
        ],
        request,
    })
}

pub fn apply_runtime_takeover_activation(
    plan: &RuntimeTakeoverActivationPlan,
    runner: &dyn ArgvRunner,
) -> RuntimeTakeoverActivationReport {
    let mut blockers = Vec::new();
    if !run(runner, &plan.stop_standby_argv) {
        blockers.push("standby_stop_failed".to_string());
    } else if !run(runner, &plan.start_active_argv) {
        blockers.push("active_start_failed".to_string());
        let _ = run(runner, &plan.rollback_start_standby_argv);
    }
    let status_output = runner.run(&argv_refs(&plan.status_active_argv));
    let health_output = runner.run(&argv_refs(&plan.health_argv));
    let identity_output = runner.run(&argv_refs(&plan.identity_argv));
    let process_active = status_output.success() && status_output.stdout.trim() == "active";
    let health = serde_json::from_str::<serde_json::Value>(health_output.stdout.trim()).ok();
    let identity = serde_json::from_str::<serde_json::Value>(identity_output.stdout.trim()).ok();
    let observed_fencing_token = health
        .as_ref()
        .and_then(|value| value.pointer("/runtime_takeover/decision/fencing_token"))
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0);
    let identity_verified = identity
        .as_ref()
        .and_then(|value| value.get("node_id"))
        .and_then(serde_json::Value::as_str)
        == Some(plan.request.node_id.as_str());
    if !status_output.success() || !process_active {
        blockers.push("active_service_read_back_failed".to_string());
    }
    if !health_output.success() {
        blockers.push("active_health_read_back_failed".to_string());
    }
    if !identity_output.success() || !identity_verified {
        blockers.push("active_identity_read_back_failed".to_string());
    }
    let observation = cynic_protocol::RuntimeTakeoverActivationObservation {
        schema: cynic_protocol::RUNTIME_TAKEOVER_ACTIVATION_SCHEMA.to_string(),
        node_id: identity
            .as_ref()
            .and_then(|value| value.get("node_id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        active_unit: plan.request.active_unit.clone(),
        process_active,
        observed_fencing_token,
        identity_verified,
        proof_refs: vec![
            "systemd://is-active".to_string(),
            plan.request.health_endpoint.clone(),
            plan.request.identity_endpoint.clone(),
        ],
    };
    let decision = cynic_protocol::reconcile_takeover_activation(&plan.request, &observation);
    blockers.extend(decision.blockers.clone());
    blockers.sort();
    blockers.dedup();
    RuntimeTakeoverActivationReport {
        schema: "cynic.host.runtime_takeover_activation.v1",
        status: if blockers.is_empty() {
            "activated_proved"
        } else {
            "blocked"
        },
        plan: plan.clone(),
        observation: Some(observation),
        decision: Some(decision),
        blockers,
    }
}

fn run(runner: &dyn ArgvRunner, argv: &[String]) -> bool {
    runner.run(&argv_refs(argv)).success()
}

fn argv_refs(argv: &[String]) -> Vec<&str> {
    argv.iter().map(String::as_str).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> cynic_protocol::RuntimeTakeoverActivationRequest {
        cynic_protocol::RuntimeTakeoverActivationRequest {
            schema: cynic_protocol::RUNTIME_TAKEOVER_ACTIVATION_SCHEMA.to_string(),
            node_id: "node-a".to_string(),
            fencing_token: 4,
            prior_fencing_token: 3,
            admission_proof_ref: "spool://proof".to_string(),
            standby_unit: "standby.service".to_string(),
            active_unit: "active.service".to_string(),
            health_endpoint: "http://127.0.0.1/health".to_string(),
            identity_endpoint: "http://127.0.0.1/identity".to_string(),
        }
    }

    #[test]
    fn planning_rejects_non_monotonic_fencing() {
        let mut request = request();
        request.fencing_token = 2;
        request.prior_fencing_token = 2;
        let admission = cynic_protocol::RuntimeTakeoverDecision {
            schema: cynic_protocol::RUNTIME_TAKEOVER_SCHEMA,
            status: "blocked",
            promote: false,
            mode: cynic_protocol::StandbyMode::Resident,
            fencing_token: 2,
            retained_capabilities: Vec::new(),
            blocked_mutations: Vec::new(),
            blockers: vec!["test"],
        };
        assert!(plan_runtime_takeover_activation(request, &admission).is_err());
    }

    #[test]
    fn activation_requires_all_three_read_backs() {
        let request = request();
        let plan = plan_runtime_takeover_activation(
            request,
            &cynic_protocol::RuntimeTakeoverDecision {
                schema: cynic_protocol::RUNTIME_TAKEOVER_SCHEMA,
                status: "promotable",
                promote: true,
                mode: cynic_protocol::StandbyMode::Active,
                fencing_token: 4,
                retained_capabilities: vec!["health"],
                blocked_mutations: Vec::new(),
                blockers: Vec::new(),
            },
        )
        .expect("plan");
        let runner = crate::update_lifecycle::MapArgvRunner {
            responses: vec![
                (plan.stop_standby_argv.clone(), success_output("")),
                (plan.start_active_argv.clone(), success_output("")),
                (plan.status_active_argv.clone(), success_output("active\n")),
                (
                    plan.health_argv.clone(),
                    success_output(r#"{"runtime_takeover":{"decision":{"fencing_token":4}}}"#),
                ),
                (
                    plan.identity_argv.clone(),
                    success_output(r#"{"node_id":"node-a"}"#),
                ),
            ],
        };
        let report = apply_runtime_takeover_activation(&plan, &runner);
        assert_eq!(report.status, "activated_proved");
    }

    fn success_output(stdout: &str) -> crate::update_lifecycle::ArgvOutput {
        crate::update_lifecycle::ArgvOutput {
            exit_code: Some(0),
            stdout: stdout.to_string(),
            stderr: String::new(),
            spawn_error: None,
        }
    }
}
