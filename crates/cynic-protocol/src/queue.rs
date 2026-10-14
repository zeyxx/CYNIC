use serde::{Deserialize, Serialize};

pub const QUEUE_PLAN_SCHEMA: &str = "cynic.queue_plan.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QueueReason {
    OrphanReclaimable,
    AgedReclaimable,
    BranchWithoutUpstream,
    StaleBranch,
    UpstreamDivergent,
    /// Extension point for non-branch domains. `domain` names the owning
    /// domain (e.g. `"build_artefact_gc"`), `code` carries the domain's own
    /// reason verbatim instead of being mislabeled as a branch-GC reason.
    /// Branch-GC ranks (0-4) always sort before foreign reasons.
    Other {
        domain: String,
        code: String,
    },
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
    /// Inputs that degraded to fallback during observation
    /// (e.g. `"branch_graph_fallback:<cause>"`). Empty means every input was
    /// observed live. Additive (`serde default`): plans written before this
    /// field parse with no unknowns recorded.
    #[serde(default)]
    pub residual_unknowns: Vec<String>,
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

/// Canonical rank for a queue reason. Branch-GC reasons sort first (0-4);
/// foreign-domain `Other` reasons sort last (5). Any future reordering goes
/// through this function, never through a call-site sort.
pub fn reason_rank(reason: &QueueReason) -> u8 {
    match reason {
        QueueReason::OrphanReclaimable => 0,
        QueueReason::AgedReclaimable => 1,
        QueueReason::BranchWithoutUpstream => 2,
        QueueReason::StaleBranch => 3,
        QueueReason::UpstreamDivergent => 4,
        QueueReason::Other { .. } => 5,
    }
}

/// The canonical [`QueuePolicy`]: reason rank, then branch, then id.
#[derive(Debug, Default, Clone, Copy)]
pub struct CanonicalQueuePolicy;

impl QueuePolicy for CanonicalQueuePolicy {
    fn order(&self, mut items: Vec<QueueItem>) -> Vec<QueueItem> {
        items.sort_by(|a, b| {
            reason_rank(&a.reason)
                .cmp(&reason_rank(&b.reason))
                .then(a.branch.cmp(&b.branch))
                .then(a.id.cmp(&b.id))
        });
        items
    }
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

#[cfg(test)]
mod policy_tests {
    use super::*;

    fn item(id: &str, branch: &str, reason: QueueReason) -> QueueItem {
        QueueItem {
            id: id.to_string(),
            branch: branch.to_string(),
            recipe_id: "r".to_string(),
            reason,
            action: BoundedAction {
                argv: vec![],
                requires_privilege: false,
            },
        }
    }

    #[test]
    fn canonical_policy_orders_rank_then_branch_then_id() {
        let other = || QueueReason::Other {
            domain: "d".to_string(),
            code: "c".to_string(),
        };
        let items = vec![
            item("b", "zeta", QueueReason::StaleBranch),
            item("a", "zeta", QueueReason::StaleBranch),
            item("c", "alpha", other()),
            item("d", "beta", QueueReason::OrphanReclaimable),
        ];
        let ordered = CanonicalQueuePolicy.order(items);
        let ids: Vec<&str> = ordered.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, vec!["d", "a", "b", "c"]);
    }

    #[test]
    fn plan_without_unknowns_field_still_parses() {
        let raw = r#"{"schema":"cynic.queue_plan.v1","root":"main","items":[],"total_reclaimable_bytes":0}"#;
        let plan: QueuePlan = serde_json::from_str(raw).expect("old plan parses");
        assert!(plan.residual_unknowns.is_empty());
    }
}
