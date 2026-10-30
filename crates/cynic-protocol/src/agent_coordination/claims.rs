use serde::{Deserialize, Serialize};

use super::*;

// ── Claim (Phase 3 — CLAIM) ──────────────────────────────────────────────

/// Hierarchical claim scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ClaimScope {
    Fleet,
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

impl ClaimScope {
    pub fn specificity(&self) -> u8 {
        match self {
            Self::Fleet => 0,
            Self::Node { .. } => 1,
            Self::Crate { .. } => 2,
            Self::Module { .. } => 3,
            Self::File { .. } => 4,
        }
    }

    pub fn contains(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Fleet, _) => true,
            (Self::Node { node_id: a }, Self::Node { node_id: b }) => a == b,
            (Self::Node { .. }, _) => true,
            (Self::Crate { crate_name: a }, Self::Crate { crate_name: b }) => a == b,
            (Self::Crate { crate_name: a }, Self::Module { crate_name: b, .. }) => a == b,
            (Self::Crate { crate_name: a }, Self::File { path: b }) => {
                b.starts_with(a) || b.contains(&format!("{}/", a)) || b.contains(&format!("{}-", a))
            }
            (
                Self::Module {
                    crate_name: a,
                    module_path: b,
                },
                Self::Module {
                    crate_name: c,
                    module_path: d,
                },
            ) => a == c && b == d,
            (
                Self::Module {
                    crate_name: a,
                    module_path: b,
                },
                Self::File { path: c },
            ) => {
                c.starts_with(&format!("{}/{}/", a, b))
                    || c.starts_with(&format!("{}/src/{}/", a, b))
            }
            (Self::File { path: a }, Self::File { path: b }) => a == b,
            _ => false,
        }
    }
}

/// Type of claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimType {
    Exclusive,
    SharedObserve,
}

/// Status of a claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Active,
    Expired,
    Released,
    Preempted,
    Blocked,
}

/// Request to claim a scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClaimRequest {
    pub schema: String,
    pub agent_id: String,
    pub session_id: String,
    pub scope: ClaimScope,
    pub claim_type: ClaimType,
    pub budget: AgentBudgets,
    pub ttl_seconds: u64,
    pub max_lease_window_ms: u64,
    pub parent_claim_id: Option<String>,
}

impl ClaimRequest {
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut codes = Vec::new();

        if self.schema != CLAIM_REQUEST_SCHEMA {
            codes.push("claim_request_schema_invalid".to_string());
        }
        if self.agent_id.trim().is_empty() {
            codes.push("agent_id_empty".to_string());
        }
        if self.session_id.trim().is_empty() {
            codes.push("session_id_empty".to_string());
        }
        if self.ttl_seconds == 0 {
            codes.push("ttl_seconds_zero".to_string());
        }

        if codes.is_empty() { Ok(()) } else { Err(codes) }
    }
}

/// A granted claim with lease and fencing token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Claim {
    pub schema: String,
    pub claim_id: String,
    pub request: ClaimRequest,
    pub lease_id: String,
    pub fencing_token: u64,
    pub granted_at_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub max_lease_window_ms: u64,
    pub status: ClaimStatus,
    pub preempted_by: Option<String>,
    /// Thermodynamic phase — RFC-001 §3.4. Defaults to Exploration.
    #[serde(default = "AgentPhase::default_exploration")]
    pub phase: AgentPhase,
    /// Budget actually consumed so far. Must never exceed `request.budget`
    /// (monotonic, saturating). Mirrors `consumed` in
    /// `cynic.agent_claims.v2`.
    #[serde(default)]
    pub consumed: AgentBudgets,
}

impl Claim {
    /// Remaining budget = granted budget − consumed budget (saturating).
    pub fn remaining_budget(&self) -> AgentBudgets {
        AgentBudgets {
            actions: self
                .request
                .budget
                .actions
                .saturating_sub(self.consumed.actions),
            wall_seconds: self
                .request
                .budget
                .wall_seconds
                .saturating_sub(self.consumed.wall_seconds),
        }
    }

    /// Pure consumption step: returns a new claim with the delta added to
    /// `consumed`, saturating at the granted budget. Refuses nothing here —
    /// admission is decided by [`admit_mutation`]; this only records.
    pub fn record_consumption(&self, delta: &AgentBudgets) -> Claim {
        let mut next = self.clone();
        next.consumed.actions = next.consumed.actions.saturating_add(delta.actions);
        next.consumed.wall_seconds = next
            .consumed
            .wall_seconds
            .saturating_add(delta.wall_seconds);
        if next.consumed.actions > next.request.budget.actions {
            next.consumed.actions = next.request.budget.actions;
        }
        if next.consumed.wall_seconds > next.request.budget.wall_seconds {
            next.consumed.wall_seconds = next.request.budget.wall_seconds;
        }
        next
    }

    pub fn is_active(&self) -> bool {
        matches!(self.status, ClaimStatus::Active)
    }

    pub fn is_expired_at(&self, now_unix_ms: u64) -> bool {
        now_unix_ms >= self.expires_at_unix_ms
    }

    pub fn is_heartbeat_stale(&self, now_unix_ms: u64) -> bool {
        let ttl_ms = self
            .expires_at_unix_ms
            .saturating_sub(self.granted_at_unix_ms);
        let stale_threshold = self.granted_at_unix_ms.saturating_add(ttl_ms * 8 / 10);
        now_unix_ms >= stale_threshold
    }
}

