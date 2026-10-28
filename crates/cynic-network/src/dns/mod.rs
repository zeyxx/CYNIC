//! DNS domain for cynic-network (Hexagonal / DDD / FCIS).
//!
//! Aggregate root / public surface.

pub mod activation;
pub mod corefile;
pub mod port;
pub mod types;

pub use corefile::*;
pub use port::*;
pub use types::*;