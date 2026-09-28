//! Temporal rate classification of recurring event keys.
//!
//! WHAT THIS IS: given opaque bucket keys supplied by an upstream adapter and
//! the unix timestamps of their events, split each key's events into a cold and a
//! hot window, convert both counts to per-unit rates, and apply a four-way
//! threshold decision — `Recurring`, `Emerging`, `Fading`, or explicit `Unknown`.
//! Then seal the result into a content-addressed report.
//!
//! WHAT THIS IS NOT, stated because the previous name claimed otherwise:
//! - It collects nothing. There is no eviction, no deletion, no removal, and no
//!   filesystem or process access anywhere in this module; its only `std` use is
//!   `std::cmp::Ordering`. A previous `eviction_authorized: bool` was hard-coded
//!   `false` at construction and read by no consumer, so it carried no
//!   information and has been removed rather than renamed.
//! - It matches nothing. Keys arrive pre-extracted and pre-grouped. The regex
//!   table that derives them lives one layer down in
//!   `cynic_intel::autoresearch::bucket_signatures` and yields names of the form
//!   `error_buckets.<domain>`; this module only counts them and rates the change.
//! - It is not generational. There are two disjoint windows, not generations.
//!
//! A note on `Fading`: a collapsing rate is evidence that the underlying issue
//! was *resolved or consolidated*, not that the data should be reclaimed. The
//! classifier reports the observation; it never authorises a mutation.
//!
//! Layering (mirrors `cynic_inference::domain::model_retention`):
//! - [`types`] — pure shapes: windowed observations, thresholds, four-state
//!   assessments with explicit `Unknown`.
//! - [`policy`] — pure reconciliation: one canonical [`classify`] decision.
//! - [`scan`] — pure windowing: counts and spans from timestamps only.
//! - [`report`] — pure tally plus a SHA-256 content seal.
//!
//! Residual unknowns (explicit, not back-filled):
//! - [`scan`] windows events purely from timestamps; the JSONL adapter that
//!   feeds it (real corpus timestamps unverified: probe blocked pending
//!   operator consent) and any span-ratio correction for unequal declared
//!   windows remain adapter work.
//! - No apply surface consumes this classification yet. A consumer must wrap any
//!   downstream mutation behind explicit consent plus read-back proof.

pub mod policy;
pub mod report;
pub mod scan;
pub mod types;

pub use policy::{SCHEMA, classify, classify_all};
pub use report::{ClassTally, PatternGcReport, REPORT_SCHEMA, build_report};
pub use scan::{WindowOutcome, window_events};
pub use types::{
    PatternClass, PatternGcAssessment, PatternObservation, PatternWindows, RetentionPolicy,
};
