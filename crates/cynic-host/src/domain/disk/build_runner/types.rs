use serde::Serialize;

pub const BUILD_CACHE_OBSERVATION_SCHEMA: &str = "cynic.build_runner.cache_observation.v1";
pub const BUILD_CACHE_PLAN_SCHEMA: &str = "cynic.build_runner.cache_plan.v1";
pub const BUILD_CACHE_REPORT_SCHEMA: &str = "cynic.build_runner.cache_report.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildCacheDesiredState {
    pub node_id: String,
    pub state_root: String,
    pub canonical_target: String,
    pub minimum_available_bytes: u64,
    pub reclaim_profile: String,
    pub preserved_profiles: Vec<String>,
    pub noncanonical_cache_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildCachePathState {
    Missing,
    SafeDirectory,
    Rejected,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildCacheObservation {
    pub schema: &'static str,
    pub node_id: String,
    pub available_bytes: Option<u64>,
    pub canonical_target: String,
    pub canonical_target_state: BuildCachePathState,
    pub reclaim_profile_path: String,
    pub reclaim_profile_state: BuildCachePathState,
    pub preserved_profile_states: Vec<BuildCachePathState>,
    pub observed_noncanonical_cache_paths: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildCacheDecision {
    Satisfied,
    ReclaimCanonicalProfile,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildCachePlan {
    pub schema: &'static str,
    pub status: &'static str,
    pub node_id: String,
    pub mutates: bool,
    pub decision: BuildCacheDecision,
    pub target_profile: String,
    pub target_path: String,
    pub preserved_profiles: Vec<String>,
    pub minimum_available_bytes: u64,
    pub observed_available_bytes: Option<u64>,
    pub blockers: Vec<String>,
    pub rollback_intent: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BuildCacheReport {
    pub schema: &'static str,
    pub status: &'static str,
    pub mutates: bool,
    pub before: BuildCacheObservation,
    pub plan: BuildCachePlan,
    pub after: Option<BuildCacheObservation>,
    pub evidence: Vec<String>,
    pub blockers: Vec<String>,
}
