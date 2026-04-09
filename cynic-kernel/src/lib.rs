// Lints enforced via [workspace.lints] in root Cargo.toml (Layer 0 enforcement).
// Lints: workspace-level deny(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)
// Test exemptions: allow unwrap/expect/dead_code in test modules.
#![cfg_attr(test, allow(dead_code, clippy::unwrap_used, clippy::expect_used))]

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

pub mod api;
pub mod backends;
pub mod dogs;
pub mod domain;
pub mod infra;
pub mod introspection;
pub mod judge;
pub mod organ;
pub mod pipeline;
pub mod probe;
pub mod sources;
pub mod storage;
