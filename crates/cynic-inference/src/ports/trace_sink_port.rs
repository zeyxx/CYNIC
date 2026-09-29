//! Best-effort sink for per-turn inference traces.
//!
//! Recording must never fail a chat turn: implementations swallow I/O errors
//! (a counter or log line at most) and return nothing.

use crate::domain::chat_trace::ChatTrace;

/// Portable boundary for trace persistence. Fire-and-forget by contract.
pub trait TraceSinkPort: Send + Sync {
    fn record(&self, trace: &ChatTrace);
}
