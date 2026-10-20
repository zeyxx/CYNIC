//! Port for append-only writers (implemented by adapters in other crates).

use crate::entry::EvidenceKind;
use crate::log::EvidenceLog;

/// Port for append-only writers (implemented by adapters in other crates).
pub trait EvidenceWriter {
    /// Append an entry. Returns the entry hash on success.
    fn append(
        &mut self,
        kind: EvidenceKind,
        observed_at_unix_s: u64,
        subject: impl Into<String>,
        evidence: serde_json::Value,
    ) -> Result<String, String>;

    /// Read the entire log.
    fn read_log(&self) -> Result<EvidenceLog, String>;

    /// Verify the log chain.
    fn verify(&self) -> Result<bool, String>;
}
