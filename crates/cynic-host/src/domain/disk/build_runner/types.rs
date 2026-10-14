use serde::Serialize;

pub const BUILD_CACHE_OBSERVATION_SCHEMA: &str = "cynic.build_runner.cache_observation.v1";
pub const BUILD_CACHE_PLAN_SCHEMA: &str = "cynic.build_runner.cache_plan.v1";
pub const BUILD_CACHE_REPORT_SCHEMA: &str = "cynic.build_runner.cache_report.v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildCacheDesiredState {
    pub node_id: String,
    pub state_root: String,
    /// The temporary root the registry may place non-canonical caches under.
    ///
    /// Carried because the mutation that removes those caches must enforce the
    /// same boundary the loader used to build their paths. Re-deriving it at
    /// removal time would be a second reading of the same policy, and the two
    /// readings could disagree.
    pub temporary_root: String,
    pub canonical_target: String,
    pub minimum_available_bytes: u64,
    pub reclaim_profile: String,
    pub preserved_profiles: Vec<String>,
    pub noncanonical_cache_paths: Vec<String>,
    /// Ephemeral scopes the retention policy governs (evidence checkouts,
    /// terminal workload workspaces). Empty when the registry declares no
    /// `ephemeral_retention` section — retention is opt-in, and its absence
    /// preserves the historical behavior exactly.
    pub ephemeral_scopes: Vec<EphemeralScope>,
    /// Hard age cap in ms: entries older than this are eligible even without
    /// disk pressure. None when retention is not declared.
    pub retention_max_age_ms: Option<u64>,
    /// Grace floor in ms: under pressure, evidence checkouts younger than
    /// this are skipped — a minutes-old checkout is almost certainly a live
    /// build with no liveness signal in the store. Terminal workspaces need
    /// no grace: terminal is verified, dead is dead. None when retention is
    /// not declared.
    pub retention_pressure_grace_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EphemeralScope {
    pub base: String,
    pub relative_path: String,
    pub kind: EphemeralKind,
    /// State-root-relative path of the workload record store, required for
    /// `WorkloadWorkspace` scopes so workspace liveness is verified against
    /// the store. Unused for checkouts, which have no store linkage.
    pub workload_store_relative_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EphemeralKind {
    EvidenceCheckout,
    WorkloadWorkspace,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EphemeralEntry {
    pub path: String,
    pub age_ms: u64,
    pub kind: EphemeralKind,
    /// Liveness read-back for workspaces: Some(true) terminal, Some(false)
    /// live, None unclassifiable (record missing or unreadable — fail closed,
    /// never eligible). Always None for checkouts.
    pub workload_terminal: Option<bool>,
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
    /// Classified ephemeral entries under the declared retention scopes.
    pub observed_ephemeral: Vec<EphemeralEntry>,
    /// Entries that could not be classified (unreadable, unparseable name,
    /// missing workload record). Listed, never eligible, never blocking:
    /// an unclassifiable directory must not wedge the canonical reclaim.
    pub ephemeral_skipped: Vec<String>,
    pub unknowns: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildCacheDecision {
    Satisfied,
    ReclaimEphemeral,
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
    /// Ephemeral targets for `ReclaimEphemeral`, eldest first. Empty for
    /// every other decision.
    pub ephemeral_targets: Vec<String>,
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
