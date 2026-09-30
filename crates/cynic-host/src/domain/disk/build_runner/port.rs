use super::types::{BuildCacheDesiredState, BuildCacheObservation};

pub trait BuildCacheObservationPort {
    fn observe(&self, desired: &BuildCacheDesiredState) -> BuildCacheObservation;
}

pub trait BuildCacheMutationPort {
    fn reclaim_profile(&self, desired: &BuildCacheDesiredState) -> Result<String, String>;

    /// Remove the non-canonical build caches the registry declares.
    ///
    /// WHY THIS IS A SEPARATE OPERATION, AND WHY IT COMES FIRST. The pure
    /// decision (`reconcile_build_cache`) refuses to reclaim the canonical
    /// profile while a declared non-canonical cache exists — it reports
    /// `noncanonical_build_cache_present` and blocks. That refusal is right: an
    /// ungoverned cache is the one to remove first, because it is the one nobody
    /// declared as the hot cache. But for a long time nothing implemented the
    /// removal, so the policy declared a step no executor performed and the
    /// canonical reclaim was unreachable. The builder filled to 100% while the
    /// registry said the answer was one cache removal away.
    ///
    /// Returns evidence naming what was removed. Paths that are already absent
    /// are skipped, not an error: an absent cache is the desired state, and
    /// failing on it would make the operation single-use.
    fn reclaim_noncanonical_caches(
        &self,
        desired: &BuildCacheDesiredState,
    ) -> Result<String, String>;
}
