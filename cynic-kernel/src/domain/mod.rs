pub mod ccm;
pub mod chat;
pub mod compliance;
pub mod coord;
pub mod dog;
pub mod embedding;
pub mod events;
pub mod inference;
pub mod storage;
pub mod usage;
pub mod verdict_cache;
// pub mod temporal; // DORMANT: math sound but wiring was fake (dog_scores[i%7] relabeling). Burned 2026-03-24. Reactivate with real per-perspective Dog prompting.
pub mod health_gate;
pub mod metrics;
pub mod probe;
pub mod sanitize;
pub mod source;
pub mod summarization;
