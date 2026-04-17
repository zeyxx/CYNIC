//! cynic-askesis — 3rd CYNIC pillar: human augmentation layer.
#![cfg_attr(test, allow(dead_code, clippy::unwrap_used, clippy::expect_used))]

pub mod anchor;
pub mod audit;
pub mod error;
pub mod log;
pub mod reflection;

pub use error::{AskesisError, Result};
pub use reflection::{Reflection, Verdict};
