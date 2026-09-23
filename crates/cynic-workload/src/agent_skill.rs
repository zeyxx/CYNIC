//! Content-addressed skill bundles for bounded agent workloads.
//!
//! A skill is context, never execution authority. The node scheduler must bind
//! an admitted bundle to a separately admitted workload before an adapter may
//! deliver it to Hermes or another agent runtime.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use cynic_protocol::{
    AgentBudget, AgentDelegationEnvelope, AgentEvidenceRef, validate_evidence_refs,
};

mod delivery;
mod metadata;

pub use delivery::{compose_agent_skill_delivery_input, validate_agent_skill_delivery_input};
pub use metadata::{
    AGENT_SKILL_LIFECYCLE_RECORD_SCHEMA, AGENT_SKILL_PROJECTION_SCHEMA, AgentSkillAvailability,
    AgentSkillDataClassification, AgentSkillDataClassification as DataClassification,
    AgentSkillLifecycleRecord, AgentSkillLifecycleState, AgentSkillMetadata, AgentSkillProjection,
    AgentSkillRetentionPolicy, project_agent_skill, validate_agent_skill_lifecycle_record,
};

pub const AGENT_SKILL_BUNDLE_SCHEMA: &str = "cynic.agent_skill_bundle.v1";
pub const AGENT_SKILL_BUNDLE_ADMISSION_SCHEMA: &str = "cynic.agent_skill_bundle_admission.v1";
pub const AGENT_SKILL_BINDING_SCHEMA: &str = "cynic.agent_skill_binding.v1";
pub const AGENT_SKILL_BINDING_ADMISSION_SCHEMA: &str = "cynic.agent_skill_binding_admission.v1";
pub const AGENT_SKILL_DELIVERY_INPUT_SCHEMA: &str = "cynic.agent_skill_delivery_input.v1";

/// A portable, immutable skill artifact. `content_digest` is the SHA-256 of
/// the exact UTF-8 bytes in `content`, so adapter formatting cannot alter the
/// instruction that was admitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSkillBundle {
    pub schema: String,
    pub skill_id: String,
    pub version: String,
    pub content: String,
    pub content_digest: String,
    /// Runtime ids are declarative compatibility claims, not an authority
    /// grant. An empty list deliberately admits no runtime.
    pub compatible_runtime_ids: Vec<String>,
    /// Capabilities the skill may require. They must be a subset of the
    /// workload delegation when a scheduler binds the bundle to execution.
    #[serde(default)]
    pub required_capability_ids: Vec<String>,
    pub maximum_budget: AgentBudget,
    pub metadata: AgentSkillMetadata,
    /// Evidence of the explicit authority that approved distribution.
    pub approval_evidence: AgentEvidenceRef,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSkillBundleViolation {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum AgentSkillBundleDecision {
    Allow,
    Deny {
        violations: Vec<AgentSkillBundleViolation>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSkillBundleAdmission {
    pub schema: String,
    pub skill_id: String,
    pub version: String,
    pub content_digest: String,
    pub target_runtime_id: String,
    pub decision: AgentSkillBundleDecision,
}

impl AgentSkillBundleAdmission {
    pub fn admitted(&self) -> bool {
        self.schema == AGENT_SKILL_BUNDLE_ADMISSION_SCHEMA
            && matches!(self.decision, AgentSkillBundleDecision::Allow)
    }
}

/// A scheduler-owned attachment of one immutable bundle to one local agent
/// delegation. The adapter receives this binding rather than interpreting an
/// arbitrary skill path or ad-hoc prompt argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSkillBinding {
    pub schema: String,
    pub skill_id: String,
    pub version: String,
    pub content_digest: String,
    pub target_runtime_id: String,
    pub delegation_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSkillBindingAdmission {
    pub schema: String,
    pub delegation_id: String,
    pub content_digest: String,
    pub target_runtime_id: String,
    pub decision: AgentSkillBundleDecision,
}

impl AgentSkillBindingAdmission {
    pub fn admitted(&self) -> bool {
        self.schema == AGENT_SKILL_BINDING_ADMISSION_SCHEMA
            && matches!(self.decision, AgentSkillBundleDecision::Allow)
    }
}

/// Typed input carried by the pre-existing durable agent workload envelope.
/// The scheduler transports this input; the runtime adapter is responsible for
/// fetching and verifying the exact bundle before it exposes it to an agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentSkillDeliveryInput {
    pub schema: String,
    pub bundle: AgentSkillBundle,
    pub binding: AgentSkillBinding,
}

/// Pure admission for a bundle targeted at one runtime adapter.
pub fn admit_agent_skill_bundle(
    bundle: &AgentSkillBundle,
    target_runtime_id: &str,
) -> AgentSkillBundleAdmission {
    let mut violations = Vec::new();
    if bundle.schema != AGENT_SKILL_BUNDLE_SCHEMA {
        violations.push(violation(
            "invalid_skill_bundle_schema",
            "skill bundle schema is not supported",
        ));
    }
    for (field, value) in [
        ("skill_id", bundle.skill_id.as_str()),
        ("version", bundle.version.as_str()),
        ("content", bundle.content.as_str()),
        ("target_runtime_id", target_runtime_id),
    ] {
        if value.trim().is_empty() {
            violations.push(violation(
                "empty_required_field",
                format!("{field} must not be empty"),
            ));
        }
    }
    let observed_digest = content_digest(&bundle.content);
    if bundle.content_digest != observed_digest {
        violations.push(violation(
            "skill_content_digest_mismatch",
            "content_digest must bind the exact UTF-8 skill content",
        ));
    }
    if bundle.compatible_runtime_ids.is_empty() {
        violations.push(violation(
            "skill_runtime_compatibility_missing",
            "skill bundle must explicitly name at least one compatible runtime",
        ));
    } else if bundle
        .compatible_runtime_ids
        .iter()
        .any(|id| id.trim().is_empty())
    {
        violations.push(violation(
            "empty_compatible_runtime_id",
            "compatible runtime ids must not be empty",
        ));
    } else if !bundle
        .compatible_runtime_ids
        .iter()
        .any(|id| id == target_runtime_id)
    {
        violations.push(violation(
            "skill_runtime_incompatible",
            "skill bundle is not compatible with the target runtime",
        ));
    }
    for capability in &bundle.required_capability_ids {
        if capability.trim().is_empty() {
            violations.push(violation(
                "empty_required_capability",
                "required capability ids must not be empty",
            ));
        }
    }
    if bundle.approval_evidence.kind != "human_approval" {
        violations.push(violation(
            "skill_human_approval_missing",
            "skill distribution requires a human_approval evidence reference",
        ));
    }
    violations.extend(metadata::validate_agent_skill_metadata(&bundle.metadata));
    violations.extend(
        validate_evidence_refs(std::slice::from_ref(&bundle.approval_evidence))
            .into_iter()
            .map(|issue| violation("invalid_skill_approval_evidence", issue.message)),
    );

    let decision = if violations.is_empty() {
        AgentSkillBundleDecision::Allow
    } else {
        AgentSkillBundleDecision::Deny { violations }
    };
    AgentSkillBundleAdmission {
        schema: AGENT_SKILL_BUNDLE_ADMISSION_SCHEMA.to_string(),
        skill_id: bundle.skill_id.clone(),
        version: bundle.version.clone(),
        content_digest: bundle.content_digest.clone(),
        target_runtime_id: target_runtime_id.to_string(),
        decision,
    }
}

pub fn content_digest(content: &str) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(content.as_bytes())))
}

/// Binds an admitted skill artifact to the delegated context. This is still
/// not execution: node scheduling, delivery lease, adapter execution, and
/// completion evidence remain separate contracts.
pub fn admit_agent_skill_binding(
    bundle: &AgentSkillBundle,
    binding: &AgentSkillBinding,
    delegation: &AgentDelegationEnvelope,
) -> AgentSkillBindingAdmission {
    let mut violations = Vec::new();
    if binding.schema != AGENT_SKILL_BINDING_SCHEMA {
        violations.push(violation(
            "invalid_skill_binding_schema",
            "skill binding schema is not supported",
        ));
    }
    for (field, value) in [
        ("skill_id", binding.skill_id.as_str()),
        ("version", binding.version.as_str()),
        ("content_digest", binding.content_digest.as_str()),
        ("target_runtime_id", binding.target_runtime_id.as_str()),
        ("delegation_id", binding.delegation_id.as_str()),
    ] {
        if value.trim().is_empty() {
            violations.push(violation(
                "empty_required_field",
                format!("{field} must not be empty"),
            ));
        }
    }
    if binding.skill_id != bundle.skill_id
        || binding.version != bundle.version
        || binding.content_digest != bundle.content_digest
    {
        violations.push(violation(
            "skill_binding_bundle_mismatch",
            "skill binding must name the exact bundle id, version, and digest",
        ));
    }
    if binding.delegation_id != delegation.delegation_id {
        violations.push(violation(
            "skill_binding_delegation_mismatch",
            "skill binding must name the receiving delegation",
        ));
    }
    let bundle_admission = admit_agent_skill_bundle(bundle, &binding.target_runtime_id);
    if !bundle_admission.admitted() {
        violations.push(violation(
            "skill_bundle_not_admitted",
            "skill bundle must be admitted for the target runtime before binding",
        ));
    }
    let expected_subject = format!("{}@{}", bundle.skill_id, bundle.version);
    let attached = delegation.context_refs.iter().any(|reference| {
        reference.kind == "agent_skill_bundle"
            && reference.subject == expected_subject
            && reference.digest == bundle.content_digest
    });
    if !attached {
        violations.push(violation(
            "skill_context_ref_missing",
            "delegation context_refs must contain the exact content-addressed skill bundle",
        ));
    }
    if !budget_fits_within(&bundle.maximum_budget, &delegation.budget) {
        violations.push(violation(
            "skill_budget_exceeds_delegation",
            "skill maximum budget must fit within the receiving delegation budget",
        ));
    }
    for capability in &bundle.required_capability_ids {
        if !delegation
            .capabilities
            .iter()
            .any(|grant| grant.id == *capability)
        {
            violations.push(violation(
                "skill_capability_not_delegated",
                format!("required skill capability '{capability}' is absent from delegation"),
            ));
        }
    }
    let decision = if violations.is_empty() {
        AgentSkillBundleDecision::Allow
    } else {
        AgentSkillBundleDecision::Deny { violations }
    };
    AgentSkillBindingAdmission {
        schema: AGENT_SKILL_BINDING_ADMISSION_SCHEMA.to_string(),
        delegation_id: binding.delegation_id.clone(),
        content_digest: binding.content_digest.clone(),
        target_runtime_id: binding.target_runtime_id.clone(),
        decision,
    }
}

fn budget_fits_within(candidate: &AgentBudget, limit: &AgentBudget) -> bool {
    candidate.context_tokens <= limit.context_tokens
        && candidate.actions <= limit.actions
        && candidate.children <= limit.children
        && candidate.wall_time_ms <= limit.wall_time_ms
}

fn violation(code: impl Into<String>, message: impl Into<String>) -> AgentSkillBundleViolation {
    AgentSkillBundleViolation {
        code: code.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence() -> AgentEvidenceRef {
        AgentEvidenceRef {
            kind: "human_approval".to_string(),
            subject: "skill:cynic-contract-first-engineering".to_string(),
            digest: format!("sha256:{}", "a".repeat(64)),
            lineage: None,
        }
    }

    fn bundle() -> AgentSkillBundle {
        let content = "# Contract first\n".to_string();
        AgentSkillBundle {
            schema: AGENT_SKILL_BUNDLE_SCHEMA.to_string(),
            skill_id: "cynic-contract-first-engineering".to_string(),
            version: "1.0.0".to_string(),
            content_digest: content_digest(&content),
            content,
            compatible_runtime_ids: vec!["hermes-agent".to_string()],
            required_capability_ids: vec!["workspace_read".to_string()],
            maximum_budget: AgentBudget {
                context_tokens: 4_000,
                actions: 4,
                children: 0,
                wall_time_ms: 600_000,
                cpu_ms: None,
            },
            metadata: AgentSkillMetadata {
                source_evidence: evidence(),
                issued_at_ms: 1,
                data_classification: AgentSkillDataClassification::Private,
                retention: AgentSkillRetentionPolicy::RetainUntilRevoked,
                consent_policy_digest: None,
            },
            approval_evidence: evidence(),
        }
    }

    fn delegation(bundle: &AgentSkillBundle) -> AgentDelegationEnvelope {
        AgentDelegationEnvelope {
            schema: "cynic.agent_delegation_envelope.v1".to_string(),
            delegation_id: "delegation-hermes-1".to_string(),
            node_id: "node-1".to_string(),
            agent_id: "hermes-agent".to_string(),
            parent_agent_id: None,
            depth: 0,
            objective: "apply contract-first skill".to_string(),
            authority_snapshot: evidence(),
            context_refs: vec![AgentEvidenceRef {
                kind: "agent_skill_bundle".to_string(),
                subject: format!("{}@{}", bundle.skill_id, bundle.version),
                digest: bundle.content_digest.clone(),
                lineage: None,
            }],
            capabilities: vec![cynic_protocol::AgentCapabilityGrant {
                id: "workspace_read".to_string(),
                mutates: false,
            }],
            budget: AgentBudget {
                context_tokens: 8_000,
                actions: 8,
                children: 1,
                wall_time_ms: 900_000,
                cpu_ms: None,
            },
            expected_proofs: vec!["agent_completion".to_string()],
        }
    }

    fn binding(bundle: &AgentSkillBundle) -> AgentSkillBinding {
        AgentSkillBinding {
            schema: AGENT_SKILL_BINDING_SCHEMA.to_string(),
            skill_id: bundle.skill_id.clone(),
            version: bundle.version.clone(),
            content_digest: bundle.content_digest.clone(),
            target_runtime_id: "hermes-agent".to_string(),
            delegation_id: "delegation-hermes-1".to_string(),
        }
    }

    #[test]
    fn admits_exact_hermes_skill_bundle() {
        assert!(admit_agent_skill_bundle(&bundle(), "hermes-agent").admitted());
    }

    #[test]
    fn rejects_tampered_skill_content() {
        let mut input = bundle();
        input.content.push_str("mutated");
        let admission = admit_agent_skill_bundle(&input, "hermes-agent");
        assert!(matches!(
            admission.decision,
            AgentSkillBundleDecision::Deny { .. }
        ));
    }

    #[test]
    fn rejects_unlisted_runtime() {
        let admission = admit_agent_skill_bundle(&bundle(), "codex-cli");
        assert!(matches!(
            admission.decision,
            AgentSkillBundleDecision::Deny { .. }
        ));
    }

    #[test]
    fn rejects_non_human_skill_approval_evidence() {
        let mut input = bundle();
        input.approval_evidence.kind = "model_output".to_string();
        assert!(matches!(
            admit_agent_skill_bundle(&input, "hermes-agent").decision,
            AgentSkillBundleDecision::Deny { .. }
        ));
    }

    #[test]
    fn binds_an_admitted_bundle_to_hermes_delegation_context() {
        let bundle = bundle();
        assert!(
            admit_agent_skill_binding(&bundle, &binding(&bundle), &delegation(&bundle)).admitted()
        );
    }

    #[test]
    fn rejects_binding_without_content_addressed_context() {
        let bundle = bundle();
        let mut delegation = delegation(&bundle);
        delegation.context_refs.clear();
        assert!(matches!(
            admit_agent_skill_binding(&bundle, &binding(&bundle), &delegation).decision,
            AgentSkillBundleDecision::Deny { .. }
        ));
    }

    #[test]
    fn composes_delivery_as_a_canonical_workload_input() {
        let bundle = bundle();
        let input =
            compose_agent_skill_delivery_input(&bundle, &binding(&bundle), &delegation(&bundle))
                .expect("admitted binding produces workload input");
        assert_eq!(input.schema, crate::AGENT_WORKLOAD_INPUT_SCHEMA);
        assert_eq!(input.kind, "agent_skill_delivery");
        assert_eq!(
            cynic_protocol::digest_json(&input.payload).unwrap(),
            input.digest
        );
        assert!(validate_agent_skill_delivery_input(&input, &delegation(&bundle)).is_empty());
    }

    #[test]
    fn rejects_skill_delivery_that_does_not_bind_its_delegation() {
        let bundle = bundle();
        let mut input =
            compose_agent_skill_delivery_input(&bundle, &binding(&bundle), &delegation(&bundle))
                .expect("admitted binding produces workload input");
        input.payload["binding"]["delegation_id"] = serde_json::json!("wrong-delegation");
        assert!(!validate_agent_skill_delivery_input(&input, &delegation(&bundle)).is_empty());
    }
}
