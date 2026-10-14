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

pub mod exchange;
pub mod ledger;
pub mod ports;
pub mod rate_calculator;
pub mod types;

pub use exchange::{EXCHANGE_RATE_TABLE_SCHEMA, ExchangeRate, ExchangeRateTable};
pub use ledger::LedgerEngine;
pub use ports::{
    ActionMeter, ComputeMeter, ContextMeter, EnergyMeter, GpuMeter, MeterError, NetworkMeter,
    StorageMeter, TimeMeter, TokenMeter,
};
pub use rate_calculator::{RateSample, compute_exchange_rates};
pub use types::EXCHANGE_RATE_SCHEMA;
pub use types::{
    LEDGER_TRANSACTION_SCHEMA, ResourceLedger, ResourceMetadata, ResourceTransaction, ResourceType,
    ResourceUnit, TransactionDirection,
};
