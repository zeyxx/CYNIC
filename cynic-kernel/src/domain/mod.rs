pub mod ccm;
pub mod chat;
pub mod coord;
pub mod dog;
pub mod embedding;
pub mod events;
pub mod inference;
pub mod storage;
pub mod usage;
pub mod verdict_cache;
// temporal.rs: dormant — math is sound (geometric mean, outlier detection),
// wiring was fake (dog_scores[i%7] relabeling). Burned 2026-03-24.
// Uncomment when real per-perspective Dog prompting is implemented.
// pub mod temporal;
pub mod health_gate;
pub mod metrics;
pub mod sanitize;
pub mod summarization;
