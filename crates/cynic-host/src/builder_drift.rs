//! Builder drift detection and repair.
//!
//! Detects ownership drift in `.git/refs/remotes/forgejo/` and repairs
//! refs to be owned by the `cynic` principal.

pub mod detect;
pub mod repair;
