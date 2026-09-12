//! Source sync — re-exported from `cynic-vcs` per Foundation §481 (SoC).

pub mod apply;
pub mod observe;

pub use cynic_vcs::sync::{
    SourceLayer, SourceState, SyncAction, SyncDecision, SyncObservation, SyncPlan, SyncStep, plan,
    reconcile,
};

pub use apply::*;
pub use observe::*;
