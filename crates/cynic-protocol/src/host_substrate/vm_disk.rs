//! VM Disk Extension — pure types for hypervisor storage management.
//!
//! CYNIC supervises Proxmox. When a VM disk is full, the organism
//! requests an extension via the REST API (not ad-hoc scripts).
//!
//! Pattern: types.rs → observe.rs → reconcile.rs → apply.rs → proof

use serde::{Deserialize, Serialize};

pub const VM_DISK_EXTENSION_SCHEMA: &str = "cynic.vm_disk_extension.v1";
pub const VM_DISK_OBSERVATION_SCHEMA: &str = "cynic.vm_disk_observation.v1";

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
pub struct VmDiskExtension {
    pub schema: String,
    pub vmid: u32,
    pub node: String,
    pub controller: DiskController,
    pub current_size_gb: u64,
    pub target_size_gb: u64,
    pub rationale: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VmDiskObservation {
    pub schema: String,
    pub vmid: u32,
    pub node: String,
    pub disk_used_percent: u8,
    pub disk_total_gb: u64,
    pub disk_available_gb: u64,
    pub vm_name: Option<String>,
    pub vm_status: Option<String>,
}

impl VmDiskExtension {
    pub fn new(vmid: u32, node: &str, current_gb: u64, target_gb: u64, rationale: &str) -> Self {
        let controller = if vmid < 200 {
            DiskController::RootFS
        } else {
            DiskController::SCSI0
        };
        Self {
            schema: VM_DISK_EXTENSION_SCHEMA.to_string(),
            vmid,
            node: node.to_string(),
            controller,
            current_size_gb: current_gb,
            target_size_gb: target_gb,
            rationale: rationale.to_string(),
        }
    }

    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        if self.schema != VM_DISK_EXTENSION_SCHEMA {
            blockers.push("vm_disk_extension_schema_invalid".to_string());
        }
        if self.vmid == 0 {
            blockers.push("vm_disk_extension_vmid_zero".to_string());
        }
        if self.node.is_empty() {
            blockers.push("vm_disk_extension_node_empty".to_string());
        }
        if self.target_size_gb <= self.current_size_gb {
            blockers.push("vm_disk_extension_target_must_exceed_current".to_string());
        }
        if self.target_size_gb > 500 {
            blockers.push("vm_disk_extension_target_too_large".to_string());
        }
        blockers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_validates_basic_invariants() {
        let ext = VmDiskExtension::new(105, "pve", 40, 60, "Builder disk full");
        assert!(ext.validate().is_empty());
    }

    #[test]
    fn extension_rejects_target_smaller_than_current() {
        let ext = VmDiskExtension::new(105, "pve", 60, 40, "Invalid");
        assert!(
            ext.validate()
                .contains(&"vm_disk_extension_target_must_exceed_current".to_string())
        );
    }

    #[test]
    fn extension_rejects_target_too_large() {
        let ext = VmDiskExtension::new(105, "pve", 40, 600, "Too big");
        assert!(
            ext.validate()
                .contains(&"vm_disk_extension_target_too_large".to_string())
        );
    }

    #[test]
    fn extension_rejects_zero_vmid() {
        let ext = VmDiskExtension::new(0, "pve", 40, 60, "Invalid");
        assert!(
            ext.validate()
                .contains(&"vm_disk_extension_vmid_zero".to_string())
        );
    }

    #[test]
    fn extension_rejects_empty_node() {
        let ext = VmDiskExtension::new(105, "", 40, 60, "Invalid");
        assert!(
            ext.validate()
                .contains(&"vm_disk_extension_node_empty".to_string())
        );
    }

    #[test]
    fn lxc_vm_uses_rootfs_controller() {
        let ext = VmDiskExtension::new(105, "pve", 40, 60, "LXC builder");
        assert_eq!(ext.controller, DiskController::RootFS);
    }

    #[test]
    fn qemu_vm_uses_scsi0_controller() {
        let ext = VmDiskExtension::new(200, "pve", 40, 60, "QEMU agent");
        assert_eq!(ext.controller, DiskController::SCSI0);
    }
}
