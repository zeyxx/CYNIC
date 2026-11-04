//! Task lifecycle sub-domain (DDD partition).
//!
//! Groups the task-related protocol modules under a single namespace while
//! preserving the top-level `task_admission` and `task_pipeline` module paths
//! for backward compatibility.

pub use crate::task_admission;
pub use crate::task_pipeline;
