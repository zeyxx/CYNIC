//! Port for append-only writers (implemented by adapters in other crates).

use crate::entry::AskesisKind;
use crate::log::AskesisLog;

/// Port for append-only writers (implemented by adapters in other crates).
pub trait AskesisWriter {
    /// Append an entry. Returns the entry hash on success.
    fn append(
        &mut self,
        kind: AskesisKind,
        observed_at_unix_s: u64,
        subject: impl Into<String>,
        evidence: serde_json::Value,
    ) -> Result<String, String>;

    /// Read the entire log.
    fn read_log(&self) -> Result<AskesisLog, String>;

    /// Verify the log chain.
    fn verify(&self) -> Result<bool, String>;
}
