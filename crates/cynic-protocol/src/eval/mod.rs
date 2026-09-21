//! Evaluation, distillation hygiene, dog scoring and epistemic wisdom contracts.
//!
//! Sub-domain grouping:
//! - `hygiene`: train/test contamination detection and overlap analysis (RFC-003).
//! - `dog`: stimulus, axiom scores, verdicts, and quorum constants.
//! - `dog_health`: per-dog stats, scoring failure kinds, and evaluation outcomes.
//! - `wisdom`: domain signal contracts and falsifiable curated knowledge.

pub mod dog;
pub mod dog_health;
pub mod hygiene;
pub mod wisdom;

pub use dog::*;
pub use dog_health::*;
pub use hygiene::*;
pub use wisdom::*;
