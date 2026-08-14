use serde::{Deserialize, Serialize};

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
