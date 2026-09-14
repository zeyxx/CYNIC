#![allow(dead_code)]
mod defaults;
mod load;
mod types;

#[allow(unused_imports)]
pub(crate) use load::*;
pub use types::*;

/// Canonical fallback repository id for build surfaces that lack an explicit
/// build_scheduler config (webhook admission, tests).
pub(crate) fn default_build_repository_id() -> String {
    defaults::default_build_repository_id()
}

pub(crate) fn default_runtime_registry_path() -> std::path::PathBuf {
    defaults::default_runtime_registry_path()
}

#[cfg(test)]
mod tests;
