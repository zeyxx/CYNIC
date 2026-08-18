//! cynic-askesis — 3rd CYNIC pillar: human augmentation layer.
#![cfg_attr(test, allow(dead_code, clippy::unwrap_used, clippy::expect_used))]

pub mod agent_loop;
pub mod anchor;
pub mod audit;
pub mod domains;
pub mod error;
pub mod eval;
pub mod kyon_scenarios;
pub mod log;
pub mod memory;
pub mod reflection;
pub mod trace_engine;

pub use error::{AskesisError, Result};
pub use memory::{DEFAULT_HUMAN_KERNEL_LOG, append_human_log_entry, resolve_human_log_path};
pub use reflection::{Reflection, Verdict};
pub use trace_engine::{TraceEngine, TrainingTrace, TRAINING_TRACE_SCHEMA};
