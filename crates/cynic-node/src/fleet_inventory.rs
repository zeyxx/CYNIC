<<<<<<< HEAD
use cynic_protocol::{
    NODE_SUBSTRATE_OBSERVATION_SCHEMA, NodeNetworkAttachment, NodeSubstrateAction,
    NodeSubstrateDesiredState, NodeSubstrateExistence, NodeSubstrateObservation,
    NodeSubstratePower,
};
=======
use crate::domain::fleet_reconcile::FleetReconcileAction;
>>>>>>> github/main
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::time::Duration;
use tokio::process::Command;

type ProviderError = String;

#[derive(Debug, Clone, Serialize)]
pub struct FleetNodeObservation {
    pub provider: String,
    pub provider_node: Option<String>,
    pub node_id: String,
    pub name: String,
    pub status: String,
    pub liveness: String,
    pub ip: Option<String>,
    pub desired: bool,
    pub observed: bool,
    pub meta: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FleetInventoryProviderSummary {
    pub provider: String,
    pub status: String,
    pub observed_nodes: u32,
    pub error: Option<String>,
}

impl FleetInventoryProviderSummary {
    fn ok(provider: &str, observed_nodes: usize) -> Self {
        Self {
            provider: provider.to_string(),
            status: "ok".to_string(),
            observed_nodes: observed_nodes as u32,
            error: None,
        }
    }

    fn degraded(provider: &str, observed_nodes: usize, error: String) -> Self {
        Self {
            provider: provider.to_string(),
            status: "degraded".to_string(),
            observed_nodes: observed_nodes as u32,
            error: Some(error),
        }
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct FleetInventorySummary {
    pub declared_nodes: u32,
    pub observed_nodes: u32,
    pub matched_nodes: u32,
    pub missing_nodes: u32,
    pub unexpected_nodes: u32,
    pub providers: Vec<FleetInventoryProviderSummary>,
    pub provider_evidence_gaps: Vec<String>,
    pub declaration_missing_nodes: Vec<String>,
    pub declaration_unexpected_nodes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FleetInventoryDetails {
    pub nodes: Vec<FleetNodeObservation>,
    pub summary: FleetInventorySummary,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderPowerApplyReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub provider: String,
    pub resource_id: String,
    pub requested_action: String,
<<<<<<< HEAD
    pub expected_status: String,
=======
>>>>>>> github/main
    pub operation_id: Option<String>,
    pub task_status: Option<String>,
    pub task_exit_status: Option<String>,
    pub observed_status: Option<String>,
<<<<<<< HEAD
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub async fn apply_proxmox_power_action(
    desired: &NodeSubstrateDesiredState,
    action: &NodeSubstrateAction,
=======
}

pub async fn apply_proxmox_power_action(
    action: &FleetReconcileAction,
>>>>>>> github/main
    explicit_apply: bool,
) -> Result<ProviderPowerApplyReport, String> {
    if !explicit_apply {
        return Err("explicit_apply_required".to_string());
    }
<<<<<<< HEAD
    let provider_name = desired.provider.as_deref().unwrap_or_default();
    let resource_id = desired.resource_id.as_deref().unwrap_or_default();
    let (requested_action, expected_status) = match action {
        NodeSubstrateAction::Start if provider_name == "proxmox" => ("start", "running"),
        NodeSubstrateAction::Stop if provider_name == "proxmox" => ("stop", "stopped"),
=======
    let (requested_action, resource_id) = match action {
        FleetReconcileAction::Start {
            provider,
            resource_id,
        } if provider == "proxmox" => ("start", resource_id),
        FleetReconcileAction::Stop {
            provider,
            resource_id,
        } if provider == "proxmox" => ("stop", resource_id),
>>>>>>> github/main
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
<<<<<<< HEAD
    let resources = provider.adapter.cluster_resources_raw().await?;
=======
    let resources = provider.request_json("/cluster/resources?type=vm").await?;
>>>>>>> github/main
    let item = resources
        .get("data")
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
<<<<<<< HEAD
                .find(|item| item.get("id").and_then(Value::as_str) == Some(resource_id))
=======
                .find(|item| item.get("id").and_then(Value::as_str) == Some(resource_id.as_str()))
>>>>>>> github/main
        })
        .ok_or_else(|| "proxmox_resource_unobserved".to_string())?;
    let node = item
        .get("node")
        .and_then(Value::as_str)
        .ok_or_else(|| "proxmox_resource_node_unobserved".to_string())?;
<<<<<<< HEAD
    let payload = provider
        .adapter
        .resource_action(node, kind, vmid, requested_action)
        .await?;
=======
    let url = format!(
        "{}/api2/json/nodes/{node}/{kind}/{vmid}/status/{requested_action}",
        provider.base_url
    );
    let response = provider
        .client
        .post(url)
        .header("Authorization", provider.auth_header())
        .send()
        .await
        .map_err(|error| format!("proxmox power action failed: {error}"))?;
    if !response.status().is_success() {
        return Err(format!(
            "proxmox power action returned HTTP {}",
            response.status().as_u16()
        ));
    }
    let payload = response
        .json::<Value>()
        .await
        .map_err(|error| format!("invalid Proxmox power action JSON: {error}"))?;
>>>>>>> github/main
    let operation_id = payload
        .get("data")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| "proxmox_power_action_missing_upid".to_string())?;
    let (task_status, task_exit_status) = provider.await_task(node, &operation_id).await?;
    if task_status != "stopped" || task_exit_status.as_deref() != Some("OK") {
        return Err("proxmox_power_task_not_successful".to_string());
    }
<<<<<<< HEAD
    let read_back = provider.adapter.cluster_resources_raw().await.ok();
=======
    let read_back = provider
        .request_json("/cluster/resources?type=vm")
        .await
        .ok();
>>>>>>> github/main
    let observed_status = read_back
        .as_ref()
        .and_then(|value| value.get("data"))
        .and_then(Value::as_array)
        .and_then(|items| {
            items
                .iter()
<<<<<<< HEAD
                .find(|entry| entry.get("id").and_then(Value::as_str) == Some(resource_id))
=======
                .find(|entry| entry.get("id").and_then(Value::as_str) == Some(resource_id.as_str()))
>>>>>>> github/main
        })
        .and_then(|entry| entry.get("status"))
        .and_then(Value::as_str)
        .map(str::to_string);
<<<<<<< HEAD
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
=======
    Ok(ProviderPowerApplyReport {
        schema: "cynic.fleet.provider_power_apply.v1",
        status: "applied",
        mutates: true,
        provider: "proxmox".to_string(),
        resource_id: resource_id.clone(),
        requested_action: requested_action.to_string(),
>>>>>>> github/main
        operation_id: Some(operation_id),
        task_status: Some(task_status),
        task_exit_status,
        observed_status,
<<<<<<< HEAD
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

pub fn node_substrate_observation(
    desired: &NodeSubstrateDesiredState,
    inventory: &FleetInventoryDetails,
) -> NodeSubstrateObservation {
    let provider = desired.provider.as_deref();
    let resource_id = desired.resource_id.as_deref();
    let provider_proved = provider.is_some_and(|provider| {
        inventory
            .summary
            .providers
            .iter()
            .any(|summary| summary.provider == provider && summary.status == "ok")
    });
    let resource = provider_proved.then(|| {
        inventory.nodes.iter().find(|node| {
            node.observed
                && Some(node.provider.as_str()) == provider
                && Some(node.node_id.as_str()) == resource_id
        })
    });
    let resource = resource.flatten();
    let observed_at_unix_ms = provider_proved.then(now_unix_ms);
    match resource {
        Some(resource) => {
            let power = match resource.status.as_str() {
                "running" => NodeSubstratePower::Running,
                "stopped" => NodeSubstratePower::Stopped,
                _ => NodeSubstratePower::Unknown,
            };
            let network_attachment = if resource.ip.is_some() {
                NodeNetworkAttachment::Attached
            } else {
                NodeNetworkAttachment::Unknown
            };
            let mut unknowns = Vec::new();
            if power == NodeSubstratePower::Unknown {
                unknowns.push("node_substrate_power_unknown".to_string());
            }
            if network_attachment == NodeNetworkAttachment::Unknown {
                unknowns.push("node_network_attachment_unknown".to_string());
            }
            unknowns.push("node_management_reachability_unobserved".to_string());
            NodeSubstrateObservation {
                schema: NODE_SUBSTRATE_OBSERVATION_SCHEMA.to_string(),
                status: "observed".to_string(),
                node_id: desired.node_id.clone(),
                provider: desired.provider.clone(),
                resource_id: desired.resource_id.clone(),
                existence: NodeSubstrateExistence::Present,
                power,
                network_attachment,
                observed_addresses: resource.ip.clone().into_iter().collect(),
                management_reachable: None,
                observation_authority: provider.map(|provider| format!("provider:{provider}:api")),
                observed_at_unix_ms,
                evidence_refs: vec![
                    "provider_inventory_read_back".to_string(),
                    format!("provider_resource_observed:{}", resource.node_id),
                ],
                unknowns,
            }
        }
        None if provider_proved => NodeSubstrateObservation {
            schema: NODE_SUBSTRATE_OBSERVATION_SCHEMA.to_string(),
            status: "observed".to_string(),
            node_id: desired.node_id.clone(),
            provider: desired.provider.clone(),
            resource_id: desired.resource_id.clone(),
            existence: NodeSubstrateExistence::Absent,
            power: NodeSubstratePower::Unknown,
            network_attachment: NodeNetworkAttachment::Unknown,
            observed_addresses: Vec::new(),
            management_reachable: None,
            observation_authority: provider.map(|provider| format!("provider:{provider}:api")),
            observed_at_unix_ms,
            evidence_refs: vec!["provider_inventory_read_back".to_string()],
            unknowns: vec!["provider_resource_absent".to_string()],
        },
        None => NodeSubstrateObservation {
            schema: NODE_SUBSTRATE_OBSERVATION_SCHEMA.to_string(),
            status: "unknown".to_string(),
            node_id: desired.node_id.clone(),
            provider: None,
            resource_id: None,
            existence: NodeSubstrateExistence::Unknown,
            power: NodeSubstratePower::Unknown,
            network_attachment: NodeNetworkAttachment::Unknown,
            observed_addresses: Vec::new(),
            management_reachable: None,
            observation_authority: None,
            observed_at_unix_ms: None,
            evidence_refs: Vec::new(),
            unknowns: vec!["provider_observer_unavailable".to_string()],
        },
    }
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

=======
    })
}

>>>>>>> github/main
enum FleetInventoryProvider {
    Proxmox(ProxmoxInventoryProvider),
    Kubernetes(KubernetesInventoryProvider),
}

impl FleetInventoryProvider {
    fn name(&self) -> &'static str {
        match self {
            Self::Proxmox(_) => "proxmox",
            Self::Kubernetes(_) => "kubernetes",
        }
    }

    async fn collect_nodes(
        &self,
        declared_nodes: &[FleetNodeObservation],
    ) -> Result<Vec<FleetNodeObservation>, ProviderError> {
        match self {
            Self::Proxmox(provider) => provider.collect_nodes(declared_nodes).await,
            Self::Kubernetes(provider) => provider.collect_nodes(declared_nodes).await,
        }
    }
}

pub async fn build_fleet_inventory_details(
    registry_path: &Path,
) -> Result<FleetInventoryDetails, String> {
    let declared_nodes = load_declared_inventory(registry_path)?;
    let providers = build_providers();

    if providers.is_empty() {
        tracing::warn!(
            "No fleet inventory providers configured. Registry-only fleet facts will be published."
        );
    }

    let mut nodes = normalize_declared(&declared_nodes);
    let mut provider_summaries = Vec::new();
    let mut provider_errors = 0usize;

    let declared_snapshot: Vec<_> = nodes.values().cloned().collect();
    for provider in &providers {
        match provider.collect_nodes(&declared_snapshot).await {
            Ok(observed_nodes) => {
                let observed_count = observed_nodes.len();
                for observed in observed_nodes {
                    upsert_node(&mut nodes, observed);
                }
                provider_summaries.push(FleetInventoryProviderSummary::ok(
                    provider.name(),
                    observed_count,
                ));
            }
            Err(err) => {
                provider_errors += 1;
                provider_summaries.push(FleetInventoryProviderSummary::degraded(
                    provider.name(),
                    0,
                    err.clone(),
                ));
                tracing::warn!(
                    provider = provider.name(),
                    error = %err,
                    "fleet inventory provider failed"
                );
            }
        }
    }

    let mut nodes: Vec<_> = nodes.into_values().collect();
    nodes.sort_unstable_by(|a, b| a.node_id.cmp(&b.node_id));

    let total = nodes.len();
    let desired = nodes.iter().filter(|n| n.desired).count();
    let observed = nodes.iter().filter(|n| n.observed).count();
    let matched = nodes.iter().filter(|n| n.desired && n.observed).count();
    let missing = desired.saturating_sub(matched);
    let unexpected = total.saturating_sub(nodes.iter().filter(|n| n.desired).count());
    let missing_nodes: Vec<String> = nodes
        .iter()
        .filter(|n| n.desired && !n.observed)
        .map(|n| n.node_id.clone())
        .collect();
    let unexpected_nodes: Vec<String> = nodes
        .iter()
        .filter(|n| !n.desired && n.observed)
        .map(|n| n.node_id.clone())
        .collect();

    let mut provider_evidence_gaps = Vec::new();
    for provider in &provider_summaries {
        if provider.status == "degraded" {
            let reason = provider
                .error
                .clone()
                .unwrap_or_else(|| "provider failed".to_string());
            provider_evidence_gaps.push(format!("provider:{}:{}", provider.provider, reason));
        }
    }
    if missing > 0 {
        provider_evidence_gaps.push(format!("fleet:missing_declared_nodes:{missing}"));
        for node_id in &missing_nodes {
            provider_evidence_gaps.push(format!("fleet:missing_node:{node_id}"));
        }
    }
    for node_id in &unexpected_nodes {
        provider_evidence_gaps.push(format!("fleet:unexpected_node:{node_id}"));
    }

    let _status = if providers.is_empty() {
        if desired == 0 {
            "unavailable"
        } else {
            "degraded"
        }
    } else if provider_errors > 0 || missing > 0 {
        "degraded"
    } else if total == 0 {
        "unavailable"
    } else {
        "ok"
    };

    Ok(FleetInventoryDetails {
        nodes,
        summary: FleetInventorySummary {
            declared_nodes: desired as u32,
            observed_nodes: observed as u32,
            matched_nodes: matched as u32,
            missing_nodes: missing as u32,
            unexpected_nodes: unexpected as u32,
            providers: provider_summaries,
            provider_evidence_gaps,
            declaration_missing_nodes: missing_nodes,
            declaration_unexpected_nodes: unexpected_nodes,
        },
    })
}

fn normalize_declared(
    declared_nodes: &[FleetNodeObservation],
) -> HashMap<String, FleetNodeObservation> {
    declared_nodes
        .iter()
        .cloned()
        .map(|node| {
            let mut merged = node;
            merged.observed = false;
            merged.desired = true;
            (merged.node_id.clone(), merged)
        })
        .collect()
}

fn upsert_node(nodes: &mut HashMap<String, FleetNodeObservation>, observed: FleetNodeObservation) {
    let node_id = observed.node_id.clone();
    match nodes.get_mut(&node_id) {
        Some(existing) => {
            existing.provider = observed.provider.clone();
            existing.provider_node = observed
                .provider_node
                .or_else(|| existing.provider_node.clone());
            existing.name = observed.name;
            existing.status = observed.status;
            existing.liveness = observed.liveness;
            existing.ip = observed.ip.or(existing.ip.clone());
            existing.observed = true;
            existing.desired = existing.desired || observed.desired;
            existing.meta = observed.meta;
        }
        None => {
            nodes.insert(node_id, observed);
        }
    }
}

fn build_providers() -> Vec<FleetInventoryProvider> {
    let provider_spec = std::env::var("CYNIC_FLEET_PROVIDERS")
        .unwrap_or_else(|_| "proxmox".to_string())
        .to_lowercase();
    let enable_k8s = bool_flag("CYNIC_FLEET_K8S_ENABLED").unwrap_or_default();
    let providers = provider_spec
        .split(',')
        .map(|token| token.trim().to_lowercase())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    let mut list = Vec::new();
    for provider in providers {
        match provider.as_str() {
            "proxmox" | "pm" | "proxmox_api" => {
                if let Some(p) = ProxmoxInventoryProvider::from_env() {
                    list.push(FleetInventoryProvider::Proxmox(p));
                }
            }
            "k8s" | "kubernetes" | "kubernetes_api" => {
                if enable_k8s {
                    if let Some(p) = KubernetesInventoryProvider::from_env() {
                        list.push(FleetInventoryProvider::Kubernetes(p));
                    }
                } else {
                    tracing::debug!("k8s provider requested but CYNIC_FLEET_K8S_ENABLED is false");
                }
            }
            other => tracing::warn!(provider = %other, "unsupported fleet provider"),
        }
    }
    list
}

fn bool_flag(key: &str) -> Option<bool> {
    std::env::var(key).ok().map(|v| {
        matches!(
            v.trim().to_lowercase().as_str(),
            "1" | "true" | "yes" | "on" | "enabled"
        )
    })
}

#[derive(Debug)]
struct ProxmoxInventoryProvider {
<<<<<<< HEAD
    adapter: crate::adapters::proxmox_control::ProxmoxControlAdapter,
    target_nodes: Vec<String>,
}

impl ProxmoxInventoryProvider {
    fn from_env() -> Option<Self> {
        let adapter = crate::adapters::proxmox_control::ProxmoxControlAdapter::from_env(
            Duration::from_secs(10),
        )
        .ok()?;
        let target_nodes = bool_env_csv("CYNIC_PROXMOX_NODES");
        Some(Self {
            adapter,
            target_nodes,
        })
    }

=======
    base_url: String,
    client_id: String,
    credential: String,
    target_nodes: Vec<String>,
    client: reqwest::Client,
}

impl ProxmoxInventoryProvider {
    fn proxmox_credential_from(env_lookup: impl Fn(&str) -> Option<String>) -> Option<String> {
        env_lookup("CYNIC_PROXMOX_TOKEN_SECRET")
            .or_else(|| env_lookup("CYNIC_PROXMOX_SECRET"))
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    fn proxmox_credential() -> Option<String> {
        Self::proxmox_credential_from(|key| std::env::var(key).ok())
    }

    fn from_env() -> Option<Self> {
        let base_url = std::env::var("CYNIC_PROXMOX_URL").ok()?;
        let client_id = std::env::var("CYNIC_PROXMOX_TOKEN_ID").ok()?;
        let credential = Self::proxmox_credential()?;
        let target_nodes = bool_env_csv("CYNIC_PROXMOX_NODES");
        let mut client = reqwest::Client::builder().timeout(Duration::from_secs(10));
        if let Some(ca_path) = std::env::var("CYNIC_PROXMOX_CA_FILE")
            .ok()
            .filter(|value| !value.trim().is_empty())
        {
            let ca_pem = std::fs::read(&ca_path).ok()?;
            let certificate = reqwest::Certificate::from_pem(&ca_pem).ok()?;
            client = client.add_root_certificate(certificate);
        }
        let client = client.build().ok()?;

        if base_url.trim().is_empty() || client_id.trim().is_empty() || credential.trim().is_empty()
        {
            return None;
        }

        Some(Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            client_id,
            credential,
            target_nodes,
            client,
        })
    }

    fn auth_header(&self) -> String {
        format!("PVEAPIToken={}={}", self.client_id, self.credential)
    }

>>>>>>> github/main
    fn declared_by_id(declared_nodes: &[FleetNodeObservation]) -> HashSet<&str> {
        declared_nodes.iter().map(|n| n.node_id.as_str()).collect()
    }

    fn parse_status(v: &Value) -> String {
        v.as_str().unwrap_or("unknown").to_string()
    }

    fn liveness_from_status(status: &str) -> &str {
        match status {
            "running" => "up",
            "stopped" | "paused" => "down",
            _ => "unknown",
        }
    }

<<<<<<< HEAD
    async fn await_task(&self, node: &str, upid: &str) -> Result<(String, Option<String>), String> {
        for _ in 0..30 {
            let payload = self.adapter.task_status(node, upid).await?;
=======
    async fn request_json(&self, path: &str) -> Result<Value, ProviderError> {
        let url = format!("{}/api2/json{}", self.base_url, path);
        let response = self
            .client
            .get(&url)
            .header("Authorization", self.auth_header())
            .send()
            .await
            .map_err(|e| format!("request failed for {path}: {e}"))?;

        if !response.status().is_success() {
            return Err(format!(
                "request {path} returned HTTP {}",
                response.status().as_u16()
            ));
        }

        response
            .json::<Value>()
            .await
            .map_err(|e| format!("invalid JSON from {path}: {e}"))
    }

    async fn await_task(&self, node: &str, upid: &str) -> Result<(String, Option<String>), String> {
        for _ in 0..30 {
            let payload = self
                .request_json(&format!("/nodes/{node}/tasks/{upid}/status"))
                .await?;
>>>>>>> github/main
            let data = payload
                .get("data")
                .ok_or_else(|| "proxmox_task_status_missing_data".to_string())?;
            let status = data
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let exit_status = data
                .get("exitstatus")
                .and_then(Value::as_str)
                .map(str::to_string);
            if status == "stopped" {
                return Ok((status, exit_status));
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Err("proxmox_task_timeout".to_string())
    }

    async fn list_nodes(&self) -> Result<Vec<String>, ProviderError> {
        if !self.target_nodes.is_empty() {
            return Ok(self.target_nodes.clone());
        }

<<<<<<< HEAD
        let body = self.adapter.nodes().await?;
=======
        let body = self.request_json("/nodes").await?;
>>>>>>> github/main
        let data = body
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| "nodes list missing data".to_string())?;

        Ok(data
            .iter()
            .filter_map(|n| {
                n.get("node")
                    .and_then(Value::as_str)
                    .map(ToString::to_string)
            })
            .collect())
    }

    fn parse_ip_field(raw: &Value) -> Option<String> {
        if let Some(s) = raw.as_str() {
            return Some(s.to_string());
        }
        if let Some(arr) = raw.as_array() {
            for item in arr {
                if let Some(s) = item.as_str() {
                    return Some(s.to_string());
                }
            }
        }
        None
    }

    fn extract_ip(item: &Value) -> Option<String> {
        if let Some(ip) = item.get("ip").and_then(Self::parse_ip_field) {
            return Some(ip);
        }
        if let Some(ip) = item
            .get("ipaddresses")
            .and_then(Self::parse_ip_field)
            .or_else(|| item.get("ip6_addresses").and_then(Self::parse_ip_field))
        {
            return Some(ip);
        }

        for key in [
            "ipconfig0",
            "ipconfig1",
            "ipconfig2",
            "ipconfig3",
            "net0",
            "net1",
            "net2",
        ] {
            if let Some(raw) = item.get(key).and_then(Value::as_str)
                && let Some(candidate) = parse_inline_network_ip(raw)
            {
                return Some(candidate);
            }
        }
        None
    }

    async fn collect_vms(
        &self,
        node_name: &str,
        vm_type: &str,
        declared_by_id: &HashSet<&str>,
    ) -> Result<Vec<FleetNodeObservation>, ProviderError> {
<<<<<<< HEAD
        let payload = self.adapter.node_resources(node_name, vm_type).await?;
=======
        let payload = self
            .request_json(&format!("/nodes/{node_name}/{vm_type}"))
            .await?;
>>>>>>> github/main
        let data = payload
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| format!("{vm_type} payload missing data"))?;

        Ok(data
            .iter()
            .filter_map(|item| {
                let id = item.get("vmid")?.as_u64()?;
                let vm_id = match vm_type {
                    "qemu" => format!("qemu/{id}"),
                    _ => format!("lxc/{id}"),
                };
                let name = item
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let status = Self::parse_status(&item.get("status").cloned().unwrap_or_default());
                let liveness = Self::liveness_from_status(&status).to_string();
                let ip = Self::extract_ip(item).or_else(|| {
                    item.get("ip")
                        .and_then(Value::as_array)
                        .and_then(|arr| arr.first())
                        .and_then(Value::as_str)
                        .map(ToString::to_string)
                });
                let desired = declared_by_id.contains(vm_id.as_str());

                Some(FleetNodeObservation {
                    provider: "proxmox".to_string(),
                    provider_node: Some(node_name.to_string()),
                    node_id: vm_id,
                    name,
                    status,
                    liveness,
                    ip,
                    desired,
                    observed: true,
                    meta: Some(BTreeMap::from([
                        ("provider".to_string(), "proxmox".to_string()),
                        ("vm_type".to_string(), vm_type.to_string()),
                        ("provider_node".to_string(), node_name.to_string()),
                        ("api".to_string(), "proxmox".to_string()),
                    ])),
                })
            })
            .collect())
    }
}

impl ProxmoxInventoryProvider {
    async fn collect_nodes(
        &self,
        declared_nodes: &[FleetNodeObservation],
    ) -> Result<Vec<FleetNodeObservation>, ProviderError> {
        let declared_by_id = Self::declared_by_id(declared_nodes);
        let nodes = self.list_nodes().await?;
        let mut observations = Vec::new();

        for node_name in nodes {
            observations.extend(
                self.collect_vms(&node_name, "qemu", &declared_by_id)
                    .await?,
            );
            observations.extend(self.collect_vms(&node_name, "lxc", &declared_by_id).await?);
        }

        Ok(observations)
    }
}

#[derive(Debug)]
struct KubernetesInventoryProvider {
    kubectl_bin: String,
    namespace: Option<String>,
    context: Option<String>,
    discover_nodes: bool,
}

impl KubernetesInventoryProvider {
    fn from_env() -> Option<Self> {
        let kubectl_bin = std::env::var("CYNIC_KUBECTL_BIN")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "kubectl".to_string());
        if kubectl_bin.trim().is_empty() {
            return None;
        }

        Some(Self {
            kubectl_bin,
            namespace: std::env::var("CYNIC_K8S_NAMESPACE").ok(),
            context: std::env::var("CYNIC_K8S_CONTEXT").ok(),
            discover_nodes: bool_flag("CYNIC_K8S_DISCOVER_NODES").unwrap_or(true),
        })
    }

    async fn run_kubectl_json(&self, args: &[&str]) -> Result<Value, ProviderError> {
        let mut command = Command::new(&self.kubectl_bin);
        if let Some(context) = &self.context {
            command.arg("--context").arg(context);
        }
        if let Some(namespace) = &self.namespace {
            command.arg("--namespace").arg(namespace);
        }
        command.args(args).arg("-o").arg("json");

        let output = command
            .output()
            .await
            .map_err(|err| kubectl_error(&self.kubectl_bin, args, &err))?;

        if !output.status.success() {
            return Err(kubectl_status_error(&self.kubectl_bin, args, &output));
        }

        parse_kubectl_stdout(&output)
    }

    fn parse_nodes(body: &Value) -> Vec<FleetNodeObservation> {
        body.get("items")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(|node| {
                        let name = node.get("metadata")?.get("name")?.as_str()?;
                        let phase = node
                            .get("status")
                            .and_then(|s| s.get("phase"))
                            .and_then(Value::as_str);
                        let conditions = node
                            .get("status")
                            .and_then(|s| s.get("conditions"))
                            .and_then(Value::as_array)
                            .map(|items| Value::Array(items.clone()));

                        let status = Self::parse_node_status(conditions.as_ref(), phase);
                        let liveness = Self::node_liveness_from_status(&status).to_string();
                        let ip = node
                            .get("status")
                            .and_then(|s| s.get("addresses"))
                            .and_then(Value::as_array)
                            .and_then(|addresses| {
                                addresses.iter().find_map(|entry| {
                                    match entry.get("type").and_then(Value::as_str) {
                                        Some("InternalIP") | Some("ExternalIP") => entry
                                            .get("address")
                                            .and_then(Value::as_str)
                                            .map(ToString::to_string),
                                        _ => None,
                                    }
                                })
                            });

                        Some(FleetNodeObservation {
                            provider: "kubernetes".to_string(),
                            provider_node: None,
                            node_id: format!("k8s/node/{name}"),
                            name: name.to_string(),
                            status,
                            liveness,
                            ip,
                            desired: false,
                            observed: true,
                            meta: Some(BTreeMap::from([
                                ("provider".to_string(), "kubernetes".to_string()),
                                ("api".to_string(), "kubectl".to_string()),
                                ("resource".to_string(), "nodes".to_string()),
                            ])),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    fn parse_node_status(conditions: Option<&Value>, phase: Option<&str>) -> String {
        if let Some(conditions) = conditions.and_then(Value::as_array)
            && conditions.iter().any(|condition| {
                condition.get("type").and_then(Value::as_str) == Some("Ready")
                    && condition
                        .get("status")
                        .and_then(Value::as_str)
                        .is_some_and(|status| status == "True")
            })
        {
            return "ready".to_string();
        }

        phase.unwrap_or("unknown").to_string()
    }

    fn node_liveness_from_status(status: &str) -> &str {
        match status {
            "ready" | "Running" | "up" => "up",
            _ => "down",
        }
    }
}

impl KubernetesInventoryProvider {
    async fn collect_nodes(
        &self,
        _declared_nodes: &[FleetNodeObservation],
    ) -> Result<Vec<FleetNodeObservation>, ProviderError> {
        if !self.discover_nodes {
            tracing::debug!("kubernetes provider has node discovery disabled");
            return Ok(Vec::new());
        }

        let nodes = self.run_kubectl_json(&["get", "nodes"]).await?;
        Ok(Self::parse_nodes(&nodes))
    }
}

fn parse_kubectl_stdout(output: &std::process::Output) -> Result<Value, ProviderError> {
    let stdout = String::from_utf8_lossy(&output.stdout);
    if stdout.trim().is_empty() {
        return Ok(serde_json::json!({
            "kind": "List",
            "apiVersion": "v1",
            "items": []
        }));
    }

    serde_json::from_str::<Value>(&stdout).map_err(|error| {
        format!(
            "kubectl returned invalid JSON: {error} (stderr: {})",
            String::from_utf8_lossy(&output.stderr),
        )
    })
}

fn kubectl_error(bin: &str, args: &[&str], err: &std::io::Error) -> String {
    format!("{bin} {args:?}: {err}")
}

fn kubectl_status_error(bin: &str, args: &[&str], output: &std::process::Output) -> String {
    format!(
        "{bin} {args:?} exited {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr).trim()
    )
}

fn load_declared_inventory(registry_path: &Path) -> Result<Vec<FleetNodeObservation>, String> {
    let content = std::fs::read_to_string(registry_path).map_err(|error| {
        format!(
            "failed to read registry {}: {error}",
            registry_path.display()
        )
    })?;
    let registry = serde_json::from_str::<Value>(&content).map_err(|error| {
        format!(
            "failed to parse registry {} as JSON: {error}",
            registry_path.display()
        )
    })?;

    let Some(nodes) = registry.get("nodes").and_then(Value::as_array) else {
        return Ok(Vec::new());
    };

    let mut declared = Vec::new();
    for node in nodes {
        let Some(name) = node.get("name").and_then(Value::as_str) else {
            continue;
        };
        let role = node.get("role").and_then(Value::as_str).unwrap_or("");
        let network = node.get("network").cloned().unwrap_or_default();
        let status = node
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("declared")
            .to_string();
        let liveness = node
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("declared")
            .to_string();
        let ip = network
            .get("tailscale")
            .or_else(|| network.get("local"))
            .and_then(Value::as_str)
            .map(ToString::to_string);

        declared.push(FleetNodeObservation {
            provider: "registry".to_string(),
            provider_node: None,
            node_id: name.to_string(),
            name: name.to_string(),
            status,
            liveness,
            ip,
            desired: true,
            observed: false,
            meta: Some(BTreeMap::from([
                ("source".to_string(), "registry".to_string()),
                ("kind".to_string(), "node".to_string()),
                ("role".to_string(), role.to_string()),
            ])),
        });

        if let Some(vms) = node.get("vms").and_then(Value::as_array) {
            for vm in vms {
                let id = match vm.get("id").and_then(Value::as_str) {
                    Some(v) => v.to_string(),
                    None => continue,
                };
                let vm_name = vm
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unnamed")
                    .to_string();
                let vm_status = vm
                    .get("status")
                    .and_then(Value::as_str)
                    .unwrap_or("declared")
                    .to_string();
                let vm_ip = vm
                    .get("ip")
                    .and_then(Value::as_str)
                    .map(ToString::to_string);
                declared.push(FleetNodeObservation {
                    provider: "registry".to_string(),
                    provider_node: Some(name.to_string()),
                    node_id: id,
                    name: vm_name,
                    status: vm_status,
                    liveness: "unknown".to_string(),
                    ip: vm_ip,
                    desired: true,
                    observed: false,
                    meta: Some(BTreeMap::from([
                        ("source".to_string(), "registry".to_string()),
                        ("kind".to_string(), "vm".to_string()),
                        ("role".to_string(), role.to_string()),
                        ("host".to_string(), name.to_string()),
                    ])),
                });
            }
        }
    }

    Ok(dedupe_declared_nodes(declared))
}

fn dedupe_declared_nodes(nodes: Vec<FleetNodeObservation>) -> Vec<FleetNodeObservation> {
    let mut map = HashMap::new();
    for node in nodes {
        map.entry(node.node_id.clone()).or_insert(node);
    }
    let mut values: Vec<_> = map.into_values().collect();
    values.sort_unstable_by(|a, b| a.node_id.cmp(&b.node_id));
    values
}

fn bool_env_csv(key: &str) -> Vec<String> {
    std::env::var(key)
        .ok()
        .map(|v| {
            v.split(',')
                .map(|entry| entry.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

fn parse_inline_network_ip(raw: &str) -> Option<String> {
    for seg in raw.split([',', ';', ' ']) {
        let seg = seg.trim();
        if seg.starts_with("ip=") {
            return seg
                .trim_start_matches("ip=")
                .split('/')
                .next()
                .map(ToString::to_string);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

<<<<<<< HEAD
    fn desired() -> NodeSubstrateDesiredState {
        NodeSubstrateDesiredState {
            node_id: "builder".to_string(),
            provider: Some("proxmox".to_string()),
            resource_id: Some("lxc/105".to_string()),
            desired_power: NodeSubstratePower::Running,
            provisioning: None,
        }
    }

=======
>>>>>>> github/main
    #[test]
    fn parse_inline_network_ip_works() {
        assert_eq!(
            parse_inline_network_ip("name=eth0,ip=10.0.0.42/24,gw=10.0.0.1"),
            Some("10.0.0.42".to_string())
        );
        assert_eq!(
            parse_inline_network_ip("foo=bar ip=127.0.0.1/16"),
            Some("127.0.0.1".to_string())
        );
        assert_eq!(parse_inline_network_ip("noip"), None);
    }

    #[test]
<<<<<<< HEAD
    fn unavailable_provider_remains_an_unproven_unknown() {
        let observation = node_substrate_observation(
            &desired(),
            &FleetInventoryDetails {
                nodes: Vec::new(),
                summary: FleetInventorySummary::default(),
            },
        );

        assert_eq!(observation.status, "unknown");
        assert_eq!(observation.existence, NodeSubstrateExistence::Unknown);
        assert!(observation.evidence_refs.is_empty());
    }

    #[test]
    fn successful_provider_inventory_proves_exact_resource_power() {
        let inventory = FleetInventoryDetails {
            nodes: vec![FleetNodeObservation {
                provider: "proxmox".to_string(),
                provider_node: Some("pve".to_string()),
                node_id: "lxc/105".to_string(),
                name: "cynic-builder".to_string(),
                status: "stopped".to_string(),
                liveness: "down".to_string(),
                ip: None,
                desired: true,
                observed: true,
                meta: None,
            }],
            summary: FleetInventorySummary {
                providers: vec![FleetInventoryProviderSummary::ok("proxmox", 1)],
                ..FleetInventorySummary::default()
            },
        };

        let observation = node_substrate_observation(&desired(), &inventory);

        assert_eq!(observation.status, "observed");
        assert_eq!(observation.existence, NodeSubstrateExistence::Present);
        assert_eq!(observation.power, NodeSubstratePower::Stopped);
        assert!(!observation.evidence_refs.is_empty());
    }

    #[test]
    fn complete_provider_inventory_proves_resource_absence_without_fabricating_power() {
        let inventory = FleetInventoryDetails {
            nodes: Vec::new(),
            summary: FleetInventorySummary {
                providers: vec![FleetInventoryProviderSummary::ok("proxmox", 0)],
                ..FleetInventorySummary::default()
            },
        };

        let observation = node_substrate_observation(&desired(), &inventory);

        assert_eq!(observation.status, "observed");
        assert_eq!(observation.existence, NodeSubstrateExistence::Absent);
        assert_eq!(observation.power, NodeSubstratePower::Unknown);
        assert!(
            observation
                .unknowns
                .contains(&"provider_resource_absent".to_string())
=======
    fn proxmox_credential_prefers_secret_with_legacy_fallback() {
        let lookup_canonical = |key: &str| match key {
            "CYNIC_PROXMOX_TOKEN_SECRET" => Some("canonical".to_string()),
            "CYNIC_PROXMOX_SECRET" => Some("legacy".to_string()),
            _ => None,
        };

        assert_eq!(
            ProxmoxInventoryProvider::proxmox_credential_from(lookup_canonical),
            Some("canonical".to_string())
>>>>>>> github/main
        );
    }
}
