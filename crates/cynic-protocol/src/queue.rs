use serde::{Deserialize, Serialize};

pub const QUEUE_PLAN_SCHEMA: &str = "cynic.queue_plan.v1";

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
    pub priority: u8,
    pub reason: String,
    pub requires_privilege: bool,
    pub argv: Vec<String>,
}
