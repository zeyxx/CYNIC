use cynic_protocol::branch_graph::BranchGraph;
use cynic_build::build_artefact::BuildArtefactGcPlan;
use cynic_build::build_discovery::BuildDiscoveryReport;
use cynic_protocol::queue::QueuePlan;

use super::probe::build_queue_plan;

pub trait QueuePlanningPort {
    fn plan_queue(
        &self,
        branch_graph: &BranchGraph,
        discovery: &BuildDiscoveryReport,
        gc_plan: &BuildArtefactGcPlan,
        now: u64,
    ) -> QueuePlan;
}

pub struct DefaultQueuePlanner;

impl QueuePlanningPort for DefaultQueuePlanner {
    fn plan_queue(
        &self,
        branch_graph: &BranchGraph,
        discovery: &BuildDiscoveryReport,
        gc_plan: &BuildArtefactGcPlan,
        now: u64,
    ) -> QueuePlan {
        build_queue_plan(branch_graph, discovery, gc_plan, now)
    }
}
