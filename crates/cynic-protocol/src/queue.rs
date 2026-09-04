use serde::{Deserialize, Serialize};

pub const QUEUE_PLAN_SCHEMA: &str = "cynic.queue_plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueReason {
    OrphanReclaimable,
    AgedReclaimable,
    BranchWithoutUpstream,
    StaleBranch,
    UpstreamDivergent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundedAction {
    pub argv: Vec<String>,
    pub requires_privilege: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuePlan {
    pub schema: String,
    pub root: String,
    pub items: Vec<QueueItem>,
    pub total_reclaimable_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueItem {
    pub id: String,
    pub branch: String,
    pub recipe_id: String,
    pub reason: QueueReason,
    pub action: BoundedAction,
}

/// Pure policy for queue ordering (one canonical decision source).
pub trait QueuePolicy {
    fn order(&self, items: Vec<QueueItem>) -> Vec<QueueItem>;
}

pub const QUEUE_EXECUTION_RESULT_SCHEMA: &str = "cynic.queue_execution_result.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueItemExecution {
    pub id: String,
    pub branch: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueueExecutionResult {
    pub schema: String,
    pub timestamp: String,
    pub success: bool,
    pub items_executed: Vec<QueueItemExecution>,
    pub total_reclaimed_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_execution_result_roundtrip() {
        let res = QueueExecutionResult {
            schema: QUEUE_EXECUTION_RESULT_SCHEMA.to_string(),
            timestamp: "2026-09-04T12:00:00Z".to_string(),
            success: true,
            items_executed: vec![QueueItemExecution {
                id: "branch:orphan-wip".to_string(),
                branch: "orphan-wip".to_string(),
                success: true,
                exit_code: Some(0),
                stdout: "Deleted branch orphan-wip".to_string(),
                stderr: String::new(),
            }],
            total_reclaimed_bytes: 1024,
        };

        let json = serde_json::to_string(&res).expect("serialize");
        let parsed: QueueExecutionResult = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(res, parsed);
        assert!(parsed.success);
        assert_eq!(parsed.items_executed.len(), 1);
        assert_eq!(parsed.items_executed[0].branch, "orphan-wip");
    }
}
