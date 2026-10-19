//! `cynic-traces`: canonical entities for agent session traces.
//!
//! Tranche 4 owns the canonical store (SQLite, transactional), the
//! application use-cases, and the full-load conservation proof that
//! replaces `apps/cynic-data/verify_conservation.py` (retired).
//!
//! ## Frontière Python / Rust (état T4)
//!
//! Rust owns: canonical entities, kernel, conservation contract, ports,
//! the Hermes reference vertical (normalizer + FIELD_FATE contract),
//! the SQLite canonical store, the normalize use-cases.
//! Python still owns: capture, the 10 other harness normalizers, Model,
//! Projection, guards (`check_nonvacuity`). Each retires behind its own
//! HITL gate (carnet T5–T6), never in bulk.
//!
//! Frozen capture adopted by tranche 1:
//! `/var/lib/cynic/vault/corpus/captures/hermes_union.sqlite`
//! (manifest `hermes_union.manifest.json`: sha256
//! `4a2c4085ce1151aa7f8bad3103e80a64c1b6548a98423980242be9856a02a95b`,
//! 314194 messages / 2795 sessions, integrity re-verified 2026-10-18).
//! Test paths default to this host's absolute locations (admitted debt M3)
//! and are overridable via `CYNIC_TRACES_SOURCE`, `CYNIC_TRACES_SILVER`,
//! `CYNIC_PRODUCER_SRC`.

pub mod adapters;
pub mod conservation;
pub mod domain;
pub mod kernel;
pub mod ports;
pub mod session;
pub mod usecases;

pub use adapters::hermes::HermesNormalizer;
pub use adapters::store_sqlite::{SqliteCanonicalStore, StoreError};

pub use conservation::{ConceptResolution, ConservationContract, Fate};
pub use domain::{
    CorrectionKind, Edge, EdgeKind, EdgeProvenance, HumanSignal, ReasoningTrace, StateSnapshot,
    ToolCall, Turn,
};
pub use kernel::contract::{Authority, Layer, LayerContract};
pub use ports::{CanonicalStorePort, NormalizerPort};
pub use session::{ContractViolation, NormalizedSession, SessionStats};
