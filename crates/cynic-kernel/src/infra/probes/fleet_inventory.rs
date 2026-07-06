//! FleetInventoryProbe — provider-agnostic infrastructure inventory sensing.
//!
//! This probe is intentionally separate from dog-health probing. It maps declared
//! fleet topology (from `infra/registry.json`) with observed VM/container nodes
//! from configured providers (currently Proxmox; Kubernetes scaffolding included).

use crate::domain::probe::{
    FleetInventoryDetails, FleetInventoryProviderSummary, FleetInventorySummary,
    FleetNodeObservation, ProbeDetails, ProbeError, ProbeResult, ProbeStatus,
};
use async_trait::async_trait;
use serde_json::{self, Value};
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::Path;
use std::time::{Duration, Instant};
use tokio::process::Command;

type ProviderError = String;

#[async_trait]
trait FleetInventoryProvider: Send + Sync {
    fn name(&self) -> &'static str;

    async fn collect_nodes(
        &self,
        declared_nodes: &[FleetNodeObservation],
    ) -> Result<Vec<FleetNodeObservation>, ProviderError>;
}

pub struct FleetInventoryProbe {
    declared_nodes: Vec<FleetNodeObservation>,
    providers: Vec<Box<dyn FleetInventoryProvider>>,
}

/// Build a provider set from environment + soT registry:
/// - declared nodes come from `infra/registry.json`
/// - providers are read from `CYNIC_FLEET_PROVIDERS`
///   (default: `proxmox`; kubernetes is scaffolded via `k8s`/`kubernetes`).
pub fn build_default_fleet_inventory_probe(project_root: &Path) -> FleetInventoryProbe {
    let declared_nodes = load_declared_inventory(project_root);
    let providers = build_providers();

    if providers.is_empty() {
        tracing::warn!(
            "No fleet inventory providers configured. Registry-only fleet facts will be published."
        );
    }

    FleetInventoryProbe {
        declared_nodes,
        providers,
    }
}

impl FleetInventoryProbe {
    /// Convert declared rows into a map keyed by `node_id` for merge operations.
    fn normalize_declared(&self) -> HashMap<String, FleetNodeObservation> {
        self.declared_nodes
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

    fn upsert_node(
        nodes: &mut HashMap<String, FleetNodeObservation>,
        observed: FleetNodeObservation,
    ) {
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
}

impl fmt::Debug for FleetInventoryProbe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("FleetInventoryProbe")
            .field("declared_nodes", &self.declared_nodes.len())
            .field("providers", &self.providers.len())
            .finish()
    }
}

#[async_trait]
impl crate::domain::probe::Probe for FleetInventoryProbe {
    fn name(&self) -> &str {
        "fleet_inventory"
    }

    fn interval(&self) -> Duration {
        Duration::from_secs(60)
    }

    async fn sense(&self) -> Result<ProbeResult, ProbeError> {
        let start = Instant::now();

        let mut nodes = self.normalize_declared();
        let mut provider_summaries = Vec::new();
        let mut provider_errors = 0usize;

        let declared_nodes: Vec<_> = nodes.values().cloned().collect();
        for provider in &self.providers {
            match provider.collect_nodes(&declared_nodes).await {
                Ok(observed_nodes) => {
                    let observed_count = observed_nodes.len();
                    for observed in observed_nodes {
                        FleetInventoryProbe::upsert_node(&mut nodes, observed);
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
        }
        if missing > 0 {
            for node_id in &missing_nodes {
                provider_evidence_gaps.push(format!("fleet:missing_node:{node_id}"));
            }
        }
        if !unexpected_nodes.is_empty() {
            for node_id in &unexpected_nodes {
                provider_evidence_gaps.push(format!("fleet:unexpected_node:{node_id}"));
            }
        }

        let summary = FleetInventorySummary {
            declared_nodes: desired as u32,
            observed_nodes: observed as u32,
            matched_nodes: matched as u32,
            missing_nodes: missing as u32,
            unexpected_nodes: unexpected as u32,
            providers: provider_summaries,
            provider_evidence_gaps,
            declaration_missing_nodes: missing_nodes,
            declaration_unexpected_nodes: unexpected_nodes,
        };

        let status = if self.providers.is_empty() {
            if desired == 0 {
                ProbeStatus::Unavailable
            } else {
                ProbeStatus::Degraded
            }
        } else if provider_errors > 0 || missing > 0 {
            ProbeStatus::Degraded
        } else if total == 0 {
            ProbeStatus::Unavailable
        } else {
            ProbeStatus::Ok
        };

        Ok(ProbeResult {
            name: "fleet_inventory".to_string(),
            status,
            details: ProbeDetails::FleetInventory(FleetInventoryDetails { nodes, summary }),
            duration_ms: start.elapsed().as_millis() as u64,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }
}

/// Build configured providers from env:
/// `CYNIC_FLEET_PROVIDERS=proxmox,k8s`
fn build_providers() -> Vec<Box<dyn FleetInventoryProvider>> {
    let provider_spec = std::env::var("CYNIC_FLEET_PROVIDERS")
        .unwrap_or_else(|_| "proxmox".to_string())
        .to_lowercase();
    let enable_k8s = bool_flag("CYNIC_FLEET_K8S_ENABLED").unwrap_or_default();
    let providers = provider_spec
        .split(',')
        .map(|token| token.trim().to_lowercase())
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();

    let mut list: Vec<Box<dyn FleetInventoryProvider>> = Vec::new();

    for provider in providers {
        match provider.as_str() {
            "proxmox" | "pm" | "proxmox_api" => {
                if let Some(p) = ProxmoxInventoryProvider::from_env() {
                    list.push(Box::new(p));
                }
            }
            "k8s" | "kubernetes" | "kubernetes_api" => {
                if enable_k8s {
                    if let Some(p) = KubernetesInventoryProvider::from_env() {
                        list.push(Box::new(p));
                    }
                } else {
                    tracing::debug!("k8s provider requested but CYNIC_FLEET_K8S_ENABLED is false");
                }
            }
            other => {
                tracing::warn!(provider = %other, "unsupported fleet provider");
            }
        }
    }

    list
}

/// Parse bool environment variable values.
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
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(10))
            .build()
            .ok()?;

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

    fn declared_by_id(declared_nodes: &[FleetNodeObservation]) -> HashSet<&str> {
        declared_nodes.iter().map(|n| n.node_id.as_str()).collect()
    }

    fn parse_status(v: &Value) -> String {
        v.as_str().unwrap_or("unknown").to_string()
    }

    fn liveness_from_status(status: &str) -> &str {
        match status {
            "running" => "up",
            "stopped" => "down",
            "paused" => "down",
            _ => "unknown",
        }
    }

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

    async fn list_nodes(&self) -> Result<Vec<String>, ProviderError> {
        if !self.target_nodes.is_empty() {
            return Ok(self.target_nodes.clone());
        }

        let body = self.request_json("/nodes").await?;
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
        let payload = self
            .request_json(&format!("/nodes/{node_name}/{vm_type}"))
            .await?;
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
                    meta: Some(std::collections::BTreeMap::from([
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

#[async_trait]
impl FleetInventoryProvider for ProxmoxInventoryProvider {
    fn name(&self) -> &'static str {
        "proxmox"
    }

    async fn collect_nodes(
        &self,
        declared_nodes: &[FleetNodeObservation],
    ) -> Result<Vec<FleetNodeObservation>, ProviderError> {
        let declared_by_id = Self::declared_by_id(declared_nodes);
        let nodes = self.list_nodes().await?;
        let mut observations = Vec::new();

        for node_name in nodes {
            let vms = self
                .collect_vms(&node_name, "qemu", &declared_by_id)
                .await?;
            if !vms.is_empty() {
                observations.extend(vms);
            }

            let lxcs = self.collect_vms(&node_name, "lxc", &declared_by_id).await?;
            if !lxcs.is_empty() {
                observations.extend(lxcs);
            }
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
                            meta: Some(std::collections::BTreeMap::from([
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
            "ready" | "Running" => "up",
            "up" => "up",
            _ => "down",
        }
    }
}

#[async_trait]
impl FleetInventoryProvider for KubernetesInventoryProvider {
    fn name(&self) -> &'static str {
        "kubernetes"
    }

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

/// Read declared nodes from `infra/registry.json`.
fn load_declared_inventory(project_root: &Path) -> Vec<FleetNodeObservation> {
    let mut declared = Vec::new();
    let registry_path = project_root.join("infra/registry.json");
    let Ok(content) = std::fs::read_to_string(&registry_path) else {
        return declared;
    };
    let Ok(registry) = serde_json::from_str::<Value>(&content) else {
        return declared;
    };

    let nodes = registry.get("nodes").and_then(Value::as_array);
    let Some(nodes) = nodes else {
        return declared;
    };

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
            meta: Some(std::collections::BTreeMap::from([
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
                    .unwrap_or("unamed")
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
                    meta: Some(std::collections::BTreeMap::from([
                        ("source".to_string(), "registry".to_string()),
                        ("kind".to_string(), "vm".to_string()),
                        ("role".to_string(), role.to_string()),
                        ("host".to_string(), name.to_string()),
                    ])),
                });
            }
        }
    }

    dedupe_declared_nodes(declared)
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
    fn proxmox_credential_prefers_secret_with_legacy_fallback() {
        let lookup_canonical = |key: &str| match key {
            "CYNIC_PROXMOX_TOKEN_SECRET" => Some("canonical".to_string()),
            "CYNIC_PROXMOX_SECRET" => Some("legacy".to_string()),
            _ => None,
        };

        assert_eq!(
            ProxmoxInventoryProvider::proxmox_credential_from(lookup_canonical),
            Some("canonical".to_string())
        );

        let lookup_legacy_only = |key: &str| match key {
            "CYNIC_PROXMOX_SECRET" => Some("legacy".to_string()),
            _ => None,
        };

        assert_eq!(
            ProxmoxInventoryProvider::proxmox_credential_from(lookup_legacy_only),
            Some("legacy".to_string())
        );

        let lookup_missing = |_key: &str| None;
        assert_eq!(
            ProxmoxInventoryProvider::proxmox_credential_from(lookup_missing),
            None
        );
    }
}
