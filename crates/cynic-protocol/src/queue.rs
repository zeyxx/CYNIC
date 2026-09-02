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
