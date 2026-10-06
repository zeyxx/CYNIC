//! Resource budget and limit types for CYNIC workload admission.
//!
//! WHY THIS EXISTS.
//!
//! CYNIC spawns processes (agents, builds, inference) without any notion of
//! resource consumption. The host crashes when a process saturates CPU, memory,
//! or I/O. This module provides the types needed for admission control and
//! resource isolation.
//!
//! DESIGN PRINCIPLES.
//!
//! 1. **Pure types, no I/O.** This module is in `cynic-protocol` (L0), so it
//!    must remain pure. No `std::process`, no `reqwest`, no `tokio::spawn`.
//! 2. **Optional fields.** `None` means "no limit declared", not "unlimited".
//!    The caller decides how to interpret absence.
//! 3. **Fits-within semantics.** A budget fits within a limit if every declared
//!    field is <= the corresponding limit field. Undeclared fields are not
//!    checked.
//!
//! LAYERS.
//!
//! - `cynic-protocol`: types only (this module)
//! - `cynic-workload`: admission check (does budget fit within available?)
//! - `cynic-host`: isolation (cgroups v2, RLIMIT, taskset)
//! - `cynic-node`: observation (read /proc, cgroups)

use serde::{Deserialize, Serialize};

/// A resource budget declared by a workload.
///
/// This is what a workload *requests*. The admission layer checks whether the
/// host can satisfy it before allowing the workload to start.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResourceBudget {
    /// CPU cores requested (e.g., 4.0 for 4 cores, 0.5 for half a core).
    pub cpu_cores: Option<f32>,
    /// Memory in bytes requested.
    pub memory_bytes: Option<u64>,
    /// Disk I/O bandwidth in bytes per second requested.
    pub io_bps: Option<u64>,
    /// Network bandwidth in bytes per second requested.
    pub net_bps: Option<u64>,
}

impl ResourceBudget {
    /// Create a new empty budget (no limits declared).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set CPU cores.
    pub fn with_cpu_cores(mut self, cores: f32) -> Self {
        self.cpu_cores = Some(cores);
        self
    }

    /// Set memory in bytes.
    pub fn with_memory_bytes(mut self, bytes: u64) -> Self {
        self.memory_bytes = Some(bytes);
        self
    }

    /// Set disk I/O bandwidth in bytes per second.
    pub fn with_io_bps(mut self, bps: u64) -> Self {
        self.io_bps = Some(bps);
        self
    }

    /// Set network bandwidth in bytes per second.
    pub fn with_net_bps(mut self, bps: u64) -> Self {
        self.net_bps = Some(bps);
        self
    }

    /// Check whether this budget fits within a given limit.
    ///
    /// Returns `true` if every declared field in `self` is <= the corresponding
    /// field in `limit`. Undeclared fields are not checked.
    ///
    /// # Examples
    ///
    /// ```
    /// use cynic_protocol::resource_budget::{ResourceBudget, ResourceLimit};
    ///
    /// let budget = ResourceBudget::new().with_cpu_cores(4.0).with_memory_bytes(8 * 1024 * 1024 * 1024);
    /// let limit = ResourceLimit::new().with_max_cpu_cores(8.0).with_max_memory_bytes(16 * 1024 * 1024 * 1024);
    /// assert!(budget.fits_within(&limit));
    ///
    /// let tight = ResourceLimit::new().with_max_cpu_cores(2.0);
    /// assert!(!budget.fits_within(&tight));
    /// ```
    pub fn fits_within(&self, limit: &ResourceLimit) -> bool {
        if let (Some(cpu), Some(max_cpu)) = (self.cpu_cores, limit.max_cpu_cores) {
            if cpu > max_cpu {
                return false;
            }
        }
        if let (Some(mem), Some(max_mem)) = (self.memory_bytes, limit.max_memory_bytes) {
            if mem > max_mem {
                return false;
            }
        }
        if let (Some(io), Some(max_io)) = (self.io_bps, limit.max_io_bps) {
            if io > max_io {
                return false;
            }
        }
        if let (Some(net), Some(max_net)) = (self.net_bps, limit.max_net_bps) {
            if net > max_net {
                return false;
            }
        }
        true
    }

    /// Check whether this budget is empty (no fields declared).
    pub fn is_empty(&self) -> bool {
        self.cpu_cores.is_none()
            && self.memory_bytes.is_none()
            && self.io_bps.is_none()
            && self.net_bps.is_none()
    }
}

/// A resource limit imposed by the host or admission layer.
///
/// This is what the host *allows*. The isolation layer (cgroups, RLIMIT) enforces
/// it; the admission layer checks budgets against it.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ResourceLimit {
    /// Maximum CPU cores allowed.
    pub max_cpu_cores: Option<f32>,
    /// Maximum memory in bytes allowed.
    pub max_memory_bytes: Option<u64>,
    /// Maximum disk I/O bandwidth in bytes per second allowed.
    pub max_io_bps: Option<u64>,
    /// Maximum network bandwidth in bytes per second allowed.
    pub max_net_bps: Option<u64>,
}

impl ResourceLimit {
    /// Create a new empty limit (no limits declared).
    pub fn new() -> Self {
        Self::default()
    }

    /// Set maximum CPU cores.
    pub fn with_max_cpu_cores(mut self, cores: f32) -> Self {
        self.max_cpu_cores = Some(cores);
        self
    }

    /// Set maximum memory in bytes.
    pub fn with_max_memory_bytes(mut self, bytes: u64) -> Self {
        self.max_memory_bytes = Some(bytes);
        self
    }

    /// Set maximum disk I/O bandwidth in bytes per second.
    pub fn with_max_io_bps(mut self, bps: u64) -> Self {
        self.max_io_bps = Some(bps);
        self
    }

    /// Set maximum network bandwidth in bytes per second.
    pub fn with_max_net_bps(mut self, bps: u64) -> Self {
        self.max_net_bps = Some(bps);
        self
    }

    /// Check whether this limit is empty (no fields declared).
    pub fn is_empty(&self) -> bool {
        self.max_cpu_cores.is_none()
            && self.max_memory_bytes.is_none()
            && self.max_io_bps.is_none()
            && self.max_net_bps.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_budget_fits_any_limit() {
        let budget = ResourceBudget::new();
        let limit = ResourceLimit::new().with_max_cpu_cores(1.0);
        assert!(budget.fits_within(&limit));
    }

    #[test]
    fn budget_fits_within_limit() {
        let budget = ResourceBudget::new()
            .with_cpu_cores(4.0)
            .with_memory_bytes(8 * 1024 * 1024 * 1024);
        let limit = ResourceLimit::new()
            .with_max_cpu_cores(8.0)
            .with_max_memory_bytes(16 * 1024 * 1024 * 1024);
        assert!(budget.fits_within(&limit));
    }

    #[test]
    fn budget_exceeds_cpu_limit() {
        let budget = ResourceBudget::new().with_cpu_cores(8.0);
        let limit = ResourceLimit::new().with_max_cpu_cores(4.0);
        assert!(!budget.fits_within(&limit));
    }

    #[test]
    fn budget_exceeds_memory_limit() {
        let budget = ResourceBudget::new().with_memory_bytes(16 * 1024 * 1024 * 1024);
        let limit = ResourceLimit::new().with_max_memory_bytes(8 * 1024 * 1024 * 1024);
        assert!(!budget.fits_within(&limit));
    }

    #[test]
    fn budget_exceeds_io_limit() {
        let budget = ResourceBudget::new().with_io_bps(1024 * 1024 * 1024);
        let limit = ResourceLimit::new().with_max_io_bps(512 * 1024 * 1024);
        assert!(!budget.fits_within(&limit));
    }

    #[test]
    fn budget_exceeds_net_limit() {
        let budget = ResourceBudget::new().with_net_bps(1024 * 1024 * 1024);
        let limit = ResourceLimit::new().with_max_net_bps(512 * 1024 * 1024);
        assert!(!budget.fits_within(&limit));
    }

    #[test]
    fn partial_budget_only_checks_declared_fields() {
        // Budget declares only CPU; memory is not checked even if limit is tight.
        let budget = ResourceBudget::new().with_cpu_cores(2.0);
        let limit = ResourceLimit::new()
            .with_max_cpu_cores(4.0)
            .with_max_memory_bytes(1024);
        assert!(budget.fits_within(&limit));
    }

    #[test]
    fn is_empty_works() {
        assert!(ResourceBudget::new().is_empty());
        assert!(!ResourceBudget::new().with_cpu_cores(1.0).is_empty());
        assert!(ResourceLimit::new().is_empty());
        assert!(!ResourceLimit::new().with_max_cpu_cores(1.0).is_empty());
    }

    #[test]
    fn budget_serializes_to_json() {
        let budget = ResourceBudget::new()
            .with_cpu_cores(4.0)
            .with_memory_bytes(8 * 1024 * 1024 * 1024);
        let json = serde_json::to_string(&budget).unwrap();
        assert!(json.contains("4.0"));
        assert!(json.contains("8589934592"));
    }

    #[test]
    fn budget_round_trips_through_json() {
        let budget = ResourceBudget::new()
            .with_cpu_cores(2.5)
            .with_memory_bytes(4 * 1024 * 1024 * 1024)
            .with_io_bps(100 * 1024 * 1024);
        let json = serde_json::to_string(&budget).unwrap();
        let parsed: ResourceBudget = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, budget);
    }
}
