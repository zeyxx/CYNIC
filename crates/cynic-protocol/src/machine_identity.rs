use serde::{Deserialize, Serialize};

pub const MACHINE_IDENTITY_SPEC_SCHEMA: &str = "cynic.machine_identity_spec.v1";
pub const MACHINE_IDENTITY_OBSERVATION_SCHEMA: &str = "cynic.machine_identity_observation.v1";
pub const MACHINE_IDENTITY_PLAN_SCHEMA: &str = "cynic.machine_identity_plan.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SecretClass {
    HumanAdmin,
    MachineEnrollment,
    RuntimeMachineIdentity,
    RecoveryBreakglass,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecretProvider {
    ProcessEnvironment,
    PrincipalOwnedFile,
    OsCredentialStore,
    ExternalSecretStore,
}

/// An inspectable reference to secret material. `locator` names an environment
/// variable, file, credential id, or external-store key; it never contains the
/// secret value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SecretReference {
    pub id: String,
    pub class: SecretClass,
    pub provider: SecretProvider,
    pub locator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineIdentitySpec {
    pub schema: String,
    pub identity_id: String,
    pub principal: String,
    pub consumer: String,
    pub secret_sources: Vec<SecretReference>,
    pub capabilities: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineIdentityObservation {
    pub schema: String,
    pub identity_id: String,
    pub principal: Option<String>,
    pub provider: Option<SecretProvider>,
    pub secret_present: bool,
    pub secret_exposed: bool,
    pub capabilities: Vec<String>,
    pub observed_at_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MachineIdentityDecision {
    Observe,
    Enroll,
    Rotate,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MachineIdentityPlan {
    pub schema: String,
    pub identity_id: String,
    pub decision: MachineIdentityDecision,
    pub requires_enrollment_authority: bool,
    pub runtime_human_secret_dependency: bool,
    pub required_proofs: Vec<String>,
    pub violations: Vec<String>,
}

pub fn reconcile_machine_identity(
    spec: &MachineIdentitySpec,
    observed: &MachineIdentityObservation,
) -> MachineIdentityPlan {
    let mut violations = Vec::new();
    if spec.schema != MACHINE_IDENTITY_SPEC_SCHEMA {
        violations.push("machine_identity_spec_schema_mismatch".to_string());
    }
    if observed.schema != MACHINE_IDENTITY_OBSERVATION_SCHEMA {
        violations.push("machine_identity_observation_schema_mismatch".to_string());
    }
    if spec.identity_id.is_empty() || observed.identity_id != spec.identity_id {
        violations.push("machine_identity_subject_mismatch".to_string());
    }
    if spec.principal.is_empty() || spec.consumer.is_empty() {
        violations.push("machine_identity_owner_or_consumer_missing".to_string());
    }
    if spec.secret_sources.is_empty() {
        violations.push("machine_identity_secret_source_missing".to_string());
    }
    if spec
        .secret_sources
        .iter()
        .any(|source| source.class != SecretClass::RuntimeMachineIdentity)
    {
        violations.push("runtime_identity_uses_non_runtime_secret_class".to_string());
    }
    if observed.secret_exposed {
        violations.push("machine_identity_secret_exposed".to_string());
    }

    let capabilities_match = spec
        .capabilities
        .iter()
        .all(|capability| observed.capabilities.contains(capability));
    let principal_matches = observed.principal.as_deref() == Some(spec.principal.as_str());

    let decision = if !violations.is_empty() {
        MachineIdentityDecision::Blocked
    } else if !observed.secret_present {
        MachineIdentityDecision::Enroll
    } else if !principal_matches || !capabilities_match {
        MachineIdentityDecision::Rotate
    } else {
        MachineIdentityDecision::Observe
    };

    MachineIdentityPlan {
        schema: MACHINE_IDENTITY_PLAN_SCHEMA.to_string(),
        identity_id: spec.identity_id.clone(),
        decision,
        requires_enrollment_authority: matches!(
            decision,
            MachineIdentityDecision::Enroll | MachineIdentityDecision::Rotate
        ),
        runtime_human_secret_dependency: false,
        required_proofs: vec![
            "credential_presence_without_value".to_string(),
            "principal_read_back".to_string(),
            "effective_capability_read_back".to_string(),
        ],
        violations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> MachineIdentitySpec {
        MachineIdentitySpec {
            schema: MACHINE_IDENTITY_SPEC_SCHEMA.to_string(),
            identity_id: "storage-runtime".to_string(),
            principal: "cynic-runtime".to_string(),
            consumer: "cynic-kernel".to_string(),
            secret_sources: vec![SecretReference {
                id: "storage-password-file".to_string(),
                class: SecretClass::RuntimeMachineIdentity,
                provider: SecretProvider::PrincipalOwnedFile,
                locator: "/etc/cynic/secrets/storage-runtime-password".to_string(),
            }],
            capabilities: vec!["database_schema_editor".to_string()],
        }
    }

    #[test]
    fn missing_runtime_identity_requires_bounded_enrollment() {
        let observed = MachineIdentityObservation {
            schema: MACHINE_IDENTITY_OBSERVATION_SCHEMA.to_string(),
            identity_id: "storage-runtime".to_string(),
            principal: None,
            provider: None,
            secret_present: false,
            secret_exposed: false,
            capabilities: vec![],
            observed_at_ms: 1,
        };

        let plan = reconcile_machine_identity(&spec(), &observed);
        assert_eq!(plan.decision, MachineIdentityDecision::Enroll);
        assert!(plan.requires_enrollment_authority);
        assert!(!plan.runtime_human_secret_dependency);
    }

    #[test]
    fn human_admin_secret_is_rejected_from_runtime_identity() {
        let mut desired = spec();
        desired.secret_sources[0].class = SecretClass::HumanAdmin;
        let observed = MachineIdentityObservation {
            schema: MACHINE_IDENTITY_OBSERVATION_SCHEMA.to_string(),
            identity_id: "storage-runtime".to_string(),
            principal: Some("cynic-runtime".to_string()),
            provider: Some(SecretProvider::PrincipalOwnedFile),
            secret_present: true,
            secret_exposed: false,
            capabilities: vec!["database_schema_editor".to_string()],
            observed_at_ms: 1,
        };

        let plan = reconcile_machine_identity(&desired, &observed);
        assert_eq!(plan.decision, MachineIdentityDecision::Blocked);
        assert!(
            plan.violations
                .contains(&"runtime_identity_uses_non_runtime_secret_class".to_string())
        );
    }
}
