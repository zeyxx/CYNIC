//! Network domain facade — Re-exports cynic-network per Foundation §481 (SoC).
//!
//! NEGATIVE BOUNDARY: this module re-exports domain and port types only. It does
//! not re-export adapters. A `pub use crate::adapters::...` here would make the
//! domain depend on an implementation, which is an inversion: the domain would
//! know which HTTP client talks to the network, when its whole purpose is to not
//! know. One such line existed and had zero consumers; it was removed. Adapters
//! are imported by their real path where they are used.

pub use cynic_network::*;
