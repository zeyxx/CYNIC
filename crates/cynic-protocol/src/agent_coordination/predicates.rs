//! Pure predicates for agent coordination.
//!
//! These functions are the canonical decision rules for the coordination
//! protocol. They are pure (no I/O, no side effects) and must be used by
//! all surfaces: MCP, CLI, kernel, tests.

use super::{AgentPhase, Claim, ClaimScope, ClaimStatus, ClaimType, CoordinationCode};

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

/// Whether a claim's consumed budget stays within the granted budget.
pub fn budget_within_granted(claim: &Claim) -> bool {
    claim.consumed.actions <= claim.request.budget.actions
        && claim.consumed.wall_seconds <= claim.request.budget.wall_seconds
}

/// Whether a claim can be renewed at the given timestamp.
pub fn can_renew(claim: &Claim, now_unix_ms: u64) -> bool {
    claim.status == ClaimStatus::Active && !claim.is_expired_at(now_unix_ms)
}

// ── RFC-001: Magnetic Coordination Predicates ─────────────────────────────

/// Whether a claim still exerts a repulsive force (is magnetized).
///
/// A claim is magnetized only when it is both logically Active AND has not
/// exceeded its TTL at the given wall time. An expired claim — regardless of
/// its recorded `status` — is inert and exerts zero topological repulsion.
/// This is the passive demagnetization rule from RFC-001 INV-03.
pub fn is_magnetized(claim: &Claim, now_unix_ms: u64) -> bool {
    claim.status == ClaimStatus::Active && !claim.is_expired_at(now_unix_ms)
}

#[derive(Debug, PartialEq, Eq)]
struct ScopeLocation<'a> {
    crate_name: Option<&'a str>,
    module_path: Option<&'a str>,
    file_name: Option<&'a str>,
}

fn locate_scope(scope: &ClaimScope) -> ScopeLocation<'_> {
    match scope {
        ClaimScope::Crate { crate_name } => ScopeLocation {
            crate_name: Some(crate_name.as_str()),
            module_path: None,
            file_name: None,
        },
        ClaimScope::Module {
            crate_name,
            module_path,
        } => ScopeLocation {
            crate_name: Some(crate_name.as_str()),
            module_path: Some(module_path.as_str()),
            file_name: None,
        },
        ClaimScope::File { path } => {
            let p = path.strip_prefix("./").unwrap_or(path);
            if let Some(rest) = p.strip_prefix("crates/") {
                if let Some((crate_name, after_crate)) = rest.split_once('/') {
                    if let Some(after_src) = after_crate.strip_prefix("src/") {
                        if let Some((mod_dir, file_name)) = after_src.rsplit_once('/') {
                            ScopeLocation {
                                crate_name: Some(crate_name),
                                module_path: Some(mod_dir),
                                file_name: Some(file_name),
                            }
                        } else {
                            let stem = after_src.strip_suffix(".rs").unwrap_or(after_src);
                            ScopeLocation {
                                crate_name: Some(crate_name),
                                module_path: Some(stem),
                                file_name: Some(after_src),
                            }
                        }
                    } else {
                        ScopeLocation {
                            crate_name: Some(crate_name),
                            module_path: None,
                            file_name: Some(after_crate),
                        }
                    }
                } else {
                    ScopeLocation {
                        crate_name: Some(rest),
                        module_path: None,
                        file_name: None,
                    }
                }
            } else {
                ScopeLocation {
                    crate_name: None,
                    module_path: None,
                    file_name: Some(p),
                }
            }
        }
        _ => ScopeLocation {
            crate_name: None,
            module_path: None,
            file_name: None,
        },
    }
}

/// Topological distance between two claim scopes in the project tree.
///
/// Distance levels (RFC-001 §3.1):
///   0 — same file (or lines within same file): infinite repulsion
///   1 — same module directory, different files: strong repulsion
///   2 — same crate, different modules: moderate repulsion
///   3 — different crates in same workspace: weak / negligible
///   u8::MAX — fully disjoint (fleet vs node, different domains): zero
pub fn topological_distance(a: &ClaimScope, b: &ClaimScope) -> u8 {
    if a == b {
        return 0;
    }

    let loc_a = locate_scope(a);
    let loc_b = locate_scope(b);

    match (loc_a.crate_name, loc_b.crate_name) {
        (Some(ca), Some(cb)) if ca == cb => match (loc_a.module_path, loc_b.module_path) {
            (Some(ma), Some(mb)) if ma == mb => 1,
            (Some(_), Some(_)) => 2,
            _ => 1,
        },
        (Some(_), Some(_)) => 3,
        _ => match (a, b) {
            (ClaimScope::Fleet, ClaimScope::Fleet) => 0,
            (ClaimScope::Node { node_id: na }, ClaimScope::Node { node_id: nb }) if na == nb => 0,
            _ => u8::MAX,
        },
    }
}

/// Whether two magnetized claims physically collide (hard exclusion).
///
/// Collision = both claims are magnetized AND in Crystallization phase AND
/// topological distance is <= 1 (same file or same module). RFC-001 INV-02.
pub fn claims_collide(a: &Claim, b: &Claim, now_unix_ms: u64) -> bool {
    is_magnetized(a, now_unix_ms)
        && is_magnetized(b, now_unix_ms)
        && matches!(a.phase, AgentPhase::Crystallization)
        && matches!(b.phase, AgentPhase::Crystallization)
        && topological_distance(&a.request.scope, &b.request.scope) <= 1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AgentBudgets, ClaimRequest};

    fn make_claim(scope: ClaimScope, claim_type: ClaimType) -> Claim {
        Claim {
            schema: "cynic.claim.v1".to_string(),
            claim_id: "claim_test".to_string(),
            request: ClaimRequest {
                schema: "cynic.claim_request.v1".to_string(),
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
            phase: AgentPhase::Exploration,
            consumed: AgentBudgets::default(),
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

    #[test]
    fn budget_within_granted_tracks_consumed() {
        let claim = make_claim(ClaimScope::Fleet, ClaimType::Exclusive);
        assert!(budget_within_granted(&claim));
        let claim = claim.record_consumption(&AgentBudgets {
            actions: 100,
            wall_seconds: 3600,
        });
        assert!(budget_within_granted(&claim));

        let mut over = make_claim(ClaimScope::Fleet, ClaimType::Exclusive);
        over.consumed = AgentBudgets {
            actions: 101,
            wall_seconds: 0,
        };
        assert!(!budget_within_granted(&over));
    }

    // ── RFC-001 Verification & Property Tests ───────────────────────────────

    #[test]
    fn test_topological_distance_matrix() {
        // d = 0: exact same file or scope
        let f1 = ClaimScope::File {
            path: "crates/cynic-protocol/src/agent_coordination/predicates.rs".to_string(),
        };
        assert_eq!(topological_distance(&f1, &f1), 0);

        let m1 = ClaimScope::Module {
            crate_name: "cynic-protocol".to_string(),
            module_path: "agent_coordination".to_string(),
        };
        assert_eq!(topological_distance(&m1, &m1), 0);

        // d = 1: different files in same module directory
        let f2 = ClaimScope::File {
            path: "crates/cynic-protocol/src/agent_coordination/mod.rs".to_string(),
        };
        assert_eq!(topological_distance(&f1, &f2), 1);
        assert_eq!(topological_distance(&m1, &f1), 1);

        // d = 2: different modules within same crate
        let m2 = ClaimScope::Module {
            crate_name: "cynic-protocol".to_string(),
            module_path: "memory".to_string(),
        };
        assert_eq!(topological_distance(&m1, &m2), 2);

        let f_diff_mod = ClaimScope::File {
            path: "crates/cynic-protocol/src/memory.rs".to_string(),
        };
        assert_eq!(topological_distance(&f1, &f_diff_mod), 2);

        // d = 3: different crates in same workspace
        let f_diff_crate = ClaimScope::File {
            path: "crates/cynic-kernel/src/lib.rs".to_string(),
        };
        assert_eq!(topological_distance(&f1, &f_diff_crate), 3);

        // d = u8::MAX: disjoint scopes
        let f_infra = ClaimScope::File {
            path: "infra/DEBT_TAXONOMY.md".to_string(),
        };
        assert_eq!(topological_distance(&f1, &f_infra), u8::MAX);
        assert_eq!(topological_distance(&ClaimScope::Fleet, &f1), u8::MAX);
    }

    #[test]
    fn test_test_01_topological_exclusion() {
        // RFC-001 §5 Test-01:
        // Disjoint modules (d=2) coexist in Crystallization phase.
        let mut claim_a = make_claim(
            ClaimScope::Module {
                crate_name: "cynic-protocol".to_string(),
                module_path: "agent_coordination".to_string(),
            },
            ClaimType::Exclusive,
        );
        claim_a.phase = AgentPhase::Crystallization;

        let mut claim_b = make_claim(
            ClaimScope::Module {
                crate_name: "cynic-protocol".to_string(),
                module_path: "memory".to_string(),
            },
            ClaimType::Exclusive,
        );
        claim_b.phase = AgentPhase::Crystallization;

        assert_eq!(
            topological_distance(&claim_a.request.scope, &claim_b.request.scope),
            2
        );
        assert!(!claims_collide(&claim_a, &claim_b, 2000));

        // Same file (d=0) collides in Crystallization phase.
        let mut claim_same_file = make_claim(
            ClaimScope::File {
                path: "crates/cynic-protocol/src/lib.rs".to_string(),
            },
            ClaimType::Exclusive,
        );
        claim_same_file.phase = AgentPhase::Crystallization;

        let mut claim_same_file_2 = make_claim(
            ClaimScope::File {
                path: "crates/cynic-protocol/src/lib.rs".to_string(),
            },
            ClaimType::Exclusive,
        );
        claim_same_file_2.phase = AgentPhase::Crystallization;

        assert_eq!(
            topological_distance(
                &claim_same_file.request.scope,
                &claim_same_file_2.request.scope
            ),
            0
        );
        assert!(claims_collide(&claim_same_file, &claim_same_file_2, 2000));

        // Same file does NOT collide if either agent is in Exploration phase.
        claim_same_file_2.phase = AgentPhase::Exploration;
        assert!(!claims_collide(&claim_same_file, &claim_same_file_2, 2000));
    }

    #[test]
    fn test_test_02_passive_demagnetization() {
        // RFC-001 §5 Test-02:
        // An active claim whose timestamp is older than expires_at_unix_ms is inert.
        let mut claim = make_claim(
            ClaimScope::File {
                path: "crates/cynic-protocol/src/lib.rs".to_string(),
            },
            ClaimType::Exclusive,
        );
        claim.phase = AgentPhase::Crystallization;
        claim.status = ClaimStatus::Active;
        claim.granted_at_unix_ms = 1000;
        claim.expires_at_unix_ms = 5000;

        // At t=4999, claim is active and magnetized.
        assert!(is_magnetized(&claim, 4999));

        // At t=5000, claim is passively demagnetized without mutating status field.
        assert!(!is_magnetized(&claim, 5000));

        // Incoming colliding claim at t=5000 experiences zero repulsion.
        let mut incoming = claim.clone();
        incoming.claim_id = "incoming".to_string();
        assert!(!claims_collide(&claim, &incoming, 5000));
    }

    #[test]
    fn test_test_03_monotonic_budget_invariant() {
        // RFC-001 §5 Test-03:
        // A parent with 20 remaining actions attempting to allocate 25 is rejected.
        let mut parent = make_claim(ClaimScope::Fleet, ClaimType::Exclusive);
        parent.request.budget = AgentBudgets {
            actions: 50,
            wall_seconds: 3600,
        };
        parent.consumed = AgentBudgets {
            actions: 30,
            wall_seconds: 1200,
        };
        assert_eq!(parent.remaining_budget().actions, 20);

        use crate::agent_coordination::{AGENT_FIELD_ENVELOPE_SCHEMA, AgentFieldEnvelope};

        let excessive_child = AgentFieldEnvelope {
            schema: AGENT_FIELD_ENVELOPE_SCHEMA.to_string(),
            envelope_id: "env_child_001".to_string(),
            parent_agent_id: "test".to_string(),
            parent_claim_id: "claim_test".to_string(),
            recursion_depth: 1,
            intent_vector: "Solve subtask".to_string(),
            invariant_constraints: vec![],
            output_schema: "output.v1".to_string(),
            budget: AgentBudgets {
                actions: 25,
                wall_seconds: 600,
            },
            permitted_scopes: vec![],
            forbidden_scopes: vec![],
            deadline_unix_ms: 10000,
        };

        let result = excessive_child.validate_against_parent(&parent);
        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.contains(&"child_budget_actions_exceeds_parent".to_string()));

        // Valid child within 20 remaining actions passes.
        let mut valid_child = excessive_child.clone();
        valid_child.budget.actions = 20;
        assert!(valid_child.validate_against_parent(&parent).is_ok());
    }
}
