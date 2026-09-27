//! Constitutional law & foundational coherence contracts.
//!
//! Translates `contracts/foundation.json` and `AGENTS.md` core rules from informal
//! prose into typed, refutable Rust contracts.
//!
//! Laws codified:
//! 1. Authority Hierarchy (§Authority): 8-level strict ordering for conflict resolution.
//! 2. CYNIC Coherence Pattern (§Coherence): desired -> observe -> reconcile -> bounded action -> proof.
//! 3. Design Gate (§Design gate): 6 mandatory questions before code/contract addition.
//! 4. Hard Constitutional Boundaries (foundation.json §hard_boundaries): 11 inviolable invariants.

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

pub const CONSTITUTIONAL_LAW_SCHEMA: &str = "cynic.constitutional_law.v1";
pub const COHERENCE_PATTERN_SCHEMA: &str = "cynic.coherence_pattern.v1";
pub const DESIGN_GATE_SCHEMA: &str = "cynic.design_gate.v1";

// ===========================================================================
// 1. AUTHORITY HIERARCHY (§Authority / foundation.json)
// ===========================================================================

/// Strict 8-level authority hierarchy for resolving disagreements across artifacts.
/// Lower integer value = higher authority.
/// Mirrors the canonical authority_order shared by contracts/foundation.json,
/// infra/runtime-authority.json, and infra/directory-tree.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorityLevel {
    /// 1. Observed runtime state (ground truth from machine read-back).
    ObservedRuntimeState = 1,
    /// 2. contracts/foundation.json (constitutional foundation).
    FoundationContract = 2,
    /// 3. infra/runtime-authority.json (installed runtime authority spec).
    RuntimeAuthority = 3,
    /// 4. infra/directory-tree.json (canonical filesystem arborescence and path SSOT).
    DirectoryTree = 4,
    /// 5. infra/registry.json (topology and desired node roles).
    InfraRegistry = 5,
    /// 6. Cargo.toml workspace (canonical dependency graph).
    WorkspaceManifest = 6,
    /// 7. canonical Rust crates under crates/ (domain types and implementations).
    CanonicalRustCrates = 7,
    /// 8. infra/agent-exposure.json (agent projection surface).
    AgentExposure = 8,
}

impl AuthorityLevel {
    /// Resolves conflict between two sources. Returns the source with higher authority.
    pub fn resolve_precedence(source_a: Self, source_b: Self) -> Ordering {
        // Lower numeric value has higher priority, so cmp is reversed.
        (source_a as u8).cmp(&(source_b as u8))
    }

    /// True if `self` strictly outranks `other`.
    pub fn outranks(&self, other: Self) -> bool {
        (*self as u8) < (other as u8)
    }
}

// ===========================================================================
// 2. CYNIC COHERENCE PATTERN (§Coherence Pattern)
// ===========================================================================

/// The 5 mandatory terms of the CYNIC Coherence Pattern.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoherencePattern {
    pub schema: String,
    pub scale: CoherenceScale,
    /// What is desired (intent that can be compared without touching target).
    pub desired_state_owner: String,
    /// Read-only account of actual substrate (port + adapter + report).
    pub observe_owner: String,
    /// Pure decision from desired and observed state (policy or plan).
    pub reconcile_owner: String,
    /// Reviewable explicit mutation (argv command, privilege, --apply).
    pub bounded_action_owner: Option<String>,
    /// Evidence after action, not claim of success (versioned JSON report/test).
    pub read_back_proof_owner: String,
}

/// The 5 recursive scales where the coherence pattern applies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoherenceScale {
    RustPrimitive,
    DomainCapability,
    CommandSurface,
    HostOrNode,
    Fleet,
}

impl CoherencePattern {
    /// Validates that every mandatory term has a concrete, non-empty owner.
    pub fn validate(&self) -> Vec<String> {
        let mut violations = Vec::new();
        if self.desired_state_owner.trim().is_empty() {
            violations.push("missing_desired_state_owner".to_string());
        }
        if self.observe_owner.trim().is_empty() {
            violations.push("missing_observe_owner".to_string());
        }
        if self.reconcile_owner.trim().is_empty() {
            violations.push("missing_reconcile_owner".to_string());
        }
        if self.read_back_proof_owner.trim().is_empty() {
            violations.push("missing_read_back_proof_owner".to_string());
        }
        violations
    }

    pub fn is_coherent(&self) -> bool {
        self.validate().is_empty()
    }
}

// ===========================================================================
// 3. DESIGN GATE (§Design gate)
// ===========================================================================

/// Answers to the 6 bottom-up design gate questions before adding code/surfaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignGateAnswers {
    pub schema: String,
    /// 1. What is the desired state, and where is its authoritative representation?
    pub desired_state: String,
    /// 2. What can be observed without mutation, through which portable port?
    pub observation_port: String,
    /// 3. What pure rule selects the next safe action from desired and observed state?
    pub pure_reconciliation_rule: String,
    /// 4. If mutation is needed, what exact argv/action, privilege boundary, and rollback intent?
    pub bounded_action_spec: Option<String>,
    /// 5. What post-action read-back proves the result, and which lower layer produced it?
    pub read_back_proof: String,
    /// 6. Which existing boundary owns each answer?
    pub owning_boundary: String,
}

impl DesignGateAnswers {
    /// Evaluates whether the design gate is satisfied or has unresolved unknowns.
    pub fn evaluate(&self) -> Result<(), Vec<String>> {
        let mut missing = Vec::new();
        if self.desired_state.trim().is_empty() {
            missing.push("q1_desired_state_unanswered".to_string());
        }
        if self.observation_port.trim().is_empty() {
            missing.push("q2_observation_port_unanswered".to_string());
        }
        if self.pure_reconciliation_rule.trim().is_empty() {
            missing.push("q3_pure_reconciliation_unanswered".to_string());
        }
        if self.read_back_proof.trim().is_empty() {
            missing.push("q5_read_back_proof_unanswered".to_string());
        }
        if self.owning_boundary.trim().is_empty() {
            missing.push("q6_owning_boundary_unanswered".to_string());
        }

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
}

// ===========================================================================
// 4. HARD CONSTITUTIONAL BOUNDARIES (foundation.json)
// ===========================================================================

/// The 13 hard constitutional boundaries declared in `contracts/foundation.json`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstitutionalBoundary {
    LowLayerFirst,
    RustRuntimeAuthority,
    ProviderAgnosticDomain,
    PortableHostContract,
    RuntimeProofOverProse,
    NoContractForest,
    FractalAgentRecursion,
    SovereignErrorCodes,
    NoManualBlockerBypass,
    OrganicOpportunitiesInformPriority,
    McpIsProjectionNotBootstrap,
    ExplicitOverImplicit,
    StrictDagAndSingleResponsibility,
}

impl ConstitutionalBoundary {
    pub const ALL: &'static [Self] = &[
        Self::LowLayerFirst,
        Self::RustRuntimeAuthority,
        Self::ProviderAgnosticDomain,
        Self::PortableHostContract,
        Self::RuntimeProofOverProse,
        Self::NoContractForest,
        Self::FractalAgentRecursion,
        Self::SovereignErrorCodes,
        Self::NoManualBlockerBypass,
        Self::OrganicOpportunitiesInformPriority,
        Self::McpIsProjectionNotBootstrap,
        Self::ExplicitOverImplicit,
        Self::StrictDagAndSingleResponsibility,
    ];
    pub fn rule_text(&self) -> &'static str {
        match self {
            Self::LowLayerFirst => {
                "Kernel, host, node, protocol, MCP, CLI, storage, and service supervision take precedence over UX and product layers."
            }
            Self::RustRuntimeAuthority => {
                "Durable runtime authority must live in Rust crates, infra placement, or machine-readable contracts."
            }
            Self::ProviderAgnosticDomain => {
                "Domain logic must not depend on a specific LLM, browser, host, hypervisor, or cloud provider."
            }
            Self::PortableHostContract => {
                "Host and node capabilities must be represented as portable ports before concrete adapters."
            }
            Self::RuntimeProofOverProse => {
                "A claim about organism liveness is valid only when it can be checked through build output, service state, health/read-back, or persisted runtime evidence."
            }
            Self::NoContractForest => {
                "Do not create broad contract trees. Add a new contract only when a stable boundary cannot be expressed in Rust types, tests, registry, or foundation.json."
            }
            Self::FractalAgentRecursion => {
                "Every node may own a local recursive agent tree. Child delegations must preserve node lineage, monotonically reduce authority and budgets, and return proofs upward."
            }
            Self::SovereignErrorCodes => {
                "Every contract violation surfaces as a typed sovereign error code, never as a free-form string."
            }
            Self::NoManualBlockerBypass => {
                "A blocker is resolved only by the typed reconciliation or authority-escalation path that owns it. Ad-hoc shell fixes are rejected."
            }
            Self::OrganicOpportunitiesInformPriority => {
                "Operator priority is continuously informed by fresh, attributable organic opportunities. An opportunity is a read-only proposal, never a task or mutation authority."
            }
            Self::McpIsProjectionNotBootstrap => {
                "MCP is a replaceable projection of stable contracts. A missing MCP artifact cannot block local read-only observation."
            }
            Self::ExplicitOverImplicit => {
                "The implicit is debt. Every intent, decision, gap, budget, recipient, schema version, and migration step must be explicit, typed, and machine-readable, or recorded as an owned explicit unknown with due scale."
            }
            Self::StrictDagAndSingleResponsibility => {
                "Every crate follows the canonical cynic-<family>-<role> taxonomy. Architecture is a strict downward DAG (Layer 4 Projections -> Layer 3 Orchestrators -> Layer 2 Domains -> Layer 1 Host Substrate -> Layer 0 Pure Protocol). Cross-family concrete coupling, upward dependencies, God-object orchestrators exceeding 25k LOC, and domain I/O outside injected ports are constitutional violations."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_hierarchy_precedence() {
        // Observed runtime state outranks foundation contract
        assert!(AuthorityLevel::ObservedRuntimeState.outranks(AuthorityLevel::FoundationContract));
        // Foundation contract outranks Cargo.toml workspace
        assert!(AuthorityLevel::FoundationContract.outranks(AuthorityLevel::WorkspaceManifest));
        // Rust crates outrank agent exposure
        assert!(AuthorityLevel::CanonicalRustCrates.outranks(AuthorityLevel::AgentExposure));
        // Precedence ordering is strict
        assert_eq!(
            AuthorityLevel::resolve_precedence(
                AuthorityLevel::ObservedRuntimeState,
                AuthorityLevel::CanonicalRustCrates
            ),
            Ordering::Less // Lower number = higher priority
        );
    }

    #[test]
    fn coherence_pattern_validation() {
        let coherent = CoherencePattern {
            schema: COHERENCE_PATTERN_SCHEMA.to_string(),
            scale: CoherenceScale::DomainCapability,
            desired_state_owner: "infra/registry.json".to_string(),
            observe_owner: "cynic_host::ports::ObservationPort".to_string(),
            reconcile_owner: "cynic_host::domain::reconcile".to_string(),
            bounded_action_owner: Some("cynic_host::domain::apply".to_string()),
            read_back_proof_owner: "cynic.runtime_proof.v1".to_string(),
        };
        assert!(coherent.is_coherent());

        let incoherent = CoherencePattern {
            schema: COHERENCE_PATTERN_SCHEMA.to_string(),
            scale: CoherenceScale::RustPrimitive,
            desired_state_owner: "".to_string(),
            observe_owner: "trait Port".to_string(),
            reconcile_owner: "".to_string(),
            bounded_action_owner: None,
            read_back_proof_owner: "".to_string(),
        };
        assert!(!incoherent.is_coherent());
        assert_eq!(incoherent.validate().len(), 3);
    }

    #[test]
    fn design_gate_evaluation() {
        let valid_gate = DesignGateAnswers {
            schema: DESIGN_GATE_SCHEMA.to_string(),
            desired_state: "infra/registry.json".to_string(),
            observation_port: "ObservationPort".to_string(),
            pure_reconciliation_rule: "reconcile() pure function".to_string(),
            bounded_action_spec: Some("apply_plan() explicit --yes".to_string()),
            read_back_proof: "cynic.proof.v1".to_string(),
            owning_boundary: "crates/cynic-host".to_string(),
        };
        assert!(valid_gate.evaluate().is_ok());

        let invalid_gate = DesignGateAnswers {
            schema: DESIGN_GATE_SCHEMA.to_string(),
            desired_state: "".to_string(),
            observation_port: "".to_string(),
            pure_reconciliation_rule: "".to_string(),
            bounded_action_spec: None,
            read_back_proof: "".to_string(),
            owning_boundary: "".to_string(),
        };
        assert!(invalid_gate.evaluate().is_err());
        assert_eq!(invalid_gate.evaluate().unwrap_err().len(), 5);
    }

    #[test]
    fn all_13_constitutional_boundaries_have_non_empty_rule_text() {
        assert_eq!(ConstitutionalBoundary::ALL.len(), 13);
        for boundary in ConstitutionalBoundary::ALL {
            assert!(!boundary.rule_text().is_empty());
        }
    }
}
