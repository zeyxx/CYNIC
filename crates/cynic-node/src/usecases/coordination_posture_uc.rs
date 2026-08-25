//! Node-owned coordination posture (design D: the kernel is a federation
//! peer, never a coordination SPOF).
//!
//! This use case is the first consumer of the previously quarantined
//! `coordination_authority` contract: it reconciles a node's local
//! coordination authority (desired membership/quorum/term/fencing vs observed
//! members) into an executable plan and classifies the resulting posture.
//!
//! Authority stays node-local by construction:
//! - quorum/fencing decisions are computed from local desired state plus
//!   locally observed member health — no kernel round-trip;
//! - mutation fencing is delegated to the node's own lease store
//!   (`StorageLeasePort` adapters) so fencing tokens stay monotonic without
//!   any central issuer;
//! - kernel unavailability can only *degrade* the posture to explicit local
//!   autonomy; it never blocks observation or reconciliation.

use cynic_protocol::LeaseGuard;
use cynic_protocol::coordination_authority::{
    CoordinationAuthorityAction, CoordinationAuthorityDesiredState,
    CoordinationAuthorityObservation, reconcile_coordination_authority,
};

use crate::ports::storage_lease_port::StorageLeasePort;

pub const COORDINATION_POSTURE_SCHEMA: &str = "cynic.coordination_posture.v1";
/// Fallback TTL when the caller does not pin one. Deliberately short: leases
/// are cheap to renew and stale fencing must expire fast.
pub const DEFAULT_LEASE_TTL_MS: u64 = 30_000;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CoordinationPosture {
    pub schema: &'static str,
    pub authority_id: String,
    /// `local_autonomous` (peers/kernel absent, contract otherwise valid),
    /// `fenced_active` (plan executable), or `read_only`.
    pub state: &'static str,
    pub action: CoordinationAuthorityAction,
    pub blockers: Vec<String>,
}

/// Reconcile this node's coordination authority from desired + observed state.
/// Pure decision: no I/O, no kernel dependency, deterministic for identical
/// inputs.
pub fn decide_coordination_posture(
    desired: &CoordinationAuthorityDesiredState,
    observed: &CoordinationAuthorityObservation,
) -> CoordinationPosture {
    let plan = reconcile_coordination_authority(desired, observed);
    let degraded = plan
        .blockers
        .iter()
        .any(|b| b == "healthy_quorum_not_observed");
    let state = if plan.blockers.is_empty() {
        "fenced_active"
    } else if degraded && plan.blockers.len() == 1 {
        "local_autonomous"
    } else {
        "read_only"
    };
    CoordinationPosture {
        schema: COORDINATION_POSTURE_SCHEMA,
        authority_id: plan.authority_id.clone(),
        state,
        action: plan.action,
        blockers: plan.blockers,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationFenceOutcome {
    Fenced,
    Rejected(MutationFenceRejection),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MutationFenceRejection {
    HeldByOther,
    StaleOrExpired,
    InvalidRequest,
}

/// Fence one bounded mutation through this node's own lease store.
///
/// The kernel never issues or verifies this fence: the node-local
/// `StorageLeasePort` adapter owns monotonic tokens, and conflicts surface as
/// typed outcomes at this boundary instead of raw adapter strings.
pub fn fence_mutation(
    leases: &dyn StorageLeasePort,
    resource_id: &str,
    agent_id: &str,
    now_ms: u64,
    ttl_ms: u64,
) -> Result<MutationFenceOutcome, String> {
    match leases.acquire(resource_id, agent_id, now_ms, ttl_ms) {
        Ok(_) => Ok(MutationFenceOutcome::Fenced),
        Err(code) => classify_rejection(&code),
    }
}

fn classify_rejection(code: &str) -> Result<MutationFenceOutcome, String> {
    match code {
        "storage_lease_already_held" => Ok(MutationFenceOutcome::Rejected(
            MutationFenceRejection::HeldByOther,
        )),
        "storage_lease_stale_or_expired" => Ok(MutationFenceOutcome::Rejected(
            MutationFenceRejection::StaleOrExpired,
        )),
        "storage_lease_request_invalid" | "storage_lease_target_invalid" => Ok(
            MutationFenceOutcome::Rejected(MutationFenceRejection::InvalidRequest),
        ),
        other => Err(other.to_string()),
    }
}

/// Renewal guard for long work under a held fence. Returns `false` when the
/// lease expired underneath the worker — the caller must stop before mutating.
pub fn renew_fence(leases: &dyn StorageLeasePort, guard: &LeaseGuard, now_ms: u64) -> bool {
    leases.renew(guard, now_ms, DEFAULT_LEASE_TTL_MS).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cynic_protocol::coordination_authority::{
        COORDINATION_AUTHORITY_DESIRED_SCHEMA, COORDINATION_AUTHORITY_OBSERVATION_SCHEMA,
        CoordinationAuthorityMemberObservation,
    };
    use cynic_protocol::{STORAGE_LEASE_SCHEMA, StorageLeaseGrant};

    fn desired() -> CoordinationAuthorityDesiredState {
        CoordinationAuthorityDesiredState {
            schema: COORDINATION_AUTHORITY_DESIRED_SCHEMA.to_string(),
            authority_id: "coord-main".to_string(),
            member_ids: vec!["n1".into(), "n2".into(), "n3".into()],
            required_member_count: 2,
            term: 3,
            next_fencing_token: 9,
        }
    }

    fn member(node_id: &str, term: u64, token: u64) -> CoordinationAuthorityMemberObservation {
        CoordinationAuthorityMemberObservation {
            node_id: node_id.into(),
            active: true,
            healthy: true,
            term,
            fencing_token: token,
            last_committed_fencing_token: token,
        }
    }

    fn healthy_observation() -> CoordinationAuthorityObservation {
        CoordinationAuthorityObservation {
            schema: COORDINATION_AUTHORITY_OBSERVATION_SCHEMA.to_string(),
            authority_id: "coord-main".to_string(),
            members: vec![member("n1", 3, 8), member("n2", 3, 8)],
        }
    }

    #[test]
    fn healthy_quorum_is_fenced_active() {
        let posture = decide_coordination_posture(&desired(), &healthy_observation());
        assert_eq!(posture.state, "fenced_active");
        assert!(posture.blockers.is_empty());
        assert_eq!(posture.schema, COORDINATION_POSTURE_SCHEMA);
    }

    #[test]
    fn peers_down_but_contract_valid_degrades_to_local_autonomy() {
        // Kernel/peers unreachable = zero observed members. The only blocker is
        // missing liveness, so the node keeps working, explicitly autonomous.
        let observed = CoordinationAuthorityObservation {
            schema: COORDINATION_AUTHORITY_OBSERVATION_SCHEMA.to_string(),
            authority_id: "coord-main".to_string(),
            members: Vec::new(),
        };
        let posture = decide_coordination_posture(&desired(), &observed);
        assert_eq!(posture.state, "local_autonomous");
        assert_eq!(posture.action, CoordinationAuthorityAction::HoldReadOnly);
        assert_eq!(posture.blockers, vec!["healthy_quorum_not_observed"]);
    }

    #[test]
    fn contract_violations_stay_read_only() {
        // Schema mismatch = real contract break beyond mere peer loss.
        let mut observed = healthy_observation();
        observed.schema = "bogus".into();
        let posture = decide_coordination_posture(&desired(), &observed);
        assert_eq!(posture.state, "read_only");
        assert!(
            posture
                .blockers
                .contains(&"observation_schema_invalid".to_string())
        );
    }

    /// Deterministic in-memory lease store standing in for
    /// `LocalStorageLeaseAdapter` — same port, no filesystem in unit tests.
    struct FakeLeases {
        behavior: FakeBehavior,
    }

    #[derive(Clone, Copy)]
    enum FakeBehavior {
        Grant,
        AlreadyHeld,
        Expired,
        Fault,
    }

    impl StorageLeasePort for FakeLeases {
        fn acquire(
            &self,
            resource_id: &str,
            agent_id: &str,
            now_ms: u64,
            ttl_ms: u64,
        ) -> Result<StorageLeaseGrant, String> {
            match self.behavior {
                FakeBehavior::Grant => Ok(StorageLeaseGrant {
                    schema: STORAGE_LEASE_SCHEMA.to_string(),
                    guard: LeaseGuard {
                        agent_id: agent_id.to_string(),
                        target: format!("storage:{resource_id}"),
                        lease_id: format!("storage-lease-{resource_id}-{now_ms}"),
                        fencing_token: 1,
                    },
                    expires_at_ms: now_ms + ttl_ms,
                }),
                FakeBehavior::AlreadyHeld => Err("storage_lease_already_held".into()),
                FakeBehavior::Expired => Err("storage_lease_stale_or_expired".into()),
                FakeBehavior::Fault => Err("fault_injected_io_error".into()),
            }
        }

        fn renew(
            &self,
            guard: &LeaseGuard,
            now_ms: u64,
            ttl_ms: u64,
        ) -> Result<StorageLeaseGrant, String> {
            match self.behavior {
                FakeBehavior::Grant => Ok(StorageLeaseGrant {
                    schema: STORAGE_LEASE_SCHEMA.to_string(),
                    guard: guard.clone(),
                    expires_at_ms: now_ms + ttl_ms,
                }),
                FakeBehavior::Expired => Err("storage_lease_stale_or_expired".into()),
                _ => Err("storage_lease_missing".into()),
            }
        }

        fn release(&self, _guard: &LeaseGuard, _now_ms: u64) -> Result<(), String> {
            Ok(())
        }
    }

    #[test]
    fn mutation_fences_through_local_lease_store() {
        let fenced = fence_mutation(
            &FakeLeases {
                behavior: FakeBehavior::Grant,
            },
            "res",
            "agent-a",
            1_000,
            DEFAULT_LEASE_TTL_MS,
        )
        .expect("grant must not be an adapter error");
        assert_eq!(fenced, MutationFenceOutcome::Fenced);

        let busy = fence_mutation(
            &FakeLeases {
                behavior: FakeBehavior::AlreadyHeld,
            },
            "res",
            "agent-b",
            1_000,
            DEFAULT_LEASE_TTL_MS,
        )
        .expect("conflict is a typed outcome, not an error");
        assert_eq!(
            busy,
            MutationFenceOutcome::Rejected(MutationFenceRejection::HeldByOther)
        );

        let stale = fence_mutation(
            &FakeLeases {
                behavior: FakeBehavior::Expired,
            },
            "res",
            "agent-b",
            1_000,
            DEFAULT_LEASE_TTL_MS,
        )
        .expect("stale lease is a typed outcome, not an error");
        assert_eq!(
            stale,
            MutationFenceOutcome::Rejected(MutationFenceRejection::StaleOrExpired)
        );
    }

    #[test]
    fn unknown_adapter_error_is_surfaced_not_swallowed() {
        let broken = fence_mutation(
            &FakeLeases {
                behavior: FakeBehavior::Fault,
            },
            "res",
            "agent-a",
            1_000,
            DEFAULT_LEASE_TTL_MS,
        );
        assert_eq!(broken, Err("fault_injected_io_error".to_string()));
    }

    #[test]
    fn renew_fence_stops_workers_on_expiry() {
        let guard = LeaseGuard {
            agent_id: "agent-a".into(),
            target: "storage:res".into(),
            lease_id: "l1".into(),
            fencing_token: 7,
        };
        assert!(renew_fence(
            &FakeLeases {
                behavior: FakeBehavior::Grant
            },
            &guard,
            2_000
        ));
        assert!(!renew_fence(
            &FakeLeases {
                behavior: FakeBehavior::Expired
            },
            &guard,
            2_000
        ));
    }
}
