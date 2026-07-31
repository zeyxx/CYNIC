use serde::{Deserialize, Serialize};

pub const STORAGE_RECONCILE_SCHEMA: &str = "cynic.storage_reconcile_report.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageDesiredState {
    pub service: String,
    pub health_url: String,
    pub desired_running: bool,
    pub warmup_seconds: u64,
    pub restart_window_seconds: u64,
    pub max_restarts_per_window: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageObservation {
    pub observed_at_unix_ms: u64,
    pub service_active: bool,
    pub service_state: String,
    pub health_ok: bool,
    pub health_status: Option<u16>,
    pub observation_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestartBudget {
    pub window_started_unix_ms: u64,
    pub restart_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum StorageDecision {
    Healthy,
    ObserveOnly {
        reason: String,
    },
    Restart {
        service: String,
        stop_condition: String,
    },
    Blocked {
        reason: String,
    },
}

pub fn reconcile_storage(
    desired: &StorageDesiredState,
    observed: &StorageObservation,
    budget: &RestartBudget,
    now_unix_ms: u64,
) -> StorageDecision {
    if desired.service != "surrealdb.service" {
        return StorageDecision::Blocked {
            reason: "service_outside_storage_lifecycle_authority".to_string(),
        };
    }
    if desired.health_url != "http://127.0.0.1:8000/health" {
        return StorageDecision::Blocked {
            reason: "health_url_outside_node_local_storage_boundary".to_string(),
        };
    }
    if !desired.desired_running {
        return StorageDecision::ObserveOnly {
            reason: "desired_state_does_not_request_running".to_string(),
        };
    }
    if observed.service_active && observed.health_ok {
        return StorageDecision::Healthy;
    }
    if observed.observation_error.is_some() && observed.service_state == "unknown" {
        return StorageDecision::Blocked {
            reason: "service_observation_unknown".to_string(),
        };
    }

    let window_ms = desired.restart_window_seconds.saturating_mul(1_000);
    let inside_window = now_unix_ms.saturating_sub(budget.window_started_unix_ms) < window_ms;
    if inside_window && budget.restart_count >= desired.max_restarts_per_window {
        return StorageDecision::Blocked {
            reason: "restart_budget_exhausted".to_string(),
        };
    }

    StorageDecision::Restart {
        service: desired.service.clone(),
        stop_condition: "one_restart_then_health_read_back".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn desired() -> StorageDesiredState {
        StorageDesiredState {
            service: "surrealdb.service".to_string(),
            health_url: "http://127.0.0.1:8000/health".to_string(),
            desired_running: true,
            warmup_seconds: 30,
            restart_window_seconds: 300,
            max_restarts_per_window: 1,
        }
    }

    #[test]
    fn healthy_storage_never_restarts() {
        let observation = StorageObservation {
            observed_at_unix_ms: 10,
            service_active: true,
            service_state: "active".to_string(),
            health_ok: true,
            health_status: Some(200),
            observation_error: None,
        };
        assert_eq!(
            reconcile_storage(
                &desired(),
                &observation,
                &RestartBudget {
                    window_started_unix_ms: 0,
                    restart_count: 0
                },
                10
            ),
            StorageDecision::Healthy
        );
    }

    #[test]
    fn exhausted_budget_blocks_restart_loop() {
        let observation = StorageObservation {
            observed_at_unix_ms: 20_000,
            service_active: false,
            service_state: "failed".to_string(),
            health_ok: false,
            health_status: None,
            observation_error: None,
        };
        assert!(matches!(
            reconcile_storage(
                &desired(),
                &observation,
                &RestartBudget {
                    window_started_unix_ms: 10_000,
                    restart_count: 1
                },
                20_000
            ),
            StorageDecision::Blocked { .. }
        ));
    }

    #[test]
    fn arbitrary_service_is_outside_authority() {
        let mut desired = desired();
        desired.service = "ssh.service".to_string();
        let observation = StorageObservation {
            observed_at_unix_ms: 20_000,
            service_active: false,
            service_state: "failed".to_string(),
            health_ok: false,
            health_status: None,
            observation_error: None,
        };
        assert!(matches!(
            reconcile_storage(
                &desired,
                &observation,
                &RestartBudget {
                    window_started_unix_ms: 20_000,
                    restart_count: 0
                },
                20_000
            ),
            StorageDecision::Blocked { .. }
        ));
    }
}
