pub mod aggregator;
pub mod contribution;
pub mod decay;
pub mod merkle;
pub mod metrics;
pub mod types;

// Re-export de la primitive Epoch depuis cynic-protocol
// pour unifier le temps logique dans le workspace.
pub use cynic_protocol::Epoch;
