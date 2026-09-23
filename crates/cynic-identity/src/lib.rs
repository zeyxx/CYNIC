#![cfg_attr(not(feature = "std"), no_std)]

//! CYNIC Identity — cryptographic node identity.
//!
//! Single source of truth for ed25519 keypair management and node_id derivation.
//! All CYNIC crates consume this — no local re-implementations.
//!
//! ## `no_std` compatibility
//!
//! With `default-features = false` (no `std` feature):
//! - `NodeId` and verification are fully available (Solana/WASM/sandbox safe).
//! - `NodeIdentity::generate()` is not available (requires OS entropy).
//!
//! Pattern mirrors `solana-program`: `Pubkey` (no_std) vs `Keypair` (std).

#[cfg(not(feature = "std"))]
extern crate alloc;

pub mod error;
pub mod types;

pub use error::IdentityError;
pub use types::{NodeId, NodeIdentity, SigningMaterial};
