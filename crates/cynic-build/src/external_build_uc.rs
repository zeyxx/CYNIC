//! Backward-compatible re-exports for external build usecases.
//! Canonical implementations now reside directly in `cynic_build::external_build`.

pub use crate::external_build::{run_external_build, run_external_command};
