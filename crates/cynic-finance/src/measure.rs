//! Measure boundary: meter ports and their error type.
//!
//! A meter measures a consumption for a *subject* identified by `&str`.
//! The subject may be an agent or a person. Accounting (ledger, exchange,
//! CYD) lives in `types`/`ledger`/`exchange` and must not be used for human
//! subjects. This module only re-exports; no behaviour moves here.

pub use crate::ports::{
    ActionMeter, ComputeMeter, ContextMeter, EnergyMeter, GpuMeter, MeterError,
    NetworkMeter, StorageMeter, TimeMeter, TokenMeter,
};
