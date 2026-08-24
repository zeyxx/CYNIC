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
//! - [`scan`] windows events purely from timestamps; the JSONL adapter that
//!   feeds it (real corpus timestamps unverified: probe blocked pending
//!   operator consent) and any span-ratio correction for unequal declared
//!   windows remain adapter work.
//! - No eviction authorization lives at this layer; an apply surface must
//!   wrap distillation behind explicit consent plus read-back proof.

pub mod model;
pub mod poh_anchor;
pub mod policy;
pub mod report;
pub mod scan;
pub mod types;

pub use model::{
    drift, seal_model, ClassTransition, ModelDrift, PatternModel, PatternState, MODEL_SCHEMA,
};
pub use policy::{classify, classify_all, SCHEMA};
pub use report::{build_report, ClassTally, PatternGcReport, REPORT_SCHEMA};
pub use scan::{window_events, WindowOutcome};
pub use types::{
    PatternClass, PatternGcAssessment, PatternObservation, PatternWindows, RetentionPolicy,
};
