//! CYNIC Market — futarchic prediction market for sovereign agent coordination.
//!
//! This module implements the market loop: proposals → predictions →
//! resolution → scoring. It integrates with the existing CPMM
//! (`cynic-intel::futarchy`), E-Score (`cynic-intel::escore`), and
//! Crystal system (`cynic-intel::epistemic`).

pub mod types;
pub mod registry;
pub mod market_loop;
pub mod scoring;
pub mod collusion;

pub use types::{
    Actor, ActorId, ActorKind, ActorState, Proposal, ProposalId,
    Prediction, PredictionId, Resolution, ResolutionId, MarketEvent,
};
pub use registry::MarketRegistry;
pub use market_loop::MarketLoop;
pub use scoring::{ScoringEngine, PowConfig, PojConfig};
pub use collusion::{CollusionDetector, CollusionReport};
