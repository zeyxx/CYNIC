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

pub mod domain;
pub mod dogs;
pub mod backends;
pub mod storage;
pub mod api;
#[cfg(feature = "grpc")]
pub mod grpc;
pub mod infra;
pub mod probe;
pub mod judge;

#[cfg(feature = "grpc")]
pub mod cynic_v2 {
    tonic::include_proto!("cynic.v2");
}
