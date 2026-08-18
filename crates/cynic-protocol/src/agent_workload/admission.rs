use std::collections::BTreeSet;

use crate::{
    AgentRecursionDecision, PolicyDecision, admit_child_delegation, admit_root_delegation,
};

use super::{
    shared::{admission, violation},
    types::{AGENT_WORKLOAD_ENVELOPE_SCHEMA, AgentWorkloadAdmission, AgentWorkloadEnvelope},
};

pub fn admit_agent_workload(envelope: &AgentWorkloadEnvelope) -> AgentWorkloadAdmission {
    let mut violations = Vec::new();
    let mut missing_evidence = Vec::new();

    if envelope.schema != AGENT_WORKLOAD_ENVELOPE_SCHEMA {
        violations.push(violation(
            "invalid_workload_schema",
            "agent workload schema is not supported",
        ));
    }
    for (field, value) in [
        ("workload_id", envelope.workload_id.as_str()),
        ("idempotency_key", envelope.idempotency_key.as_str()),
    ] {
        if value.trim().is_empty() {
            violations.push(violation(
                "empty_required_field",
                format!("{field} must not be empty"),
            ));
        }
    }
    if !valid_sha256_digest(&envelope.plan_digest) {
        violations.push(violation(
            "invalid_plan_digest",
            "plan_digest must be a sha256 content digest",
        ));
    }
    match crate::digest_json(&envelope.action) {
        Ok(digest) if digest == envelope.plan_digest => {}
        Ok(_) => violations.push(violation(
            "action_plan_digest_mismatch",
            "plan_digest must bind the exact bounded action",
        )),
        Err(_) => violations.push(violation(
            "action_plan_not_serializable",
            "bounded action could not be canonicalized",
        )),
    }
    if let Some(input) = &envelope.input {
        if input.schema != crate::AGENT_WORKLOAD_INPUT_SCHEMA || input.kind.trim().is_empty() {
            violations.push(violation(
                "invalid_workload_input",
                "workload input requires the canonical schema and a typed kind",
            ));
        }
        match crate::digest_json(&input.payload) {
            Ok(digest) if digest == input.digest && valid_sha256_digest(&input.digest) => {}
            _ => violations.push(violation(
                "workload_input_digest_mismatch",
                "workload input digest must bind the exact canonical payload",
            )),
        }
        if input.kind == "agent_skill_delivery" {
            violations.extend(
                crate::validate_agent_skill_delivery_input(input, &envelope.delegation)
                    .into_iter()
                    .map(|issue| {
                        violation(format!("skill_delivery_{}", issue.code), issue.message)
                    }),
            );
        }
    }
    if envelope.action.argv.is_empty()
        || envelope
            .action
            .argv
            .iter()
            .any(|argument| argument.is_empty())
    {
        violations.push(violation(
            "invalid_action_argv",
            "bounded action requires a non-empty argv with no empty arguments",
        ));
    }
    if envelope
        .action
        .argv
        .first()
        .and_then(|program| std::path::Path::new(program).file_name())
        .and_then(|program| program.to_str())
        .is_some_and(|program| {
            matches!(
                program,
                "sh" | "bash" | "zsh" | "fish" | "pwsh" | "powershell" | "cmd"
            )
        })
    {
        violations.push(violation(
            "shell_action_forbidden",
            "agent mutation actions must use typed argv adapters, not shell composition",
        ));
    }
    if envelope.action.stop_condition.trim().is_empty() {
        violations.push(violation(
            "stop_condition_missing",
            "bounded action must declare an explicit stop condition",
        ));
    }
    match envelope.action.input_digest.as_deref() {
        Some(digest) if valid_sha256_digest(digest) => {}
        Some(_) => violations.push(violation(
            "invalid_action_input_digest",
            "bounded action input_digest must be a sha256 content digest",
        )),
        None if envelope.action.mutates => violations.push(violation(
            "mutating_action_input_digest_missing",
            "a mutating action must bind its exact semantic input",
        )),
        None => {}
    }

    let recursion = match envelope.parent_delegation.as_ref() {
        Some(parent) => admit_child_delegation(parent, &envelope.delegation),
        None => admit_root_delegation(&envelope.delegation),
    };
    match recursion.decision {
        AgentRecursionDecision::Allow => {}
        AgentRecursionDecision::Deny {
            violations: recursion_violations,
        } => violations.extend(recursion_violations.into_iter().map(|item| {
            violation(
                format!("delegation_{}", item.code),
                format!("delegation rejected: {}", item.message),
            )
        })),
        AgentRecursionDecision::Unknown {
            missing_evidence: missing,
        } => missing_evidence.extend(missing),
    }

    let policy = &envelope.policy_admission;
    if !policy.admitted() {
        match &policy.decision {
            PolicyDecision::Unknown {
                missing_evidence: missing,
            } => missing_evidence.extend(missing.iter().cloned()),
            _ => violations.push(violation(
                "policy_not_admitted",
                "the canonical policy decision must explicitly allow this workload",
            )),
        }
    }

    let authorization = &envelope.execution_authorization;
    let mutation = &policy.mutation;
    if envelope.action.mutates != mutation.mutates
        || envelope.action.requires_privilege != mutation.requires_privilege
        || envelope.action.rollback_intent != mutation.rollback_intent
    {
        violations.push(violation(
            "bounded_action_policy_mismatch",
            "bounded action mutation, privilege, and rollback must match canonical policy",
        ));
    }
    if authorization.schema != crate::EXECUTION_AUTHORIZATION_SCHEMA {
        violations.push(violation(
            "invalid_execution_authorization_schema",
            "execution authorization schema is not supported",
        ));
    }
    if authorization.actor != envelope.delegation.agent_id {
        violations.push(violation(
            "authorization_actor_mismatch",
            "execution authorization actor must match the delegated agent",
        ));
    }
    if mutation.requires_explicit_apply && !authorization.explicit_apply {
        violations.push(violation(
            "explicit_apply_missing",
            "the admitted mutation requires explicit apply consent",
        ));
    }
    if authorization.authority != mutation.required_authority {
        violations.push(violation(
            "authorization_authority_mismatch",
            "execution authorization must carry the exact authority admitted by policy",
        ));
    }
    if policy.intent.actor != envelope.delegation.agent_id {
        violations.push(violation(
            "policy_actor_mismatch",
            "policy intent actor must match the delegated agent",
        ));
    }
    if policy.intent.kind != mutation.kind {
        violations.push(violation(
            "policy_intent_kind_mismatch",
            "policy intent kind must match the admitted mutation",
        ));
    }
    if policy.intent.subject != mutation.scope.target {
        violations.push(violation(
            "policy_subject_mismatch",
            "policy intent subject must match the mutation target",
        ));
    }
    if policy.intent.plan_digest != envelope.plan_digest {
        violations.push(violation(
            "policy_plan_digest_mismatch",
            "policy intent must bind the exact workload plan digest",
        ));
    }
    if let PolicyDecision::Allow {
        required_authority,
        required_proofs,
    } = &policy.decision
    {
        if *required_authority != mutation.required_authority {
            violations.push(violation(
                "policy_authority_mismatch",
                "policy decision authority must match the mutation contract",
            ));
        }
        if required_proofs != &mutation.expected_proofs {
            violations.push(violation(
                "policy_proof_contract_mismatch",
                "policy decision proofs must match the mutation contract",
            ));
        }
    }
    for precondition in &mutation.preconditions {
        if !authorization
            .satisfied_preconditions
            .iter()
            .any(|candidate| candidate == precondition)
        {
            missing_evidence.push(format!("precondition:{precondition}"));
        }
    }
    let capability = envelope
        .delegation
        .capabilities
        .iter()
        .find(|candidate| candidate.id == mutation.kind);
    match capability {
        None => violations.push(violation(
            "capability_not_delegated",
            "the policy mutation kind is not present in the delegated capabilities",
        )),
        Some(capability) if mutation.mutates && !capability.mutates => violations.push(violation(
            "mutation_capability_missing",
            "the delegated capability does not permit mutation",
        )),
        Some(_) => {}
    }
    if mutation.mutates && mutation.rollback_intent.trim().is_empty() {
        violations.push(violation(
            "rollback_intent_missing",
            "a mutating workload must declare rollback intent",
        ));
    }

    let delegated_proofs = envelope
        .delegation
        .expected_proofs
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    for proof in &mutation.expected_proofs {
        if !delegated_proofs.contains(proof.gate.as_str()) {
            violations.push(violation(
                "policy_proof_not_delegated",
                format!(
                    "policy proof gate '{}' is absent from the delegation",
                    proof.gate
                ),
            ));
        }
    }

    admission(envelope.workload_id.clone(), violations, missing_evidence)
}

pub fn admit_agent_workload_dispatch(
    envelope: &AgentWorkloadEnvelope,
    active_idempotency_keys: &[String],
) -> AgentWorkloadAdmission {
    let base = admit_agent_workload(envelope);
    if !base.admitted() {
        return base;
    }
    if active_idempotency_keys
        .iter()
        .any(|candidate| candidate == &envelope.idempotency_key)
    {
        return admission(
            envelope.workload_id.clone(),
            vec![violation(
                "duplicate_idempotency_key",
                "an active workload already owns this idempotency key",
            )],
            Vec::new(),
        );
    }
    base
}

fn valid_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}
