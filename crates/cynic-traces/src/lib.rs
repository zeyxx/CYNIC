//! `cynic-traces`: canonical entities for agent session traces.
//!
//! Tranche 2 owns the kernel (contracts, errors, stable ids), the
//! conservation contract (FIELD_FATE closure, α1–α4 checkability), and the
//! ports. Adapters and proof runners land in tranche 3 — no stubs here.
//!
//! Frozen capture adopted by tranche 1:
//! `/var/lib/cynic/vault/corpus/captures/hermes_union.sqlite`
//! (manifest `hermes_union.manifest.json`: sha256
//! `4a2c4085ce1151aa7f8bad3103e80a64c1b6548a98423980242be9856a02a95b`,
//! 314194 messages / 2795 sessions, integrity re-verified 2026-10-18).

pub mod conservation;
pub mod domain;
pub mod kernel;
pub mod ports;
pub mod session;

pub use conservation::{ConceptResolution, ConservationContract, Fate};
pub use domain::{
    CorrectionKind, Edge, EdgeKind, EdgeProvenance, HumanSignal, ReasoningTrace, StateSnapshot,
    ToolCall, Turn,
};
pub use kernel::contract::{Authority, Layer, LayerContract};
pub use ports::{CanonicalStorePort, NormalizerPort};
pub use session::{ContractViolation, NormalizedSession, SessionStats};
