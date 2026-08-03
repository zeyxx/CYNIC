use super::{
    policy::reconcile_build_cache,
    port::{BuildCacheMutationPort, BuildCacheObservationPort},
    types::{
        BUILD_CACHE_REPORT_SCHEMA, BuildCacheDecision, BuildCacheDesiredState, BuildCachePathState,
        BuildCacheReport,
    },
};

pub fn reconcile_build_cache_resources<O, M>(
    desired: &BuildCacheDesiredState,
    apply: bool,
    observer: &O,
    mutation: &M,
) -> BuildCacheReport
where
    O: BuildCacheObservationPort,
    M: BuildCacheMutationPort,
{
    let before = observer.observe(desired);
    let plan = reconcile_build_cache(desired, &before);
    if plan.decision != BuildCacheDecision::ReclaimCanonicalProfile || !apply {
        return BuildCacheReport {
            schema: BUILD_CACHE_REPORT_SCHEMA,
            status: plan.status,
            mutates: false,
            blockers: plan.blockers.clone(),
            before,
            plan,
            after: None,
            evidence: Vec::new(),
        };
    }
    let evidence = match mutation.reclaim_profile(desired) {
        Ok(evidence) => evidence,
        Err(error) => {
            return BuildCacheReport {
                schema: BUILD_CACHE_REPORT_SCHEMA,
                status: "blocked",
                mutates: true,
                blockers: vec![error],
                before,
                plan,
                after: None,
                evidence: Vec::new(),
            };
        }
    };
    let after = observer.observe(desired);
    let after_plan = reconcile_build_cache(desired, &after);
    let proved = after.reclaim_profile_state == BuildCachePathState::Missing
        && after.preserved_profile_states == before.preserved_profile_states
        && after_plan.decision == BuildCacheDecision::Satisfied;
    BuildCacheReport {
        schema: BUILD_CACHE_REPORT_SCHEMA,
        status: if proved { "proved" } else { "blocked" },
        mutates: true,
        blockers: if proved {
            Vec::new()
        } else if after_plan.blockers.is_empty() {
            vec!["build_cache_reclaim_threshold_not_reached".to_string()]
        } else {
            after_plan.blockers
        },
        before,
        plan,
        after: Some(after),
        evidence: vec![evidence, "available_bytes_read_back".to_string()],
    }
}
