//! Bounded OS effects for CYNIC-owned child processes.
//!
//! Pure decisions live in `cynic-protocol::supervision`. This crate owns only
//! the imperative shell: group-aware spawn and the SIGTERM/KILL stop ladder.
//! All processes run foreground; SysV double-fork is prohibited. Callers own
//! clocks, backoff math, and read-back proof.

pub mod spawn;
pub mod stop;

pub use spawn::spawn_backend;
pub use stop::graceful_stop;
