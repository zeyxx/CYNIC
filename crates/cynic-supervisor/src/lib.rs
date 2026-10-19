//! Bounded OS effects for CYNIC-owned child processes.
//!
//! Pure decisions live in `cynic-protocol::supervision`. This crate owns only
//! the imperative shell: group-aware spawn and the SIGTERM/KILL stop ladder.
//! All processes run foreground; SysV double-fork is prohibited. Callers own
//! clocks, backoff math, and read-back proof.

pub mod parent;
pub mod spawn;
pub mod stop;
pub mod watch;

pub use parent::{
    DEFAULT_SUPERVISE_STATE_DIR, INFERENCE_ROLE_ID, RoleRunConfig, RoleRunOutcome, is_role_parented,
    run_role,
};
pub use spawn::spawn_backend;
pub use stop::graceful_stop;
pub use watch::{WatchConfig, WatchPollState, WatchRunOutcome, watch_once, watch_role};
