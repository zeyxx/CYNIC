//! Node-facing projection of the Rust-native bounded LXC bridge.
//!
//! Policy and execution stay owned by `cynic-host`; this provider adapter does
//! not invent a second command grammar.

pub use cynic_lxc::{
    LxcBridgeAuthority, LxcBridgeDecision, LxcBridgeRequest, LxcBridgeResponse, admit, execute,
};
