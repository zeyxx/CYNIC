use super::{
    policy::reconcile_build_cache,
    port::{BuildCacheMutationPort, BuildCacheObservationPort},
    types::{
        BUILD_CACHE_REPORT_SCHEMA, BuildCacheDecision, BuildCacheDesiredState, BuildCacheObservation,
        BuildCachePathState, BuildCacheReport,
    },
};

/// Reconcile the builder's cache against the registry's declared policy.
///
/// THE ORDER IS THE POINT, AND IT WAS THE BUG. `reconcile_build_cache` refuses
/// to reclaim the canonical profile while a declared non-canonical cache exists:
/// it reports `noncanonical_build_cache_present` and blocks. That refusal is
/// correct — the ungoverned cache is the one to remove first, because it is the
/// one the policy never named as the hot cache.
///
/// What was missing was the executor for that first step. The registry declared
/// `noncanonical_cache_candidates`, the pure decision blocked on their presence,
/// and nothing in the workspace removed them. So the canonical reclaim was
/// unreachable, the trigger `available_bytes_below_minimum` stayed latched, and
/// the builder filled to 100% while the policy described the answer. A declared
/// step with no executor is a policy that cannot be obeyed, which reads to the
/// operator exactly like a policy that is being obeyed.
///
/// So: remove the declared non-canonical caches, RE-OBSERVE, then run the
/// canonical decision against the observed state. The re-observation is what
/// makes the second step honest — the plan is built from what is, not from what
/// the first step was expected to leave behind.
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

    // 1. The declared non-canonical caches, when any are present. A dry run
    //    stops before this and reports the plan the decision would produce.
    let noncanonical_present = !before.observed_noncanonical_cache_paths.is_empty();
    if apply && noncanonical_present {
        let evidence = match mutation.reclaim_noncanonical_caches(desired) {
            Ok(evidence) => evidence,
            Err(error) => {
                let plan = reconcile_build_cache(desired, &before);
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

        // 2. Re-observe, then decide against the observed state.
        let after_noncanonical = observer.observe(desired);
        let plan = reconcile_build_cache(desired, &after_noncanonical);
        let noncanonical_cleared = after_noncanonical
            .observed_noncanonical_cache_paths
            .is_empty();

        if plan.decision != BuildCacheDecision::ReclaimCanonicalProfile {
            // The non-canonical removal happened; the canonical profile was not
            // needed (threshold now met) or is not reclaimable. Report the real
            // observed outcome rather than assuming the second step.
            return BuildCacheReport {
                schema: BUILD_CACHE_REPORT_SCHEMA,
                status: if noncanonical_cleared {
                    "proved"
                } else {
                    "blocked"
                },
                mutates: true,
                blockers: if noncanonical_cleared {
                    Vec::new()
                } else {
                    vec!["noncanonical_cache_reclaim_read_back_failed".to_string()]
                },
                before,
                plan,
                after: Some(after_noncanonical),
                evidence: vec![
                    evidence,
                    "noncanonical_caches_absent_read_back".to_string(),
                ],
            };
        }

        return reclaim_canonical(
            desired,
            observer,
            mutation,
            before,
            after_noncanonical,
            Some(evidence),
        );
    }

    let plan = reconcile_build_cache(desired, &before);
    if plan.decision != BuildCacheDecision::ReclaimCanonicalProfile || !apply {
        return BuildCacheReport {
            schema: BUILD_CACHE_REPORT_SCHEMA,
            status: plan.status,
            mutates: false,
            blockers: plan.blockers.clone(),
            before: before.clone(),
            plan,
            after: None,
            evidence: Vec::new(),
        };
    }

    reclaim_canonical(desired, observer, mutation, before.clone(), before, None)
}

/// The canonical-profile step, shared by both entry paths.
fn reclaim_canonical<O, M>(
    desired: &BuildCacheDesiredState,
    observer: &O,
    mutation: &M,
    before: BuildCacheObservation,
    current: BuildCacheObservation,
    prior_evidence: Option<String>,
) -> BuildCacheReport
where
    O: BuildCacheObservationPort,
    M: BuildCacheMutationPort,
{
    let plan = reconcile_build_cache(desired, &current);
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

    let mut all_evidence = Vec::new();
    if let Some(prior) = prior_evidence {
        all_evidence.push(prior);
        all_evidence.push("noncanonical_caches_absent_read_back".to_string());
    }
    all_evidence.push(evidence);
    all_evidence.push("available_bytes_read_back".to_string());

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
        evidence: all_evidence,
    }
}
