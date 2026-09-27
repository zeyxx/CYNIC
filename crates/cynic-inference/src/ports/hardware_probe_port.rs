//! Live host hardware observation for model sizing and launch planning.
//!
//! The domain owns pure parsing and budget decisions
//! (`domain::hardware_profile`); this port owns the impure reads
//! (`/proc`, sysfs, thread count, operator bandwidth override).

use crate::domain::hardware_profile::HardwareProfile;

/// Portable boundary for live hardware observation. Always live, never cached.
pub trait HardwareProbePort: Send + Sync {
    fn probe_hardware(&self) -> HardwareProfile;
}
