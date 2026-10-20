//! Mined log signature types. Evidence references, never raw dumps.

use serde::{Deserialize, Serialize};

/// One recurrent log signature: normalized pattern + counts + one sample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogSignature {
    /// Stable source (`kern`, `journal`, `cynic_trace`).
    pub source: String,
    /// Normalized signature (`oom_kill`, `gpu_page_fault`, `journal_203_exec`).
    pub signature: String,
    /// Occurrence count in the mined window.
    pub count: u64,
    /// One representative raw line (truncated to 512 chars by the probe).
    pub sample: String,
}

/// Mined report for one log bundle: signatures sorted by count desc.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LogMineReport {
    /// Schema marker for typed argv surfaces.
    pub schema: String,
    /// Total lines scanned.
    pub lines_scanned: u64,
    /// Signatures found, count-desc.
    pub signatures: Vec<LogSignature>,
}
