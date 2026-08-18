use serde::{Deserialize, Serialize};

pub const COORDINATION_TASK_OBSERVATION_SCHEMA: &str = "cynic.coordination.task_observation.v1";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CoordinationTaskStatus {
    Triage,
    Todo,
    Ready,
    Running,
    Review,
    Done,
    Blocked,
    Scheduled,
    Archived,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CoordinationTaskObservation {
    pub schema: String,
    pub coordination_kind: String,
    pub task_id: String,
    pub status: CoordinationTaskStatus,
    pub assignee: Option<String>,
    pub run_id: Option<String>,
    pub lease_expires_at: Option<String>,
    pub heartbeat_at: Option<String>,
    pub parent_ids: Vec<String>,
    pub child_ids: Vec<String>,
    pub correlation_id: Option<String>,
    pub message: Option<String>,
}

impl CoordinationTaskObservation {
    pub fn unknown(
        kind: impl Into<String>,
        task_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            schema: COORDINATION_TASK_OBSERVATION_SCHEMA.into(),
            coordination_kind: kind.into(),
            task_id: task_id.into(),
            status: CoordinationTaskStatus::Unknown,
            assignee: None,
            run_id: None,
            lease_expires_at: None,
            heartbeat_at: None,
            parent_ids: Vec::new(),
            child_ids: Vec::new(),
            correlation_id: None,
            message: Some(message.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_coordination_readback_is_unknown() {
        let observation =
            CoordinationTaskObservation::unknown("hermes_kanban", "task-1", "board_unreachable");
        assert_eq!(observation.schema, COORDINATION_TASK_OBSERVATION_SCHEMA);
        assert_eq!(observation.status, CoordinationTaskStatus::Unknown);
        assert!(observation.lease_expires_at.is_none());
    }

    #[test]
    fn status_serialization_is_stable() {
        let json = serde_json::to_string(&CoordinationTaskStatus::Running).unwrap();
        assert_eq!(json, "\"running\"");
    }
}
