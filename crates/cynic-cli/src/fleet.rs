use std::process::Command;

const PVE_HOST: &str = "10.10.10.1";
const PVE_USER: &str = "root";
const SSH_KEY: &str = "/var/lib/cynic/.ssh/id_ed25519";

#[derive(Debug, Clone, serde::Serialize)]
pub struct FleetResource {
    pub id: String,
    pub name: String,
    pub status: String,
    pub resource_type: String,
    pub vcpus: Option<u32>,
    pub ram_gb: Option<u64>,
    pub disk_gb: Option<u64>,
}

pub fn observe_fleet() -> Result<Vec<FleetResource>, String> {
    let ssh_cmd = format!(
        "ssh -o ConnectTimeout=5 -i {} {}@{} 'pvesh get /cluster/resources --output-format json 2>/dev/null'",
        SSH_KEY, PVE_USER, PVE_HOST
    );
    let output = Command::new("sh")
        .arg("-c")
        .arg(&ssh_cmd)
        .output()
        .map_err(|e| format!("SSH to Proxmox failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Proxmox API error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("JSON parse error: {e}"))?;

    let resources = json.as_array().ok_or("expected JSON array")?;
    let mut fleet = Vec::new();

    for r in resources {
        let id = r["id"].as_str().unwrap_or("unknown").to_string();
        let name = r["name"].as_str().unwrap_or("unknown").to_string();
        let status = r["status"].as_str().unwrap_or("unknown").to_string();
        let resource_type = r["type"].as_str().unwrap_or("unknown").to_string();

        if resource_type == "node" || resource_type == "storage" || resource_type == "sdn" {
            continue;
        }

        let vcpus = r["maxcpu"].as_u64().map(|v| v as u32);
        let ram_gb = r["maxmem"].as_u64().map(|v| v / 1024 / 1024);
        let disk_gb = r["maxdisk"].as_u64().map(|v| v / 1024 / 1024 / 1024);

        fleet.push(FleetResource {
            id,
            name,
            status,
            resource_type,
            vcpus,
            ram_gb,
            disk_gb,
        });
    }

    Ok(fleet)
}

pub fn destroy_resource(vmid: u32) -> Result<String, String> {
    let ssh_cmd = format!(
        "ssh -o ConnectTimeout=5 -i {} {}@{} 'pct destroy {} 2>&1 || qm destroy {} 2>&1'",
        SSH_KEY, PVE_USER, PVE_HOST, vmid, vmid
    );
    let output = Command::new("sh")
        .arg("-c")
        .arg(&ssh_cmd)
        .output()
        .map_err(|e| format!("SSH to Proxmox failed: {e}"))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fleet_resource_creation() {
        let r = FleetResource {
            id: "lxc/100".to_string(),
            name: "test".to_string(),
            status: "running".to_string(),
            resource_type: "lxc".to_string(),
            vcpus: Some(2),
            ram_gb: Some(1),
            disk_gb: Some(4),
        };
        assert_eq!(r.id, "lxc/100");
    }
}
