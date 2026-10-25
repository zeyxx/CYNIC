//! Pure domain logic for personal anchoring.
//!
//! This module is 100% pure: no I/O, no external process execution,
//! no HTTP client, no async spawning. All decision logic (event
//! classification, milestone computation, outcome evaluation) lives here
//! so it can be unit-tested deterministically without network access.
//!
//! Hexagonal layout:
//! - `domain/` — pure domain logic (types, classification, milestones).
//! - `port.rs` — driven ports (trait definitions for event store, projection, sync).
//! - `adapters/` — storage adapters (Phase 2).
//!
//! Authority: docs/specs/personal-anchoring-federation.md

pub mod domain;
pub mod port;
pub mod adapters;

pub use domain::{
    AnchorDomain, AnchorEvent, AnchorOutcome, SobrietyMilestone, SobrietySubstance,
};
pub use port::{AnchorEventStore, AnchorProjection, AnchorSyncPort};