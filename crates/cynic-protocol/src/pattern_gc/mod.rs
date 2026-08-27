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

pub mod ingest;
pub mod model;
pub mod poh_anchor;
pub mod policy;
pub mod report;
pub mod scan;
pub mod types;

pub use ingest::{
    CoverageBlocker, CoverageVerdict, INGEST_SCHEMA, IngestPolicy, SourceCoverage, SourceCursor,
    SourceFreshness, WatermarkPolicy, coverage_guard, demote_on_coverage_gap, freshness,
};

pub use model::{
    ClassTransition, MODEL_SCHEMA, ModelDrift, PatternModel, PatternState, drift, seal_model,
};
pub use policy::{SCHEMA, classify, classify_all};
pub use report::{ClassTally, PatternGcReport, REPORT_SCHEMA, build_report};
pub use scan::{WindowOutcome, window_events};
pub use types::{
    PatternClass, PatternGcAssessment, PatternObservation, PatternWindows, RetentionPolicy,
};
