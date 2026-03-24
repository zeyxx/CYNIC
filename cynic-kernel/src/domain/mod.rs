pub mod dog;
pub mod ccm;
pub mod storage;
pub mod coord;
pub mod events;
pub mod inference;
pub mod chat;
pub mod embedding;
pub mod verdict_cache;
pub mod usage;
// temporal.rs: dormant — math is sound (geometric mean, outlier detection),
// wiring was fake (dog_scores[i%7] relabeling). Burned 2026-03-24.
// Uncomment when real per-perspective Dog prompting is implemented.
// pub mod temporal;
pub mod summarization;
pub mod metrics;
pub mod health_gate;
