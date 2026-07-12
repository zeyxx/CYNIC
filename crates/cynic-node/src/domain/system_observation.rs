use serde_json::{Value, json};

#[derive(Debug, Clone)]
pub struct PlatformObservation {
    pub zfs_health: Value,
    pub proxmox_resources: Value,
    pub critical_vmids_to_restart: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct SystemMetricsSample {
    pub cpu_usage_percent: f32,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub disk_total_gb: f64,
    pub disk_available_gb: f64,
    pub load_average_1m: f64,
    pub uptime_seconds: u64,
}

pub fn parse_zpool_health(stdout: &[u8]) -> Value {
    let text = String::from_utf8_lossy(stdout);
    let pools: Vec<_> = text
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                Some(json!({ "name": parts[0], "health": parts[1] }))
            } else {
                None
            }
        })
        .collect();
    json!(pools)
}

pub fn critical_proxmox_vmids_to_restart(resources: &Value) -> Vec<String> {
    resources
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|resource| {
            let name = resource.get("name").and_then(|value| value.as_str())?;
            let status = resource.get("status").and_then(|value| value.as_str())?;
            let id = resource.get("id").and_then(|value| value.as_str())?;
            if name == "cynic-vm-v2" && status == "stopped" {
                id.split('/').next_back().map(str::to_string)
            } else {
                None
            }
        })
        .collect()
}

pub fn system_metrics_payload(sample: SystemMetricsSample, platform: PlatformObservation) -> Value {
    json!({
        "cpu_usage_percent": sample.cpu_usage_percent,
        "memory_used_gb": sample.memory_used_gb,
        "memory_total_gb": sample.memory_total_gb,
        "disk_total_gb": sample.disk_total_gb,
        "disk_available_gb": sample.disk_available_gb,
        "load_average_1m": sample.load_average_1m,
        "uptime_seconds": sample.uptime_seconds,
        "zfs_pools": platform.zfs_health,
        "proxmox_resources": platform.proxmox_resources,
    })
}

pub fn system_observation_payload(node_name: &str, metrics: Value) -> Value {
    json!({
        "tool": "SystemProbe",
        "domain": "infra",
        "status": "ok",
        "agent_id": node_name,
        "context": format!("System metrics for node {node_name}"),
        "value": metrics,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        PlatformObservation, SystemMetricsSample, critical_proxmox_vmids_to_restart,
        parse_zpool_health, system_metrics_payload, system_observation_payload,
    };
    use serde_json::json;

    #[test]
    fn zpool_output_is_parsed_into_json_array() {
        let parsed = parse_zpool_health(b"tank ONLINE\nfastpool DEGRADED\n");
        assert_eq!(
            parsed,
            json!([
                {"name": "tank", "health": "ONLINE"},
                {"name": "fastpool", "health": "DEGRADED"}
            ])
        );
    }

    #[test]
    fn only_critical_stopped_kernel_vm_is_selected_for_restart() {
        let resources = json!([
            {"name": "cynic-vm-v2", "status": "stopped", "id": "qemu/101"},
            {"name": "cynic-vm-v2", "status": "running", "id": "qemu/102"},
            {"name": "other", "status": "stopped", "id": "qemu/103"}
        ]);
        assert_eq!(critical_proxmox_vmids_to_restart(&resources), vec!["101"]);
    }

    #[test]
    fn metrics_payload_builder_is_pure() {
        let payload = system_metrics_payload(
            SystemMetricsSample {
                cpu_usage_percent: 12.5,
                memory_used_gb: 4.0,
                memory_total_gb: 16.0,
                disk_total_gb: 100.0,
                disk_available_gb: 60.0,
                load_average_1m: 0.3,
                uptime_seconds: 123,
            },
            PlatformObservation {
                zfs_health: json!([{"name": "tank", "health": "ONLINE"}]),
                proxmox_resources: json!({"resources": []}),
                critical_vmids_to_restart: vec!["101".to_string()],
            },
        );

        assert_eq!(payload["cpu_usage_percent"], 12.5);
        assert_eq!(payload["zfs_pools"][0]["name"], "tank");
        assert_eq!(payload["proxmox_resources"]["resources"], json!([]));
    }

    #[test]
    fn system_observation_payload_wraps_metrics() {
        let payload = system_observation_payload("node-a", json!({"cpu_usage_percent": 1.0}));
        assert_eq!(payload["tool"], "SystemProbe");
        assert_eq!(payload["agent_id"], "node-a");
        assert_eq!(payload["value"]["cpu_usage_percent"], 1.0);
    }
}
