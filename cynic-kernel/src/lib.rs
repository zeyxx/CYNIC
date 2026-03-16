use std::sync::atomic::AtomicBool;

/// Global MCP mode flag — when true, all logging goes to stderr (stdout reserved for JSON-RPC).
pub static MCP_MODE: AtomicBool = AtomicBool::new(false);

/// Log macro that routes to stderr in MCP mode, stdout otherwise.
/// In MCP mode, stdout is reserved for JSON-RPC 2.0 protocol — any non-JSON corrupts it.
#[macro_export]
macro_rules! klog {
    ($($arg:tt)*) => {
        if $crate::MCP_MODE.load(std::sync::atomic::Ordering::Relaxed) {
            eprintln!($($arg)*);
        } else {
            println!($($arg)*);
        }
    };
}

pub mod probe;
pub mod supervisor;
pub mod hal;
pub mod pulse;
pub mod storage_http;
pub mod storage_port;
pub mod backend;
pub mod backend_llamacpp;
pub mod backend_openai;
pub mod router;
pub mod dog;
pub mod inference_dog;
pub mod deterministic_dog;
pub mod judge;
pub mod rest;
pub mod chat_port;
pub mod config;
pub mod ccm;
pub mod temporal;
pub mod circuit_breaker;
pub mod mcp;
pub mod usage;

pub mod cynic_v2 {
    tonic::include_proto!("cynic.v2");
}
