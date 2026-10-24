//! `cynic-traces`: canonical entities for agent session traces.
//!
//! Tranche 4 owns the canonical store (SQLite, transactional), the
//! application use-cases, and the full-load conservation proof that
//! replaced the retired `verify_conservation.py` (the Rust proof now lives
//! in `crates/cynic-traces/tests/conservation_proof.rs`).
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
//! Frozen capture adopted by tranche 1, re-frozen 2026-10-19:
//! `/var/lib/cynic/vault/corpus/captures/hermes_union_20261019.sqlite`
//! (manifest `hermes_union_20261019.manifest.json`: sha256
//! `51d31dbb7e040d0ca9af7b187a02012c3b412b3ef25b091adc132ee54db4d231`,
//! 418516 messages / 3149 sessions, integrity re-verified 2026-10-19).
//! Test paths default to this host's absolute locations (admitted debt M3)
//! and are overridable via `CYNIC_TRACES_SOURCE`, `CYNIC_TRACES_SILVER`,
//! `CYNIC_PRODUCER_SRC`.

pub mod adapters;
pub mod conservation;
pub mod domain;
pub mod kernel;
pub mod log_mine;
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
pub use log_mine::*;
pub use ports::{CanonicalStorePort, NormalizerPort};
pub use session::{ContractViolation, NormalizedSession, SessionStats};
