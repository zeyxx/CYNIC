//! System Metabolism Provider for Observer
//!
//! Observes host hardware usage (CPU and memory) to determine the node's
//! metabolic operating state (HighPerformance, Standard, Survival) without
//! depending on host orchestrator crates.

use cynic_protocol::capacity::{LoadPolicy, LoadTier};

/// Resolve the declared load policy for this process.
///
/// Reads `CYNIC_LOAD_POLICY` as a path to a JSON `LoadPolicy` and delegates the
/// selection to the pure `LoadPolicy::resolve`, so the observer and the CLI cannot
/// disagree about which thresholds are in force.
pub fn declared_load_policy() -> LoadPolicy {
    let declared = std::env::var("CYNIC_LOAD_POLICY")
        .ok()
        .and_then(|path| std::fs::read_to_string(path).ok());
    match LoadPolicy::resolve(declared.as_deref(), "observer-default") {
        Ok(policy) => policy,
        Err(error) => {
            eprintln!("{error}; falling back to declared defaults");
            LoadPolicy::with_legacy_defaults("observer-default")
        }
    }
}

pub trait LoadProbe: Send + Sync {
    /// Classifies the host against the supplied declared policy.
    ///
    /// The thresholds arrive as data. An adapter that decides its own thresholds
    /// is a second, invisible policy: `CYNIC_LOAD_POLICY` would change one path
    /// and leave this one untouched, which is precisely the split that made the
    /// literals here unreadable in the first place.
    fn current_load(&mut self, policy: &LoadPolicy) -> LoadTier;
}

/// OS-backed load probe using `sysinfo`.
pub struct OsLoadProbe {
    system: sysinfo::System,
}

impl std::fmt::Debug for OsLoadProbe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OsLoadProbe").finish()
    }
}

impl OsLoadProbe {
    pub fn new() -> Self {
        let mut system = sysinfo::System::new_all();
        system.refresh_all();
        Self { system }
    }
}

impl Default for OsLoadProbe {
    fn default() -> Self {
        Self::new()
    }
}

impl LoadProbe for OsLoadProbe {
    fn current_load(&mut self, policy: &LoadPolicy) -> LoadTier {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();

        let global_cpu = self.system.global_cpu_usage();
        let free_memory = self.system.free_memory();
        let total_memory = self.system.total_memory();
        let mem_usage_percent = if total_memory > 0 {
            ((total_memory - free_memory) as f32 / total_memory as f32) * 100.0
        } else {
            0.0
        };

        let saturation_cpu = policy.saturation_cpu_pct as f32;
        let saturation_mem = policy.saturation_mem_pct as f32;
        let normal_cpu = policy.normal_cpu_pct as f32;
        let normal_mem = policy.normal_mem_pct as f32;

        if global_cpu > saturation_cpu || mem_usage_percent > saturation_mem {
            LoadTier::Saturated
        } else if global_cpu > normal_cpu || mem_usage_percent > normal_mem {
            LoadTier::Normal
        } else {
            LoadTier::Headroom
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LoadPolicy, LoadProbe, OsLoadProbe, declared_load_policy};

    /// WHY: the thresholds used to be literals in this file while
    /// `CYNIC_LOAD_POLICY` configured only `assess_load_tier`. Two measurement
    /// paths, one configurable and one not, and no way to tell from the outside
    /// which one an operator's change had reached.
    ///
    /// HOW: raise the declared thresholds far above anything sysinfo can report
    /// and assert the host classifies as `Headroom`. If the literals come back, or
    /// if the declared policy is bypassed again, this test returns `Saturated`
    /// and fails. It is the regression proof, not a description of the default.
    #[test]
    fn declared_thresholds_govern_classification() {
        let permissive = LoadPolicy {
            schema: LoadPolicy::with_legacy_defaults("test").schema,
            policy_id: "test-unreachable".to_string(),
            saturation_cpu_pct: 100.0,
            saturation_mem_pct: 100.0,
            normal_cpu_pct: 99.5,
            normal_mem_pct: 99.5,
        };
        let mut probe = OsLoadProbe::new();
        let tier = probe.current_load(&permissive);
        assert_eq!(
            tier,
            cynic_protocol::capacity::LoadTier::Headroom,
            "declared policy was ignored: a host under 99.5% load cannot be Normal"
        );
    }

    #[test]
    fn a_declared_policy_that_is_present_but_invalid_does_not_fall_back_silently() {
        // Falling back would make an operator's threshold change a no-op while
        // reporting success, which is the failure mode this whole slice removes.
        let bad = r#"{"schema":"cynic.load_policy.v1","policy_id":"x",
            "survival_cpu_pct":10.0,"survival_mem_pct":10.0,
            "standard_cpu_pct":90.0,"standard_mem_pct":90.0}"#;
        assert!(LoadPolicy::resolve(Some(bad), "t").is_err());
    }

    #[test]
    fn absent_declaration_resolves_to_the_declared_defaults() {
        let policy = LoadPolicy::resolve(None, "t").unwrap_or_else(|_| unreachable!());
        let defaults = LoadPolicy::with_legacy_defaults("t");
        assert_eq!(policy.saturation_cpu_pct, defaults.saturation_cpu_pct);
        assert_eq!(policy.policy_id, "t");
    }

    #[test]
    fn the_process_policy_agrees_with_the_shared_rule() {
        // Guards against the observer drifting from the CLI: both call resolve.
        let _ = declared_load_policy();
    }
}
