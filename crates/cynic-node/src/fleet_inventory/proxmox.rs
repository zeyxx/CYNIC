use super::{FleetNodeObservation, ProviderError};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::time::Duration;

pub(crate) struct ProxmoxInventoryProvider {
    pub(crate) adapter: crate::adapters::proxmox_control::ProxmoxControlAdapter,
    target_nodes: Vec<String>,
}

impl ProxmoxInventoryProvider {
    pub(crate) fn from_env() -> Option<Self> {
        let adapter = crate::adapters::proxmox_control::ProxmoxControlAdapter::from_env(
            Duration::from_secs(10),
        )
        .ok()?;
        let target_nodes = super::super::bool_env_csv("CYNIC_PROXMOX_NODES");
        Some(Self {
            adapter,
            target_nodes,
        })
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
            "stopped" | "paused" => "down",
            _ => "unknown",
        }
    }

    pub(crate) async fn await_task(
        &self,
        node: &str,
        upid: &str,
    ) -> Result<(String, Option<String>), String> {
        for _ in 0..30 {
            let payload = self.adapter.task_status(node, upid).await?;
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

        let body = self.adapter.nodes().await?;
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
                && let Some(candidate) = super::super::parse_inline_network_ip(raw)
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
        let payload = self.adapter.node_resources(node_name, vm_type).await?;
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
    pub(crate) async fn collect_nodes(
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
