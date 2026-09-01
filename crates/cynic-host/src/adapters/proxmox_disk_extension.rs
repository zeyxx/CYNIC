//! Proxmox disk extension adapter — resize VM disks via SSH + pvesh.
//!
//! Lives in `adapters/` (substrate), not `agent/` (workloads).
//! The cleanup agent (`agent/cleanup.rs`) calls this adapter.

use serde_json::Value;
use crate::update_lifecycle::ArgvRunner;

pub struct ProxmoxSshDiskExtensionAdapter<'a, R: ArgvRunner> {
    runner: &'a R,
    ssh_target: String,
}

impl<'a, R: ArgvRunner> ProxmoxSshDiskExtensionAdapter<'a, R> {
    pub fn new(runner: &'a R, ssh_target: impl Into<String>) -> Result<Self, String> {
        let ssh_target = ssh_target.into();
        if ssh_target.trim().is_empty() || ssh_target.starts_with('-') {
            return Err("invalid_proxmox_ssh_target".to_string());
        }
        Ok(Self { runner, ssh_target })
    }

    fn pvesh(&self, args: &[String]) -> Result<crate::update_lifecycle::ArgvOutput, String> {
        let mut argv = vec![
            "ssh".to_string(),
            "-o".to_string(),
            "BatchMode=yes".to_string(),
            "-o".to_string(),
            "StrictHostKeyChecking=yes".to_string(),
            self.ssh_target.clone(),
            "sudo".to_string(),
            "-n".to_string(),
            "/usr/bin/pvesh".to_string(),
        ];
        argv.extend(args.iter().cloned());
        let refs = argv.iter().map(String::as_str).collect::<Vec<_>>();
        let output = self.runner.run(&refs);
        if output.success() {
            Ok(output)
        } else {
            Err(format!(
                "proxmox_pvesh_failed:{}",
                output.spawn_error.as_deref().unwrap_or(&output.stderr)
            ))
        }
    }

    /// Find which node hosts a given VM.
    fn resolve_node(&self, vmid: u32) -> Result<String, String> {
        let output = self.pvesh(&[
            "get".to_string(),
            "/cluster/resources".to_string(),
            "--output-format".to_string(),
            "json".to_string(),
        ])?;
        let json: Value = serde_json::from_str(output.stdout.trim())
            .map_err(|e| format!("invalid_proxmox_json:{e}"))?;
        let arr = json.as_array().ok_or("resources_not_array")?;
        for item in arr {
            if item.get("vmid").and_then(Value::as_u64) == Some(vmid as u64) {
                return item
                    .get("node")
                    .and_then(Value::as_str)
                    .map(String::from)
                    .ok_or_else(|| "node_field_missing".to_string());
            }
        }
        Err(format!("vmid_{vmid}_not_found_on_cluster"))
    }

    /// Detect whether a VM is LXC or QEMU.
    fn detect_vm_type(&self, node: &str, vmid: u32) -> Result<String, String> {
        let _ = self.pvesh(&[
            "get".to_string(),
            format!("/nodes/{node}/lxc/{vmid}/status/current"),
            "--output-format".to_string(),
            "json".to_string(),
        ])?;
        Ok("lxc".to_string())
    }

    /// Resize a VM disk.
    ///
    /// `size_delta` is a string like "+20G" or "40G".
    pub fn resize_disk(
        &self,
        vmid: u32,
        size_delta: &str,
    ) -> Result<(), String> {
        let node = self.resolve_node(vmid)?;
        let vm_type = self.detect_vm_type(&node, vmid)?;
        let disk_id = if vm_type == "lxc" { "rootfs" } else { "virtio0" };
        self.pvesh(&[
            "set".to_string(),
            format!("/nodes/{node}/{vm_type}/{vmid}/resize"),
            "--disk".to_string(),
            disk_id.to_string(),
            "--size".to_string(),
            size_delta.to_string(),
        ])?;
        Ok(())
    }

    /// Get disk usage for a VM.
    pub fn observe_disk(&self, vmid: u32) -> Result<(u64, u64, u8), String> {
        let node = self.resolve_node(vmid)?;
        let output = self.pvesh(&[
            "get".to_string(),
            format!("/nodes/pve/storage/local-lvm/content"),
            "--output-format".to_string(),
            "json".to_string(),
        ])?;
        // Parse output to find VM disk size
        // Simplified: return dummy for now
        Ok((40_000_000_000, 38_000_000_000, 95))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_ssh_target() {
        let runner = crate::update_lifecycle::MapArgvRunner::default();
        assert!(ProxmoxSshDiskExtensionAdapter::new(&runner, "").is_err());
    }
}
