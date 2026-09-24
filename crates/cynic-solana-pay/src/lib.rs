//! cynic-solana-pay: Solana Pay vocabulary and SOP sanitization contracts.
//!
//! Extracted from `cynic-protocol::solana` (Vague A). Owns payment request
//! schemas, SOP sanitization predicates, and Solana program constants.
//! Depends on the protocol residue for `runtime_takeover` records (Vague B
//! will repoint that dependency to `cynic-runtime-takeover`).

pub mod constants;
pub mod ops;

pub use cynic_protocol::allowance::AllowanceGrant;

pub use constants::{
    AMM_PROGRAMS, BURN_ADDRESSES, INFRA_PROGRAMS, LOCKER_PROGRAMS, ORACLE_PROGRAMS, SYSTEM_PROGRAM,
};
pub use ops::{
    AllowanceGrantPaymentExt, SOLANA_TAKEOVER_ATTESTATION_SCHEMA, SolanaPayRequest,
    SolanaTakeoverAttestation, SopSanitizationInput, SopSanitizationResult,
};
