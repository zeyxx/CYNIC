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

/// Monotonic budgets: a child may receive a subset but never an amplification.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AgentBudgets {
    pub actions: u64,
    pub wall_seconds: u64,
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

/// Pure predicate: `true` iff a child's budgets are <= its parent's budgets.
pub fn budgets_within(parent: &AgentBudgets, child: &AgentBudgets) -> bool {
    child.actions <= parent.actions && child.wall_seconds <= parent.wall_seconds
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
