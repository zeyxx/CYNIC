use crate::SecurityEvidenceState;
use crate::error_catalog::CynicErrorCode;

pub const AGENT_CITIZENSHIP_SCHEMA: &str = "cynic.agent_citizenship.v1";

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AgentScope {
    Fleet,
    Node,
    Crate,
    Module,
    File,
}

impl AgentScope {
    pub fn specificity(&self) -> u8 {
        match self {
            Self::Fleet => 0,
            Self::Node => 1,
            Self::Crate => 2,
            Self::Module => 3,
            Self::File => 4,
        }
    }

    pub fn contains(&self, other: &Self) -> bool {
        self.specificity() <= other.specificity()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    Baseline,
    Cynic,
}

impl AgentMode {
    pub fn preempts(self, other: Self) -> bool {
        matches!((self, other), (Self::Cynic, Self::Baseline))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AgentBudgets {
    pub actions: u64,
    pub wall_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum MutationAdmission {
    Admitted,
    Refused { codes: Vec<CynicErrorCode> },
}

impl MutationAdmission {
    pub fn is_admitted(&self) -> bool {
        matches!(self, Self::Admitted)
    }

    pub fn codes(&self) -> &[CynicErrorCode] {
        match self {
            Self::Admitted => &[],
            Self::Refused { codes } => codes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AgentCitizenshipRecord {
    pub schema: &'static str,
    pub agent_id: String,
    pub scope: AgentScope,
    pub budgets: AgentBudgets,
    pub residual_risk: Option<String>,
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

    if !store_available {
        codes.push(CynicErrorCode::AgentCoordinationStoreUnavailable);
    }

    match held_claim {
        None => codes.push(CynicErrorCode::AgentMutationWithoutClaim),
        Some(held) => {
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
        assert!(admission
            .codes()
            .contains(&CynicErrorCode::AgentMutationWithoutClaim));
    }

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
        assert!(admission
            .codes()
            .contains(&CynicErrorCode::AgentCoordinationStoreUnavailable));
    }

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
        assert!(admission
            .codes()
            .contains(&CynicErrorCode::AgentScopeConflict));
    }

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
        assert!(admission
            .codes()
            .contains(&CynicErrorCode::AgentBudgetExceeded));
    }

    #[test]
    fn cynic_preempts_baseline_only() {
        assert!(AgentMode::Cynic.preempts(AgentMode::Baseline));
        assert!(!AgentMode::Baseline.preempts(AgentMode::Cynic));
        assert!(!AgentMode::Baseline.preempts(AgentMode::Baseline));
        assert!(!AgentMode::Cynic.preempts(AgentMode::Cynic));
    }

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

pub trait Citoyenneté {
    fn agent_id(&self) -> &str;
    fn scope(&self) -> AgentScope;
    fn budgets(&self) -> AgentBudgets;
    fn is_admitted(&self, requested: AgentBudgets) -> bool {
        budgets_within(&self.budgets(), &requested)
    }
}
