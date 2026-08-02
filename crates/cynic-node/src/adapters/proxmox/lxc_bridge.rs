//! Node-facing projection of the Rust-native bounded LXC bridge.
//!
//! Policy and execution stay owned by `cynic-host`; this provider adapter does
//! not invent a second command grammar.

pub use cynic_host::lxc_exec_bridge::{
    LxcBridgeAuthority, LxcBridgeDecision, LxcBridgeRequest, LxcBridgeResponse, admit, execute,
};
