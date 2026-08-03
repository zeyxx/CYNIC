use serde::{Deserialize, Serialize};

use crate::{
    AgentWorkloadState, DurableAgentWorkloadRecord, validate_durable_agent_workload_record,
};

use super::canonical_build_actions;

pub const BUILD_WORKLOAD_GRANT_SCHEMA: &str = "cynic.build.workload_grant.v1";
pub const SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA: &str = "cynic.signed_build_workload_grant.v1";
pub const BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE: &str = "cynic-build-workload-grant";
pub const BUILD_WORKLOAD_GRANT_MAX_TTL_SECONDS: u64 = 900;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildWorkloadGrant {
    pub schema: String,
    pub status: String,
    pub grant_id: String,
    pub issuer_node_id: String,
    pub workload_id: String,
    pub workload_revision: u64,
    pub workload_record_digest: String,
    pub node_id: String,
    pub role: String,
    pub source_commit: String,
    pub policy_digest: String,
    pub lease_id: String,
    pub fencing_token: u64,
    pub cancellation_generation: u64,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    pub allowed_actions: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SignedBuildWorkloadGrant {
    pub schema: String,
    pub authority_key_id: String,
    pub signature_namespace: String,
    pub grant: BuildWorkloadGrant,
    pub signature_hex: String,
}

pub fn build_workload_grant_signing_bytes(
    signed: &SignedBuildWorkloadGrant,
) -> Result<Vec<u8>, serde_json::Error> {
    crate::canonical_json_bytes(&serde_json::json!({
        "schema": signed.schema,
        "authority_key_id": signed.authority_key_id,
        "signature_namespace": signed.signature_namespace,
        "grant": signed.grant,
    }))
}

pub fn validate_signed_build_workload_grant(signed: &SignedBuildWorkloadGrant) -> Vec<String> {
    let mut blockers = Vec::new();
    if signed.schema != SIGNED_BUILD_WORKLOAD_GRANT_SCHEMA {
        blockers.push("signed_build_workload_grant_schema_mismatch".to_string());
    }
    if signed.signature_namespace != BUILD_WORKLOAD_GRANT_SIGNATURE_NAMESPACE {
        blockers.push("signed_build_workload_grant_namespace_mismatch".to_string());
    }
    if !valid_sha256(&signed.authority_key_id) {
        blockers.push("signed_build_workload_grant_authority_key_id_invalid".to_string());
    }
    if signed.signature_hex.len() != 128
        || !signed
            .signature_hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        blockers.push("signed_build_workload_grant_signature_invalid".to_string());
    }
    blockers
}

pub fn derive_build_workload_grant(
    durable: &DurableAgentWorkloadRecord,
    issuer_node_id: &str,
    now_unix_s: u64,
) -> Result<BuildWorkloadGrant, Vec<String>> {
    let mut blockers = validate_durable_agent_workload_record(durable);
    let record = &durable.record;
    let envelope = &record.envelope;

    if issuer_node_id.trim().is_empty() {
        blockers.push("build_workload_grant_issuer_missing".to_string());
    }
    if record.state != AgentWorkloadState::Claimed {
        blockers.push("build_workload_not_claimed".to_string());
    }
    if record.consumed_actions != 1 {
        blockers.push("build_workload_action_not_reserved".to_string());
    }
    let claim = record.claim.as_ref();
    let claim_expiry_unix_s = claim.map(|claim| claim.expires_at_ms / 1_000);
    if claim_expiry_unix_s.is_none_or(|expiry| expiry <= now_unix_s) {
        blockers.push("build_workload_claim_expired".to_string());
    }
    if claim.is_some_and(|claim| {
        claim.cancellation_generation != record.cancellation_generation
            || claim.guard.fencing_token != record.last_fencing_token
    }) {
        blockers.push("build_workload_claim_stale".to_string());
    }
    if envelope.delegation.node_id.trim().is_empty() {
        blockers.push("build_workload_target_node_missing".to_string());
    }
    if !envelope
        .delegation
        .capabilities
        .iter()
        .any(|capability| capability.id == "repository_validation" && capability.mutates)
    {
        blockers.push("build_workload_capability_missing".to_string());
    }
    if envelope.policy_admission.mutation.scope.domain != "build" {
        blockers.push("build_workload_policy_scope_mismatch".to_string());
    }

    let source_commit = envelope.policy_admission.intent.subject.as_str();
    if !is_commit_id(source_commit)
        || envelope.policy_admission.mutation.scope.target != source_commit
    {
        blockers.push("build_workload_source_commit_unbound".to_string());
    }
    if envelope
        .action
        .working_directory
        .as_deref()
        .is_none_or(str::is_empty)
        || envelope
            .action
            .input_digest
            .as_deref()
            .is_none_or(str::is_empty)
    {
        blockers.push("build_workload_source_materialization_unbound".to_string());
    }

    let allowed_actions = match canonical_build_action_from_argv(&envelope.action.argv)
        .and_then(|action| canonical_build_actions(action).ok())
    {
        Some(actions) => actions,
        None => {
            blockers.push("build_workload_action_not_canonical".to_string());
            Vec::new()
        }
    };

    if !blockers.is_empty() {
        blockers.sort();
        blockers.dedup();
        return Err(blockers);
    }

    let (claim, claim_expiry_unix_s) = match (claim, claim_expiry_unix_s) {
        (Some(claim), Some(expiry)) => (claim, expiry),
        _ => return Err(vec!["build_workload_claim_missing".to_string()]),
    };
    let expires_at_unix_s = now_unix_s
        .saturating_add(BUILD_WORKLOAD_GRANT_MAX_TTL_SECONDS)
        .min(claim_expiry_unix_s);
    if expires_at_unix_s <= now_unix_s {
        return Err(vec!["build_workload_grant_expiry_invalid".to_string()]);
    }

    Ok(BuildWorkloadGrant {
        schema: BUILD_WORKLOAD_GRANT_SCHEMA.to_string(),
        status: "admitted".to_string(),
        grant_id: format!(
            "build-workload:{}:{}:{}",
            envelope.workload_id, durable.revision, claim.guard.fencing_token
        ),
        issuer_node_id: issuer_node_id.to_string(),
        workload_id: envelope.workload_id.clone(),
        workload_revision: durable.revision,
        workload_record_digest: durable.record_digest.clone(),
        node_id: envelope.delegation.node_id.clone(),
        role: "build_runner".to_string(),
        source_commit: source_commit.to_string(),
        policy_digest: envelope.policy_admission.policy_set.digest.clone(),
        lease_id: claim.guard.lease_id.clone(),
        fencing_token: claim.guard.fencing_token,
        cancellation_generation: record.cancellation_generation,
        issued_at_unix_s: now_unix_s,
        expires_at_unix_s,
        allowed_actions,
        evidence_refs: vec![
            format!("workload-record:{}", durable.record_digest),
            format!("workload-revision:{}", durable.revision),
            format!(
                "workload-lease:{}:{}",
                claim.guard.lease_id, claim.guard.fencing_token
            ),
            format!("policy:{}", envelope.policy_admission.policy_set.digest),
        ],
    })
}

pub fn validate_build_workload_grant(
    grant: &BuildWorkloadGrant,
    durable: &DurableAgentWorkloadRecord,
    observed_node_id: &str,
    observed_source_commit: &str,
    requested_action: &str,
    now_unix_s: u64,
) -> Result<(), Vec<String>> {
    let mut blockers = validate_durable_agent_workload_record(durable);
    let record = &durable.record;
    let envelope = &record.envelope;
    let claim = record.claim.as_ref();

    if grant.schema != BUILD_WORKLOAD_GRANT_SCHEMA || grant.status != "admitted" {
        blockers.push("build_workload_grant_not_admitted".to_string());
    }
    if grant.grant_id.trim().is_empty() || grant.issuer_node_id.trim().is_empty() {
        blockers.push("build_workload_grant_identity_missing".to_string());
    }
    if grant.workload_id != envelope.workload_id
        || grant.workload_revision != durable.revision
        || grant.workload_record_digest != durable.record_digest
    {
        blockers.push("build_workload_grant_record_mismatch".to_string());
    }
    if record.state != AgentWorkloadState::Claimed
        || claim.is_none_or(|claim| {
            grant.lease_id != claim.guard.lease_id
                || grant.fencing_token != claim.guard.fencing_token
                || grant.cancellation_generation != claim.cancellation_generation
                || claim.cancellation_generation != record.cancellation_generation
                || claim.expires_at_ms / 1_000 <= now_unix_s
        })
    {
        blockers.push("build_workload_grant_claim_not_live".to_string());
    }
    if grant.node_id != observed_node_id
        || grant.node_id != envelope.delegation.node_id
        || grant.role != "build_runner"
    {
        blockers.push("build_workload_grant_node_mismatch".to_string());
    }
    if grant.source_commit != observed_source_commit
        || grant.source_commit != envelope.policy_admission.intent.subject
        || grant.source_commit != envelope.policy_admission.mutation.scope.target
    {
        blockers.push("build_workload_grant_source_mismatch".to_string());
    }
    if grant.policy_digest != envelope.policy_admission.policy_set.digest {
        blockers.push("build_workload_grant_policy_mismatch".to_string());
    }
    let expected_actions = canonical_build_actions(requested_action).ok();
    if expected_actions.as_ref() != Some(&grant.allowed_actions) {
        blockers.push("build_workload_grant_action_mismatch".to_string());
    }
    if grant.issued_at_unix_s == 0
        || grant.expires_at_unix_s <= grant.issued_at_unix_s
        || now_unix_s < grant.issued_at_unix_s
        || now_unix_s >= grant.expires_at_unix_s
        || grant.expires_at_unix_s - grant.issued_at_unix_s > BUILD_WORKLOAD_GRANT_MAX_TTL_SECONDS
        || claim.is_some_and(|claim| grant.expires_at_unix_s > claim.expires_at_ms / 1_000)
    {
        blockers.push("build_workload_grant_expired_or_invalid".to_string());
    }
    if grant.evidence_refs.is_empty() {
        blockers.push("build_workload_grant_evidence_missing".to_string());
    }

    blockers.sort();
    blockers.dedup();
    if blockers.is_empty() {
        Ok(())
    } else {
        Err(blockers)
    }
}

pub fn canonical_build_action_from_argv(argv: &[String]) -> Option<&'static str> {
    match argv {
        [program, target] if program == "make" && target == "check" => Some("make:check"),
        [program, target] if program == "make" && target == "build" => Some("make:build"),
        [program, target] if program == "make" && target == "build-release" => {
            Some("make:build-release")
        }
        _ => None,
    }
}

fn is_commit_id(value: &str) -> bool {
    matches!(value.len(), 40 | 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 71
        && value.starts_with("sha256:")
        && value[7..].bytes().all(|byte| byte.is_ascii_hexdigit())
}

#[cfg(test)]
mod action_tests {
    use super::canonical_build_action_from_argv;

    #[test]
    fn canonical_action_is_derived_once_from_exact_argv() {
        assert_eq!(
            canonical_build_action_from_argv(&["make".to_string(), "check".to_string()]),
            Some("make:check")
        );
        assert_eq!(
            canonical_build_action_from_argv(&[
                "bash".to_string(),
                "-c".to_string(),
                "make check".to_string()
            ]),
            None
        );
        assert_eq!(
            canonical_build_action_from_argv(&["make".to_string(), "build-release".to_string()]),
            Some("make:build-release")
        );
    }
}
