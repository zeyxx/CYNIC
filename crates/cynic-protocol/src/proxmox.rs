//! Proxmox domain types — pure types for VM lifecycle management.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmState {
    Running,
    Stopped,
    Paused,
    Suspended,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskController {
    RootFS,
    SCSI0,
    SCSI1,
    IDE0,
    VirtIO0,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VmInfo {
    pub vmid: u32,
    pub name: String,
    pub node: String,
    pub state: VmState,
    pub disk_total_gb: Option<u64>,
    pub disk_used_gb: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiskExtension {
    pub vmid: u32,
    pub controller: DiskController,
    pub current_gb: u64,
    pub target_gb: u64,
    pub node: Option<String>,
}

impl DiskExtension {
    pub fn new(vmid: u32, target_gb: u64) -> Self {
        Self {
            vmid,
            controller: if vmid < 200 {
                DiskController::RootFS
            } else {
                DiskController::VirtIO0
            },
            current_gb: 0,
            target_gb,
            node: None,
        }
    }

    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.vmid == 0 {
            blockers.push("vmid_zero".to_string());
        }
        if self.target_gb == 0 {
            blockers.push("target_gb_zero".to_string());
        }
        if self.target_gb > 500 {
            blockers.push("target_gb_too_large".to_string());
        }
        blockers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lxc_uses_rootfs() {
        let ext = DiskExtension::new(105, 80);
        assert_eq!(ext.controller, DiskController::RootFS);
    }

    #[test]
    fn qemu_uses_virtio0() {
        let ext = DiskExtension::new(200, 80);
        assert_eq!(ext.controller, DiskController::VirtIO0);
    }

    #[test]
    fn validate_rejects_zero_target() {
        let ext = DiskExtension::new(105, 0);
        assert!(ext.validate().contains(&"target_gb_zero".to_string()));
    }
}
