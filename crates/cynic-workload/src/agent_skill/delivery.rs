//! Composition and semantic validation of skill delivery inputs.

use crate::{AGENT_WORKLOAD_INPUT_SCHEMA, AgentWorkloadInput, digest_json};
use cynic_protocol::AgentDelegationEnvelope;

use super::{
    AGENT_SKILL_DELIVERY_INPUT_SCHEMA, AgentSkillBinding, AgentSkillBindingAdmission,
    AgentSkillBundle, AgentSkillBundleDecision, AgentSkillBundleViolation, AgentSkillDeliveryInput,
    admit_agent_skill_binding, violation,
};

/// Composes the content-addressed input for an already-admitted workload.
/// This function never dispatches or executes an agent; callers must still
/// create, admit, lease, and read back a normal `AgentWorkloadEnvelope`.
pub fn compose_agent_skill_delivery_input(
    bundle: &AgentSkillBundle,
    binding: &AgentSkillBinding,
    delegation: &AgentDelegationEnvelope,
) -> Result<AgentWorkloadInput, AgentSkillBindingAdmission> {
    let admission = admit_agent_skill_binding(bundle, binding, delegation);
    if !admission.admitted() {
        return Err(admission);
    }
    let payload = serde_json::to_value(AgentSkillDeliveryInput {
        schema: AGENT_SKILL_DELIVERY_INPUT_SCHEMA.to_string(),
        bundle: bundle.clone(),
        binding: binding.clone(),
    })
    .expect("agent skill delivery input is serializable");
    let digest = digest_json(&payload).expect("agent skill delivery input is digestible");
    Ok(AgentWorkloadInput {
        schema: AGENT_WORKLOAD_INPUT_SCHEMA.to_string(),
        kind: "agent_skill_delivery".to_string(),
        digest,
        payload,
    })
}

/// Validates the semantic payload carried by the generic workload input.
/// Keeping this rule in the protocol means a durable backend such as
/// SurrealDB stores an independently revalidatable fact rather than accepting
/// an opaque JSON blob merely because its outer digest is well formed.
pub fn validate_agent_skill_delivery_input(
    input: &AgentWorkloadInput,
    delegation: &AgentDelegationEnvelope,
) -> Vec<AgentSkillBundleViolation> {
    let mut violations = Vec::new();
    if input.schema != AGENT_WORKLOAD_INPUT_SCHEMA {
        violations.push(violation(
            "skill_delivery_input_schema_invalid",
            "skill delivery input must use the canonical workload input schema",
        ));
    }
    if input.kind != "agent_skill_delivery" {
        violations.push(violation(
            "skill_delivery_input_kind_invalid",
            "skill delivery input kind must be agent_skill_delivery",
        ));
    }
    let delivery = match serde_json::from_value::<AgentSkillDeliveryInput>(input.payload.clone()) {
        Ok(delivery) => delivery,
        Err(_) => {
            violations.push(violation(
                "skill_delivery_input_decode_failed",
                "skill delivery input payload must decode as the typed delivery schema",
            ));
            return violations;
        }
    };
    if delivery.schema != AGENT_SKILL_DELIVERY_INPUT_SCHEMA {
        violations.push(violation(
            "skill_delivery_payload_schema_invalid",
            "skill delivery payload schema is not supported",
        ));
    }
    let admission = admit_agent_skill_binding(&delivery.bundle, &delivery.binding, delegation);
    if let AgentSkillBundleDecision::Deny {
        violations: admission_violations,
    } = admission.decision
    {
        violations.extend(admission_violations);
    }
    violations
}
