use cynic_build::adapters::local_build_cache::ProtectedExecutable;
use crate::adapters::live_service_paths::observe_unit_executables;

pub fn host_build_cache_adapter() -> LocalBuildCacheAdapter {
    LocalBuildCacheAdapter::protecting(|| {
        observe_unit_executables()
            .into_iter()
            .map(|u| ProtectedExecutable {
                name: u.unit,
                path: u.executable,
            })
            .collect()
    })
}

pub use cynic_build::adapters::local_build_cache::LocalBuildCacheAdapter;
