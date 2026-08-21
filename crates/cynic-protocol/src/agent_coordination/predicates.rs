//! Pure predicates for agent coordination.
//!
//! These functions are the canonical decision rules for the coordination
//! protocol. They are pure (no I/O, no side effects) and must be used by
//! all surfaces: MCP, CLI, kernel, tests.

use super::{
    Claim, ClaimScope, ClaimStatus, ClaimType, CoordinationCode, HeartbeatRequest, ReleaseRequest,
};

/// Validate a heartbeat request against the active claim.
pub fn validate_heartbeat(
    request: &HeartbeatRequest,
    claim: &Claim,
    now_unix_ms: u64,
) -> Result<(), Vec<String>> {
    let mut codes = Vec::new();

    if request.lease_id != claim.lease_id {
        codes.push("lease_id_mismatch".to_string());
    }
    if request.fencing_token != claim.fencing_token {
        codes.push("fencing_token_mismatch".to_string());
    }
    if claim.is_expired_at(now_unix_ms) {
        codes.push("claim_expired".to_string());
    }
    if claim.status != ClaimStatus::Active {
        codes.push("claim_not_active".to_string());
    }

    if codes.is_empty() {
        Ok(())
    } else {
        Err(codes)
    }
}

/// Validate a release request against the active claim.
pub fn validate_release(
    request: &ReleaseRequest,
    claim: &Claim,
) -> Result<(), Vec<String>> {
    let mut codes = Vec::new();

    if request.lease_id != claim.lease_id {
        codes.push("lease_id_mismatch".to_string());
    }
    if request.fencing_token != claim.fencing_token {
        codes.push("fencing_token_mismatch".to_string());
    }
    if claim.status != ClaimStatus::Active {
        codes.push("claim_not_active".to_string());
    }

    if codes.is_empty() {
        Ok(())
    } else {
        Err(codes)
    }
}

/// Detect conflicts between local claims and kernel claims.
pub fn detect_conflicts(local: &[Claim], kernel: &[Claim]) -> Vec<(String, String)> {
    let mut conflicts = Vec::new();
    for local_claim in local {
        for kernel_claim in kernel {
            if local_claim.claim_id != kernel_claim.claim_id
                && local_claim.agent_id != kernel_claim.agent_id
                && scopes_overlap(&local_claim.request.scope, &kernel_claim.request.scope)
                && kernel_claim.status == ClaimStatus::Active
            {
                conflicts.push((local_claim.claim_id.clone(), kernel_claim.claim_id.clone()));
            }
        }
    }
    conflicts
}

/// Whether two claims have overlapping scopes.
pub fn scopes_overlap(a: &ClaimScope, b: &ClaimScope) -> bool {
    a.contains(b) || b.contains(a)
}

/// Whether a claim has expired at the given timestamp.
pub fn is_expired(claim: &Claim, now_unix_ms: u64) -> bool {
    claim.is_expired_at(now_unix_ms)
}

/// Whether a claim's heartbeat is stale at the given timestamp.
pub fn is_heartbeat_stale(claim: &Claim, now_unix_ms: u64) -> bool {
    claim.is_heartbeat_stale(now_unix_ms)
}

/// Whether an incoming claim conflicts with an existing active claim.
pub fn claim_conflicts(existing: &Claim, incoming_scope: &ClaimScope) -> Option<CoordinationCode> {
    if !existing.is_active() {
        return None;
    }
    if !scopes_overlap(&existing.request.scope, incoming_scope) {
        return None;
    }
    Some(CoordinationCode::ScopeConflict)
}

/// Whether one claim can preempt another.
///
/// Rules:
/// - Cynic mode preempts baseline mode.
/// - Same mode: no preemption (first-wins).
/// - Higher priority wins if modes differ.
pub fn can_preempt(existing: &Claim, incoming_claim_type: ClaimType) -> bool {
    // For now, only Exclusive preempts SharedObserve if they overlap.
    // Full Cynic-vs-Baseline preemption is handled at the agent mode level,
    // not the claim type level.
    matches!(existing.request.claim_type, ClaimType::SharedObserve)
        && matches!(incoming_claim_type, ClaimType::Exclusive)
}

/// Whether a claim's budget is within the granted budget.
pub fn budget_within_granted(_claim: &Claim) -> bool {
    // TODO: Track consumed budget in Claim
    true
}

/// Whether a claim can be renewed at the given timestamp.
pub fn can_renew(claim: &Claim, now_unix_ms: u64) -> bool {
    claim.status == ClaimStatus::Active && !claim.is_expired_at(now_unix_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentBudgets, ClaimRequest};

    fn make_claim(scope: ClaimScope, claim_type: ClaimType) -> Claim {
        Claim {
            schema: "cynic.claim.v1",
            claim_id: "claim_test".to_string(),
            request: ClaimRequest {
                schema: "cynic.claim_request.v1",
                agent_id: "test".to_string(),
                session_id: "session_test".to_string(),
                scope,
                claim_type,
                budget: AgentBudgets {
                    actions: 100,
                    wall_seconds: 3600,
                },
                ttl_seconds: 1800,
                max_lease_window_ms: 3600000,
                parent_claim_id: None,
            },
            lease_id: "lease_test".to_string(),
            fencing_token: 1,
            granted_at_unix_ms: 1000,
            expires_at_unix_ms: 5000,
            max_lease_window_ms: 3600000,
            status: ClaimStatus::Active,
            preempted_by: None,
        }
    }

    #[test]
    fn scopes_overlap_when_one_contains_other() {
        let a = ClaimScope::Crate {
            crate_name: "cynic-protocol".to_string(),
        };
        let b = ClaimScope::File {
            path: "crates/cynic-protocol/src/lib.rs".to_string(),
        };
        assert!(scopes_overlap(&a, &b));
        assert!(scopes_overlap(&b, &a));
    }

    #[test]
    fn scopes_do_not_overlap_when_disjoint() {
        let a = ClaimScope::Crate {
            crate_name: "cynic-protocol".to_string(),
        };
        let b = ClaimScope::Crate {
            crate_name: "cynic-kernel".to_string(),
        };
        assert!(!scopes_overlap(&a, &b));
    }

    #[test]
    fn claim_conflicts_when_active_and_overlapping() {
        let existing = make_claim(
            ClaimScope::Crate {
                crate_name: "cynic-protocol".to_string(),
            },
            ClaimType::Exclusive,
        );
        let incoming = ClaimScope::File {
            path: "crates/cynic-protocol/src/lib.rs".to_string(),
        };
        assert!(claim_conflicts(&existing, &incoming).is_some());
    }

    #[test]
    fn claim_does_not_conflict_when_disjoint() {
        let existing = make_claim(
            ClaimScope::Crate {
                crate_name: "cynic-protocol".to_string(),
            },
            ClaimType::Exclusive,
        );
        let incoming = ClaimScope::Crate {
            crate_name: "cynic-kernel".to_string(),
        };
        assert!(claim_conflicts(&existing, &incoming).is_none());
    }

    #[test]
    fn can_preempt_shared_observe_with_exclusive() {
        let existing = make_claim(
            ClaimScope::Crate {
                crate_name: "cynic-protocol".to_string(),
            },
            ClaimType::SharedObserve,
        );
        assert!(can_preempt(&existing, ClaimType::Exclusive));
    }

    #[test]
    fn cannot_preempt_exclusive_with_shared_observe() {
        let existing = make_claim(
            ClaimScope::Crate {
                crate_name: "cynic-protocol".to_string(),
            },
            ClaimType::Exclusive,
        );
        assert!(!can_preempt(&existing, ClaimType::SharedObserve));
    }

    #[test]
    fn is_expired_at_expiry_time() {
        let claim = make_claim(ClaimScope::Fleet, ClaimType::Exclusive);
        assert!(is_expired(&claim, 5000));
        assert!(!is_expired(&claim, 4999));
    }

    #[test]
    fn can_renew_active_non_expired_claim() {
        let claim = make_claim(ClaimScope::Fleet, ClaimType::Exclusive);
        assert!(can_renew(&claim, 3000));
    }

    #[test]
    fn cannot_renew_expired_claim() {
        let claim = make_claim(ClaimScope::Fleet, ClaimType::Exclusive);
        assert!(!can_renew(&claim, 6000));
    }
}
