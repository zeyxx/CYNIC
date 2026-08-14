use serde::Serialize;

pub const SERVICE_SUPERVISOR_OBSERVATION_SCHEMA: &str = "cynic.service_supervisor_observation.v1";
pub const SERVICE_SUPERVISOR_OBSERVATION_EVIDENCE_ID: &str = "service_supervisor_observation";

#[derive(Debug, Clone, Serialize)]
pub struct ServiceSupervisorObservation {
    pub schema: &'static str,
    pub evidence_id: &'static str,
    pub status: &'static str,
    pub subject: String,
    pub supervisor_kind: String,
    pub supervisor_scope: String,
    pub target_source: String,
    pub observed_at_unix_ms: u128,
    pub service_count: usize,
    pub active_count: usize,
    pub degraded_count: usize,
    pub unknown_count: usize,
    pub services: Vec<ServiceSupervisorUnitObservation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceSupervisorUnitObservation {
    pub service_id: String,
    pub unit_name: String,
    pub health_port: Option<u16>,
    pub health_kind: Option<String>,
    pub desired_provider: Option<String>,
    pub selected_provider: String,
    pub selected_scope: String,
    pub target_source: String,
    pub desired_state: String,
    pub restart_policy: String,
    pub probe_status: &'static str,
    pub probe_detail: String,
    pub provider_attempts: Vec<ServiceSupervisorProviderAttempt>,
    pub load_state: String,
    pub unit_file_state: String,
    pub active_state: String,
    pub sub_state: String,
    pub restart_count: Option<u64>,
    pub active_enter_timestamp_monotonic: Option<u64>,
    pub status: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct ServiceSupervisorProviderAttempt {
    pub provider: String,
    pub scope: String,
    pub probe_status: &'static str,
    pub probe_detail: String,
}

pub fn service_supervisor_status(
    units: &[ServiceSupervisorUnitObservation],
) -> (&'static str, usize, usize, usize) {
    let active_count = units.iter().filter(|unit| unit.status == "active").count();
    let degraded_count = units
        .iter()
        .filter(|unit| unit.status == "degraded")
        .count();
    let unknown_count = units.iter().filter(|unit| unit.status == "unknown").count();
    let status = if units.is_empty() || unknown_count > 0 {
        "unknown"
    } else if degraded_count > 0 {
        "degraded"
    } else {
        "active"
    };
    (status, active_count, degraded_count, unknown_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supervisor_status_does_not_hide_unknown_units() {
        let units = vec![ServiceSupervisorUnitObservation {
            service_id: "kernel".to_string(),
            unit_name: "cynic-kernel.service".to_string(),
            health_port: Some(3030),
            health_kind: Some("tcp".to_string()),
            desired_provider: Some("process-port".to_string()),
            selected_provider: "systemd".to_string(),
            selected_scope: "system".to_string(),
            target_source: "test".to_string(),
            desired_state: "active".to_string(),
            restart_policy: "on-failure".to_string(),
            probe_status: "ok",
            probe_detail: "test".to_string(),
            provider_attempts: vec![ServiceSupervisorProviderAttempt {
                provider: "systemd".to_string(),
                scope: "system".to_string(),
                probe_status: "ok",
                probe_detail: "test".to_string(),
            }],
            load_state: "unknown".to_string(),
            unit_file_state: "unknown".to_string(),
            active_state: "unknown".to_string(),
            sub_state: "unknown".to_string(),
            restart_count: None,
            active_enter_timestamp_monotonic: None,
            status: "unknown",
        }];
        let (status, active, degraded, unknown) = service_supervisor_status(&units);
        assert_eq!(status, "unknown");
        assert_eq!(active, 0);
        assert_eq!(degraded, 0);
        assert_eq!(unknown, 1);
    }
}
