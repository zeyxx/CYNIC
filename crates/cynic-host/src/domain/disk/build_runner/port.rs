use super::types::{BuildCacheDesiredState, BuildCacheObservation};

pub trait BuildCacheObservationPort {
    fn observe(&self, desired: &BuildCacheDesiredState) -> BuildCacheObservation;
}

pub trait BuildCacheMutationPort {
    fn reclaim_profile(&self, desired: &BuildCacheDesiredState) -> Result<String, String>;
}
