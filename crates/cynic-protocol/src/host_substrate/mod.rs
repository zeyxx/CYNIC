//! Host substrate, hardware probes, memory, disk, and hypervisor contracts.
//!
//! Sub-domain grouping:
//! - `cpu`: CPU physical topology, jiffies utilization, and frequency scaling contracts.
//! - `disk`: disk filesystems, directories, cleanup targets, and space observations.
//! - `fleet_disk`: fleet-wide disk inventory and cluster storage distribution.
//! - `hypervisor`: VM lifecycle states, hypervisor control, and supervisor operations.
//! - `memory`: physical RAM, page tables, memory pressure, and cgroup accounting.
//! - `proxmox`: Proxmox VE VM identifiers, controllers, and extension policies.
//! - `vm_disk`: VM disk resizing, SCSI/VirtIO controllers, and size limits.

pub mod cpu;
pub mod disk;
pub mod fleet_disk;
pub mod hypervisor;
pub mod memory;
pub mod proxmox;
pub mod vm_disk;
