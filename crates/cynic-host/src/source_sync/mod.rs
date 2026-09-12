//! Source sync — re-exported from `cynic-vcs` per Foundation §481 (SoC).

pub mod apply;
pub mod observe;

pub use cynic_vcs::sync as types;
pub use cynic_vcs::sync::*;

pub use apply::*;
pub use observe::*;
