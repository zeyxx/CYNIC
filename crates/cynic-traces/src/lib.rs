//! `cynic-traces`: canonical entities for agent session traces.
//!
//! Tranche 3 owns the Hermes reference vertical: NormalizerPort over the
//! frozen capture, machine-transcribed conservation contract, strict port
//! of the Python normalizer. Exit gate: Rust output == Python output on
//! the gel (tests/hermes_parity.rs).
//!
//! Frozen capture adopted by tranche 1:
//! `/var/lib/cynic/vault/corpus/captures/hermes_union.sqlite`
//! (manifest `hermes_union.manifest.json`: sha256
//! `4a2c4085ce1151aa7f8bad3103e80a64c1b6548a98423980242be9856a02a95b`,
//! 314194 messages / 2795 sessions, integrity re-verified 2026-10-18).

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
