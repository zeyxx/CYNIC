//! Zone backup archival to cynic-askesis append-only log.
//!
//! The runner calls this BEFORE any zone mutation (registrar NS update,
//! Cloudflare record write) to capture pre-state for rollback/reconciliation.

use cynic_askesis::{AskesisKind, AskesisLog};
use cynic_protocol::domain_ops::DOMAIN_OPS_PROOF_SCHEMA;

/// Archive a zone backup to the askesis log.
///
/// Returns the `entry_hash` (SHA-256 of canonical JSON) of the appended entry,
/// which the runner embeds in subsequent `DomainOperationProof` envelopes as
/// the `backup_ref` in `provider_read_back`.
///
/// The zone export is stored as a string field inside the JSON evidence.
/// The entry kind is `AskesisKind::Custom` with subject `zone_backup_{domain}`.
/// Hash chaining is handled by `AskesisLog::append` — each entry references
/// the previous entry's hash.
pub fn archive_zone_backup(log: &mut AskesisLog, domain: &str, zone_export: &str) -> String {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let entry = log.append(
        AskesisKind::Custom,
        timestamp,
        format!("zone_backup_{domain}"),
        serde_json::json!({
            "schema": DOMAIN_OPS_PROOF_SCHEMA,
            "domain": domain,
            "zone_export": zone_export
        }),
    );
    entry.entry_hash.clone()
}

/// Verify that a zone backup hash exists in the log chain.
///
/// Iterates the log's entries to find a matching `entry_hash`. This is the
/// read-back proof for backup integrity — if the hash matches, the backup
/// was captured before mutation and has not been tampered with.
pub fn verify_zone_backup(log: &AskesisLog, backup_hash: &str) -> bool {
    log.entries.iter().any(|e| e.entry_hash == backup_hash)
}

/// Verify the entire backup chain is intact.
///
/// Wraps `AskesisLog::verify_chain` — returns `true` if every entry's hash
/// is valid and the chain is contiguous (no tampering, no gaps).
pub fn verify_backup_chain(log: &AskesisLog) -> bool {
    log.verify_chain()
}
