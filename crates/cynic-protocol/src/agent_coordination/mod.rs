//! Agent coordination protocol — canonical types and pure predicates.
//!
//! This module extends the coordination primitives from `coordination_lease`
//! with the full lifecycle: identity, claim, heartbeat, release, mutation.
//!
//! Existing `LeaseGuard`/`LeaseGrant` types are reused for backward
//! compatibility. New types (`Claim`, `ClaimRequest`, etc.) build on them.

pub mod predicates;
pub mod identity;
pub mod claims;
pub mod lifecycle;
pub mod mutation;
pub mod proofs;
pub mod field_envelope;

// ── Re-exports from existing coordination_lease (backward compat) ────────

pub use crate::coordination_lease::{
    BatchClaimResult, LeaseAction, LeaseConflict, LeaseGrant, LeaseGuard, LeaseMutationOutcome,
    LeaseRejectionCode, TargetClaimOutcome,
};

use crate::error_catalog::CynicErrorCode;

pub use crate::agent_citizenship::{AgentBudgets, MutationAdmission};
pub use crate::agent_session::types::{
    AgentRootLaunchRequest, AgentSessionAuthority, AgentSessionCredentialKind,
    AgentSessionEnrollment, AgentSessionPeerObservation, AgentSessionRecord,
    AgentSessionRenewalRequest, AgentSessionRequest, AgentSessionState, AgentSessionTransport,
};

pub use crate::SecurityEvidenceState;

// ── Protocol version ──────────────────────────────────────────────────────

pub const AGENT_COORDINATION_SCHEMA: &str = "cynic.agent_coordination.v1";
pub const AGENT_IDENTITY_SCHEMA: &str = "cynic.agent_identity.v1";
pub const CLAIM_REQUEST_SCHEMA: &str = "cynic.claim_request.v1";
pub const CLAIM_SCHEMA: &str = "cynic.claim.v1";
pub const HEARTBEAT_REQUEST_SCHEMA: &str = "cynic.heartbeat_request.v1";
pub const RELEASE_REQUEST_SCHEMA: &str = "cynic.release_request.v1";
pub const REGISTRATION_RECORD_SCHEMA: &str = "cynic.registration_record.v1";



// ── Re-exports from submodules ───────────────────────────────────────────

pub use identity::*;
pub use claims::*;
pub use lifecycle::*;
pub use mutation::*;
pub use proofs::*;
pub use field_envelope::*;

// ── Tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_agent_identity_passes_validation() {
        let identity = AgentIdentity {
            schema: AGENT_IDENTITY_SCHEMA.to_string(),
            agent_id: "hermes:test:node1:1234".to_string(),
            agent_kind: AgentKind::Hermes,
            credentials: CredentialAttestation::LocalPeerBinding {
                process_id: 1234,
                user_id: 1000,
                attested_at_unix_ms: 1000,
            },
            attested_at_unix_ms: 1000,
            evidence: Some(SecurityEvidenceState::Proven),
        };
        assert!(identity.validate().is_ok());
    }

    #[test]
    fn empty_agent_id_fails_validation() {
        let identity = AgentIdentity {
            schema: AGENT_IDENTITY_SCHEMA.to_string(),
            agent_id: "".to_string(),
            agent_kind: AgentKind::Hermes,
            credentials: CredentialAttestation::LocalPeerBinding {
                process_id: 1234,
                user_id: 1000,
                attested_at_unix_ms: 1000,
            },
            attested_at_unix_ms: 1000,
            evidence: None,
        };
        assert!(identity.validate().is_err());
    }

    #[test]
    fn jwt_expiry_before_attestation_fails() {
        let identity = AgentIdentity {
            schema: AGENT_IDENTITY_SCHEMA.to_string(),
            agent_id: "hermes:test".to_string(),
            agent_kind: AgentKind::Hermes,
            credentials: CredentialAttestation::SignedJwt {
                issuer: "test".to_string(),
                subject: "hermes".to_string(),
                expires_at_unix_ms: 100,
                attested_at_unix_ms: 1000,
            },
            attested_at_unix_ms: 1000,
            evidence: None,
        };
        assert!(identity.validate().is_err());
    }

    #[test]
    fn fleet_scope_contains_everything() {
        let fleet = ClaimScope::Fleet;
        assert!(fleet.contains(&ClaimScope::Node {
            node_id: "node1".to_string()
        }));
        assert!(fleet.contains(&ClaimScope::Crate {
            crate_name: "cynic-protocol".to_string()
        }));
        assert!(fleet.contains(&ClaimScope::File {
            path: "crates/cynic-protocol/src/lib.rs".to_string()
        }));
    }

    #[test]
    fn crate_scope_contains_module_and_file() {
        let crate_scope = ClaimScope::Crate {
            crate_name: "cynic-protocol".to_string(),
        };
        assert!(crate_scope.contains(&ClaimScope::Module {
            crate_name: "cynic-protocol".to_string(),
            module_path: "src".to_string(),
        }));
        assert!(crate_scope.contains(&ClaimScope::File {
            path: "crates/cynic-protocol/src/lib.rs".to_string(),
        }));
    }

    #[test]
    fn active_claim_is_active() {
        let claim = Claim {
            schema: CLAIM_SCHEMA.to_string(),
            claim_id: "claim_001".to_string(),
            request: claim_request(
                "hermes".to_string(),
                ClaimScope::Crate {
                    crate_name: "cynic-protocol".to_string(),
                },
            ),
            lease_id: "lease_001".to_string(),
            fencing_token: 1,
            granted_at_unix_ms: 1000,
            expires_at_unix_ms: 5000,
            max_lease_window_ms: 10000,
            status: ClaimStatus::Active,
            preempted_by: None,
            phase: AgentPhase::Exploration,
            consumed: AgentBudgets::default(),
        };
        assert!(claim.is_active());
        assert!(!claim.is_expired_at(4000));
        assert!(claim.is_expired_at(6000));
    }

    #[test]
    fn valid_heartbeat_passes() {
        let claim = Claim {
            schema: CLAIM_SCHEMA.to_string(),
            claim_id: "claim_001".to_string(),
            request: claim_request(
                "hermes".to_string(),
                ClaimScope::Crate {
                    crate_name: "cynic-protocol".to_string(),
                },
            ),
            lease_id: "lease_001".to_string(),
            fencing_token: 1,
            granted_at_unix_ms: 1000,
            expires_at_unix_ms: 5000,
            max_lease_window_ms: 10000,
            status: ClaimStatus::Active,
            preempted_by: None,
            phase: AgentPhase::Exploration,
            consumed: AgentBudgets::default(),
        };
        let heartbeat = HeartbeatRequest {
            schema: HEARTBEAT_REQUEST_SCHEMA.to_string(),
            agent_id: "hermes".to_string(),
            lease_id: "lease_001".to_string(),
            fencing_token: 1,
        };
        assert!(validate_heartbeat(&heartbeat, &claim, 3000).is_ok());
    }

    #[test]
    fn heartbeat_with_wrong_fencing_token_fails() {
        let claim = Claim {
            schema: CLAIM_SCHEMA.to_string(),
            claim_id: "claim_001".to_string(),
            request: claim_request(
                "hermes".to_string(),
                ClaimScope::Crate {
                    crate_name: "cynic-protocol".to_string(),
                },
            ),
            lease_id: "lease_001".to_string(),
            fencing_token: 1,
            granted_at_unix_ms: 1000,
            expires_at_unix_ms: 5000,
            max_lease_window_ms: 10000,
            status: ClaimStatus::Active,
            preempted_by: None,
            phase: AgentPhase::Exploration,
            consumed: AgentBudgets::default(),
        };
        let heartbeat = HeartbeatRequest {
            schema: HEARTBEAT_REQUEST_SCHEMA.to_string(),
            agent_id: "hermes".to_string(),
            lease_id: "lease_001".to_string(),
            fencing_token: 2,
        };
        assert!(validate_heartbeat(&heartbeat, &claim, 3000).is_err());
    }

    #[test]
    fn admit_mutation_within_scope() {
        let claim = make_claim_with_consumed(AgentBudgets::default());
        let request = mutation_request(
            ClaimScope::File {
                path: "crates/cynic-protocol/src/lib.rs".to_string(),
            },
            AgentBudgets {
                actions: 1,
                wall_seconds: 60,
            },
        );
        assert!(admit_mutation(&claim, &request, 3000, true).is_admitted());
    }

    #[test]
    fn mutation_outside_scope_is_refused() {
        let claim = Claim {
            schema: CLAIM_SCHEMA.to_string(),
            claim_id: "claim_001".to_string(),
            request: claim_request(
                "hermes".to_string(),
                ClaimScope::Crate {
                    crate_name: "cynic-protocol".to_string(),
                },
            ),
            lease_id: "lease_001".to_string(),
            fencing_token: 1,
            granted_at_unix_ms: 1000,
            expires_at_unix_ms: 5000,
            max_lease_window_ms: 10000,
            status: ClaimStatus::Active,
            preempted_by: None,
            phase: AgentPhase::Exploration,
            consumed: AgentBudgets::default(),
        };
        let request = MutationRequest {
            schema: "cynic.mutation_request.v1".to_string(),
            agent_id: "hermes".to_string(),
            claim_id: "claim_001".to_string(),
            target_scope: ClaimScope::Crate {
                crate_name: "cynic-kernel".to_string(),
            },
            action: MutationAction {
                argv: vec!["cargo".to_string(), "check".to_string()],
                working_directory: None,
                mutates: true,
                requires_privilege: false,
                stop_condition: "stop on first failure".to_string(),
            },
            budget_delta: AgentBudgets {
                actions: 1,
                wall_seconds: 60,
            },
            rollback_intent: "discard changes".to_string(),
        };
        assert!(!admit_mutation(&claim, &request, 3000, true).is_admitted());
    }

    #[test]
    fn consumed_budget_reduces_remaining() {
        let claim = make_claim_with_consumed(AgentBudgets {
            actions: 49,
            wall_seconds: 3540,
        });
        let request = mutation_request(
            ClaimScope::File {
                path: "crates/cynic-protocol/src/lib.rs".to_string(),
            },
            AgentBudgets {
                actions: 2,
                wall_seconds: 60,
            },
        );
        assert!(matches!(
            admit_mutation(&claim, &request, 3000, true),
            MutationAdmission::Refused { .. }
        ));
    }

    #[test]
    fn record_consumption_is_monotonic_and_saturating() {
        let claim = make_claim_with_consumed(AgentBudgets::default());
        let next = claim.record_consumption(&AgentBudgets {
            actions: 40,
            wall_seconds: 1800,
        });
        assert_eq!(next.consumed.actions, 40);
        assert_eq!(next.consumed.wall_seconds, 1800);
        assert_eq!(next.remaining_budget().actions, 10);

        let saturated = next.record_consumption(&AgentBudgets {
            actions: u64::MAX,
            wall_seconds: u64::MAX,
        });
        assert_eq!(saturated.consumed, saturated.request.budget);
        assert_eq!(saturated.remaining_budget().actions, 0);
        assert_eq!(saturated.remaining_budget().wall_seconds, 0);
    }

    #[test]
    fn exhausted_budget_refuses_mutation() {
        let claim = make_claim_with_consumed(AgentBudgets {
            actions: 50,
            wall_seconds: 3600,
        });
        let request = mutation_request(
            ClaimScope::File {
                path: "crates/cynic-protocol/src/lib.rs".to_string(),
            },
            AgentBudgets {
                actions: 1,
                wall_seconds: 60,
            },
        );
        assert!(!admit_mutation(&claim, &request, 3000, true).is_admitted());
    }

    fn make_claim_with_consumed(consumed: AgentBudgets) -> Claim {
        Claim {
            schema: CLAIM_SCHEMA.to_string(),
            claim_id: "claim_001".to_string(),
            request: claim_request(
                "hermes".to_string(),
                ClaimScope::Crate {
                    crate_name: "cynic-protocol".to_string(),
                },
            ),
            lease_id: "lease_001".to_string(),
            fencing_token: 1,
            granted_at_unix_ms: 1000,
            expires_at_unix_ms: 5000,
            max_lease_window_ms: 10000,
            status: ClaimStatus::Active,
            preempted_by: None,
            phase: AgentPhase::Exploration,
            consumed,
        }
    }

    fn mutation_request(target_scope: ClaimScope, budget_delta: AgentBudgets) -> MutationRequest {
        MutationRequest {
            schema: "cynic.mutation_request.v1".to_string(),
            agent_id: "hermes".to_string(),
            claim_id: "claim_001".to_string(),
            target_scope,
            action: MutationAction {
                argv: vec!["cargo".to_string(), "check".to_string()],
                working_directory: None,
                mutates: true,
                requires_privilege: false,
                stop_condition: "stop on first failure".to_string(),
            },
            budget_delta,
            rollback_intent: "discard changes".to_string(),
        }
    }

    fn claim_request(agent_id: String, scope: ClaimScope) -> ClaimRequest {
        ClaimRequest {
            schema: CLAIM_REQUEST_SCHEMA.to_string(),
            agent_id,
            session_id: "session_001".to_string(),
            scope,
            claim_type: ClaimType::Exclusive,
            budget: AgentBudgets {
                actions: 50,
                wall_seconds: 3600,
            },
            ttl_seconds: 1800,
            max_lease_window_ms: 3600000,
            parent_claim_id: None,
        }
    }
}


