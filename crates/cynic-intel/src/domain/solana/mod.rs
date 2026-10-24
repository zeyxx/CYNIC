//! Solana Pay domain vocabulary, program constants, and QR rendering.

pub mod constants;
pub mod ops;
pub mod qr;

pub use constants::{
    AMM_PROGRAMS, BURN_ADDRESSES, INFRA_PROGRAMS, LOCKER_PROGRAMS, ORACLE_PROGRAMS, SYSTEM_PROGRAM,
};
pub use cynic_protocol::allowance::AllowanceGrant;
pub use ops::{
    AllowanceGrantPaymentExt, SolanaPayRequest, SolanaTakeoverAttestation,
    SopSanitizationInput, SopSanitizationResult, SOLANA_TAKEOVER_ATTESTATION_SCHEMA,
};
pub use qr::render_qr_ascii;
