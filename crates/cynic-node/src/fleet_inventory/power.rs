//! Bounded provider power mutation and read-back.

use cynic_protocol::{NodeSubstrateAction, NodeSubstrateDesiredState};
use serde::Serialize;
use serde_json::Value;

use super::providers::ProxmoxInventoryProvider;

#[derive(Debug, Clone, Serialize)]
pub struct ProviderPowerApplyReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub provider: String,
    pub resource_id: String,
    pub requested_action: String,
    pub expected_status: String,
    pub operation_id: Option<String>,
    pub task_status: Option<String>,
    pub task_exit_status: Option<String>,
    pub observed_status: Option<String>,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub async fn apply_proxmox_power_action(
    desired: &NodeSubstrateDesiredState,
    action: &NodeSubstrateAction,
    explicit_apply: bool,
) -> Result<ProviderPowerApplyReport, String> {
    if !explicit_apply {
        return Err("explicit_apply_required".to_string());
    }
    let provider_name = desired.provider.as_deref().unwrap_or_default();
    let resource_id = desired.resource_id.as_deref().unwrap_or_default();
    let (requested_action, expected_status) = match action {
        NodeSubstrateAction::Start if provider_name == "proxmox" => ("start", "running"),
        NodeSubstrateAction::Stop if provider_name == "proxmox" => ("stop", "stopped"),
        _ => return Err("unsupported_provider_action".to_string()),
    };
    let provider = ProxmoxInventoryProvider::from_env()
        .ok_or_else(|| "proxmox_credentials_or_tls_configuration_unavailable".to_string())?;
    let (kind, vmid) = resource_id
        .split_once('/')
        .ok_or_else(|| "invalid_proxmox_resource_id".to_string())?;
    if !matches!(kind, "qemu" | "lxc") || vmid.parse::<u64>().is_err() {
        return Err("invalid_proxmox_resource_id".to_string());
    }
    let resources = provider.adapter.cluster_resources_raw().await?;
    let item = resources
        .get("data")
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("id").and_then(Value::as_str) == Some(resource_id))
        })
        .ok_or_else(|| "proxmox_resource_unobserved".to_string())?;
    let node = item
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| "proxmox_resource_node_unobserved".to_string())?;
    let payload = provider
        .adapter
        .resource_action(node, kind, vmid, requested_action)
        .await?;
    let operation_id = payload
        .get("data")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "proxmox_power_action_missing_upid".to_string())?;
    let (task_status, task_exit_status) = provider.await_task(node, &operation_id).await?;
    if task_status != "stopped" || task_exit_status.as_deref() != Some("OK") {
        return Err("proxmox_power_task_not_successful".to_string());
    }
    let observed_status = provider
        .adapter
        .cluster_resources_raw()
        .await
        .ok()
        .as_ref()
        .and_then(|value| value.get("data"))
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
                .find(|entry| entry.get("id").and_then(Value::as_str) == Some(resource_id))
        })
        .and_then(|entry| entry.get("status"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let blockers = if observed_status.as_deref() == Some(expected_status) {
        Vec::new()
    } else {
        vec!["provider_power_read_back_mismatch".to_string()]
    };
    Ok(ProviderPowerApplyReport {
        schema: "cynic.fleet.provider_power_apply.v1",
        status: if blockers.is_empty() {
            "applied"
        } else {
            "unknown_after_mutation"
        },
        mutates: true,
        provider: "proxmox".to_string(),
        resource_id: resource_id.to_string(),
        requested_action: requested_action.to_string(),
        expected_status: expected_status.to_string(),
        operation_id: Some(operation_id),
        task_status: Some(task_status),
        task_exit_status,
        observed_status,
        evidence_refs: if blockers.is_empty() {
            vec![
                "provider_action_task_read_back".to_string(),
                "desired_power_observed".to_string(),
            ]
        } else {
            vec!["provider_action_task_read_back".to_string()]
        },
        blockers,
    })
}

pub async fn apply_power_action(
    desired: &NodeSubstrateDesiredState,
    action: &NodeSubstrateAction,
    explicit_apply: bool,
) -> Result<ProviderPowerApplyReport, String> {
    if !explicit_apply {
        return Err("explicit_apply_required".to_string());
    }
    match desired.provider.as_deref() {
        Some("proxmox") => apply_proxmox_power_action(desired, action, true).await,
        Some(provider) => Ok(ProviderPowerApplyReport {
            schema: "cynic.fleet.provider_power_apply.v1",
            status: "blocked_provider_adapter",
            mutates: false,
            provider: provider.to_string(),
            resource_id: desired.resource_id.clone().unwrap_or_default(),
            requested_action: format!("{action:?}").to_ascii_lowercase(),
            expected_status: String::new(),
            operation_id: None,
            task_status: None,
            task_exit_status: None,
            observed_status: None,
            blockers: vec![format!("provider_power_adapter_unavailable:{provider}")],
            evidence_refs: vec!["provider_adapter_selection_observed".to_string()],
        }),
        None => Ok(ProviderPowerApplyReport {
            schema: "cynic.fleet.provider_power_apply.v1",
            status: "blocked_provider_unknown",
            mutates: false,
            provider: String::new(),
            resource_id: desired.resource_id.clone().unwrap_or_default(),
            requested_action: format!("{action:?}").to_ascii_lowercase(),
            expected_status: String::new(),
            operation_id: None,
            task_status: None,
            task_exit_status: None,
            observed_status: None,
            blockers: vec!["provider_power_adapter_unknown".to_string()],
            evidence_refs: Vec::new(),
        }),
    }
}
