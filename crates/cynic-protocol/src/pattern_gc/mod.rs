//! Generational pattern garbage collection contract.
//!
//! CYNIC's GC collects *patterns*, not bytes: raw agent-session traces are a
//! renewable observation substrate, while distilled artifacts (skills,
//! evidence envelopes) and chained proof-of-history entries are tenured and
//! never evicted by this layer.
//!
//! Layering (mirrors `cynic-inference::domain::model_retention`):
//! - [`types`] — pure shapes: windowed observations, retention policy,
//!   tri-state assessments (`Recurring` / `Emerging` / `Fading`, with
//!   explicit `Unknown`).
//! - [`policy`] — pure reconciliation: one canonical `classify` decision.
//!
//! Residual unknowns (explicit, not back-filled):
//! - Window spans are adapter-owned; no span-ratio correction exists yet, so
//!   `proportional_baseline` currently assumes comparable spans.
//! - No eviction authorization lives at this layer; an apply surface must
//!   wrap distillation behind explicit consent plus read-back proof.

pub mod policy;
pub mod types;

pub use policy::{classify, classify_all, SCHEMA};
pub use types::{
    PatternClass, PatternGcAssessment, PatternObservation, PatternWindows, RetentionPolicy,
};
