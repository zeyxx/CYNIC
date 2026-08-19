//! Source sync — observe → reconcile → apply across 3 layers.
//!
//! Cortex (worktree) → Forgejo (sovereign SSOT) → GitHub (public relay).

pub mod apply;
pub mod observe;
pub mod reconcile;
pub mod types;

pub use apply::execute_plan;
pub use observe::{observe_all_layers, observe_layer};
pub use reconcile::{plan, reconcile};
pub use types::*;
