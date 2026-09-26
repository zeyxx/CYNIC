use cynic_eval::convergence::ConvergenceStatus;
use cynic_protocol::{ServiceSupervisorObservation, ServiceSupervisorUnitObservation};

use super::types::{
    PROCESS_CONVERGENCE_DECISION_SCHEMA, ProcessConvergenceDecision, ProcessConvergenceRequest,
};

pub fn decide_process_convergence(
    request: &ProcessConvergenceRequest,
    observed: &ServiceSupervisorObservation,
    read_back: Option<&ServiceSupervisorObservation>,
) -> Result<ProcessConvergenceDecision, String> {
    let desired = request.desired_unit()?;
    let mut blockers = Vec::new();
    let mut residual_unknowns = Vec::new();

    if request.epoch_id.trim().is_empty() {
        blockers.push("process_convergence_epoch_id_missing".to_string());
    }
    if request.subject_id.trim().is_empty() {
        blockers.push("process_convergence_subject_missing".to_string());
    }
    if desired.source.trim().is_empty() {
        residual_unknowns.push("process_desired_state_source_unknown".to_string());
    }
    if observed.subject != request.subject_id {
        blockers.push("process_observation_subject_mismatch".to_string());
    }

    let observed_unit = find_unit(observed, &request.service_id);
    classify_unit(
        "observation",
        observed_unit,
        &mut blockers,
        &mut residual_unknowns,
    );

    let read_back_unit = read_back.and_then(|report| {
        if report.subject != request.subject_id {
            blockers.push("process_read_back_subject_mismatch".to_string());
        }
        find_unit(report, &request.service_id)
    });
    if read_back.is_none() {
        residual_unknowns.push("process_read_back_missing".to_string());
    } else {
        classify_unit(
            "read_back",
            read_back_unit,
            &mut blockers,
            &mut residual_unknowns,
        );
    }

    if let (Some(before), Some(after)) = (observed_unit, read_back_unit)
        && before.status != after.status
    {
        residual_unknowns.push("process_observation_unstable".to_string());
    }

    blockers.sort();
    blockers.dedup();
    residual_unknowns.sort();
    residual_unknowns.dedup();
    let requested_status = if !blockers.is_empty() {
        ConvergenceStatus::Blocked
    } else if !residual_unknowns.is_empty() {
        ConvergenceStatus::Unknown
    } else if observed_unit.is_some_and(|unit| unit.status == "active")
        && read_back_unit.is_some_and(|unit| unit.status == "active")
    {
        ConvergenceStatus::Complete
    } else {
        ConvergenceStatus::Unknown
    };

    Ok(ProcessConvergenceDecision {
        schema: PROCESS_CONVERGENCE_DECISION_SCHEMA.to_string(),
        subject_id: request.subject_id.clone(),
        vertical_id: "process".to_string(),
        service_id: request.service_id.clone(),
        desired_state: if desired.enable_on_boot {
            "enabled_and_active"
        } else {
            "active"
        }
        .to_string(),
        observed_status: observed_unit.map(|unit| unit.status.to_string()),
        read_back_status: read_back_unit.map(|unit| unit.status.to_string()),
        requested_status,
        residual_unknowns,
        blockers,
    })
}

fn find_unit<'a>(
    report: &'a ServiceSupervisorObservation,
    service_id: &str,
) -> Option<&'a ServiceSupervisorUnitObservation> {
    report
        .services
        .iter()
        .find(|unit| unit.service_id == service_id || unit.unit_name == service_id)
}

fn classify_unit(
    phase: &str,
    unit: Option<&ServiceSupervisorUnitObservation>,
    blockers: &mut Vec<String>,
    residual_unknowns: &mut Vec<String>,
) {
    match unit.map(|unit| unit.status) {
        Some("active") => {}
        Some("degraded") => blockers.push(format!("process_{phase}_degraded")),
        Some(_) => residual_unknowns.push(format!("process_{phase}_unknown")),
        None => residual_unknowns.push(format!("process_{phase}_service_missing")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::services::{ServiceManagerKind, ServicePolicy, ServiceUnitPolicy};
    use cynic_protocol::{
        SERVICE_SUPERVISOR_OBSERVATION_EVIDENCE_ID, SERVICE_SUPERVISOR_OBSERVATION_SCHEMA,
        ServiceSupervisorUnitObservation,
    };

    fn request() -> ProcessConvergenceRequest {
        ProcessConvergenceRequest {
            epoch_id: "epoch-1".to_string(),
            subject_id: "local-kernel/process".to_string(),
            service_id: "cynic-kernel".to_string(),
            policy: ServicePolicy {
                manager: ServiceManagerKind::Systemd,
                unit_dir: "/etc/systemd/system".to_string(),
                units: vec![ServiceUnitPolicy {
                    service_id: "cynic-kernel".to_string(),
                    unit_name: "cynic-kernel.service".to_string(),
                    enable_on_boot: true,
                    restart_mode: "on-failure".to_string(),
                    source: "registry:infra/registry.json".to_string(),
                    health_port: Some(3030),
                    process_name: Some("cynic-kernel".to_string()),
                    supervisor_provider: Some("process-port".to_string()),
                    health_kind: Some("tcp".to_string()),
                }],
            },
        }
    }

    fn observation(
        status: &'static str,
        observed_at_unix_ms: u128,
    ) -> ServiceSupervisorObservation {
        ServiceSupervisorObservation {
            schema: SERVICE_SUPERVISOR_OBSERVATION_SCHEMA,
            evidence_id: SERVICE_SUPERVISOR_OBSERVATION_EVIDENCE_ID,
            status,
            subject: "local-kernel/process".to_string(),
            supervisor_kind: "process_port".to_string(),
            supervisor_scope: "local".to_string(),
            target_source: "registry:infra/registry.json".to_string(),
            observed_at_unix_ms,
            service_count: 1,
            active_count: usize::from(status == "active"),
            degraded_count: usize::from(status == "degraded"),
            unknown_count: usize::from(status == "unknown"),
            services: vec![ServiceSupervisorUnitObservation {
                service_id: "cynic-kernel".to_string(),
                unit_name: "cynic-kernel.service".to_string(),
                health_port: Some(3030),
                health_kind: Some("tcp".to_string()),
                desired_provider: Some("process-port".to_string()),
                selected_provider: "process_port".to_string(),
                selected_scope: "local".to_string(),
                target_source: "registry:infra/registry.json".to_string(),
                desired_state: "enabled_and_active".to_string(),
                restart_policy: "on-failure".to_string(),
                probe_status: "ok",
                probe_detail: "tcp probe".to_string(),
                provider_attempts: Vec::new(),
                load_state: "unknown".to_string(),
                unit_file_state: "unknown".to_string(),
                active_state: "process_running".to_string(),
                sub_state: "unknown".to_string(),
                restart_count: None,
                active_enter_timestamp_monotonic: None,
                status,
            }],
        }
    }

    #[test]
    fn two_active_observations_request_complete_convergence() {
        let before = observation("active", 1);
        let after = observation("active", 2);
        let decision = decide_process_convergence(&request(), &before, Some(&after)).unwrap();

        assert_eq!(decision.requested_status, ConvergenceStatus::Complete);
        assert!(decision.blockers.is_empty());
        assert!(decision.residual_unknowns.is_empty());
    }

    #[test]
    fn missing_read_back_remains_unknown() {
        let before = observation("active", 1);
        let decision = decide_process_convergence(&request(), &before, None).unwrap();

        assert_eq!(decision.requested_status, ConvergenceStatus::Unknown);
        assert_eq!(
            decision.residual_unknowns,
            vec!["process_read_back_missing".to_string()]
        );
    }
}
