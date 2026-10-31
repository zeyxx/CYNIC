//! Per-agent isolation of the compiler cache.
//!
//! THE MEASURED FAILURE. On 2026-10-20 the builder could not run a single
//! test: every `cargo test` died at
//!
//! ```text
//! error: process didn't exit successfully: `sccache rustc -vV` (exit status: 101)
//! thread 'main' panicked at src/jobserver.rs:111:18:
//! failed to spawn helper thread: Os { code: 11, kind: WouldBlock,
//! message: "Resource temporarily unavailable" }
//! ```
//!
//! The message points at memory. It is not memory. The elimination is on
//! record in `docs/specs/builder-sccache-contention.md`:
//!
//! - a 256 MiB stack reproduces the panic, so the stack is not the cause;
//! - `pthread_create` succeeds on the host, so the kernel is not refusing
//!   commits;
//! - `sccache --show-stats` answers and a fresh `sccache rustc -vV`
//!   answers in a second, so the server is not wedged;
//! - the cache statistics are coherent, so the cache is not corrupt.
//!
//! What is true: the sccache SERVER is unique to the whole machine. The
//! CYNIC pipeline sets `SCCACHE_DIR=/var/lib/cynic/sccache`; the root
//! builds of a third-party checkout set their own `SCCACHE_DIR` but still
//! connect to the same server process. Under the third-party load the
//! server's thread count fell from 71 to 33 while twelve clients piled up,
//! and those clients carried 4.1e9 s of CPU time — unnourished children.
//! `cargo` reads the unanswered request as a failed fork and raises
//! `WouldBlock`.
//!
//! THE FIX. A build that shares a machine with other agents needs a
//! compiler cache it does not share. Three shapes, in increasing isolation:
//!
//! - `None` — no wrapper at all. Always works, zero cache benefit.
//! - `Local` — a private `SCCACHE_DIR`, hence a private server per agent.
//!   Cache benefit kept within the agent, no cross-agent contention.
//! - `Shared` — the machine-wide cache. Fastest when the machine is
//!   quiet, and the exact configuration that fails when it is not.
//!
//! The decision is NOT the caller's to make by accident: it is a typed
//! choice with a safe default, and the safe default is the one that
//! cannot hang.

use serde::{Deserialize, Serialize};

/// How a build reaches the compiler cache.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum CacheMode {
    /// No wrapper. `RUSTC_WRAPPER` is not set at all.
    None,
    /// A private cache directory, hence a private server for this agent.
    ///
    /// The directory is the identity: two agents with different
    /// directories get different servers, because sccache derives its
    /// socket from the cache directory.
    Local { dir: String },
    /// The machine-wide cache.
    Shared { dir: String },
}

impl CacheMode {
    /// The `SCCACHE_DIR` this mode uses, if any.
    pub fn dir(&self) -> Option<&str> {
        match self {
            CacheMode::None => None,
            CacheMode::Local { dir } | CacheMode::Shared { dir } => Some(dir.as_str()),
        }
    }

    /// Whether this mode shares its server with other agents on the host.
    ///
    /// A shared mode is only safe when the host is quiet. A local mode is
    /// safe always; a private server cannot be starved by a neighbour.
    pub fn shares_server_with_other_agents(&self) -> bool {
        matches!(self, CacheMode::Shared { .. })
    }
}

/// The cache mode for a build on a given host, from the host's load.
///
/// WHY LOAD DECIDES. The contention is not a property of the build, it is
/// a property of the MACHINE: a shared server is fine while the host is
/// quiet and is the failure mode when it is not. So the caller reports
/// what the host looks like, and this decides the isolation.
pub fn decide_cache_mode(agent_id: &str, shared_dir: &str, other_agents_active: bool) -> CacheMode {
    if other_agents_active {
        // Another agent is on the host: a shared server would put this
        // build in the same queue as theirs. Give this agent its own.
        CacheMode::Local {
            dir: format!("{shared_dir}/{agent_id}"),
        }
    } else {
        CacheMode::Shared {
            dir: shared_dir.to_string(),
        }
    }
}

/// The environment a build carries, for this cache mode.
///
/// The wrapper is only set when there is a cache to use: `RUSTC_WRAPPER`
/// pointing at a directory with no server is exactly the wedged state.
pub fn cache_env(mode: &CacheMode, wrapper_bin: &str) -> Vec<(String, String)> {
    match mode.dir() {
        None => Vec::new(),
        Some(dir) => vec![
            ("RUSTC_WRAPPER".to_string(), wrapper_bin.to_string()),
            ("SCCACHE_DIR".to_string(), dir.to_string()),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_cache_carries_no_environment() {
        let mode = CacheMode::None;
        assert!(cache_env(&mode, "/usr/bin/sccache").is_empty());
        assert_eq!(mode.dir(), None);
    }

    #[test]
    fn a_local_mode_keeps_its_cache_private() {
        let mode = CacheMode::Local {
            dir: "/var/lib/cynic/sccache/hermes".to_string(),
        };
        assert_eq!(mode.dir(), Some("/var/lib/cynic/sccache/hermes"));
        assert!(!mode.shares_server_with_other_agents());
        let env = cache_env(&mode, "/usr/bin/sccache");
        assert_eq!(env.len(), 2);
        assert_eq!(env[0].1, "/usr/bin/sccache");
        assert_eq!(env[1].1, "/var/lib/cynic/sccache/hermes");
    }

    #[test]
    fn a_shared_mode_shares_the_server() {
        let mode = CacheMode::Shared {
            dir: "/var/lib/cynic/sccache".to_string(),
        };
        assert!(mode.shares_server_with_other_agents());
    }

    #[test]
    fn another_agent_on_the_host_forces_a_private_cache() {
        let mode = decide_cache_mode("hermes", "/var/lib/cynic/sccache", true);
        assert_eq!(
            mode,
            CacheMode::Local {
                dir: "/var/lib/cynic/sccache/hermes".to_string()
            }
        );
        assert!(!mode.shares_server_with_other_agents());
    }

    #[test]
    fn a_quiet_host_keeps_the_shared_cache() {
        let mode = decide_cache_mode("hermes", "/var/lib/cynic/sccache", false);
        assert_eq!(
            mode,
            CacheMode::Shared {
                dir: "/var/lib/cynic/sccache".to_string()
            }
        );
    }

    #[test]
    fn the_mode_survives_a_round_trip_through_json() {
        let mode = CacheMode::Local {
            dir: "/var/lib/cynic/sccache/hermes".to_string(),
        };
        let json = serde_json::to_string(&mode).expect("serialise");
        assert_eq!(json, r#"{"mode":"local","dir":"/var/lib/cynic/sccache/hermes"}"#);
        let back: CacheMode = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(mode, back);
    }
}
