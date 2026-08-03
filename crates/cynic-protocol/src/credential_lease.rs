use serde::{Deserialize, Serialize};

pub const CREDENTIAL_LEASE_DESIRED_STATE_SCHEMA: &str = "cynic.credential_lease.desired_state.v1";
pub const CREDENTIAL_LEASE_OBSERVATION_SCHEMA: &str = "cynic.credential_lease.observation.v1";
pub const CREDENTIAL_LEASE_PLAN_SCHEMA: &str = "cynic.credential_lease.plan.v1";
pub const CREDENTIAL_LEASE_METADATA_SCHEMA: &str = "cynic.credential_lease.metadata.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialLeaseMetadata {
    pub schema: String,
    pub authority_key_id: String,
    pub signature_namespace: String,
    pub signature_hex: String,
    pub provider: String,
    pub subject: String,
    pub audience: String,
    pub generation: u64,
    pub expires_at_unix_seconds: u64,
    pub material_digest: String,
}

pub fn credential_lease_metadata_signing_bytes(
    metadata: &CredentialLeaseMetadata,
) -> Result<Vec<u8>, serde_json::Error> {
    crate::canonical_json_bytes(&serde_json::json!({
        "schema": metadata.schema,
        "authority_key_id": metadata.authority_key_id,
        "signature_namespace": metadata.signature_namespace,
        "provider": metadata.provider,
        "subject": metadata.subject,
        "audience": metadata.audience,
        "generation": metadata.generation,
        "expires_at_unix_seconds": metadata.expires_at_unix_seconds,
        "material_digest": metadata.material_digest,
    }))
}

pub fn validate_credential_lease_metadata(
    metadata: &CredentialLeaseMetadata,
    expected_provider: &str,
    expected_material_digest: Option<&str>,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if metadata.schema != CREDENTIAL_LEASE_METADATA_SCHEMA {
        blockers.push("credential_lease_metadata_schema_mismatch".to_string());
    }
    if !metadata.authority_key_id.starts_with("sha256:") {
        blockers.push("credential_lease_metadata_authority_key_id_invalid".to_string());
    }
    if metadata.signature_namespace.trim().is_empty() {
        blockers.push("credential_lease_metadata_signature_namespace_missing".to_string());
    }
    if metadata.signature_hex.len() != 128
        || !metadata
            .signature_hex
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        blockers.push("credential_lease_metadata_signature_invalid".to_string());
    }
    if metadata.provider != expected_provider {
        blockers.push("credential_lease_metadata_provider_mismatch".to_string());
    }
    if metadata.subject.trim().is_empty() {
        blockers.push("credential_lease_metadata_subject_missing".to_string());
    }
    if metadata.audience.trim().is_empty() {
        blockers.push("credential_lease_metadata_audience_missing".to_string());
    }
    if metadata.generation == 0 {
        blockers.push("credential_lease_metadata_generation_invalid".to_string());
    }
    if !metadata.material_digest.starts_with("sha256:") {
        blockers.push("credential_lease_metadata_digest_invalid".to_string());
    }
    if expected_material_digest != Some(metadata.material_digest.as_str()) {
        blockers.push("credential_lease_metadata_digest_mismatch".to_string());
    }
    blockers
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialLeaseDesiredState {
    pub schema: String,
    pub provider: String,
    pub secret_ref: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lease_metadata_ref: Option<String>,
    pub subject: String,
    pub audience: String,
    pub renewal_window_seconds: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialLeaseState {
    Absent,
    Present,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialLeaseObservation {
    pub schema: String,
    pub state: CredentialLeaseState,
    pub provider: Option<String>,
    pub subject: Option<String>,
    pub audience: Option<String>,
    pub generation: Option<u64>,
    pub expires_at_unix_seconds: Option<u64>,
    pub material_digest: Option<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialLeaseAction {
    Issue,
    Renew,
    Revoke,
    Observe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CredentialLeasePlan {
    pub schema: String,
    pub status: String,
    pub provider: String,
    pub action: Option<CredentialLeaseAction>,
    pub blockers: Vec<String>,
    pub unknowns: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub rollback_intent: String,
}

pub fn reconcile_credential_lease(
    desired: &CredentialLeaseDesiredState,
    observed: &CredentialLeaseObservation,
    now_unix_seconds: u64,
) -> CredentialLeasePlan {
    let mut blockers = validate_desired(desired);
    let mut unknowns = observed.unknowns.clone();
    if observed.schema != CREDENTIAL_LEASE_OBSERVATION_SCHEMA {
        unknowns.push("credential_lease_observation_schema_mismatch".to_string());
    }
    if observed.state == CredentialLeaseState::Unknown {
        unknowns.push("credential_lease_state_unobserved".to_string());
    }
    if observed.state == CredentialLeaseState::Present {
        if observed.provider.as_deref() != Some(desired.provider.as_str()) {
            blockers.push("credential_lease_provider_mismatch".to_string());
        }
        if observed.subject.as_deref() != Some(desired.subject.as_str()) {
            blockers.push("credential_lease_subject_mismatch".to_string());
        }
        if observed.audience.as_deref() != Some(desired.audience.as_str()) {
            blockers.push("credential_lease_audience_mismatch".to_string());
        }
        if observed.material_digest.is_none() {
            unknowns.push("credential_lease_material_digest_unobserved".to_string());
        }
        if observed.generation.is_none() {
            unknowns.push("credential_lease_generation_unobserved".to_string());
        }
        match observed.expires_at_unix_seconds {
            Some(expires_at) if expires_at <= now_unix_seconds => {
                blockers.push("credential_lease_expired".to_string())
            }
            Some(expires_at)
                if expires_at.saturating_sub(now_unix_seconds)
                    <= desired.renewal_window_seconds
                    && blockers.is_empty()
                    && unknowns.is_empty() =>
            {
                return plan(
                    desired,
                    "planned",
                    Some(CredentialLeaseAction::Renew),
                    blockers,
                    unknowns,
                    observed,
                );
            }
            None => unknowns.push("credential_lease_expiry_unobserved".to_string()),
            _ => {}
        }
    }
    let action = match observed.state {
        CredentialLeaseState::Absent => Some(CredentialLeaseAction::Issue),
        CredentialLeaseState::Present => Some(CredentialLeaseAction::Observe),
        CredentialLeaseState::Unknown => None,
    };
    let status = if !blockers.is_empty() {
        "blocked"
    } else if !unknowns.is_empty() {
        "unknown"
    } else if observed.state == CredentialLeaseState::Present {
        "converged"
    } else {
        "planned"
    };
    plan(desired, status, action, blockers, unknowns, observed)
}

fn validate_desired(desired: &CredentialLeaseDesiredState) -> Vec<String> {
    let mut blockers = Vec::new();
    if desired.schema != CREDENTIAL_LEASE_DESIRED_STATE_SCHEMA {
        blockers.push("credential_lease_desired_state_schema_mismatch".to_string());
    }
    for (value, blocker) in [
        (&desired.provider, "credential_lease_provider_missing"),
        (&desired.secret_ref, "credential_lease_secret_ref_missing"),
        (&desired.subject, "credential_lease_subject_missing"),
        (&desired.audience, "credential_lease_audience_missing"),
    ] {
        if value.trim().is_empty() {
            blockers.push(blocker.to_string());
        }
    }
    if desired
        .lease_metadata_ref
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        blockers.push("credential_lease_metadata_ref_invalid".to_string());
    }
    if desired.renewal_window_seconds == 0 {
        blockers.push("credential_lease_renewal_window_invalid".to_string());
    }
    blockers
}

fn plan(
    desired: &CredentialLeaseDesiredState,
    status: &str,
    action: Option<CredentialLeaseAction>,
    mut blockers: Vec<String>,
    mut unknowns: Vec<String>,
    observed: &CredentialLeaseObservation,
) -> CredentialLeasePlan {
    blockers.sort();
    blockers.dedup();
    unknowns.sort();
    unknowns.dedup();
    CredentialLeasePlan {
        schema: CREDENTIAL_LEASE_PLAN_SCHEMA.to_string(),
        status: status.to_string(),
        provider: desired.provider.clone(),
        action,
        blockers,
        unknowns,
        evidence_refs: observed.evidence_refs.clone(),
        rollback_intent: "retain the last proven lease until the replacement is read back; revoke only the prior generation after successful cutover".to_string(),
    }
}
