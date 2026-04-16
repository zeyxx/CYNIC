//! CCM — Cognitive Crystallization Mechanism.
//! Ephemeral verdicts → persistent wisdom. Domain-pure logic.
//!
//! 4D crystal model: quality (Q-score mean), certainty (Welford variance + volume),
//! polarity (dominant verdict kind), time (decay relevance).
//! Crystallization gate: certainty >= phi-inverse (not quality). This allows negative truths
//! (BARK/GROWL) to crystallize when the system is certain, not just when content is "good."

mod crystal;
mod engine;
mod intake;

pub use crystal::*;
pub use engine::*;
pub use intake::*;
