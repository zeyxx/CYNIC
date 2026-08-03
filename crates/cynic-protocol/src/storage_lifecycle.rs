use serde::{Deserialize, Serialize};

pub const STORAGE_CLASS_SCHEMA: &str = "cynic.storage_class.v1";
pub const STORAGE_VOLUME_SCHEMA: &str = "cynic.storage_volume.v1";
pub const STORAGE_CLAIM_SCHEMA: &str = "cynic.storage_claim.v1";
pub const STORAGE_RESIZE_REQUEST_SCHEMA: &str = "cynic.storage_resize_request.v1";
pub const STORAGE_OBSERVATION_SCHEMA: &str = "cynic.storage_observation.v1";
pub const STORAGE_PLAN_SCHEMA: &str = "cynic.storage_plan.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageManagementAction {
    Observe,
    Create,
    Update,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageReclaimPolicy {
    Retain,
    Quarantine,
    Delete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageResizeMode {
    Temporary,
    Permanent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StoragePhase {
    Unknown,
    Pending,
    Ready,
    Resizing,
    Degraded,
    Quarantined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageClassSpec {
    pub schema: String,
    pub id: String,
    pub provider: String,
    pub management_policy: Vec<StorageManagementAction>,
    pub reclaim_policy: StorageReclaimPolicy,
    pub allow_expansion: bool,
    pub allow_shrink: bool,
    pub minimum_bytes: u64,
    pub maximum_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageVolumeSpec {
    pub schema: String,
    pub id: String,
    pub class_id: String,
    pub provider_resource_id: String,
    pub owner: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageClaimSpec {
    pub schema: String,
    pub id: String,
    pub owner: String,
    pub class_id: String,
    pub requested_bytes: u64,
    pub mode: StorageResizeMode,
    pub expires_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageResizeRequest {
    pub schema: String,
    pub request_id: String,
    pub claim_id: String,
    pub target_bytes: u64,
    pub mode: StorageResizeMode,
    pub requested_at_ms: u64,
    pub expires_at_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageObservation {
    pub schema: String,
    pub volume_id: String,
    pub phase: StoragePhase,
    pub provider_bytes: Option<u64>,
    pub filesystem_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub service_ready: Option<bool>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageDecision {
    Satisfied,
    ExpandProvider,
    ExpandFilesystem,
    MigrateForShrink,
    RetainTemporary,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoragePlan {
    pub schema: String,
    pub claim_id: String,
    pub decision: StorageDecision,
    pub mutates: bool,
    pub target_bytes: u64,
    pub blockers: Vec<String>,
    pub expected_proofs: Vec<String>,
    pub rollback_intent: String,
}

pub fn reconcile_storage(
    class: &StorageClassSpec,
    volume: &StorageVolumeSpec,
    claim: &StorageClaimSpec,
    request: &StorageResizeRequest,
    observed: &StorageObservation,
) -> StoragePlan {
    let mut blockers = validate(class, volume, claim, request, observed);
    let observed_provider = observed.provider_bytes;
    let observed_filesystem = observed.filesystem_bytes;
    let target = request.target_bytes;

    if observed_provider.is_none()
        || observed_filesystem.is_none()
        || observed.service_ready != Some(true)
    {
        blockers.push("storage_resize_read_back_incomplete".to_string());
    }
    if target < class.minimum_bytes || target > class.maximum_bytes {
        blockers.push("storage_resize_outside_class_bounds".to_string());
    }
    if target < claim.requested_bytes {
        blockers.push("storage_resize_below_claim_request".to_string());
    }
    if target < observed_provider.unwrap_or_default() && !class.allow_shrink {
        blockers.push("storage_class_shrink_forbidden".to_string());
    }

    let decision = if !blockers.is_empty() {
        StorageDecision::Blocked
    } else if target == observed_provider.unwrap_or_default()
        && target == observed_filesystem.unwrap_or_default()
    {
        if request.mode == StorageResizeMode::Temporary {
            StorageDecision::RetainTemporary
        } else {
            StorageDecision::Satisfied
        }
    } else if target > observed_provider.unwrap_or_default() {
        if class.allow_expansion
            && class
                .management_policy
                .contains(&StorageManagementAction::Update)
        {
            StorageDecision::ExpandProvider
        } else {
            StorageDecision::Blocked
        }
    } else if target > observed_filesystem.unwrap_or_default() {
        StorageDecision::ExpandFilesystem
    } else if target < observed_provider.unwrap_or_default() {
        StorageDecision::MigrateForShrink
    } else {
        StorageDecision::Blocked
    };

    if decision == StorageDecision::Blocked && blockers.is_empty() {
        blockers.push("storage_resize_requires_external_decision".to_string());
    }
    StoragePlan {
        schema: STORAGE_PLAN_SCHEMA.to_string(),
        claim_id: claim.id.clone(),
        decision,
        mutates: matches!(
            decision,
            StorageDecision::ExpandProvider
                | StorageDecision::ExpandFilesystem
                | StorageDecision::MigrateForShrink
        ),
        target_bytes: target,
        blockers,
        expected_proofs: vec![
            "provider_resize_read_back".to_string(),
            "filesystem_resize_read_back".to_string(),
            "service_health_read_back".to_string(),
            "capacity_read_back".to_string(),
        ],
        rollback_intent: if decision == StorageDecision::MigrateForShrink {
            "retain_old_volume_until_new_binding_and_restore_proof".to_string()
        } else {
            "provider_growth_is_not_reversible_in_place".to_string()
        },
    }
}

fn validate(
    class: &StorageClassSpec,
    volume: &StorageVolumeSpec,
    claim: &StorageClaimSpec,
    request: &StorageResizeRequest,
    observed: &StorageObservation,
) -> Vec<String> {
    let mut blockers = Vec::new();
    if class.schema != STORAGE_CLASS_SCHEMA {
        blockers.push("storage_class_schema_mismatch".to_string());
    }
    if volume.schema != STORAGE_VOLUME_SCHEMA || volume.class_id != class.id {
        blockers.push("storage_volume_contract_mismatch".to_string());
    }
    if claim.schema != STORAGE_CLAIM_SCHEMA || claim.class_id != class.id {
        blockers.push("storage_claim_contract_mismatch".to_string());
    }
    if request.schema != STORAGE_RESIZE_REQUEST_SCHEMA || request.claim_id != claim.id {
        blockers.push("storage_resize_request_contract_mismatch".to_string());
    }
    if observed.schema != STORAGE_OBSERVATION_SCHEMA || observed.volume_id != volume.id {
        blockers.push("storage_observation_contract_mismatch".to_string());
    }
    if class.minimum_bytes > class.maximum_bytes || class.management_policy.is_empty() {
        blockers.push("storage_class_policy_invalid".to_string());
    }
    if request.requested_at_ms == 0 {
        blockers.push("storage_resize_request_timestamp_missing".to_string());
    }
    if request.mode == StorageResizeMode::Temporary && request.expires_at_ms.is_none() {
        blockers.push("temporary_storage_resize_expiry_missing".to_string());
    }
    if request.mode == StorageResizeMode::Permanent && request.expires_at_ms.is_some() {
        blockers.push("permanent_storage_resize_must_not_expire".to_string());
    }
    if !observed.unknowns.is_empty() {
        blockers.push("storage_observation_has_residual_unknowns".to_string());
    }
    blockers
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> (
        StorageClassSpec,
        StorageVolumeSpec,
        StorageClaimSpec,
        StorageResizeRequest,
        StorageObservation,
    ) {
        (
            StorageClassSpec {
                schema: STORAGE_CLASS_SCHEMA.to_string(),
                id: "artifact".to_string(),
                provider: "proxmox".to_string(),
                management_policy: vec![
                    StorageManagementAction::Observe,
                    StorageManagementAction::Update,
                ],
                reclaim_policy: StorageReclaimPolicy::Retain,
                allow_expansion: true,
                allow_shrink: false,
                minimum_bytes: 10,
                maximum_bytes: 1_000,
            },
            StorageVolumeSpec {
                schema: STORAGE_VOLUME_SCHEMA.to_string(),
                id: "vol-1".to_string(),
                class_id: "artifact".to_string(),
                provider_resource_id: "lxc/103:mp0".to_string(),
                owner: "runtime-artifact-store".to_string(),
            },
            StorageClaimSpec {
                schema: STORAGE_CLAIM_SCHEMA.to_string(),
                id: "claim-1".to_string(),
                owner: "runtime-artifact-store".to_string(),
                class_id: "artifact".to_string(),
                requested_bytes: 100,
                mode: StorageResizeMode::Permanent,
                expires_at_ms: None,
            },
            StorageResizeRequest {
                schema: STORAGE_RESIZE_REQUEST_SCHEMA.to_string(),
                request_id: "resize-1".to_string(),
                claim_id: "claim-1".to_string(),
                target_bytes: 200,
                mode: StorageResizeMode::Permanent,
                requested_at_ms: 1,
                expires_at_ms: None,
            },
            StorageObservation {
                schema: STORAGE_OBSERVATION_SCHEMA.to_string(),
                volume_id: "vol-1".to_string(),
                phase: StoragePhase::Ready,
                provider_bytes: Some(100),
                filesystem_bytes: Some(100),
                free_bytes: Some(50),
                service_ready: Some(true),
                evidence_refs: vec!["proof:before".to_string()],
                unknowns: Vec::new(),
            },
        )
    }

    #[test]
    fn growth_is_explicit_and_requires_provider_update_policy() {
        let (class, volume, claim, request, observation) = fixtures();
        let plan = reconcile_storage(&class, &volume, &claim, &request, &observation);
        assert_eq!(plan.decision, StorageDecision::ExpandProvider);
        assert!(plan.mutates);
    }

    #[test]
    fn temporary_resize_requires_expiry() {
        let (class, volume, claim, mut request, observation) = fixtures();
        request.mode = StorageResizeMode::Temporary;
        request.expires_at_ms = None;
        let plan = reconcile_storage(&class, &volume, &claim, &request, &observation);
        assert_eq!(plan.decision, StorageDecision::Blocked);
        assert!(
            plan.blockers
                .iter()
                .any(|item| item == "temporary_storage_resize_expiry_missing")
        );
    }

    #[test]
    fn shrink_is_migration_not_in_place() {
        let (class, volume, claim, mut request, observation) = fixtures();
        request.target_bytes = 50;
        let plan = reconcile_storage(&class, &volume, &claim, &request, &observation);
        assert_eq!(plan.decision, StorageDecision::Blocked);
        assert!(
            plan.blockers
                .iter()
                .any(|item| item == "storage_resize_below_claim_request")
        );
    }
}
