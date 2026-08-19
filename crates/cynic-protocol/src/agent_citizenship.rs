//! Agent citizenship (domain).
//!
//! Canonical home for agent *citoyenneté*: a bounded, monotonic declaration of
//! an agent's identity, authority scope, budgets, and residual unknowns. This
//! is the protocol-level contract that every harness-launch surface
//! (`cynic mask`, `cynic runtime inference-start`, MCP tool admission)
//! projects from — policy is portable; the trait owns no substrate.
//!
//! Structural invariant: citizenship is data + a pure predicate over it, not
//! a runtime executor.

use crate::SecurityEvidenceState;
use crate::error_catalog::CynicErrorCode;

/// Schema anchor for citizenship records.
pub const AGENT_CITIZENSHIP_SCHEMA: &str = "cynic.agent_citizenship.v1";

/// Hierarchical authority scope a citizenship record may be bound to.
///
/// Mirrors `cynic_protocol::coordination_lease::LeaseScope` but carried as a
/// value so a citizenship record can be serialized standalone.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AgentScope {
    Fleet,
    Node,
    Crate,
    Module,
    File,
}

impl AgentScope {
    /// Specificity rank: `Fleet` is broadest, `File` narrowest.
    ///
    /// Overlap is decided by containment, not equality: a `Crate` claim already
    /// covers a `File` inside it, so treating the two as disjoint would let two
    /// agents mutate the same source through differently-named scopes.
    pub fn specificity(&self) -> u8 {
        match self {
            Self::Fleet => 0,
            Self::Node => 1,
            Self::Crate => 2,
            Self::Module => 3,
            Self::File => 4,
        }
    }

    /// Whether `self` contains or equals `other` in the scope hierarchy.
    pub fn contains(&self, other: &Self) -> bool {
        self.specificity() <= other.specificity()
    }
}

/// Enforcement mode of a citizen.
///
/// Per AGENTS.md: `baseline` claims are advisory/cooperative, `cynic` claims
/// are durably enforced and may preempt baseline. The mode is therefore part of
/// admission, not a label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    Baseline,
    Cynic,
}

impl AgentMode {
    /// Whether `self` may preempt `other`.
    ///
    /// Cynic preempts baseline. Same-mode collisions are resolved first-wins by
    /// the claim store, never by preemption — otherwise two baseline agents
    /// could keep evicting each other.
    pub fn preempts(self, other: Self) -> bool {
        matches!((self, other), (Self::Cynic, Self::Baseline))
    }
}

/// Monotonic budgets: a child may receive a subset but never an amplification.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AgentBudgets {
    pub actions: u64,
    pub wall_seconds: u64,
}

/// Verdict of a mutation admission request.
///
/// Returns the violated codes rather than a bool: a refused mutation must tell
/// the operator which contract blocked it, and `Refused` carries the same
/// canonical codes the health signal and gallery layers consume.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum MutationAdmission {
    /// Admitted: claim held, budgets sufficient, evidence proven.
    Admitted,
    /// Refused with the canonical codes that blocked it.
    Refused { codes: Vec<CynicErrorCode> },
}

impl MutationAdmission {
    pub fn is_admitted(&self) -> bool {
        matches!(self, Self::Admitted)
    }

    /// Codes that blocked the mutation; empty when admitted.
    pub fn codes(&self) -> &[CynicErrorCode] {
        match self {
            Self::Admitted => &[],
            Self::Refused { codes } => codes,
        }
    }
}

/// Agent citizenship record.
///
/// Immutable declaration of who an agent is, what it may touch, how much it may
/// consume, and what risk is still outstanding. Produced by reconciliation of
/// a desired state + observed identity, and consumed by bounded-action surfaces
/// as proof of admitted authority.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AgentCitizenshipRecord {
    pub schema: &'static str,
    pub agent_id: String,
    pub scope: AgentScope,
    pub budgets: AgentBudgets,
    /// Residual, machine-readable risk still outstanding for this run.
    pub residual_risk: Option<String>,
    /// Read-back proof token binding this record to an observed identity.
    pub evidence: Option<SecurityEvidenceState>,
}

/// Pure decision: may this citizen mutate `target_scope`?
///
/// This is the single canonical admission rule. Surfaces project it; they must
/// not re-derive it locally (invariant #6).
///
/// `held_claim` is the scope the agent actually holds, if any. `None` means no
/// claim — and no claim means no mutation, which is the whole point of the
/// coordination contract. A store that cannot be read is *not* an implicit
/// grant: the caller passes `store_available = false` and the mutation is
/// refused with `AgentCoordinationStoreUnavailable`, never silently allowed.
pub fn admit_mutation(
    record: &AgentCitizenshipRecord,
    held_claim: Option<&AgentScope>,
    requested: &AgentBudgets,
    target_scope: &AgentScope,
    store_available: bool,
) -> MutationAdmission {
    let mut codes = Vec::new();

    // An unreadable coordination store is an explicit unknown, not permission.
    if !store_available {
        codes.push(CynicErrorCode::AgentCoordinationStoreUnavailable);
    }

    match held_claim {
        None => codes.push(CynicErrorCode::AgentMutationWithoutClaim),
        Some(held) => {
            // The held claim must contain the target: holding a File claim does
            // not authorize mutating the whole Crate.
            if !held.contains(target_scope) {
                codes.push(CynicErrorCode::AgentScopeConflict);
            }
        }
    }

    if !budgets_within(&record.budgets, requested) {
        codes.push(CynicErrorCode::AgentBudgetExceeded);
    }

    if codes.is_empty() {
        MutationAdmission::Admitted
    } else {
        MutationAdmission::Refused { codes }
    }
}

/// Pure predicate: `true` iff a child's budgets are <= its parent's budgets.
pub fn budgets_within(parent: &AgentBudgets, child: &AgentBudgets) -> bool {
    child.actions <= parent.actions && child.wall_seconds <= parent.wall_seconds
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(actions: u64, wall_seconds: u64) -> AgentCitizenshipRecord {
        AgentCitizenshipRecord {
            schema: AGENT_CITIZENSHIP_SCHEMA,
            agent_id: "hermes:main".to_string(),
            scope: AgentScope::Crate,
            budgets: AgentBudgets {
                actions,
                wall_seconds,
            },
            residual_risk: None,
            evidence: None,
        }
    }

    fn budgets(actions: u64, wall_seconds: u64) -> AgentBudgets {
        AgentBudgets {
            actions,
            wall_seconds,
        }
    }

    #[test]
    fn claim_containing_target_is_admitted() {
        let admission = admit_mutation(
            &record(50, 3600),
            Some(&AgentScope::Crate),
            &budgets(10, 600),
            &AgentScope::File,
            true,
        );
        assert!(admission.is_admitted());
        assert!(admission.codes().is_empty());
    }

    /// No claim means no mutation — the entire point of the coordination
    /// contract. Silence must never read as permission.
    #[test]
    fn mutation_without_claim_is_refused() {
        let admission = admit_mutation(
            &record(50, 3600),
            None,
            &budgets(1, 1),
            &AgentScope::File,
            true,
        );
        assert!(!admission.is_admitted());
        assert!(
            admission
                .codes()
                .contains(&CynicErrorCode::AgentMutationWithoutClaim)
        );
    }
    /// An unreadable coordination store must refuse the mutation, never grant
    /// it. "If coordination storage is unavailable, say so explicitly and do
    /// not pretend the organism remembered the session."
    #[test]
    fn unavailable_store_refuses_even_with_valid_claim() {
        let admission = admit_mutation(
            &record(50, 3600),
            Some(&AgentScope::Crate),
            &budgets(1, 1),
            &AgentScope::File,
            false,
        );
        assert!(!admission.is_admitted());
        assert!(
            admission
                .codes()
                .contains(&CynicErrorCode::AgentCoordinationStoreUnavailable)
        );
    }

    /// Holding a narrow claim does not authorize a broader mutation.
    #[test]
    fn narrow_claim_cannot_mutate_broader_scope() {
        let admission = admit_mutation(
            &record(50, 3600),
            Some(&AgentScope::File),
            &budgets(1, 1),
            &AgentScope::Crate,
            true,
        );
        assert!(!admission.is_admitted());
        assert!(
            admission
                .codes()
                .contains(&CynicErrorCode::AgentScopeConflict)
        );
    }

    /// Budgets are monotonic: a citizen may never amplify its own authority.
    #[test]
    fn budget_amplification_is_refused() {
        let admission = admit_mutation(
            &record(10, 60),
            Some(&AgentScope::Crate),
            &budgets(999, 999),
            &AgentScope::File,
            true,
        );
        assert!(!admission.is_admitted());
        assert!(
            admission
                .codes()
                .contains(&CynicErrorCode::AgentBudgetExceeded)
        );
    }
    /// Cynic preempts baseline; same-mode collisions are never preemption.
    #[test]
    fn cynic_preempts_baseline_only() {
        assert!(AgentMode::Cynic.preempts(AgentMode::Baseline));
        assert!(!AgentMode::Baseline.preempts(AgentMode::Cynic));
        assert!(!AgentMode::Baseline.preempts(AgentMode::Baseline));
        assert!(!AgentMode::Cynic.preempts(AgentMode::Cynic));
    }

    /// Every violated contract must surface at once: reporting only the first
    /// would make the operator repair one blocker and hit the next blind.
    #[test]
    fn all_violated_contracts_are_reported_together() {
        let admission = admit_mutation(
            &record(1, 1),
            None,
            &budgets(999, 999),
            &AgentScope::Crate,
            false,
        );
        let codes = admission.codes();
        assert!(codes.contains(&CynicErrorCode::AgentCoordinationStoreUnavailable));
        assert!(codes.contains(&CynicErrorCode::AgentMutationWithoutClaim));
        assert!(codes.contains(&CynicErrorCode::AgentBudgetExceeded));
        assert_eq!(codes.len(), 3);
    }

    #[test]
    fn scope_containment_follows_hierarchy() {
        assert!(AgentScope::Fleet.contains(&AgentScope::File));
        assert!(AgentScope::Crate.contains(&AgentScope::Module));
        assert!(AgentScope::File.contains(&AgentScope::File));
        assert!(!AgentScope::Module.contains(&AgentScope::Node));
    }

    #[test]
    fn admission_round_trips_through_json() {
        let refused = MutationAdmission::Refused {
            codes: vec![CynicErrorCode::AgentMutationWithoutClaim],
        };
        let json = serde_json::to_string(&refused).expect("serialize");
        let back: MutationAdmission = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, refused);
    }
}

/// Trait: any surface that carries agent citizenship must expose its intent
/// and answer the admission predicate. Implementors (mask, runtime, MCP
/// tool-callers) project canonical decisions; they do not recompute policy.
pub trait Citoyenneté {
    /// The declared identity the surface is acting as.
    fn agent_id(&self) -> &str;

    /// The scope the identity is authorized for.
    fn scope(&self) -> AgentScope;

    /// The monotonic budgets carried by this surface.
    fn budgets(&self) -> AgentBudgets;

    /// Admission gate: can this citizen perform `requested`?
    /// Implementations consult `residual_risk` and `evidence` (LeaseVerification).
    fn is_admitted(&self, requested: AgentBudgets) -> bool {
        budgets_within(&self.budgets(), &requested)
    }
}
