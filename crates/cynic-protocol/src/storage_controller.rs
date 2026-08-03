use serde::{Deserialize, Serialize};

use crate::{
    LeaseGuard,
    storage_lifecycle::{StorageObservation, StoragePhase, StoragePlan},
};

pub const STORAGE_CONTROLLER_RECORD_SCHEMA: &str = "cynic.storage_controller_record.v1";
pub const STORAGE_OPERATION_SCHEMA: &str = "cynic.storage_operation.v1";
pub const STORAGE_FINALIZER: &str = "storage.cynic.io/protect-binding";
pub const STORAGE_LEASE_SCHEMA: &str = "cynic.storage_lease.v1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageLeaseGrant {
    pub schema: String,
    pub guard: LeaseGuard,
    pub expires_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageMetadata {
    pub id: String,
    pub generation: u64,
    pub observed_generation: Option<u64>,
    pub finalizers: Vec<String>,
    pub lease_id: Option<String>,
    pub lease_expires_at_ms: Option<u64>,
    pub lease_fencing_token: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StorageOperationPhase {
    Pending,
    Applying,
    Verifying,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageOperation {
    pub schema: String,
    pub id: String,
    pub generation: u64,
    pub phase: StorageOperationPhase,
    pub target_bytes: u64,
    pub started_at_ms: u64,
    pub completed_at_ms: Option<u64>,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageStatus {
    pub phase: StoragePhase,
    pub observed_generation: Option<u64>,
    pub operation_id: Option<String>,
    pub provider_bytes: Option<u64>,
    pub filesystem_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
    pub service_ready: Option<bool>,
    pub evidence_refs: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StorageControllerRecord {
    pub schema: String,
    pub metadata: StorageMetadata,
    pub status: StorageStatus,
    pub operation: Option<StorageOperation>,
    pub record_digest: String,
    pub previous_record_digest: Option<String>,
    pub committed_at_ms: u64,
}

pub fn new_storage_metadata(id: impl Into<String>) -> StorageMetadata {
    StorageMetadata {
        id: id.into(),
        generation: 1,
        observed_generation: None,
        finalizers: vec![STORAGE_FINALIZER.to_string()],
        lease_id: None,
        lease_expires_at_ms: None,
        lease_fencing_token: None,
    }
}

pub fn reconcile_storage_status(
    metadata: &StorageMetadata,
    plan: &StoragePlan,
    observed: &StorageObservation,
    now_ms: u64,
) -> (StorageStatus, Option<StorageOperation>) {
    let mut evidence_refs = observed.evidence_refs.clone();
    evidence_refs.extend(plan.expected_proofs.iter().cloned());
    evidence_refs.sort();
    evidence_refs.dedup();
    let status = StorageStatus {
        phase: observed.phase,
        observed_generation: (observed.unknowns.is_empty() && observed.service_ready == Some(true))
            .then_some(metadata.generation),
        operation_id: plan
            .mutates
            .then(|| format!("storage-op-{}-{}", metadata.id, metadata.generation)),
        provider_bytes: observed.provider_bytes,
        filesystem_bytes: observed.filesystem_bytes,
        free_bytes: observed.free_bytes,
        service_ready: observed.service_ready,
        evidence_refs,
        unknowns: observed.unknowns.clone(),
    };
    let operation = plan.mutates.then(|| StorageOperation {
        schema: STORAGE_OPERATION_SCHEMA.to_string(),
        id: status.operation_id.clone().unwrap_or_default(),
        generation: metadata.generation,
        phase: if plan.blockers.is_empty() {
            StorageOperationPhase::Pending
        } else {
            StorageOperationPhase::Failed
        },
        target_bytes: plan.target_bytes,
        started_at_ms: now_ms,
        completed_at_ms: None,
        evidence_refs: status.evidence_refs.clone(),
        blockers: plan.blockers.clone(),
    });
    (status, operation)
}

pub fn validate_storage_controller_record(record: &StorageControllerRecord) -> Vec<String> {
    let mut blockers = Vec::new();
    if record.schema != STORAGE_CONTROLLER_RECORD_SCHEMA {
        blockers.push("storage_controller_record_schema_mismatch".to_string());
    }
    if record.metadata.id.trim().is_empty() || record.metadata.generation == 0 {
        blockers.push("storage_controller_metadata_invalid".to_string());
    }
    if record.metadata.lease_id.is_some() != record.metadata.lease_expires_at_ms.is_some()
        || record.metadata.lease_id.is_some() != record.metadata.lease_fencing_token.is_some()
        || record.metadata.lease_fencing_token == Some(0)
    {
        blockers.push("storage_controller_lease_invalid".to_string());
    }
    if !record
        .metadata
        .finalizers
        .iter()
        .any(|finalizer| finalizer == STORAGE_FINALIZER)
    {
        blockers.push("storage_controller_finalizer_missing".to_string());
    }
    if let Some(operation) = &record.operation {
        if operation.schema != STORAGE_OPERATION_SCHEMA
            || operation.generation != record.metadata.generation
        {
            blockers.push("storage_controller_operation_generation_mismatch".to_string());
        }
    }
    blockers
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage_lifecycle::{
        STORAGE_OBSERVATION_SCHEMA, STORAGE_PLAN_SCHEMA, StorageDecision,
    };
    #[test]
    fn mutation_creates_generation_bound_pending_operation() {
        let metadata = new_storage_metadata("artifacts");
        let plan = StoragePlan {
            schema: STORAGE_PLAN_SCHEMA.to_string(),
            claim_id: "claim".to_string(),
            decision: StorageDecision::ExpandProvider,
            mutates: true,
            target_bytes: 20,
            blockers: Vec::new(),
            expected_proofs: vec!["provider_resize_read_back".to_string()],
            rollback_intent: "retain".to_string(),
        };
        let observed = StorageObservation {
            schema: STORAGE_OBSERVATION_SCHEMA.to_string(),
            volume_id: "artifacts".to_string(),
            phase: StoragePhase::Ready,
            provider_bytes: Some(10),
            filesystem_bytes: Some(10),
            free_bytes: Some(5),
            service_ready: Some(true),
            evidence_refs: vec!["provider".to_string()],
            unknowns: Vec::new(),
        };
        let (status, operation) = reconcile_storage_status(&metadata, &plan, &observed, 10);
        assert_eq!(status.observed_generation, Some(1));
        assert_eq!(operation.unwrap().phase, StorageOperationPhase::Pending);
    }
}
