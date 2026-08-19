//! Read-only operator diagnosis assembled from owner-level observations.

#![allow(dead_code)]
use std::process::ExitCode;

use serde::Serialize;

use crate::emit_json;
use crate::operator_status::{OperatorStatusReport, build_operator_status};

#[derive(Serialize)]
struct DoctorReport {
    schema: &'static str,
    status: &'static str,
    mutates: bool,
    checks: Vec<DoctorCheck>,
    inference: OperatorStatusReport,
    resources: cynic_host::inference_observation::InferenceResourceObservation,
    next_command: Option<String>,
}

#[derive(Serialize)]
struct DoctorCheck {
    id: &'static str,
    status: &'static str,
    detail: String,
    command: Option<String>,
}

pub(crate) fn execute_operator_doctor(json: bool) -> ExitCode {
    let inference = build_operator_status();
    let network = cynic_host::runtime_network_preflight::observe_runtime_network_preflight(&[]);
    let repo_access = cynic_host::repo_access::current_repo_access_status();
    let resources = cynic_host::inference_observation::observe_inference_resources();

    let inference_check = DoctorCheck {
        id: "local_llm",
        status: doctor_status_from_inference(inference.status()),
        detail: format!("local model supervision is {}", inference.status()),
        command: inference.next_command().map(str::to_owned),
    };
    let network_check = DoctorCheck {
        id: "network",
        status: doctor_status_from_network(network.status),
        detail: format!(
            "resolver has {} nameserver(s); {} of {} probe(s) are reachable",
            network.resolver.nameservers.len(),
            network
                .probes
                .iter()
                .filter(|probe| probe.status == "reachable")
                .count(),
            network.probes.len()
        ),
        command: network.next_command,
    };
    let forgejo = repo_access
        .sovereign_remotes
        .iter()
        .find(|remote| remote.id == "forgejo");
    let repo_check = match forgejo {
        Some(remote) => DoctorCheck {
            id: "sovereign_git",
            status: if remote.service_health_status == "reachable" {
                "ready"
            } else {
                "attention"
            },
            detail: format!(
                "Forgejo service={}, dns={}, tcp={}, repository_auth={}",
                remote.service_health_status,
                remote.host_resolution_status,
                remote.tcp_reachability_status,
                remote.repo_auth_readiness
            ),
            command: (remote.service_health_status != "reachable")
                .then(|| "cynic runtime repo-access --json".to_string()),
        },
        None => DoctorCheck {
            id: "sovereign_git",
            status: "unknown",
            detail: "no Forgejo sovereign remote is observable".to_string(),
            command: Some("cynic runtime repo-access --json".to_string()),
        },
    };
    let checks = vec![inference_check, network_check, repo_check];
    let status = aggregate_status(&checks);
    let next_command = checks
        .iter()
        .find(|check| check.status != "ready")
        .and_then(|check| check.command.clone());
    let report = DoctorReport {
        schema: "cynic.operator.doctor.v1",
        status,
        mutates: false,
        checks,
        inference,
        resources,
        next_command,
    };

    if json {
        return emit_json(&report, false);
    }

    println!("CYNIC doctor: {} (read-only)", report.status);
    for check in &report.checks {
        println!("  {:<14} {:<10} {}", check.id, check.status, check.detail);
    }
    if let Some(command) = report.next_command {
        println!("  Recommended next command: {command}");
    }
    ExitCode::SUCCESS
}

fn doctor_status_from_inference(status: &str) -> &'static str {
    match status {
        "active" => "ready",
        "inactive" => "attention",
        _ => "unknown",
    }
}

fn doctor_status_from_network(status: &str) -> &'static str {
    match status {
        "ready" => "ready",
        "blocked" => "blocked",
        _ => "attention",
    }
}

fn aggregate_status(checks: &[DoctorCheck]) -> &'static str {
    if checks.iter().any(|check| check.status == "blocked") {
        "blocked"
    } else if checks.iter().any(|check| check.status == "unknown") {
        "unknown"
    } else if checks.iter().any(|check| check.status == "attention") {
        "attention"
    } else {
        "ready"
    }
}

#[cfg(test)]
mod tests {
    use super::{DoctorCheck, aggregate_status};

    #[test]
    fn blocked_check_has_priority_over_attention() {
        let checks = vec![
            DoctorCheck {
                id: "one",
                status: "attention",
                detail: String::new(),
                command: None,
            },
            DoctorCheck {
                id: "two",
                status: "blocked",
                detail: String::new(),
                command: None,
            },
        ];
        assert_eq!(aggregate_status(&checks), "blocked");
    }
}
