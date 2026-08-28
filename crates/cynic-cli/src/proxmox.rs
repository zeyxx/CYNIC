use std::process::Command;

const PVE_HOST: &str = "10.10.10.1";
const PVE_USER: &str = "root";
const SSH_KEY: &str = "/var/lib/cynic/.ssh/id_ed25519";

pub fn observe_resources() -> Result<String, String> {
    let ssh_cmd = format!(
        "ssh -o ConnectTimeout=5 -i {} {}@{} 'pvesh get /cluster/resources --output-format json 2>/dev/null'",
        SSH_KEY, PVE_USER, PVE_HOST
    );
    let output = Command::new("sh")
        .arg("-c")
        .arg(&ssh_cmd)
        .output()
        .map_err(|e| format!("SSH failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Proxmox error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
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
        .map_err(|e| format!("SSH failed: {e}"))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn list_templates() -> Result<String, String> {
    let ssh_cmd = format!(
        "ssh -o ConnectTimeout=5 -i {} {}@{} 'pveam list local 2>/dev/null'",
        SSH_KEY, PVE_USER, PVE_HOST
    );
    let output = Command::new("sh")
        .arg("-c")
        .arg(&ssh_cmd)
        .output()
        .map_err(|e| format!("SSH failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Proxmox error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

pub fn info_resource(resource_id: &str) -> Result<String, String> {
    let ssh_cmd = format!(
        "ssh -o ConnectTimeout=5 -i {} {}@{} 'pvesh get /cluster/resources --output-format json 2>/dev/null'",
        SSH_KEY, PVE_USER, PVE_HOST
    );
    let output = Command::new("sh")
        .arg("-c")
        .arg(&ssh_cmd)
        .output()
        .map_err(|e| format!("SSH failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "Proxmox error: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let json: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("JSON parse: {e}"))?;

    let resources = json.as_array().ok_or("expected array")?;
    for r in resources {
        if r["id"].as_str() == Some(resource_id) {
            return Ok(serde_json::to_string_pretty(r).unwrap());
        }
    }

    Err(format!("resource {resource_id} not found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ssh_key_path_is_canonical() {
        assert_eq!(SSH_KEY, "/var/lib/cynic/.ssh/id_ed25519");
    }

    #[test]
    fn pve_host_is_hypervisor() {
        assert_eq!(PVE_HOST, "10.10.10.1");
    }
}
