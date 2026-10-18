//! `cynic-traces`: canonical entities for agent session traces.
//!
//! Tranche 1 of the Rust rewrite (see `docs/specs/cynic-data-rust-rewrite.md`):
//! domain entities + session aggregate with machine-checked invariants.
//! Ports, adapters, and conservation proofs land in tranche 2 — no stubs here.
//!
//! Frozen capture adopted by this tranche:
//! `/var/lib/cynic/vault/corpus/captures/hermes_union.sqlite`
//! (manifest `hermes_union.manifest.json`: sha256
//! `4a2c4085ce1151aa7f8bad3103e80a64c1b6548a98423980242be9856a02a95b`,
//! 314194 messages / 2795 sessions, integrity re-verified 2026-10-18).

pub mod domain;
pub mod session;

pub use domain::{
    CorrectionKind, Edge, EdgeKind, EdgeProvenance, HumanSignal, ReasoningTrace, StateSnapshot,
    ToolCall, Turn,
};
pub use session::{ContractViolation, NormalizedSession, SessionStats};
