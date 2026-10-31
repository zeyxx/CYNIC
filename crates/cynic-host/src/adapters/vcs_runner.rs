//! Host-level re-export of the VCS runner.
//!
//! `LocalVcsRunner` has been repatriated into `cynic-vcs`. This module keeps
//! a compatibility re-export for existing host consumers.

pub use cynic_vcs::LocalVcsRunner;
