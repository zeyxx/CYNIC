//! Pure reconciliation: BuildRequest + observed substrate → BuildPlan.
//!
//! No I/O. No mutation. Just the safe sequence: rsync → check → extract.
//! This is the "reconcile" term in the CYNIC coherence pattern.

use crate::types::{BuildRequest, BuildResult};

#[derive(Debug, Clone)]
pub struct BuildPlan {
    pub rsync_dest: String,
    pub manifest_path: String,
    pub packages: Vec<String>,
    pub env: BuildEnvSnapshot,
}

#[derive(Debug, Clone)]
pub struct BuildEnvSnapshot {
    pub min_stack: u64,
    pub jobs: usize,
}

impl BuildRequest {
    /// Pure: derive the next safe action sequence from desired + observed state.
    ///
    /// No filesystem access, no network — just the plan the apply layer executes.
    pub fn reconcile(&self, discovered_host: &str, workdir: &str) -> BuildPlan {
        let manifest = self.manifest.to_str().unwrap_or("Cargo.toml");
        BuildPlan {
            rsync_dest: format!("{host}:{workdir}", host = discovered_host, workdir = workdir),
            manifest_path: manifest.to_string(),
            packages: self.packages.clone(),
            env: BuildEnvSnapshot {
                min_stack: self.env.min_stack,
                jobs: self.env.jobs,
            },
        }
    }
}

/// Result classification — pure, no I/O.
///
/// Mirrors the shell `grep -c "^error" /tmp/vm.log | tail` logic
/// as a typed enum instead of fragile string matching.
impl BuildResult {
    pub fn classify(&self) -> ResultClass {
        if self.exit_code == 0 && self.errors.errors.is_empty() {
            ResultClass::Pass
        } else if self.exit_code == 0 {
            ResultClass::PassWithWarnings
        } else {
            ResultClass::Fail
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResultClass {
    Pass,
    PassWithWarnings,
    Fail,
}
