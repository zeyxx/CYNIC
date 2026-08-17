//! Node-owned authority controller boundary.
//!
//! This controller intentionally stops after observe + reconcile. Applying a
//! plan requires an injected mutation adapter and a read-back proof; there is
//! no implicit fallback to a local write or to a guessed single-node quorum.

use std::sync::Arc;

use crate::domain::coord::CoordPort;

#[async_trait::async_trait]
pub trait CoordinationAuthorityMutationPort: Send + Sync {
    /// Apply one already-reconciled plan and return the adapter's read-back.
    /// Implementations must perform one bounded transaction; they may not
    /// silently retry against another authority.
    async fn apply_plan(
        &self,
        plan: &cynic_protocol::CoordinationAuthorityPlan,
    ) -> Result<cynic_protocol::CoordinationAuthorityObservation, String>;
}

#[derive(Debug, Clone)]
pub struct CoordinationAuthorityController {
    desired: cynic_protocol::CoordinationAuthorityDesiredState,
}

impl CoordinationAuthorityController {
    pub fn new(desired: cynic_protocol::CoordinationAuthorityDesiredState) -> Self {
        Self { desired }
    }

    pub fn desired(&self) -> &cynic_protocol::CoordinationAuthorityDesiredState {
        &self.desired
    }

    pub async fn observe(
        &self,
        coord: &Arc<dyn CoordPort>,
    ) -> cynic_protocol::CoordinationAuthorityObservation {
        coord.authority_observation().await
    }

    pub fn reconcile(
        &self,
        observed: &cynic_protocol::CoordinationAuthorityObservation,
    ) -> cynic_protocol::CoordinationAuthorityPlan {
        cynic_protocol::reconcile_coordination_authority(&self.desired, observed)
    }

    pub async fn apply_with_read_back(
        &self,
        adapter: &dyn CoordinationAuthorityMutationPort,
        plan: &cynic_protocol::CoordinationAuthorityPlan,
    ) -> Result<cynic_protocol::CoordinationAuthorityObservation, String> {
        if !plan.blockers.is_empty()
            || plan.action == cynic_protocol::CoordinationAuthorityAction::HoldReadOnly
        {
            return Err(format!(
                "coordination authority plan is not admissible: {:?}",
                plan.blockers
            ));
        }
        let observed = adapter.apply_plan(plan).await?;
        let proof = self.reconcile(&observed);
        if !proof.blockers.is_empty()
            || proof.action == cynic_protocol::CoordinationAuthorityAction::HoldReadOnly
        {
            return Err(format!(
                "coordination authority read-back failed: {:?}",
                proof.blockers
            ));
        }
        Ok(observed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::{
        COORDINATION_AUTHORITY_OBSERVATION_SCHEMA, CoordinationAuthorityAction,
        CoordinationAuthorityObservation,
    };

    #[test]
    fn controller_keeps_unobserved_authority_read_only() {
        let controller = CoordinationAuthorityController::new(
            cynic_protocol::CoordinationAuthorityDesiredState {
                schema: cynic_protocol::COORDINATION_AUTHORITY_DESIRED_SCHEMA.to_string(),
                authority_id: "coord-main".to_string(),
                member_ids: vec!["node-a".to_string()],
                required_member_count: 1,
                term: 1,
                next_fencing_token: 1,
            },
        );
        let plan = controller.reconcile(&CoordinationAuthorityObservation {
            schema: COORDINATION_AUTHORITY_OBSERVATION_SCHEMA.to_string(),
            authority_id: "coord-main".to_string(),
            members: Vec::new(),
        });
        assert_eq!(plan.action, CoordinationAuthorityAction::HoldReadOnly);
    }
}
