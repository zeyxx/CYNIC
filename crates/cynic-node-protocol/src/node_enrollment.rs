use serde::{Deserialize, Serialize};

use cynic_protocol::SecurityEvidenceState;

pub const NODE_ENROLLMENT_REQUEST_SCHEMA: &str = "cynic.node_enrollment_request.v1";
pub const NODE_ENROLLMENT_GRANT_SCHEMA: &str = "cynic.node_enrollment_grant.v1";
pub const NODE_ENROLLMENT_OBSERVATION_SCHEMA: &str = "cynic.node_enrollment_observation.v1";
pub const NODE_ENROLLMENT_PLAN_SCHEMA: &str = "cynic.node_enrollment_plan.v1";
pub const SIGNED_NODE_ENROLLMENT_GRANT_SCHEMA: &str = "cynic.signed_node_enrollment_grant.v1";
pub const NODE_ENROLLMENT_SIGNATURE_NAMESPACE: &str = "cynic-node-enrollment";
pub const NODE_ENROLLMENT_RENEWAL_REQUEST_SCHEMA: &str = "cynic.node_enrollment_renewal_request.v1";
pub const NODE_ENROLLMENT_DESIRED_STATE_SCHEMA: &str = "cynic.node_enrollment.desired_state.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentDesiredState {
    pub schema: String,
    pub authority_role: String,
    pub endpoint_service: String,
    pub credential_delivery: String,
    pub public_key_delivery: String,
    pub authority_policy: MachineEnrollmentAuthorityPolicy,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineEnrollmentAuthorityPolicy {
    pub authority_id: String,
    pub token_ttl_s: u64,
    pub renewal_authority_ttl_s: u64,
    pub max_grant_ttl_s: u64,
    #[serde(default)]
    pub allowed_roles: Vec<String>,
    #[serde(default)]
    pub allowed_capabilities: Vec<String>,
}

pub fn validate_node_enrollment_desired_state(desired: &NodeEnrollmentDesiredState) -> Vec<String> {
    let mut blockers = Vec::new();
    check(
        desired.schema == NODE_ENROLLMENT_DESIRED_STATE_SCHEMA,
        "node_enrollment_desired_state_schema_mismatch",
        &mut blockers,
    );
    check(
        !desired.authority_role.trim().is_empty(),
        "node_enrollment_authority_role_missing",
        &mut blockers,
    );
    check(
        !desired.endpoint_service.trim().is_empty(),
        "node_enrollment_endpoint_service_missing",
        &mut blockers,
    );
    check(
        desired.credential_delivery == "machine_enrollment_staging",
        "node_enrollment_credential_delivery_unsupported",
        &mut blockers,
    );
    check(
        desired.public_key_delivery == "machine_enrollment_staging",
        "node_enrollment_public_key_delivery_unsupported",
        &mut blockers,
    );
    let policy = &desired.authority_policy;
    check(
        !policy.authority_id.trim().is_empty(),
        "node_enrollment_authority_id_missing",
        &mut blockers,
    );
    check(
        policy.token_ttl_s > 0,
        "node_enrollment_token_ttl_invalid",
        &mut blockers,
    );
    check(
        policy.renewal_authority_ttl_s >= policy.token_ttl_s,
        "node_enrollment_renewal_ttl_invalid",
        &mut blockers,
    );
    check(
        policy.max_grant_ttl_s > 0 && policy.max_grant_ttl_s <= policy.renewal_authority_ttl_s,
        "node_enrollment_grant_ttl_invalid",
        &mut blockers,
    );
    check(
        !policy.allowed_roles.is_empty()
            && policy
                .allowed_roles
                .iter()
                .all(|role| !role.trim().is_empty()),
        "node_enrollment_allowed_roles_missing",
        &mut blockers,
    );
    check(
        !policy.allowed_capabilities.is_empty()
            && policy
                .allowed_capabilities
                .iter()
                .all(|capability| !capability.trim().is_empty()),
        "node_enrollment_allowed_capabilities_missing",
        &mut blockers,
    );
    blockers
}

pub fn required_node_enrollment_capabilities(role: &str) -> Vec<String> {
    let mut capabilities = vec!["node_control".to_string(), role.to_string()];
    if role == "build_runner" {
        capabilities.extend(
            crate::BUILD_SCHEDULER_RUNTIME_CAPABILITIES
                .iter()
                .map(|capability| (*capability).to_string()),
        );
    }
    capabilities
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentRequest {
    pub schema: String,
    pub request_id: String,
    pub seed_id: String,
    pub node_id: String,
    pub role: String,
    pub machine_identity_digest: String,
    pub machine_public_key_hex: String,
    pub nonce: String,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    #[serde(default)]
    pub requested_capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineEnrollmentAuthority {
    pub authority_id: String,
    pub seed_id: String,
    pub token_digest: String,
    pub expires_at_unix_s: u64,
    pub renewal_authority_expires_at_unix_s: u64,
    pub max_grant_ttl_s: u64,
    #[serde(default)]
    pub allowed_roles: Vec<String>,
    #[serde(default)]
    pub allowed_capabilities: Vec<String>,
}

/// Completeness blockers for a machine enrollment authority record.
///
/// Pure shape check (no clock, no request): every field the admission and
/// renewal paths dereference must be present and non-degenerate. Sorted and
/// deduped like every other blocker list in this module.
///
/// Canonical home since 2026-09-28; moved up from the node authority loader,
/// which collapsed all six cases into one message. The loader keeps its
/// single message (behavior-preserving delegation); the per-field codes
/// below exist for M3 admission composition and diagnostics.
pub fn validate_machine_enrollment_authority(
    authority: &MachineEnrollmentAuthority,
) -> Vec<String> {
    let mut blockers = Vec::new();
    check(
        !authority.authority_id.trim().is_empty(),
        "node_enrollment_authority_id_missing",
        &mut blockers,
    );
    check(
        !authority.seed_id.trim().is_empty(),
        "node_enrollment_authority_seed_missing",
        &mut blockers,
    );
    check(
        !authority.token_digest.trim().is_empty(),
        NODE_ENROLLMENT_AUTHORITY_TOKEN_MISSING,
        &mut blockers,
    );
    check(
        authority.max_grant_ttl_s > 0,
        "node_enrollment_grant_ttl_invalid",
        &mut blockers,
    );
    check(
        authority.renewal_authority_expires_at_unix_s > 0,
        "node_enrollment_renewal_authority_invalid",
        &mut blockers,
    );
    check(
        !authority.allowed_roles.is_empty(),
        "node_enrollment_allowed_roles_missing",
        &mut blockers,
    );
    blockers.sort();
    blockers.dedup();
    blockers
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentAdmission {
    pub status: String,
    pub request_id: String,
    pub node_id: String,
    pub role: String,
    pub granted_capabilities: Vec<String>,
    pub grant_expires_at_unix_s: Option<u64>,
    pub blockers: Vec<String>,
}

/// Blocker code for a missing single-use issuance token in an authority record.
/// Shared between the completeness validator and the renewal path, which
/// consumes the token at issuance and must not require it back.
pub const NODE_ENROLLMENT_AUTHORITY_TOKEN_MISSING: &str = "node_enrollment_authority_token_missing";

pub fn admit_node_enrollment(
    request: &NodeEnrollmentRequest,
    authority: &MachineEnrollmentAuthority,
    presented_token_digest: &str,
    now_unix_s: u64,
    require_token: bool,
) -> NodeEnrollmentAdmission {
    // M3 composition: completeness first, so a degenerate authority record
    // blocks before any relational check dereferences its fields. Issuance
    // requires the single-use token; renewal consumed it and passes false.
    let mut blockers: Vec<String> = validate_machine_enrollment_authority(authority)
        .into_iter()
        .filter(|blocker| require_token || blocker != NODE_ENROLLMENT_AUTHORITY_TOKEN_MISSING)
        .collect();
    check(
        request.schema == NODE_ENROLLMENT_REQUEST_SCHEMA,
        "node_enrollment_request_schema_mismatch",
        &mut blockers,
    );
    check(
        request.seed_id == authority.seed_id,
        "node_enrollment_seed_authority_mismatch",
        &mut blockers,
    );
    check(
        authority.token_digest == presented_token_digest,
        "node_enrollment_token_invalid",
        &mut blockers,
    );
    check(
        now_unix_s > 0
            && now_unix_s < authority.expires_at_unix_s
            && now_unix_s < request.expires_at_unix_s,
        "node_enrollment_request_or_token_expired",
        &mut blockers,
    );
    check(
        authority.allowed_roles.contains(&request.role),
        "node_enrollment_role_not_allowed",
        &mut blockers,
    );
    check(
        request
            .requested_capabilities
            .iter()
            .all(|capability| authority.allowed_capabilities.contains(capability)),
        "node_enrollment_capability_not_allowed",
        &mut blockers,
    );
    let public_key_digest = hex::decode(&request.machine_public_key_hex)
        .ok()
        .filter(|bytes| bytes.len() == 32)
        .map(|bytes| {
            use sha2::{Digest, Sha256};
            format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
        });
    check(
        public_key_digest.as_deref() == Some(request.machine_identity_digest.as_str()),
        "node_enrollment_machine_identity_invalid",
        &mut blockers,
    );
    let grant_expiry = now_unix_s
        .checked_add(authority.max_grant_ttl_s)
        .map(|value| value.min(request.expires_at_unix_s));
    check(
        authority.max_grant_ttl_s > 0 && grant_expiry.is_some(),
        "node_enrollment_grant_lifetime_invalid",
        &mut blockers,
    );
    blockers.sort();
    blockers.dedup();
    let admitted = blockers.is_empty();
    NodeEnrollmentAdmission {
        status: if admitted { "admitted" } else { "blocked" }.to_string(),
        request_id: request.request_id.clone(),
        node_id: request.node_id.clone(),
        role: request.role.clone(),
        granted_capabilities: if admitted {
            request.requested_capabilities.clone()
        } else {
            Vec::new()
        },
        grant_expires_at_unix_s: if admitted { grant_expiry } else { None },
        blockers,
    }
}

pub fn admit_node_enrollment_renewal(
    renewal: &NodeEnrollmentRenewalRequest,
    current: &NodeEnrollmentGrant,
    authority: &MachineEnrollmentAuthority,
    now_unix_s: u64,
) -> NodeEnrollmentAdmission {
    let request = &renewal.request;
    let mut authority_without_token = authority.clone();
    authority_without_token.token_digest.clear();
    authority_without_token.expires_at_unix_s = authority.renewal_authority_expires_at_unix_s;
    let mut admission =
        admit_node_enrollment(request, &authority_without_token, "", now_unix_s, false);
    for (condition, blocker) in [
        (
            renewal.schema == NODE_ENROLLMENT_RENEWAL_REQUEST_SCHEMA,
            "node_enrollment_renewal_schema_mismatch",
        ),
        (
            renewal.current_grant_id == current.grant_id
                && renewal.expected_generation == current.generation,
            "node_enrollment_renewal_fence_mismatch",
        ),
        (
            request.node_id == current.node_id
                && request.role == current.role
                && request.machine_identity_digest == current.machine_identity_digest,
            "node_enrollment_renewal_identity_mismatch",
        ),
        (
            request
                .requested_capabilities
                .iter()
                .all(|capability| current.granted_capabilities.contains(capability)),
            "node_enrollment_renewal_capability_amplification",
        ),
        (
            current.expires_at_unix_s > now_unix_s,
            "node_enrollment_current_grant_expired",
        ),
        (
            authority.renewal_authority_expires_at_unix_s > now_unix_s,
            "node_enrollment_renewal_authority_expired",
        ),
    ] {
        check(condition, blocker, &mut admission.blockers);
    }
    admission.blockers.sort();
    admission.blockers.dedup();
    if !admission.blockers.is_empty() {
        admission.status = "blocked".to_string();
        admission.granted_capabilities.clear();
        admission.grant_expires_at_unix_s = None;
    }
    admission
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentGrant {
    pub schema: String,
    pub grant_id: String,
    pub authority_id: String,
    pub request_id: String,
    pub seed_id: String,
    pub node_id: String,
    pub role: String,
    pub machine_identity_digest: String,
    pub generation: u64,
    pub issued_at_unix_s: u64,
    pub expires_at_unix_s: u64,
    #[serde(default)]
    pub granted_capabilities: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentRenewalRequest {
    pub schema: String,
    pub current_grant_id: String,
    pub expected_generation: u64,
    pub request: NodeEnrollmentRequest,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SignedNodeEnrollmentGrant {
    pub schema: String,
    pub authority_key_id: String,
    pub signature_namespace: String,
    pub grant: NodeEnrollmentGrant,
    pub signature_hex: String,
}

pub fn node_enrollment_grant_signing_bytes(
    signed: &SignedNodeEnrollmentGrant,
) -> Result<Vec<u8>, serde_json::Error> {
    cynic_protocol::canonical_json_bytes(&serde_json::json!({
        "schema": signed.schema,
        "authority_key_id": signed.authority_key_id,
        "signature_namespace": signed.signature_namespace,
        "grant": signed.grant,
    }))
}

pub fn validate_signed_node_enrollment_grant(signed: &SignedNodeEnrollmentGrant) -> Vec<String> {
    let mut violations = Vec::new();
    check(
        signed.schema == SIGNED_NODE_ENROLLMENT_GRANT_SCHEMA,
        "signed_enrollment_grant_schema_mismatch",
        &mut violations,
    );
    check(
        signed.signature_namespace == NODE_ENROLLMENT_SIGNATURE_NAMESPACE,
        "signed_enrollment_grant_namespace_mismatch",
        &mut violations,
    );
    check(
        valid_sha256(&signed.authority_key_id),
        "signed_enrollment_authority_key_id_invalid",
        &mut violations,
    );
    check(
        signed.signature_hex.len() == 128
            && signed
                .signature_hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit()),
        "signed_enrollment_signature_invalid",
        &mut violations,
    );
    violations
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentObservation {
    pub schema: String,
    pub observed_at_unix_s: u64,
    pub node_id: String,
    pub machine_identity_digest: String,
    pub machine_identity: SecurityEvidenceState,
    pub authority_signature: SecurityEvidenceState,
    pub prior_enrollment: SecurityEvidenceState,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeEnrollmentDecision {
    AlreadyEnrolled,
    PersistGrant,
    Unknown,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeEnrollmentPlan {
    pub schema: String,
    pub status: String,
    pub node_id: String,
    pub role: String,
    pub decision: NodeEnrollmentDecision,
    pub granted_capabilities: Vec<String>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub rollback_intent: String,
}

pub fn reconcile_node_enrollment(
    request: &NodeEnrollmentRequest,
    grant: &NodeEnrollmentGrant,
    observed: &NodeEnrollmentObservation,
) -> NodeEnrollmentPlan {
    let mut blockers = Vec::new();
    let mut unknowns = observed.unknowns.clone();
    check(
        request.schema == NODE_ENROLLMENT_REQUEST_SCHEMA,
        "node_enrollment_request_schema_mismatch",
        &mut blockers,
    );
    check(
        grant.schema == NODE_ENROLLMENT_GRANT_SCHEMA,
        "node_enrollment_grant_schema_mismatch",
        &mut blockers,
    );
    check(
        observed.schema == NODE_ENROLLMENT_OBSERVATION_SCHEMA,
        "node_enrollment_observation_schema_mismatch",
        &mut blockers,
    );
    for (condition, blocker) in [
        (
            !request.request_id.trim().is_empty(),
            "enrollment_request_id_missing",
        ),
        (
            !request.seed_id.trim().is_empty(),
            "enrollment_seed_id_missing",
        ),
        (
            !request.node_id.trim().is_empty(),
            "enrollment_node_id_missing",
        ),
        (!request.role.trim().is_empty(), "enrollment_role_missing"),
        (!request.nonce.trim().is_empty(), "enrollment_nonce_missing"),
        (
            !grant.grant_id.trim().is_empty(),
            "enrollment_grant_id_missing",
        ),
        (
            !grant.authority_id.trim().is_empty(),
            "enrollment_authority_id_missing",
        ),
    ] {
        check(condition, blocker, &mut blockers);
    }
    check(
        valid_sha256(&request.seed_id),
        "enrollment_seed_digest_invalid",
        &mut blockers,
    );
    check(
        valid_sha256(&request.machine_identity_digest),
        "enrollment_machine_identity_digest_invalid",
        &mut blockers,
    );
    check(
        request.machine_public_key_hex.len() == 64
            && request
                .machine_public_key_hex
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit()),
        "enrollment_machine_public_key_invalid",
        &mut blockers,
    );
    check(
        grant.generation > 0,
        "enrollment_grant_generation_invalid",
        &mut blockers,
    );
    check(
        request.request_id == grant.request_id,
        "enrollment_request_binding_mismatch",
        &mut blockers,
    );
    check(
        request.seed_id == grant.seed_id,
        "enrollment_seed_binding_mismatch",
        &mut blockers,
    );
    check(
        request.node_id == grant.node_id && request.node_id == observed.node_id,
        "enrollment_node_binding_mismatch",
        &mut blockers,
    );
    check(
        request.role == grant.role,
        "enrollment_role_binding_mismatch",
        &mut blockers,
    );
    check(
        request.machine_identity_digest == grant.machine_identity_digest
            && request.machine_identity_digest == observed.machine_identity_digest,
        "enrollment_machine_identity_binding_mismatch",
        &mut blockers,
    );
    check(
        request.issued_at_unix_s > 0
            && request.expires_at_unix_s > request.issued_at_unix_s
            && grant.issued_at_unix_s >= request.issued_at_unix_s
            && grant.expires_at_unix_s <= request.expires_at_unix_s
            && observed.observed_at_unix_s < grant.expires_at_unix_s,
        "enrollment_lifetime_invalid_or_expired",
        &mut blockers,
    );
    check(
        grant
            .granted_capabilities
            .iter()
            .all(|capability| request.requested_capabilities.contains(capability)),
        "enrollment_capability_amplification",
        &mut blockers,
    );
    check(
        !grant.evidence_refs.is_empty(),
        "enrollment_grant_evidence_missing",
        &mut blockers,
    );

    match observed.machine_identity {
        SecurityEvidenceState::Absent => blockers.push("machine_identity_absent".to_string()),
        SecurityEvidenceState::Refuted | SecurityEvidenceState::NotApplicable => {
            blockers.push("machine_identity_invalid".to_string())
        }
        SecurityEvidenceState::Unknown | SecurityEvidenceState::Declared => {
            unknowns.push("machine_identity_unproved".to_string())
        }
        SecurityEvidenceState::Proven => {}
    }
    match observed.authority_signature {
        SecurityEvidenceState::Absent
        | SecurityEvidenceState::Refuted
        | SecurityEvidenceState::NotApplicable => {
            blockers.push("enrollment_signature_invalid".to_string())
        }
        SecurityEvidenceState::Unknown | SecurityEvidenceState::Declared => {
            unknowns.push("enrollment_signature_unproved".to_string())
        }
        SecurityEvidenceState::Proven => {}
    }

    blockers.sort();
    blockers.dedup();
    unknowns.sort();
    unknowns.dedup();
    let decision = if !blockers.is_empty() {
        NodeEnrollmentDecision::Blocked
    } else if !unknowns.is_empty() {
        NodeEnrollmentDecision::Unknown
    } else if observed.prior_enrollment == SecurityEvidenceState::Proven {
        NodeEnrollmentDecision::AlreadyEnrolled
    } else {
        NodeEnrollmentDecision::PersistGrant
    };
    let status = match decision {
        NodeEnrollmentDecision::AlreadyEnrolled => "converged",
        NodeEnrollmentDecision::PersistGrant => "planned",
        NodeEnrollmentDecision::Unknown => "unknown",
        NodeEnrollmentDecision::Blocked => "blocked",
    };

    let mut evidence_refs = observed.evidence_refs.clone();
    evidence_refs.extend(grant.evidence_refs.clone());
    evidence_refs.sort();
    evidence_refs.dedup();
    NodeEnrollmentPlan {
        schema: NODE_ENROLLMENT_PLAN_SCHEMA.to_string(),
        status: status.to_string(),
        node_id: request.node_id.clone(),
        role: request.role.clone(),
        decision,
        granted_capabilities: grant.granted_capabilities.clone(),
        blockers,
        unknowns,
        evidence_refs,
        rollback_intent:
            "invalidate the enrollment grant and retain the machine in seed-only state".to_string(),
    }
}

fn check(condition: bool, blocker: &str, blockers: &mut Vec<String>) {
    if !condition {
        blockers.push(blocker.to_string());
    }
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> NodeEnrollmentRequest {
        use sha2::{Digest, Sha256};
        let machine_public_key_hex = "b".repeat(64);
        let machine_identity_digest = format!(
            "sha256:{}",
            hex::encode(Sha256::digest(
                hex::decode(&machine_public_key_hex).unwrap()
            ))
        );
        NodeEnrollmentRequest {
            schema: NODE_ENROLLMENT_REQUEST_SCHEMA.to_string(),
            request_id: "request-1".to_string(),
            seed_id: format!("sha256:{}", "a".repeat(64)),
            node_id: "node-a".to_string(),
            role: "build_runner".to_string(),
            machine_identity_digest,
            machine_public_key_hex,
            nonce: "nonce-1".to_string(),
            issued_at_unix_s: 100,
            expires_at_unix_s: 200,
            requested_capabilities: vec!["build.execute".to_string(), "proof.publish".to_string()],
        }
    }

    fn grant() -> NodeEnrollmentGrant {
        let request = request();
        NodeEnrollmentGrant {
            schema: NODE_ENROLLMENT_GRANT_SCHEMA.to_string(),
            grant_id: "grant-1".to_string(),
            authority_id: "runtime-authority".to_string(),
            request_id: request.request_id,
            seed_id: request.seed_id,
            node_id: request.node_id,
            role: request.role,
            machine_identity_digest: request.machine_identity_digest,
            generation: 1,
            issued_at_unix_s: 110,
            expires_at_unix_s: 190,
            granted_capabilities: vec!["proof.publish".to_string()],
            evidence_refs: vec!["sha256:grant-proof".to_string()],
        }
    }

    fn authority() -> MachineEnrollmentAuthority {
        MachineEnrollmentAuthority {
            authority_id: "enrollment-authority".to_string(),
            seed_id: request().seed_id,
            token_digest: format!("sha256:{}", "c".repeat(64)),
            expires_at_unix_s: 150,
            renewal_authority_expires_at_unix_s: 1_000,
            max_grant_ttl_s: 25,
            allowed_roles: vec!["build_runner".to_string()],
            allowed_capabilities: vec!["build.execute".to_string(), "proof.publish".to_string()],
        }
    }

    #[test]
    fn desired_state_requires_one_canonical_staging_contract() {
        let desired = NodeEnrollmentDesiredState {
            schema: NODE_ENROLLMENT_DESIRED_STATE_SCHEMA.to_string(),
            authority_role: "kernel_primary".to_string(),
            endpoint_service: "cynic-kernel".to_string(),
            credential_delivery: "machine_enrollment_staging".to_string(),
            public_key_delivery: "machine_enrollment_staging".to_string(),
            authority_policy: MachineEnrollmentAuthorityPolicy {
                authority_id: "cynic-runtime-machine-enrollment".to_string(),
                token_ttl_s: 900,
                renewal_authority_ttl_s: 2_592_000,
                max_grant_ttl_s: 86_400,
                allowed_roles: vec!["build_runner".to_string()],
                allowed_capabilities: vec!["build.execute".to_string()],
            },
        };
        assert!(validate_node_enrollment_desired_state(&desired).is_empty());

        let mut unsupported = desired;
        unsupported.credential_delivery = "manual_grant".to_string();
        assert_eq!(
            validate_node_enrollment_desired_state(&unsupported),
            vec!["node_enrollment_credential_delivery_unsupported"]
        );
    }

    #[test]
    fn bootstrap_token_admits_only_bounded_role_and_capabilities() {
        let request = request();
        let authority = authority();
        let admission =
            admit_node_enrollment(&request, &authority, &authority.token_digest, 110, true);
        assert_eq!(admission.status, "admitted");
        assert_eq!(admission.grant_expires_at_unix_s, Some(135));
        assert_eq!(
            admission.granted_capabilities,
            request.requested_capabilities
        );
    }

    #[test]
    fn bootstrap_token_cannot_amplify_capabilities() {
        let mut request = request();
        request
            .requested_capabilities
            .push("runtime.admin".to_string());
        let authority = authority();
        let admission =
            admit_node_enrollment(&request, &authority, &authority.token_digest, 110, true);
        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"node_enrollment_capability_not_allowed".to_string())
        );
    }

    #[test]
    fn degenerate_authority_blocks_before_relational_checks() {
        let request = request();
        let mut authority = authority();
        authority.authority_id.clear();
        let admission =
            admit_node_enrollment(&request, &authority, &authority.token_digest, 110, true);
        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"node_enrollment_authority_id_missing".to_string())
        );
    }

    #[test]
    fn issuance_without_presented_token_is_refused() {
        let request = request();
        let authority = authority();
        let admission = admit_node_enrollment(&request, &authority, "", 110, true);
        assert_eq!(admission.status, "blocked");
        assert!(
            admission
                .blockers
                .contains(&"node_enrollment_token_invalid".to_string())
        );
    }

    #[test]
    fn renewal_uses_current_grant_generation_as_fence() {
        let current = grant();
        let mut request = request();
        request.requested_capabilities = current.granted_capabilities.clone();
        let renewal = NodeEnrollmentRenewalRequest {
            schema: NODE_ENROLLMENT_RENEWAL_REQUEST_SCHEMA.to_string(),
            current_grant_id: current.grant_id.clone(),
            expected_generation: current.generation,
            request,
        };
        let admission = admit_node_enrollment_renewal(&renewal, &current, &authority(), 120);
        assert_eq!(admission.status, "admitted");

        let mut stale = renewal;
        stale.expected_generation = 0;
        let rejected = admit_node_enrollment_renewal(&stale, &current, &authority(), 120);
        assert!(
            rejected
                .blockers
                .contains(&"node_enrollment_renewal_fence_mismatch".to_string())
        );
    }

    fn observation() -> NodeEnrollmentObservation {
        let request = request();
        NodeEnrollmentObservation {
            schema: NODE_ENROLLMENT_OBSERVATION_SCHEMA.to_string(),
            observed_at_unix_s: 120,
            node_id: request.node_id,
            machine_identity_digest: request.machine_identity_digest,
            machine_identity: SecurityEvidenceState::Proven,
            authority_signature: SecurityEvidenceState::Proven,
            prior_enrollment: SecurityEvidenceState::Absent,
            evidence_refs: vec!["sha256:machine-proof".to_string()],
            unknowns: Vec::new(),
        }
    }

    #[test]
    fn valid_grant_plans_persistence_without_capability_amplification() {
        let plan = reconcile_node_enrollment(&request(), &grant(), &observation());
        assert_eq!(plan.status, "planned");
        assert_eq!(plan.decision, NodeEnrollmentDecision::PersistGrant);
        assert_eq!(plan.granted_capabilities, vec!["proof.publish"]);
    }

    #[test]
    fn grant_cannot_amplify_requested_capabilities() {
        let mut grant = grant();
        grant.granted_capabilities.push("host.root".to_string());
        let plan = reconcile_node_enrollment(&request(), &grant, &observation());
        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .contains(&"enrollment_capability_amplification".to_string())
        );
    }

    #[test]
    fn unproved_machine_identity_remains_unknown() {
        let mut observed = observation();
        observed.machine_identity = SecurityEvidenceState::Unknown;
        let plan = reconcile_node_enrollment(&request(), &grant(), &observed);
        assert_eq!(plan.status, "unknown");
        assert_eq!(plan.decision, NodeEnrollmentDecision::Unknown);
    }

    #[test]
    fn refuted_machine_identity_blocks_enrollment() {
        let mut observed = observation();
        observed.machine_identity = SecurityEvidenceState::Refuted;
        let plan = reconcile_node_enrollment(&request(), &grant(), &observed);
        assert_eq!(plan.status, "blocked");
        assert!(
            plan.blockers
                .contains(&"machine_identity_invalid".to_string())
        );
    }

    #[test]
    fn authority_completeness_names_each_missing_field() {
        assert!(validate_machine_enrollment_authority(&authority()).is_empty());
        for (mutate, code) in [
            (
                (|authority: &mut MachineEnrollmentAuthority| authority.authority_id.clear())
                    as fn(&mut _),
                "node_enrollment_authority_id_missing",
            ),
            (
                (|authority: &mut MachineEnrollmentAuthority| authority.seed_id.clear()),
                "node_enrollment_authority_seed_missing",
            ),
            (
                (|authority: &mut MachineEnrollmentAuthority| authority.token_digest.clear()),
                "node_enrollment_authority_token_missing",
            ),
            (
                (|authority: &mut MachineEnrollmentAuthority| authority.max_grant_ttl_s = 0),
                "node_enrollment_grant_ttl_invalid",
            ),
            (
                (|authority: &mut MachineEnrollmentAuthority| {
                    authority.renewal_authority_expires_at_unix_s = 0
                }),
                "node_enrollment_renewal_authority_invalid",
            ),
            (
                (|authority: &mut MachineEnrollmentAuthority| authority.allowed_roles.clear()),
                "node_enrollment_allowed_roles_missing",
            ),
        ] {
            let mut broken = authority();
            mutate(&mut broken);
            assert_eq!(
                validate_machine_enrollment_authority(&broken),
                vec![code.to_string()]
            );
        }
    }
}
