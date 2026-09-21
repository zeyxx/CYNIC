//! Solana Web3 constants, payment requests, and SOP operations.
//!
//! Sub-domain grouping:
//! - `constants`: AMM program IDs, burn addresses, locker programs, and oracle program IDs.
//! - `ops`: Solana Pay requests, spend allowances, and SOP sanitization contracts.

pub mod constants;
pub mod ops;

pub use constants::*;
pub use ops::*;
