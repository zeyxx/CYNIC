#![allow(dead_code)]
mod defaults;
mod load;
mod types;

#[allow(unused_imports)]
pub(crate) use load::*;
pub(crate) use types::*;

/// Canonical fallback repository id for build surfaces that lack an explicit
/// build_scheduler config (webhook admission, tests).
pub(crate) fn default_build_repository_id() -> String {
    defaults::default_build_repository_id()
}

#[cfg(test)]
mod tests;
