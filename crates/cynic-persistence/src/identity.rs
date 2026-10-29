use serde::{Deserialize, Serialize};

use cynic_protocol::SecretClass;

pub const STORAGE_IDENTITY_SPEC_SCHEMA: &str = "cynic.storage_identity_spec.v1";
pub const STORAGE_IDENTITY_OBSERVATION_SCHEMA: &str = "cynic.storage_identity_observation.v1";
pub const STORAGE_IDENTITY_PLAN_SCHEMA: &str = "cynic.storage_identity_plan.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageIdentityRole {
    Editor,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageEvidenceState {
    Present,
    Absent,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageIdentitySpec {
    pub schema: String,
    pub identity_id: String,
    pub endpoint: String,
    pub namespace: String,
    pub database: String,
    pub username: String,
    pub role: StorageIdentityRole,
    pub token_duration: String,
    pub session_duration: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageIdentityObservation {
    pub schema: String,
    pub identity_id: String,
    pub database_identity: StorageEvidenceState,
    pub observed_role: Option<StorageIdentityRole>,
    pub local_credential: StorageEvidenceState,
    pub runtime_authentication: StorageEvidenceState,
    pub admin_authority_present: bool,
    pub secret_exposed: bool,
    pub observed_at_ms: u64,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageIdentityDecision {
    Observe,
    EnrollAtomically,
    RotateAtomically,
    CompensateDatabaseIdentity,
    RemoveOrphanedLocalCredential,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageIdentityPlan {
    pub schema: String,
    pub identity_id: String,
    pub decision: StorageIdentityDecision,
    pub mutates: bool,
    pub required_secret_class: SecretClass,
    pub requires_explicit_apply: bool,
    pub preconditions: Vec<String>,
    pub expected_proofs: Vec<String>,
    pub compensation: Vec<String>,
    pub blockers: Vec<String>,
}

pub fn reconcile_storage_identity(
    spec: &StorageIdentitySpec,
    observed: &StorageIdentityObservation,
) -> StorageIdentityPlan {
    let mut blockers = validate(spec, observed);
    let unknown_state = [
        observed.database_identity,
        observed.local_credential,
        observed.runtime_authentication,
    ]
    .contains(&StorageEvidenceState::Unknown);
    if unknown_state {
        blockers.push("storage_identity_observation_incomplete".to_string());
    }
    if !observed.unknowns.is_empty() {
        blockers.push("storage_identity_observation_has_residual_unknowns".to_string());
    }

    let decision = if !blockers.is_empty() {
        StorageIdentityDecision::Blocked
    } else {
        match (
            observed.database_identity,
            observed.local_credential,
            observed.runtime_authentication,
        ) {
            (
                StorageEvidenceState::Absent,
                StorageEvidenceState::Absent,
                StorageEvidenceState::Absent,
            ) if observed.admin_authority_present => StorageIdentityDecision::EnrollAtomically,
            (
                StorageEvidenceState::Present,
                StorageEvidenceState::Present,
                StorageEvidenceState::Present,
            ) if observed.observed_role == Some(spec.role) => StorageIdentityDecision::Observe,
            (
                StorageEvidenceState::Present,
                StorageEvidenceState::Present,
                StorageEvidenceState::Absent,
            ) if observed.admin_authority_present => StorageIdentityDecision::RotateAtomically,
            (StorageEvidenceState::Present, StorageEvidenceState::Absent, _)
                if observed.admin_authority_present =>
            {
                StorageIdentityDecision::CompensateDatabaseIdentity
            }
            (StorageEvidenceState::Absent, StorageEvidenceState::Present, _) => {
                StorageIdentityDecision::RemoveOrphanedLocalCredential
            }
            _ => StorageIdentityDecision::Blocked,
        }
    };
    if decision == StorageIdentityDecision::Blocked && blockers.is_empty() {
        blockers.push("storage_identity_state_requires_external_decision".to_string());
    }

    StorageIdentityPlan {
        schema: STORAGE_IDENTITY_PLAN_SCHEMA.to_string(),
        identity_id: spec.identity_id.clone(),
        decision,
        mutates: !matches!(
            decision,
            StorageIdentityDecision::Observe | StorageIdentityDecision::Blocked
        ),
        required_secret_class: SecretClass::HumanAdmin,
        requires_explicit_apply: !matches!(decision, StorageIdentityDecision::Observe),
        preconditions: vec![
            "database_endpoint_observed".to_string(),
            "human_admin_authority_supplied_on_stdin_for_bootstrap_only".to_string(),
            "local_secret_target_absent_or_transactionally_owned".to_string(),
        ],
        expected_proofs: vec![
            "database_user_role_read_back".to_string(),
            "credential_presence_without_value".to_string(),
            "runtime_authentication_read_back".to_string(),
            "secret_exposed_false".to_string(),
        ],
        compensation: vec![
            "remove_database_identity_if_local_install_fails".to_string(),
            "remove_matching_local_credential_after_database_compensation".to_string(),
            "retain_residual_state_as_blocked_if_compensation_fails".to_string(),
        ],
        blockers,
    }
}

pub fn storage_identity_spec_blockers(spec: &StorageIdentitySpec) -> Vec<String> {
    let mut blockers = Vec::new();
    if spec.schema != STORAGE_IDENTITY_SPEC_SCHEMA {
        blockers.push("storage_identity_spec_schema_mismatch".to_string());
    }
    if spec.identity_id != "storage-runtime" {
        blockers.push("storage_identity_subject_invalid".to_string());
    }
    if !valid_identifier(&spec.namespace)
        || !valid_identifier(&spec.database)
        || !valid_identifier(&spec.username)
    {
        blockers.push("storage_identity_identifier_invalid".to_string());
    }
    if !valid_endpoint(&spec.endpoint) {
        blockers.push("storage_identity_endpoint_invalid".to_string());
    }
    if !valid_duration(&spec.token_duration) || !valid_duration(&spec.session_duration) {
        blockers.push("storage_identity_duration_invalid".to_string());
    }
    blockers
}

fn validate(spec: &StorageIdentitySpec, observed: &StorageIdentityObservation) -> Vec<String> {
    let mut blockers = storage_identity_spec_blockers(spec);
    if observed.schema != STORAGE_IDENTITY_OBSERVATION_SCHEMA {
        blockers.push("storage_identity_observation_schema_mismatch".to_string());
    }
    if observed.identity_id != spec.identity_id {
        blockers.push("storage_identity_subject_mismatch".to_string());
    }
    if observed.secret_exposed {
        blockers.push("storage_identity_secret_exposed".to_string());
    }
    blockers
}

fn valid_endpoint(value: &str) -> bool {
    let (secure, authority) = if let Some(authority) = value.strip_prefix("https://") {
        (true, authority)
    } else if let Some(authority) = value.strip_prefix("http://") {
        (false, authority)
    } else {
        return false;
    };
    let authority = authority.strip_suffix('/').unwrap_or(authority);
    if authority.is_empty()
        || authority
            .bytes()
            .any(|byte| matches!(byte, b'/' | b'?' | b'#' | b'@'))
    {
        return false;
    }
    if secure {
        return true;
    }
    authority == "127.0.0.1"
        || authority == "[::1]"
        || authority.strip_prefix("127.0.0.1:").is_some_and(valid_port)
        || authority.strip_prefix("[::1]:").is_some_and(valid_port)
}

fn valid_port(value: &str) -> bool {
    !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit())
}

fn valid_identifier(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

fn valid_duration(value: &str) -> bool {
    let split = value
        .bytes()
        .position(|byte| !byte.is_ascii_digit())
        .unwrap_or(value.len());
    split > 0
        && split < value.len()
        && value[..split].bytes().all(|byte| byte.is_ascii_digit())
        && matches!(&value[split..], "s" | "m" | "h" | "d" | "w")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> StorageIdentitySpec {
        StorageIdentitySpec {
            schema: STORAGE_IDENTITY_SPEC_SCHEMA.to_string(),
            identity_id: "storage-runtime".to_string(),
            endpoint: "http://127.0.0.1:8000".to_string(),
            namespace: "cynic".to_string(),
            database: "main".to_string(),
            username: "cynic-runtime".to_string(),
            role: StorageIdentityRole::Editor,
            token_duration: "15m".to_string(),
            session_duration: "12h".to_string(),
        }
    }

    fn observation(
        database_identity: StorageEvidenceState,
        local_credential: StorageEvidenceState,
        runtime_authentication: StorageEvidenceState,
    ) -> StorageIdentityObservation {
        StorageIdentityObservation {
            schema: STORAGE_IDENTITY_OBSERVATION_SCHEMA.to_string(),
            identity_id: "storage-runtime".to_string(),
            database_identity,
            observed_role: None,
            local_credential,
            runtime_authentication,
            admin_authority_present: true,
            secret_exposed: false,
            observed_at_ms: 1,
            unknowns: Vec::new(),
        }
    }

    #[test]
    fn clean_absence_plans_one_atomic_enrollment() {
        let plan = reconcile_storage_identity(
            &spec(),
            &observation(
                StorageEvidenceState::Absent,
                StorageEvidenceState::Absent,
                StorageEvidenceState::Absent,
            ),
        );
        assert_eq!(plan.decision, StorageIdentityDecision::EnrollAtomically);
        assert!(plan.mutates);
        assert!(!plan.compensation.is_empty());
    }

    #[test]
    fn half_enrolled_database_identity_plans_compensation() {
        let plan = reconcile_storage_identity(
            &spec(),
            &observation(
                StorageEvidenceState::Present,
                StorageEvidenceState::Absent,
                StorageEvidenceState::Absent,
            ),
        );
        assert_eq!(
            plan.decision,
            StorageIdentityDecision::CompensateDatabaseIdentity
        );
    }

    #[test]
    fn unknown_is_never_treated_as_absence() {
        let plan = reconcile_storage_identity(
            &spec(),
            &observation(
                StorageEvidenceState::Unknown,
                StorageEvidenceState::Absent,
                StorageEvidenceState::Absent,
            ),
        );
        assert_eq!(plan.decision, StorageIdentityDecision::Blocked);
        assert!(
            plan.blockers
                .contains(&"storage_identity_observation_incomplete".to_string())
        );
    }

    #[test]
    fn residual_unknowns_and_injected_durations_block_reconciliation() {
        let mut observed = observation(
            StorageEvidenceState::Absent,
            StorageEvidenceState::Absent,
            StorageEvidenceState::Absent,
        );
        observed.unknowns.push("probe_incomplete".to_string());
        let mut invalid = spec();
        invalid.token_duration = "15m; REMOVE USER root".to_string();

        let plan = reconcile_storage_identity(&invalid, &observed);

        assert_eq!(plan.decision, StorageIdentityDecision::Blocked);
        assert!(
            plan.blockers
                .contains(&"storage_identity_duration_invalid".to_string())
        );
        assert!(
            plan.blockers
                .contains(&"storage_identity_observation_has_residual_unknowns".to_string())
        );
    }

    #[test]
    fn plaintext_endpoint_is_bounded_to_loopback() {
        let mut remote_plaintext = spec();
        remote_plaintext.endpoint = "http://10.10.10.101:8000".to_string();
        let mut remote_tls = spec();
        remote_tls.endpoint = "https://surreal.internal:8000".to_string();

        assert!(
            storage_identity_spec_blockers(&remote_plaintext)
                .contains(&"storage_identity_endpoint_invalid".to_string())
        );
        assert!(storage_identity_spec_blockers(&remote_tls).is_empty());
    }
}
