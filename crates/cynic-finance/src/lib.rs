//! CYNIC Finance — multi-resource accounting, ledgers, and exchange rates.
//!
//! Pure domain crate: owns canonical resource types, ledger invariants,
//! and exchange rate structures. No I/O, no process spawning, no network.
//!
//! Architecture:
//! - `types` — ResourceUnit, ResourceTransaction, ResourceLedger, ExchangeRate
//! - `ports` — Meter traits (Compute, Gpu, Storage, Network, Energy, Token, Context, Time, Action)
//! - `ledger` — append-only ledger with double-entry invariants
//! - `exchange` — measured exchange rate table

pub mod types;
pub mod ports;
pub mod ledger;
pub mod exchange;

pub use types::{
    ResourceUnit, ResourceTransaction, ResourceLedger, TransactionDirection,
    ResourceMetadata, ResourceType, LEDGER_TRANSACTION_SCHEMA,
};
pub use ports::{
    ComputeMeter, GpuMeter, StorageMeter, NetworkMeter, EnergyMeter,
    TokenMeter, ContextMeter, TimeMeter, ActionMeter, MeterError,
};
pub use ledger::LedgerEngine;
pub use exchange::{ExchangeRate, ExchangeRateTable, EXCHANGE_RATE_TABLE_SCHEMA};
pub use types::EXCHANGE_RATE_SCHEMA;
