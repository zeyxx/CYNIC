mod policy;
mod port;
mod reconcile;
mod types;

pub use policy::{load_build_cache_desired_state, reconcile_build_cache};
pub use port::{BuildCacheMutationPort, BuildCacheObservationPort};
pub use reconcile::reconcile_build_cache_resources;
pub use types::{
    BUILD_CACHE_OBSERVATION_SCHEMA, BUILD_CACHE_PLAN_SCHEMA, BUILD_CACHE_REPORT_SCHEMA,
    BuildCacheDecision, BuildCacheDesiredState, BuildCacheObservation, BuildCachePathState,
    BuildCachePlan, BuildCacheReport,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_space_and_safe_debug_cache_plan_one_bounded_reclaim() {
        let desired = BuildCacheDesiredState {
            node_id: "builder-a".to_string(),
            state_root: "/state".to_string(),
            canonical_target: "/state/targets".to_string(),
            minimum_available_bytes: 100,
            reclaim_profile: "debug".to_string(),
            preserved_profiles: vec!["release".to_string()],
            noncanonical_cache_paths: Vec::new(),
        };
        let observation = BuildCacheObservation {
            schema: BUILD_CACHE_OBSERVATION_SCHEMA,
            node_id: desired.node_id.clone(),
            available_bytes: Some(10),
            canonical_target: desired.canonical_target.clone(),
            canonical_target_state: BuildCachePathState::SafeDirectory,
            reclaim_profile_path: "/state/targets/debug".to_string(),
            reclaim_profile_state: BuildCachePathState::SafeDirectory,
            preserved_profile_states: vec![BuildCachePathState::SafeDirectory],
            observed_noncanonical_cache_paths: Vec::new(),
            unknowns: Vec::new(),
        };

        let plan = reconcile_build_cache(&desired, &observation);

        assert_eq!(plan.decision, BuildCacheDecision::ReclaimCanonicalProfile);
        assert!(plan.mutates);
        assert_eq!(plan.target_profile, "debug");
        assert_eq!(plan.preserved_profiles, ["release"]);
    }
}
