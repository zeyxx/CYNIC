//! Hypervisor supervision — canonical capability for VM lifecycle management.
//!
//! CYNIC supervises hypervisors (Proxmox, VMware, cloud). This module
//! defines the canonical types and ports for VM lifecycle, storage,
//! network, and snapshot operations.
//!
//! Architecture:
//!   Protocol (pure types) → Port (observe/control) → Adapter (Proxmox/VMware/...)
//!   OneShotCapability for privileged operations
//!   Budget enforcement (Sec1 / Monotonic Budget)

use serde::{Deserialize, Serialize};

pub const HYPERVISOR_OBSERVATION_SCHEMA: &str = "cynic.hypervisor_observation.v1";
pub const HYPERVISOR_CONTROL_SCHEMA: &str = "cynic.hypervisor_control.v1";
pub const VM_OBSERVATION_SCHEMA: &str = "cynic.vm_observation.v1";
pub const VM_CONTROL_SCHEMA: &str = "cynic.vm_control.v1";

/// Which hypervisor is being supervised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HypervisorKind {
    Proxmox,
    VMware,
    Cloud,
}

/// VM lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmState {
    Running,
    Stopped,
    Paused,
    Suspended,
    Unknown,
}

/// VM resource types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VmResource {
    Cpu,
    Memory,
    Disk,
    Network,
}

/// VM disk controller types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiskController {
    RootFS,
    SCSI0,
    SCSI1,
    IDE0,
    VirtIO0,
}

/// VM network model types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkModel {
    VirtIO,
    E1000,
    RTL8139,
    Vmxnet3,
}

/// Observation of a single VM.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VmObservation {
    pub schema: String,
    pub vmid: u32,
    pub name: String,
    pub state: VmState,
    pub node: String,
    pub hypervisor: HypervisorKind,
    pub cpu_cores: Option<u32>,
    pub memory_mb: Option<u64>,
    pub disk_total_gb: Option<u64>,
    pub disk_used_gb: Option<u64>,
    pub disk_used_percent: Option<u8>,
    pub ip_addresses: Vec<String>,
    pub uptime_seconds: Option<u64>,
}

/// Observation of the hypervisor cluster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HypervisorObservation {
    pub schema: String,
    pub hypervisor: HypervisorKind,
    pub nodes: Vec<String>,
    pub vms: Vec<VmObservation>,
    pub total_disk_gb: u64,
    pub used_disk_gb: u64,
    pub available_disk_gb: u64,
}

/// VM control operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "operation")]
pub enum VmControlOp {
    /// Start a VM.
    Start { vmid: u32 },
    /// Stop a VM.
    Stop { vmid: u32 },
    /// Reboot a VM.
    Reboot { vmid: u32 },
    /// Extend a VM disk.
    ExtendDisk {
        vmid: u32,
        controller: DiskController,
        current_gb: u64,
        target_gb: u64,
    },
    /// Add a new disk to a VM.
    AddDisk {
        vmid: u32,
        controller: DiskController,
        size_gb: u64,
    },
    /// Remove a disk from a VM.
    RemoveDisk {
        vmid: u32,
        controller: DiskController,
    },
    /// Add a network interface.
    AddNetwork {
        vmid: u32,
        model: NetworkModel,
        bridge: String,
    },
    /// Remove a network interface.
    RemoveNetwork { vmid: u32, index: u32 },
    /// Create a snapshot.
    CreateSnapshot {
        vmid: u32,
        name: String,
        description: Option<String>,
    },
    /// Delete a snapshot.
    DeleteSnapshot { vmid: u32, name: String },
    /// Rollback to a snapshot.
    RollbackSnapshot { vmid: u32, name: String },
    /// Clone a VM.
    CloneVm {
        vmid: u32,
        new_vmid: u32,
        new_name: String,
    },
    /// Delete a VM.
    DeleteVm { vmid: u32 },
}

/// Result of a VM control operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VmControlResult {
    pub schema: String,
    pub operation: VmControlOp,
    pub success: bool,
    pub error: Option<String>,
    pub evidence_ref: Option<String>,
}

/// Port for observing hypervisor state.
pub trait HypervisorObserver: Send + Sync {
    /// Observe the full hypervisor cluster state.
    fn observe_hypervisor(&self) -> Result<HypervisorObservation, String>;

    /// Observe a single VM.
    fn observe_vm(&self, vmid: u32) -> Result<VmObservation, String>;
}

/// Port for controlling VMs.
pub trait HypervisorController: Send + Sync {
    /// Execute a VM control operation.
    fn control_vm(&self, op: &VmControlOp) -> Result<VmControlResult, String>;

    /// Validate a control operation before execution.
    fn validate_control(&self, op: &VmControlOp) -> Vec<String>;
}

impl VmControlOp {
    /// Pure validation of a control operation.
    pub fn validate(&self) -> Vec<String> {
        let mut blockers = Vec::new();
        match self {
            VmControlOp::ExtendDisk {
                target_gb,
                current_gb,
                ..
            } => {
                if *target_gb <= *current_gb {
                    blockers.push("extend_disk_target_must_exceed_current".to_string());
                }
                if *target_gb > 500 {
                    blockers.push("extend_disk_target_too_large".to_string());
                }
            }
            VmControlOp::AddDisk { size_gb, .. } => {
                if *size_gb == 0 {
                    blockers.push("add_disk_size_zero".to_string());
                }
                if *size_gb > 500 {
                    blockers.push("add_disk_size_too_large".to_string());
                }
            }
            VmControlOp::Start { .. }
            | VmControlOp::Stop { .. }
            | VmControlOp::Reboot { .. }
            | VmControlOp::RemoveDisk { .. }
            | VmControlOp::AddNetwork { .. }
            | VmControlOp::RemoveNetwork { .. }
            | VmControlOp::CreateSnapshot { .. }
            | VmControlOp::DeleteSnapshot { .. }
            | VmControlOp::RollbackSnapshot { .. }
            | VmControlOp::CloneVm { .. }
            | VmControlOp::DeleteVm { .. } => {}
        }
        blockers
    }

    /// Extract the vmid from any operation.
    pub fn vmid(&self) -> u32 {
        match self {
            VmControlOp::Start { vmid } => *vmid,
            VmControlOp::Stop { vmid } => *vmid,
            VmControlOp::Reboot { vmid } => *vmid,
            VmControlOp::ExtendDisk { vmid, .. } => *vmid,
            VmControlOp::AddDisk { vmid, .. } => *vmid,
            VmControlOp::RemoveDisk { vmid, .. } => *vmid,
            VmControlOp::AddNetwork { vmid, .. } => *vmid,
            VmControlOp::RemoveNetwork { vmid, .. } => *vmid,
            VmControlOp::CreateSnapshot { vmid, .. } => *vmid,
            VmControlOp::DeleteSnapshot { vmid, .. } => *vmid,
            VmControlOp::RollbackSnapshot { vmid, .. } => *vmid,
            VmControlOp::CloneVm { vmid, .. } => *vmid,
            VmControlOp::DeleteVm { vmid } => *vmid,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extend_disk_validates_target_exceeds_current() {
        let op = VmControlOp::ExtendDisk {
            vmid: 105,
            controller: DiskController::RootFS,
            current_gb: 40,
            target_gb: 60,
        };
        assert!(op.validate().is_empty());
    }

    #[test]
    fn extend_disk_rejects_smaller_target() {
        let op = VmControlOp::ExtendDisk {
            vmid: 105,
            controller: DiskController::RootFS,
            current_gb: 60,
            target_gb: 40,
        };
        assert!(
            op.validate()
                .contains(&"extend_disk_target_must_exceed_current".to_string())
        );
    }

    #[test]
    fn extend_disk_rejects_too_large() {
        let op = VmControlOp::ExtendDisk {
            vmid: 105,
            controller: DiskController::RootFS,
            current_gb: 40,
            target_gb: 600,
        };
        assert!(
            op.validate()
                .contains(&"extend_disk_target_too_large".to_string())
        );
    }

    #[test]
    fn add_disk_validates_size() {
        let op = VmControlOp::AddDisk {
            vmid: 105,
            controller: DiskController::SCSI0,
            size_gb: 100,
        };
        assert!(op.validate().is_empty());
    }

    #[test]
    fn add_disk_rejects_zero_size() {
        let op = VmControlOp::AddDisk {
            vmid: 105,
            controller: DiskController::SCSI0,
            size_gb: 0,
        };
        assert!(op.validate().contains(&"add_disk_size_zero".to_string()));
    }

    #[test]
    fn vmid_extraction() {
        let op = VmControlOp::Start { vmid: 42 };
        assert_eq!(op.vmid(), 42);
    }

    #[test]
    fn vm_observation_schema() {
        let obs = VmObservation {
            schema: VM_OBSERVATION_SCHEMA.to_string(),
            vmid: 105,
            name: "cynic-builder".to_string(),
            state: VmState::Running,
            node: "pve".to_string(),
            hypervisor: HypervisorKind::Proxmox,
            cpu_cores: Some(4),
            memory_mb: Some(8192),
            disk_total_gb: Some(40),
            disk_used_gb: Some(38),
            disk_used_percent: Some(95),
            ip_addresses: vec!["10.10.10.119".to_string()],
            uptime_seconds: Some(3600),
        };
        assert_eq!(obs.schema, "cynic.vm_observation.v1");
    }
}
