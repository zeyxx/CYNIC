use serde::{Deserialize, Serialize};

pub const LEASE_VERIFICATION_SCHEMA: &str = "cynic.coord.lease_verification.v1";
pub const AGENT_CLAIMS_SCHEMA: &str = "cynic.agent_claims.v2";

// ── Legacy lease types (v1 — used by CoordPort/SurrealHttpStorage) ────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LeaseGrant {
    pub lease_id: String,
    pub fencing_token: u64,
    pub expires_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LeaseGuard {
    pub agent_id: String,
    pub target: String,
    pub lease_id: String,
    pub fencing_token: u64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LeaseAction {
    Renew,
    Release,
    Finalize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LeaseRejectionCode {
    GuardRejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LeaseMutationOutcome {
    Applied {
        action: LeaseAction,
        target: String,
        fencing_token: u64,
        expires_at: Option<String>,
        proof_ref: Option<String>,
    },
    Rejected {
        action: LeaseAction,
        target: String,
        fencing_token: u64,
        code: LeaseRejectionCode,
    },
}

impl LeaseMutationOutcome {
    pub fn applied(
        action: LeaseAction,
        target: impl Into<String>,
        fencing_token: u64,
        expires_at: Option<String>,
        proof_ref: Option<String>,
    ) -> Self {
        Self::Applied {
            action,
            target: target.into(),
            fencing_token,
            expires_at,
            proof_ref,
        }
    }

    pub fn rejected(action: LeaseAction, target: impl Into<String>, fencing_token: u64) -> Self {
        Self::Rejected {
            action,
            target: target.into(),
            fencing_token,
            code: LeaseRejectionCode::GuardRejected,
        }
    }

    pub fn is_applied(&self) -> bool {
        matches!(self, Self::Applied { .. })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LeaseVerification {
    pub schema: String,
    pub status: String,
    pub guard: LeaseGuard,
    pub expires_at: Option<String>,
    pub blockers: Vec<String>,
}

impl LeaseVerification {
    pub fn proved(guard: LeaseGuard, expires_at: Option<String>) -> Self {
        Self {
            schema: LEASE_VERIFICATION_SCHEMA.to_string(),
            status: "proved".to_string(),
            guard,
            expires_at,
            blockers: Vec::new(),
        }
    }

    pub fn rejected(guard: LeaseGuard) -> Self {
        Self {
            schema: LEASE_VERIFICATION_SCHEMA.to_string(),
            status: "rejected".to_string(),
            guard,
            expires_at: None,
            blockers: vec!["lease_expired_superseded_or_guard_mismatch".to_string()],
        }
    }

    pub fn unavailable(guard: LeaseGuard, blocker: impl Into<String>) -> Self {
        Self {
            schema: LEASE_VERIFICATION_SCHEMA.to_string(),
            status: "unavailable".to_string(),
            guard,
            expires_at: None,
            blockers: vec![blocker.into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LeaseConflict {
    pub agent_id: String,
    pub claimed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum TargetClaimOutcome {
    Claimed {
        target: String,
        grant: LeaseGrant,
    },
    Conflict {
        target: String,
        conflicts: Vec<LeaseConflict>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BatchClaimResult {
    pub outcomes: Vec<TargetClaimOutcome>,
}

impl BatchClaimResult {
    pub fn claimed_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|outcome| matches!(outcome, TargetClaimOutcome::Claimed { .. }))
            .count()
    }

    pub fn conflict_count(&self) -> usize {
        self.outcomes
            .iter()
            .filter(|outcome| matches!(outcome, TargetClaimOutcome::Conflict { .. }))
            .count()
    }
}

// ── v2 agent claims model (baseline/cynic coordination) ───────────────────

/// Agent execution mode.
///
/// `baseline` — external agent (Hermes, Codex, CLI) claims work directly.
///   No durable scheduler enforcement; claims are best-effort, cooperative.
///
/// `cynic` — native workload scheduled by the organism's scheduler.
///   Claims are durably enforced; cynic claims may preempt baseline claims.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    Baseline,
    Cynic,
}

/// Hierarchical scope of a work claim.
///
/// Scaling: a `Fleet` claim may decompose into `Node` claims, each
/// into `Crate` claims, each into `Module` / `File` claims.
/// Parent budget must be >= sum of children budgets (monotonic).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClaimScope {
    Fleet {
        host_pattern: Option<String>,
    },
    Node {
        node_id: String,
    },
    Crate {
        crate_name: String,
    },
    Module {
        crate_name: String,
        module_path: String,
    },
    File {
        path: String,
    },
}

/// Budget granted for a claim — monotonic: children sum must be <= parent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimBudget {
    pub actions: u32,
    pub wall_seconds: u64,
}

/// Temporal bounds of a claim. Expires without heartbeat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimTemporal {
    pub started_at_unix_s: u64,
    pub ttl_seconds: u64,
    pub heartbeat_at_unix_s: u64,
    pub expires_at_unix_s: u64,
}

/// A single agent work claim — the unit of coordination concurrency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentClaim {
    pub claim_id: String,
    pub agent_id: String,
    pub mode: AgentMode,
    pub scope: ClaimScope,
    pub budget: ClaimBudget,
    pub consumed: ClaimBudget,
    pub temporal: ClaimTemporal,
    pub priority: u32,
    pub parent_claim_id: Option<String>,
    pub dependencies: Vec<String>,
    pub status: ClaimStatus,
    pub preempted_by: Option<String>,
    pub branch: Option<String>,
    pub baseline_ref: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Claiming,
    Active,
    Completing,
    Done,
    Preempted,
    Expired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentClaimsFile {
    pub schema: String,
    pub revision: u64,
    pub claims: Vec<AgentClaim>,
    pub merge_queue: Vec<MergeQueueEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeQueueEntry {
    pub claim_id: String,
    pub branch: String,
    pub baseline_ref: String,
    pub status: MergeStatus,
    pub conflict_resolution: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStatus {
    Waiting,
    Merging,
    Conflict,
    Done,
}

/// Conflict detected when two claims target overlapping scopes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimConflictV2 {
    pub claim_id: String,
    pub agent_id: String,
    pub mode: AgentMode,
    pub reason: ClaimConflictReason,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClaimConflictReason {
    ScopeOverlap { scope: String },
    ParentPreemptedByCynic { cynic_claim_id: String },
    BudgetExhausted { available: u32, requested: u32 },
    DependencyNotMet { dependency_id: String },
}

/// Pure predicates — no I/O, no mutation, fully unit-testable.
pub mod predicates {
    use super::*;

    /// Check if two scopes overlap (same scope, or one contains the other).
    pub fn scopes_overlap(a: &ClaimScope, b: &ClaimScope) -> bool {
        match (a, b) {
            (ClaimScope::Fleet { .. }, _) => true,
            (_, ClaimScope::Fleet { .. }) => true,
            (ClaimScope::Node { node_id: a_id }, ClaimScope::Node { node_id: b_id }) => {
                a_id == b_id
            }
            (
                ClaimScope::Crate {
                    crate_name: a_name,
                },
                ClaimScope::Crate {
                    crate_name: b_name,
                },
            ) => a_name == b_name,
            (
                ClaimScope::Module {
                    crate_name: a_crate,
                    module_path: a_path,
                },
                ClaimScope::Module {
                    crate_name: b_crate,
                    module_path: b_path,
                },
            ) => a_crate == b_crate && a_path == b_path,
            (ClaimScope::File { path: a_path }, ClaimScope::File { path: b_path }) => {
                a_path == b_path
            }
            (
                ClaimScope::Crate { crate_name },
                ClaimScope::Module {
                    crate_name: m_crate,
                    ..
                },
            )
            | (
                ClaimScope::Module {
                    crate_name: m_crate,
                    ..
                },
                ClaimScope::Crate { crate_name },
            ) => crate_name == m_crate,
            (ClaimScope::Crate { crate_name }, ClaimScope::File { path })
            | (ClaimScope::File { path }, ClaimScope::Crate { crate_name }) => {
                path.starts_with(&format!("crates/{crate_name}/"))
            }
            (ClaimScope::Module { module_path, .. }, ClaimScope::File { path })
            | (ClaimScope::File { path }, ClaimScope::Module { module_path, .. }) => {
                path.starts_with(module_path)
            }
            (ClaimScope::Node { .. }, _) | (_, ClaimScope::Node { .. }) => {
                // Different scope kinds — node does not conflict with crate/file
                false
            }
        }
    }

    /// Check if a claim has expired (current time > expires_at).
    pub fn is_expired(temporal: &ClaimTemporal, now_unix_s: u64) -> bool {
        now_unix_s > temporal.expires_at_unix_s
    }

    /// Check if a heartbeat is stale (now - heartbeat_at > ttl).
    pub fn is_heartbeat_stale(temporal: &ClaimTemporal, now_unix_s: u64) -> bool {
        now_unix_s.saturating_sub(temporal.heartbeat_at_unix_s) > temporal.ttl_seconds
    }

    /// Check if cynic can preempt this claim.
    /// Cynic can always preempt baseline. Cynic cannot preempt cynic.
    pub fn can_preempt(proclaimer_mode: AgentMode, target_mode: AgentMode) -> bool {
        matches!(proclaimer_mode, AgentMode::Cynic)
            && matches!(target_mode, AgentMode::Baseline)
    }

    /// Check if budget is monotonic: consumed <= granted for both dimensions.
    pub fn budget_within_granted(consumed: &ClaimBudget, granted: &ClaimBudget) -> bool {
        consumed.actions <= granted.actions && consumed.wall_seconds <= granted.wall_seconds
    }
}

#[cfg(test)]
mod tests {
    use super::predicates::*;
    use super::*;

    #[test]
    fn file_scope_overlaps_itself() {
        let a = ClaimScope::File {
            path: "crates/cynic-kernel/src/lib.rs".into(),
        };
        let b = ClaimScope::File {
            path: "crates/cynic-kernel/src/lib.rs".into(),
        };
        assert!(scopes_overlap(&a, &b));
    }

    #[test]
    fn crate_scope_contains_file_scope() {
        let a = ClaimScope::Crate {
            crate_name: "cynic-kernel".into(),
        };
        let b = ClaimScope::File {
            path: "crates/cynic-kernel/src/lib.rs".into(),
        };
        assert!(scopes_overlap(&a, &b));
        assert!(scopes_overlap(&b, &a));
    }

    #[test]
    fn different_crates_no_overlap() {
        let a = ClaimScope::Crate {
            crate_name: "cynic-kernel".into(),
        };
        let b = ClaimScope::Crate {
            crate_name: "cynic-inference".into(),
        };
        assert!(!scopes_overlap(&a, &b));
    }

    #[test]
    fn fleet_overlaps_everything() {
        let fleet = ClaimScope::Fleet { host_pattern: None };
        let file = ClaimScope::File {
            path: "src/lib.rs".into(),
        };
        assert!(scopes_overlap(&fleet, &file));
    }

    #[test]
    fn cynic_can_preempt_baseline() {
        assert!(can_preempt(AgentMode::Cynic, AgentMode::Baseline));
    }

    #[test]
    fn baseline_cannot_preempt_cynic() {
        assert!(!can_preempt(AgentMode::Baseline, AgentMode::Cynic));
    }

    #[test]
    fn baseline_cannot_preempt_baseline() {
        assert!(!can_preempt(AgentMode::Baseline, AgentMode::Baseline));
    }

    #[test]
    fn expired_claim_is_detected() {
        let temporal = ClaimTemporal {
            started_at_unix_s: 1000,
            ttl_seconds: 300,
            heartbeat_at_unix_s: 1200,
            expires_at_unix_s: 1500,
        };
        assert!(is_expired(&temporal, 1501));
        assert!(!is_expired(&temporal, 1500));
    }

    #[test]
    fn stale_heartbeat_is_detected() {
        let temporal = ClaimTemporal {
            started_at_unix_s: 1000,
            ttl_seconds: 300,
            heartbeat_at_unix_s: 1200,
            expires_at_unix_s: 2000,
        };
        assert!(is_heartbeat_stale(&temporal, 1501));
        assert!(!is_heartbeat_stale(&temporal, 1499));
    }

    #[test]
    fn budget_within_granted() {
        let granted = ClaimBudget {
            actions: 50,
            wall_seconds: 3600,
        };
        let consumed = ClaimBudget {
            actions: 30,
            wall_seconds: 1800,
        };
        assert!(budget_within_granted(&consumed, &granted));
    }

    #[test]
    fn budget_exceeds_granted() {
        let granted = ClaimBudget {
            actions: 50,
            wall_seconds: 3600,
        };
        let consumed = ClaimBudget {
            actions: 51,
            wall_seconds: 1800,
        };
        assert!(!budget_within_granted(&consumed, &granted));
    }

    #[test]
    fn lease_verification_proved_has_empty_blockers() {
        let guard = LeaseGuard {
            agent_id: "a".into(),
            target: "t".into(),
            lease_id: "l".into(),
            fencing_token: 1,
        };
        let v = LeaseVerification::proved(guard, None);
        assert_eq!(v.status, "proved");
        assert!(v.blockers.is_empty());
    }

    #[test]
    fn batch_claimed_and_conflict_counts() {
        let batch = BatchClaimResult {
            outcomes: vec![
                TargetClaimOutcome::Claimed {
                    target: "a".into(),
                    grant: LeaseGrant {
                        lease_id: "l1".into(),
                        fencing_token: 1,
                        expires_at: "".into(),
                    },
                },
                TargetClaimOutcome::Conflict {
                    target: "b".into(),
                    conflicts: vec![],
                },
            ],
        };
        assert_eq!(batch.claimed_count(), 1);
        assert_eq!(batch.conflict_count(), 1);
    }
}
