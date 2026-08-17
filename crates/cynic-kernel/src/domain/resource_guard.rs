//! Pure resource-safety policy for a CYNIC node.
//!
//! Observation belongs to probes and adapters. This module only turns a
//! normalized snapshot into a bounded decision that can be projected locally
//! or federated upward to the human operator.

use serde::{Deserialize, Serialize};

use super::probe::{EnvironmentSnapshot, ProbeDetails, ResourceDetails};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceState {
    Normal,
    Throttled,
    Degraded,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceAction {
    Continue,
    ThrottleOptionalWorkloads,
    BlockBuilds,
    PauseNonEssentialIngestion,
    PreserveEvidence,
    RequestHumanIntervention,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceObservation {
    pub node_id: String,
    pub observed_at_unix_ms: u128,
    pub disk_available_bytes: Option<u64>,
    pub disk_total_bytes: Option<u64>,
    pub memory_available_bytes: Option<u64>,
    pub memory_total_bytes: Option<u64>,
    /// PSI values are percentages in the Linux kernel's 0..100 scale.
    pub memory_pressure_some_avg10_micros: Option<u64>,
    pub io_pressure_some_avg10_micros: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourcePolicy {
    pub disk_warning_percent: u8,
    pub disk_critical_percent: u8,
    pub memory_warning_percent: u8,
    pub memory_critical_percent: u8,
    pub pressure_warning_micros: u64,
    pub pressure_critical_micros: u64,
}

impl Default for ResourcePolicy {
    fn default() -> Self {
        Self {
            disk_warning_percent: 15,
            disk_critical_percent: 5,
            memory_warning_percent: 85,
            memory_critical_percent: 95,
            // PSI reports percentage points; conversion uses 10_000 micros
            // per percentage point (10% = 100_000 micros).
            pressure_warning_micros: 100_000,
            pressure_critical_micros: 250_000,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceDecision {
    pub schema: String,
    pub node_id: String,
    pub state: ResourceState,
    pub actions: Vec<ResourceAction>,
    pub reasons: Vec<String>,
    pub residual_unknowns: Vec<String>,
    pub autonomous_mutation_allowed: bool,
}

/// Normalize the existing kernel probes into the guard's byte- and
/// pressure-based contract. Missing probes remain explicit unknowns.
pub fn observation_from_environment(
    node_id: impl Into<String>,
    snapshot: &EnvironmentSnapshot,
) -> ResourceObservation {
    let resource = snapshot.probes.iter().find_map(|probe| {
        if let ProbeDetails::Resource(details) = &probe.details {
            Some(details)
        } else {
            None
        }
    });
    let pressure = snapshot.probes.iter().find_map(|probe| {
        if let ProbeDetails::Pressure(details) = &probe.details {
            Some(details)
        } else {
            None
        }
    });

    ResourceObservation {
        node_id: node_id.into(),
        observed_at_unix_ms: snapshot.timestamp.max(0) as u128,
        disk_available_bytes: resource.and_then(|details| gb_to_bytes(details.disk_available_gb)),
        disk_total_bytes: resource.and_then(|details| gb_to_bytes(details.disk_total_gb)),
        memory_available_bytes: resource.and_then(memory_available_bytes),
        memory_total_bytes: resource.and_then(|details| gb_to_bytes(details.memory_total_gb)),
        memory_pressure_some_avg10_micros: pressure
            .and_then(|details| psi_to_micros(details.memory_some_avg10)),
        io_pressure_some_avg10_micros: pressure
            .and_then(|details| psi_to_micros(details.io_some_avg10)),
    }
}

pub fn evaluate_resources(
    observation: &ResourceObservation,
    policy: &ResourcePolicy,
) -> ResourceDecision {
    let mut reasons = Vec::new();
    let mut unknowns = Vec::new();
    let mut state = ResourceState::Normal;

    let disk_percent = availability_percent(
        observation.disk_available_bytes,
        observation.disk_total_bytes,
    );
    match disk_percent {
        Some(percent) if percent <= u64::from(policy.disk_critical_percent) => {
            state = ResourceState::Critical;
            reasons.push(format!("disk availability at {percent}%"));
        }
        Some(percent) if percent <= u64::from(policy.disk_warning_percent) => {
            state = max_state(state, ResourceState::Degraded);
            reasons.push(format!("disk availability at {percent}%"));
        }
        None => unknowns.push("disk capacity or availability is unknown".to_string()),
        _ => {}
    }

    let memory_used_percent = match (
        observation.memory_available_bytes,
        observation.memory_total_bytes,
    ) {
        (Some(available), Some(total)) if total > 0 && available <= total => {
            Some(100 - (available.saturating_mul(100) / total).min(100))
        }
        _ => None,
    };
    match memory_used_percent {
        Some(percent) if percent >= u64::from(policy.memory_critical_percent) => {
            state = max_state(state, ResourceState::Critical);
            reasons.push(format!("memory usage at {percent}%"));
        }
        Some(percent) if percent >= u64::from(policy.memory_warning_percent) => {
            state = max_state(state, ResourceState::Throttled);
            reasons.push(format!("memory usage at {percent}%"));
        }
        None => unknowns.push("memory capacity or availability is unknown".to_string()),
        _ => {}
    }

    for (name, value) in [
        ("memory", observation.memory_pressure_some_avg10_micros),
        ("io", observation.io_pressure_some_avg10_micros),
    ] {
        match value {
            Some(value) if value >= policy.pressure_critical_micros => {
                state = max_state(state, ResourceState::Critical);
                reasons.push(format!("{name} pressure is critical"));
            }
            Some(value) if value >= policy.pressure_warning_micros => {
                state = max_state(state, ResourceState::Throttled);
                reasons.push(format!("{name} pressure is elevated"));
            }
            None => unknowns.push(format!("{name} pressure is unknown")),
            _ => {}
        }
    }

    if state == ResourceState::Normal && !unknowns.is_empty() {
        state = ResourceState::Unknown;
    }

    let actions = match state {
        ResourceState::Normal => vec![ResourceAction::Continue],
        ResourceState::Throttled => vec![ResourceAction::ThrottleOptionalWorkloads],
        ResourceState::Degraded => vec![
            ResourceAction::ThrottleOptionalWorkloads,
            ResourceAction::PauseNonEssentialIngestion,
        ],
        ResourceState::Critical => vec![
            ResourceAction::BlockBuilds,
            ResourceAction::PauseNonEssentialIngestion,
            ResourceAction::PreserveEvidence,
            ResourceAction::RequestHumanIntervention,
        ],
        ResourceState::Unknown => vec![ResourceAction::RequestHumanIntervention],
    };

    ResourceDecision {
        schema: "cynic.resource_decision.v1".to_string(),
        node_id: observation.node_id.clone(),
        autonomous_mutation_allowed: false,
        state,
        actions,
        reasons,
        residual_unknowns: unknowns,
    }
}

fn availability_percent(available: Option<u64>, total: Option<u64>) -> Option<u64> {
    match (available, total) {
        (Some(available), Some(total)) if total > 0 && available <= total => {
            Some((available.saturating_mul(100) / total).min(100))
        }
        _ => None,
    }
}

fn memory_available_bytes(details: &ResourceDetails) -> Option<u64> {
    let total = gb_to_bytes(details.memory_total_gb)?;
    let used = gb_to_bytes(details.memory_used_gb)?;
    (used <= total).then_some(total - used)
}

fn gb_to_bytes(value: Option<f64>) -> Option<u64> {
    let value = value?;
    (value.is_finite() && value >= 0.0).then_some((value * 1024.0 * 1024.0 * 1024.0) as u64)
}

fn psi_to_micros(value: Option<f64>) -> Option<u64> {
    let value = value?;
    (value.is_finite() && value >= 0.0).then_some((value * 10_000.0) as u64)
}

fn max_state(left: ResourceState, right: ResourceState) -> ResourceState {
    fn rank(state: ResourceState) -> u8 {
        match state {
            ResourceState::Normal => 0,
            ResourceState::Throttled => 1,
            ResourceState::Degraded => 2,
            ResourceState::Critical => 3,
            ResourceState::Unknown => 4,
        }
    }
    if rank(right) > rank(left) {
        right
    } else {
        left
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observation() -> ResourceObservation {
        ResourceObservation {
            node_id: "node-1".to_string(),
            observed_at_unix_ms: 1,
            disk_available_bytes: Some(100),
            disk_total_bytes: Some(1_000),
            memory_available_bytes: Some(800),
            memory_total_bytes: Some(1_000),
            memory_pressure_some_avg10_micros: Some(0),
            io_pressure_some_avg10_micros: Some(0),
        }
    }

    #[test]
    fn full_disk_blocks_mutating_workloads() {
        let mut current = observation();
        current.disk_available_bytes = Some(0);
        let decision = evaluate_resources(&current, &ResourcePolicy::default());
        assert_eq!(decision.state, ResourceState::Critical);
        assert!(decision.actions.contains(&ResourceAction::BlockBuilds));
        assert!(!decision.autonomous_mutation_allowed);
    }

    #[test]
    fn unknown_observation_does_not_look_healthy() {
        let current = ResourceObservation {
            node_id: "node-1".to_string(),
            observed_at_unix_ms: 1,
            disk_available_bytes: None,
            disk_total_bytes: None,
            memory_available_bytes: None,
            memory_total_bytes: None,
            memory_pressure_some_avg10_micros: None,
            io_pressure_some_avg10_micros: None,
        };
        let decision = evaluate_resources(&current, &ResourcePolicy::default());
        assert_eq!(decision.state, ResourceState::Unknown);
        assert!(decision.residual_unknowns.len() >= 4);
    }

    #[test]
    fn environment_snapshot_is_normalized_for_the_guard() {
        let snapshot = EnvironmentSnapshot {
            timestamp: 42,
            overall: super::super::probe::ProbeStatus::Ok,
            probes: vec![
                super::super::probe::ProbeResult {
                    name: "resource".to_string(),
                    status: super::super::probe::ProbeStatus::Ok,
                    details: ProbeDetails::Resource(ResourceDetails {
                        cpu_usage_percent: Some(10.0),
                        memory_used_gb: Some(8.0),
                        memory_total_gb: Some(16.0),
                        disk_available_gb: Some(1.0),
                        disk_total_gb: Some(10.0),
                        load_average_1m: Some(1.0),
                        uptime_seconds: Some(1),
                    }),
                    duration_ms: 1,
                    timestamp: 42,
                },
                super::super::probe::ProbeResult {
                    name: "pressure".to_string(),
                    status: super::super::probe::ProbeStatus::Ok,
                    details: ProbeDetails::Pressure(crate::domain::probe::PressureDetails {
                        cpu_some_avg10: None,
                        cpu_some_avg60: None,
                        memory_some_avg10: Some(2.0),
                        memory_full_avg10: None,
                        io_some_avg10: Some(3.0),
                        io_full_avg10: None,
                    }),
                    duration_ms: 1,
                    timestamp: 42,
                },
            ],
        };
        let observation = observation_from_environment("node-1", &snapshot);
        assert_eq!(observation.observed_at_unix_ms, 42);
        assert_eq!(observation.disk_available_bytes, Some(1_073_741_824));
        assert_eq!(observation.memory_available_bytes, Some(8_589_934_592));
        assert_eq!(observation.memory_pressure_some_avg10_micros, Some(20_000));
    }
}
