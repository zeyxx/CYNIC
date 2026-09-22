//! CYNIC Market — futarchic prediction market for sovereign agent coordination.
//!
//! This module implements the market loop: proposals → predictions →
//! resolution → scoring. It integrates with the existing CPMM
//! (`cynic-intel::futarchy`), E-Score (`cynic-intel::escore`), and
//! Crystal system (`cynic-intel::epistemic`).

pub mod collusion;
pub mod market_loop;
pub mod registry;
pub mod scoring;
pub mod types;

pub use collusion::{CollusionDetector, CollusionReport};
pub use market_loop::MarketLoop;
pub use registry::MarketRegistry;
pub use scoring::{PojConfig, PowConfig, ScoringEngine};
pub use types::{
    Actor, ActorId, ActorKind, ActorState, MarketEvent, Prediction, PredictionId, Proposal,
    ProposalId, Resolution, ResolutionId,
};
