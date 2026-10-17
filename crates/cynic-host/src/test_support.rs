//! Test-only shared helpers: process-wide environment isolation.
//!
//! Unit tests run in threads of one binary while `std::env` is process-global.
//! Every test that mutates environment variables must hold [`env_lock`] for
//! the whole mutate-observe-restore window, or parallel tests observe each
//! other's values. [`EnvOverride`] restores the previous value on drop so a
//! test never leaks its fixture into another test's world.

use std::ffi::OsString;
use std::sync::{Mutex, MutexGuard};

static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Hold for the entire mutate-observe-restore window of an env-fixture test.
pub(crate) fn env_lock() -> MutexGuard<'static, ()> {
    ENV_LOCK.lock().unwrap_or_else(|error| error.into_inner())
}

pub(crate) struct EnvOverride {
    key: &'static str,
    original: Option<OsString>,
}

impl EnvOverride {
    pub(crate) fn set(key: &'static str, value: &str) -> Self {
        let original = std::env::var_os(key);
        unsafe { std::env::set_var(key, value) };
        Self { key, original }
    }
}

impl Drop for EnvOverride {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => unsafe { std::env::set_var(self.key, value) },
            None => unsafe { std::env::remove_var(self.key) },
        }
    }
}
