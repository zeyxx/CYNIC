// Compiler-enforced code hygiene — the compiler IS the impact checker.
// dead_code: every public symbol must have a caller (Rule #9)
// unused_imports: no stale imports accumulate
#![deny(dead_code, unused_imports)]
// Allow dead_code in test modules (test helpers, fixtures)
#![cfg_attr(test, allow(dead_code))]

use std::sync::atomic::AtomicBool;

/// Global MCP mode flag — when true, all logging goes to stderr (stdout reserved for JSON-RPC).
pub static MCP_MODE: AtomicBool = AtomicBool::new(false);

/// Log macro that routes to stderr in MCP mode, stdout otherwise.
/// In MCP mode, stdout is reserved for JSON-RPC 2.0 protocol — any non-JSON corrupts it.
///
/// Uses locked handles + BufWriter to batch syscalls — one write(2) per log line
/// instead of fragmenting across format arguments.
#[macro_export]
macro_rules! klog {
    ($($arg:tt)*) => {{
        use std::io::Write;
        if $crate::MCP_MODE.load(std::sync::atomic::Ordering::Relaxed) {
            let stderr = std::io::stderr();
            let mut buf = std::io::BufWriter::new(stderr.lock());
            let _ = writeln!(buf, $($arg)*);
        } else {
            let stdout = std::io::stdout();
            let mut buf = std::io::BufWriter::new(stdout.lock());
            let _ = writeln!(buf, $($arg)*);
        }
    }};
}

pub mod domain;
pub mod dogs;
pub mod backends;
pub mod storage;
pub mod api;
pub mod infra;
pub mod probe;
pub mod judge;
pub mod pipeline;
