use cynic_protocol::build_discovery::BuildDiscoveryReport;
use cynic_protocol::branch_graph::BranchGraph;
use cynic_protocol::queue::QueuePlan;
use cynic_protocol::retention::RetentionPlan;

use super::probe::build_queue_plan;

pub trait QueuePlanningPort {
    fn plan_queue(
        &self,
        branch_graph: &BranchGraph,
        discovery: &BuildDiscoveryReport,
        gc_plan: &RetentionPlan,
        now: u64,
    ) -> QueuePlan;
}

pub struct DefaultQueuePlanner;

impl QueuePlanningPort for DefaultQueuePlanner {
    fn plan_queue(
        &self,
        branch_graph: &BranchGraph,
        discovery: &BuildDiscoveryReport,
        gc_plan: &RetentionPlan,
        now: u64,
    ) -> QueuePlan {
        build_queue_plan(branch_graph, discovery, gc_plan, now)
    }
}
