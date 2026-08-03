use cynic_protocol::{STORAGE_OBSERVATION_SCHEMA, StorageObservation, StoragePhase};
use serde_json::Value;

use crate::adapters::proxmox_control::ProxmoxControlAdapter;
use crate::ports::storage_mutation_port::{
    StorageMutationPort, StorageMutationRequest, StorageMutationResult,
};
use crate::ports::storage_provider_port::{StorageProviderPort, StorageProviderTarget};

#[derive(Debug, Clone, Copy)]
pub struct ProxmoxStorageAdapter<'a> {
    provider: &'a ProxmoxControlAdapter,
}

impl<'a> ProxmoxStorageAdapter<'a> {
    pub fn new(provider: &'a ProxmoxControlAdapter) -> Self {
        Self { provider }
    }
}

/// Join provider and guest read-backs without allowing either adapter to
/// manufacture the other's evidence. Missing dimensions remain unknown.
pub fn merge_storage_observations(
    provider: &StorageObservation,
    guest: &StorageObservation,
) -> Result<StorageObservation, String> {
    if provider.volume_id != guest.volume_id {
        return Err("storage provider and guest observations target different volumes".to_string());
    }
    let mut evidence_refs = provider.evidence_refs.clone();
    evidence_refs.extend(guest.evidence_refs.iter().cloned());
    let mut unknowns = provider.unknowns.clone();
    unknowns.extend(guest.unknowns.iter().cloned());
    if guest.filesystem_bytes.is_some() {
        unknowns.retain(|unknown| {
            unknown != "guest_filesystem_size_not_observed"
                && unknown != "guest_filesystem_capacity_not_read_back"
        });
    }
    if guest.free_bytes.is_some() {
        unknowns.retain(|unknown| {
            unknown != "guest_free_space_not_observed"
                && unknown != "guest_filesystem_capacity_not_read_back"
        });
    }
    if guest.service_ready == Some(true) {
        unknowns.retain(|unknown| {
            unknown != "storage_service_health_not_observed"
                && unknown != "storage_service_health_not_ready"
                && unknown != "storage_service_state_not_read_back"
        });
    }
    unknowns.sort();
    unknowns.dedup();
    Ok(StorageObservation {
        schema: STORAGE_OBSERVATION_SCHEMA.to_string(),
        volume_id: provider.volume_id.clone(),
        phase: if unknowns.is_empty() {
            StoragePhase::Ready
        } else {
            StoragePhase::Unknown
        },
        provider_bytes: provider.provider_bytes,
        filesystem_bytes: guest.filesystem_bytes.or(provider.filesystem_bytes),
        free_bytes: guest.free_bytes.or(provider.free_bytes),
        service_ready: guest.service_ready.or(provider.service_ready),
        evidence_refs,
        unknowns,
    })
}

impl StorageProviderPort for ProxmoxStorageAdapter<'_> {
    async fn observe_volume(
        &self,
        target: &StorageProviderTarget,
    ) -> Result<StorageObservation, String> {
        let payload = self.provider.lxc_config(&target.node, &target.vmid).await?;
        let disk = payload
            .pointer("/data")
            .and_then(Value::as_object)
            .and_then(|config| config.get(&target.disk_key))
            .and_then(Value::as_str)
            .ok_or_else(|| format!("Proxmox LXC disk key missing: {}", target.disk_key))?;
        let provider_bytes = parse_size_bytes(disk)?;
        Ok(StorageObservation {
            schema: STORAGE_OBSERVATION_SCHEMA.to_string(),
            volume_id: target.volume_id.clone(),
            phase: StoragePhase::Unknown,
            provider_bytes: Some(provider_bytes),
            filesystem_bytes: None,
            free_bytes: None,
            service_ready: None,
            evidence_refs: vec!["proxmox_lxc_config_read_back".to_string()],
            unknowns: vec![
                "guest_filesystem_size_not_observed".to_string(),
                "guest_free_space_not_observed".to_string(),
                "storage_service_health_not_observed".to_string(),
            ],
        })
    }
}

impl StorageMutationPort for ProxmoxStorageAdapter<'_> {
    async fn resize_volume(
        &self,
        request: &StorageMutationRequest,
    ) -> Result<StorageMutationResult, String> {
        if !request.approved {
            return Err("storage_mutation_requires_explicit_approval".to_string());
        }
        if request.target_bytes <= request.observed_provider_bytes {
            return Err("storage_provider_resize_must_grow".to_string());
        }
        let gib = 1024_u64.pow(3);
        if request.target_bytes % gib != 0 {
            return Err("storage_provider_resize_requires_gib_alignment".to_string());
        }
        let payload = self
            .provider
            .lxc_resize(
                &request.node,
                &request.vmid,
                &request.disk_key,
                request.target_bytes / gib,
            )
            .await?;
        let operation_ref = payload
            .pointer("/data")
            .and_then(Value::as_str)
            .unwrap_or("proxmox_resize_task")
            .to_string();
        Ok(StorageMutationResult {
            accepted: true,
            operation_ref,
            evidence_refs: vec!["proxmox_lxc_resize_accepted".to_string()],
        })
    }
}

fn parse_size_bytes(value: &str) -> Result<u64, String> {
    let size = value
        .split(',')
        .find_map(|part| part.strip_prefix("size="))
        .ok_or_else(|| "Proxmox disk config has no size attribute".to_string())?;
    let (number, multiplier) = if let Some(number) = size.strip_suffix('G') {
        (number, 1024_u64.pow(3))
    } else if let Some(number) = size.strip_suffix('M') {
        (number, 1024_u64.pow(2))
    } else if let Some(number) = size.strip_suffix('K') {
        (number, 1024_u64)
    } else if let Some(number) = size.strip_suffix('B') {
        (number, 1)
    } else {
        return Err(format!("unsupported Proxmox disk size unit: {size}"));
    };
    number
        .parse::<u64>()
        .map(|value| value.saturating_mul(multiplier))
        .map_err(|error| format!("invalid Proxmox disk size {size}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::parse_size_bytes;

    #[test]
    fn parses_proxmox_size_attribute() {
        assert_eq!(
            parse_size_bytes("local-lvm:vm-103-disk-0,size=8G").unwrap(),
            8 * 1024_u64.pow(3)
        );
    }

    #[test]
    fn rejects_unknown_size_unit() {
        assert!(parse_size_bytes("local-lvm:disk,size=8T").is_err());
    }
}
