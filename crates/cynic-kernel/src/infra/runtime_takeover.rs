//! Standby/takeover execution boundary.
//!
//! Promotion is never inferred from a dead connection. A scheduler must
//! supply an observed primary lease, release proof, candidate readiness,
//! storage health, and a newer fencing token before an adapter may promote.

use std::sync::Arc;

use crate::domain::coord::CoordPort;

#[async_trait::async_trait]
pub trait RuntimeTakeoverMutationPort: Send + Sync {
    async fn promote(
        &self,
        observation: &cynic_protocol::RuntimeTakeoverObservation,
    ) -> Result<cynic_protocol::RuntimeTakeoverObservation, String>;
}

#[derive(Debug, Clone, Copy)]
pub struct RuntimeTakeoverController;

pub fn configured_standby_mode() -> cynic_protocol::StandbyMode {
    match std::env::var("CYNIC_STANDBY_MODE")
        .ok()
        .as_deref()
        .map(str::trim)
    {
        Some("resident") => cynic_protocol::StandbyMode::Resident,
        Some("active") => cynic_protocol::StandbyMode::Active,
        _ => cynic_protocol::StandbyMode::Disabled,
    }
}

impl RuntimeTakeoverController {
    pub async fn publish_primary_lease(
        &self,
        coord: &Arc<dyn CoordPort>,
        primary_node_id: &str,
        ttl_seconds: u64,
    ) -> Result<cynic_protocol::TakeoverLeaseObservation, String> {
        coord
            .publish_takeover_lease(primary_node_id, ttl_seconds)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn release_primary_lease(
        &self,
        coord: &Arc<dyn CoordPort>,
        primary_node_id: &str,
        fencing_token: u64,
        proof_ref: &str,
    ) -> Result<cynic_protocol::TakeoverLeaseObservation, String> {
        coord
            .release_takeover_lease(primary_node_id, fencing_token, proof_ref)
            .await
            .map_err(|error| error.to_string())
    }

    pub async fn observe(
        &self,
        coord: &Arc<dyn CoordPort>,
        candidate_ready: bool,
        storage_available: bool,
    ) -> cynic_protocol::RuntimeTakeoverObservation {
        let lease = coord.takeover_lease_observation().await;
        cynic_protocol::RuntimeTakeoverObservation {
            mode: configured_standby_mode(),
            primary_live: lease.primary_live,
            lease_valid: lease.lease_valid,
            candidate_ready,
            release_proved: lease.release_proved,
            storage_available,
            fencing_token: lease.fencing_token,
            prior_fencing_token: lease.prior_fencing_token,
        }
    }

    pub fn decide(
        &self,
        observation: cynic_protocol::RuntimeTakeoverObservation,
    ) -> cynic_protocol::RuntimeTakeoverDecision {
        cynic_protocol::decide_runtime_takeover(observation)
    }

    pub async fn promote_with_read_back(
        &self,
        adapter: &dyn RuntimeTakeoverMutationPort,
        observation: cynic_protocol::RuntimeTakeoverObservation,
    ) -> Result<cynic_protocol::RuntimeTakeoverDecision, String> {
        let decision = self.decide(observation.clone());
        if !decision.promote {
            return Err(format!("takeover blocked: {:?}", decision.blockers));
        }
        let read_back = adapter.promote(&observation).await?;
        let proof = self.decide(read_back);
        if !proof.promote || proof.mode != cynic_protocol::StandbyMode::Active {
            return Err(format!("takeover read-back failed: {:?}", proof.blockers));
        }
        Ok(proof)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_liveness_unknown_is_not_promotable() {
        let observation = cynic_protocol::RuntimeTakeoverObservation {
            mode: cynic_protocol::StandbyMode::Resident,
            primary_live: true,
            lease_valid: true,
            candidate_ready: true,
            release_proved: true,
            storage_available: true,
            fencing_token: 2,
            prior_fencing_token: 1,
        };
        assert!(!RuntimeTakeoverController {}.decide(observation).promote);
    }
}
