//! Proxmox VE hypervisor substrate (L1).
//!
//! Owns every Proxmox-specific adapter: LXC clone, disk extension, substrate
//! observation/control, VM control, storage, and the hypervisor watchdog.
//! Pure types and ports live in `cynic-protocol`; this crate holds the
//! adapters that drive `pct`/`qm` through `cynic-lifecycle`'s `ArgvRunner`
//! (never raw shell) and the PVE REST API.
//!
//! Layering: L1 host substrate. Depends on `cynic-protocol` (L0) and
//! `cynic-lifecycle` (ArgvRunner). Never depends on orchestrator crates.

pub mod adapters;
pub mod agent_host;
pub mod ports;
