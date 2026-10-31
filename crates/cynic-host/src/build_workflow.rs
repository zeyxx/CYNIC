//! Build workflow runtime — re-exported from `cynic_build`.
//!
//! The implementation (config, result, runner) now lives in
//! `cynic_build::build_workflow`. This module keeps the historical
//! `cynic_host::build_workflow` path working for callers.

pub use cynic_build::build_workflow::{BuildWorkflowConfig, BuildWorkflowResult, run_build_workflow};