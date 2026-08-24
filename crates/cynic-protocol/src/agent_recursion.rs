use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

mod evidence;

pub use evidence::{
    AgentEvidenceDependency, AgentEvidenceLineage, AgentEvidenceRef, AgentEvidenceRelation,
    classify_agent_evidence_dependency, relate_agent_evidence,
};
use evidence::{lineage_unknowns, valid_sha256_digest};

pub const AGENT_DELEGATION_ENVELOPE_SCHEMA: &str = "cynic.agent_delegation_envelope.v1";
pub const AGENT_COMPLETION_ENVELOPE_SCHEMA: &str = "cynic.agent_completion_envelope.v1";
pub const AGENT_RECURSION_ADMISSION_SCHEMA: &str = "cynic.agent_recursion_admission.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCapabilityGrant {
    pub id: String,
    pub mutates: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentBudget {
    pub context_tokens: u64,
    pub actions: u32,
    pub children: u16,
    pub wall_time_ms: u64,
    /// Measured cpu milliseconds for consumed budgets (close records).
    /// None = not measured — an explicit unknown, never zero. Reserved
    /// budgets (delegation) leave it None until a completion reports it.
    /// Schema evolution: added with #[serde(default)] so v1 payloads
    /// without the field still deserialize.
    #[serde(default)]
    pub cpu_ms: Option<u64>,
}

impl AgentBudget {
    fn fits_within(&self, parent: &Self) -> bool {
        self.context_tokens <= parent.context_tokens
            && self.actions <= parent.actions
            && self.children <= parent.children
            && self.wall_time_ms <= parent.wall_time_ms
            && match (self.cpu_ms, parent.cpu_ms) {
                // Unmeasured consumption is an unknown of the receipt, not
                // a violation (same rule as EnergyConsumed::within).
                (None, _) => true,
                (Some(_), None) => false,
                (Some(c), Some(p)) => c <= p,
            }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentDelegationEnvelope {
    pub schema: String,
    pub delegation_id: String,
    pub node_id: String,
    pub agent_id: String,
    pub parent_agent_id: Option<String>,
    pub depth: u16,
    pub objective: String,
    pub authority_snapshot: AgentEvidenceRef,
    #[serde(default)]
    pub context_refs: Vec<AgentEvidenceRef>,
    #[serde(default)]
    pub capabilities: Vec<AgentCapabilityGrant>,
    pub budget: AgentBudget,
    #[serde(default)]
    pub expected_proofs: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCompletionStatus {
    Completed,
    Blocked,
    Failed,
    Degraded,
    RecoveredUnattributed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentCompletionEnvelope {
    pub schema: String,
    pub delegation_id: String,
    pub node_id: String,
    pub agent_id: String,
    pub parent_agent_id: Option<String>,
    pub status: AgentCompletionStatus,
    #[serde(default)]
    pub evidence_refs: Vec<AgentEvidenceRef>,
    #[serde(default)]
    pub unknowns: Vec<String>,
    pub consumed_budget: AgentBudget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRecursionViolation {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum AgentRecursionDecision {
    Allow,
    Deny {
        violations: Vec<AgentRecursionViolation>,
    },
    Unknown {
        missing_evidence: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRecursionAdmission {
    pub schema: String,
    pub delegation_id: String,
    pub decision: AgentRecursionDecision,
}

impl AgentRecursionAdmission {
    pub fn admitted(&self) -> bool {
        self.schema == AGENT_RECURSION_ADMISSION_SCHEMA
            && matches!(self.decision, AgentRecursionDecision::Allow)
    }
}

pub fn admit_child_delegation(
    parent: &AgentDelegationEnvelope,
    child: &AgentDelegationEnvelope,
) -> AgentRecursionAdmission {
    let mut violations = validate_delegation(child);
    let mut missing_evidence = Vec::new();

    if parent.schema != AGENT_DELEGATION_ENVELOPE_SCHEMA {
        violations.push(violation(
            "invalid_parent_schema",
            "parent delegation schema is not supported",
        ));
    }
    if child.node_id != parent.node_id {
        violations.push(violation(
            "node_boundary_crossed",
            "local recursion cannot cross a node boundary; use federation",
        ));
    }
    if child.parent_agent_id.as_deref() != Some(parent.agent_id.as_str()) {
        violations.push(violation(
            "parent_lineage_mismatch",
            "child parent_agent_id must name the delegating agent",
        ));
    }
    if parent.depth.checked_add(1) != Some(child.depth) {
        violations.push(violation(
            "invalid_recursion_depth",
            "child depth must be exactly one greater than parent depth",
        ));
    }
    if child.authority_snapshot.digest != parent.authority_snapshot.digest {
        missing_evidence.push("authority_subset_proof".to_string());
    }
    if !child.budget.fits_within(&parent.budget) {
        violations.push(violation(
            "budget_amplification",
            "child budget must not exceed the parent budget",
        ));
    }

    for capability in &child.capabilities {
        let parent_grant = parent
            .capabilities
            .iter()
            .find(|candidate| candidate.id == capability.id);
        match parent_grant {
            None => violations.push(violation(
                "capability_amplification",
                format!(
                    "child capability '{}' is not granted to its parent",
                    capability.id
                ),
            )),
            Some(parent_grant) if capability.mutates && !parent_grant.mutates => {
                violations.push(violation(
                    "mutation_authority_amplification",
                    format!(
                        "child capability '{}' adds mutation authority",
                        capability.id
                    ),
                ));
            }
            Some(_) => {}
        }
    }

    admission(child.delegation_id.clone(), violations, missing_evidence)
}

pub fn admit_root_delegation(delegation: &AgentDelegationEnvelope) -> AgentRecursionAdmission {
    let mut violations = validate_delegation(delegation);
    if delegation.parent_agent_id.is_some() {
        violations.push(violation(
            "root_has_parent",
            "a root delegation must not name a parent agent",
        ));
    }
    if delegation.depth != 0 {
        violations.push(violation(
            "invalid_root_depth",
            "a root delegation must have depth zero",
        ));
    }
    admission(delegation.delegation_id.clone(), violations, Vec::new())
}

pub fn admit_completion(
    delegation: &AgentDelegationEnvelope,
    completion: &AgentCompletionEnvelope,
) -> AgentRecursionAdmission {
    let mut violations = Vec::new();

    if completion.schema != AGENT_COMPLETION_ENVELOPE_SCHEMA {
        violations.push(violation(
            "invalid_completion_schema",
            "completion schema is not supported",
        ));
    }
    if completion.delegation_id != delegation.delegation_id
        || completion.node_id != delegation.node_id
        || completion.agent_id != delegation.agent_id
        || completion.parent_agent_id != delegation.parent_agent_id
    {
        violations.push(violation(
            "completion_lineage_mismatch",
            "completion identity must match the admitted delegation",
        ));
    }
    if !completion.consumed_budget.fits_within(&delegation.budget) {
        violations.push(violation(
            "budget_exceeded",
            "reported consumption exceeds the delegated budget",
        ));
    }
    violations.extend(validate_evidence_refs(&completion.evidence_refs));
    if completion.status == AgentCompletionStatus::Completed && completion.evidence_refs.is_empty()
    {
        violations.push(violation(
            "completion_without_evidence",
            "completed work must return at least one content-addressed evidence reference",
        ));
    }
    if completion.status == AgentCompletionStatus::Blocked && completion.unknowns.is_empty() {
        violations.push(violation(
            "blocked_without_unknown",
            "blocked work must return at least one explicit unknown",
        ));
    }

    admission(completion.delegation_id.clone(), violations, Vec::new())
}

fn validate_delegation(envelope: &AgentDelegationEnvelope) -> Vec<AgentRecursionViolation> {
    let mut violations = Vec::new();
    if envelope.schema != AGENT_DELEGATION_ENVELOPE_SCHEMA {
        violations.push(violation(
            "invalid_delegation_schema",
            "delegation schema is not supported",
        ));
    }
    for (field, value) in [
        ("delegation_id", envelope.delegation_id.as_str()),
        ("node_id", envelope.node_id.as_str()),
        ("agent_id", envelope.agent_id.as_str()),
        ("objective", envelope.objective.as_str()),
    ] {
        if value.trim().is_empty() {
            violations.push(violation(
                "empty_required_field",
                format!("{field} must not be empty"),
            ));
        }
    }
    if envelope.expected_proofs.is_empty()
        || envelope
            .expected_proofs
            .iter()
            .any(|proof| proof.trim().is_empty())
    {
        violations.push(violation(
            "expected_proof_missing",
            "every delegation must declare at least one expected proof",
        ));
    }
    violations.extend(validate_evidence_refs(std::slice::from_ref(
        &envelope.authority_snapshot,
    )));
    violations.extend(validate_evidence_refs(&envelope.context_refs));

    let mut capabilities = BTreeSet::new();
    for capability in &envelope.capabilities {
        if capability.id.trim().is_empty() {
            violations.push(violation(
                "empty_capability",
                "capability id must not be empty",
            ));
        } else if !capabilities.insert(capability.id.as_str()) {
            violations.push(violation(
                "duplicate_capability",
                format!("capability '{}' is declared more than once", capability.id),
            ));
        }
    }
    violations
}

fn collect_evidence_ref_violations(
    evidence_refs: &[AgentEvidenceRef],
    violations: &mut Vec<AgentRecursionViolation>,
) {
    for evidence in evidence_refs {
        if evidence.kind.trim().is_empty()
            || evidence.subject.trim().is_empty()
            || !valid_sha256_digest(&evidence.digest)
        {
            violations.push(violation(
                "invalid_evidence_ref",
                "evidence references require kind, subject, and a sha256 digest",
            ));
        }
        if !lineage_unknowns("evidence", evidence).is_empty() && evidence.lineage.is_some() {
            violations.push(violation(
                "invalid_evidence_lineage",
                "evidence lineage requires an attested producer, at least one origin, and sha256 digests",
            ));
        }
    }
}

pub fn validate_evidence_refs(evidence_refs: &[AgentEvidenceRef]) -> Vec<AgentRecursionViolation> {
    let mut violations = Vec::new();
    collect_evidence_ref_violations(evidence_refs, &mut violations);
    violations
}

fn admission(
    delegation_id: String,
    violations: Vec<AgentRecursionViolation>,
    missing_evidence: Vec<String>,
) -> AgentRecursionAdmission {
    let decision = if !violations.is_empty() {
        AgentRecursionDecision::Deny { violations }
    } else if !missing_evidence.is_empty() {
        AgentRecursionDecision::Unknown { missing_evidence }
    } else {
        AgentRecursionDecision::Allow
    };
    AgentRecursionAdmission {
        schema: AGENT_RECURSION_ADMISSION_SCHEMA.to_string(),
        delegation_id,
        decision,
    }
}

fn violation(code: impl Into<String>, message: impl Into<String>) -> AgentRecursionViolation {
    AgentRecursionViolation {
        code: code.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests;
