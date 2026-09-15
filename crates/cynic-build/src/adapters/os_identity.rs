//! [`OsIdentity`]: the production [`IdentityPort`] adapter.
//!
//! Reads process identity from the OS directly: no `id -u` subprocess,
//! no shell, no ambient global reads in the domain.

use crate::ports::IdentityPort;

/// OS process-identity adapter. Carries no state.
#[derive(Debug, Clone, Copy, Default)]
pub struct OsIdentity;

impl IdentityPort for OsIdentity {
    fn uid(&self) -> u32 {
        // SAFETY: getuid is async-signal-safe and has no failure mode.
        unsafe { libc::getuid() }
    }

    fn pid(&self) -> u32 {
        std::process::id()
    }
}
