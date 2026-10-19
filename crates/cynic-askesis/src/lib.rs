#![forbid(unsafe_code)]

//! Cynic Askesis — append-only proof and history log.
//!
//! Domain pure: no I/O, no chrono, no FS. Types, hashes, verification.
//! Adapters live in other crates (`cynic-askesis-fs`, kernel, ...).
//!
//! Invariants:
//! - Every entry has a content-addressed hash (SHA-256 of canonical JSON).
//! - Hash chain: entry N includes hash(N-1) — tamper-evident.
//! - No deletion, no mutation, no silent overwrite.

pub mod entry;
pub mod log;
pub mod writer;

pub use entry::{ASKESIS_ENTRY_SCHEMA, AskesisEntry, AskesisKind, GENESIS_HASH};
pub use log::{ASKESIS_SCHEMA, AskesisLog};
pub use writer::AskesisWriter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_entry_hashes_correctly() {
        let entry = AskesisEntry::new(
            AskesisKind::CourtAudit,
            1700000000,
            1,
            GENESIS_HASH,
            "test",
            serde_json::json!({"ok": true}),
        );
        assert!(entry.verify());
        assert_ne!(entry.entry_hash, GENESIS_HASH);
    }

    #[test]
    fn log_chains_entries() {
        let mut log = AskesisLog::new("test-log");
        log.append(
            AskesisKind::CourtAudit,
            1700000000,
            "court-cycle-1",
            serde_json::json!({"violations": 0}),
        );
        log.append(
            AskesisKind::Forensics,
            1700000100,
            "boot-abc123",
            serde_json::json!({"rcu_stalls": 1}),
        );

        assert_eq!(log.entries.len(), 2);
        assert!(log.verify_chain());
        assert_eq!(log.entries[1].prev_hash, log.entries[0].entry_hash);
    }

    #[test]
    fn log_detects_tampering() {
        let mut log = AskesisLog::new("test");
        log.append(
            AskesisKind::CourtAudit,
            1700000000,
            "cycle-1",
            serde_json::json!({"ok": true}),
        );

        // Tamper with evidence
        log.entries[0].evidence = serde_json::json!({"ok": false});
        assert!(!log.verify_chain());
    }

    #[test]
    fn askesis_kind_roundtrip() {
        for kind in [
            AskesisKind::CourtAudit,
            AskesisKind::Forensics,
            AskesisKind::ReleaseProof,
            AskesisKind::AgentEvidence,
            AskesisKind::Custom,
        ] {
            let s = kind.as_str();
            let parsed: AskesisKind = serde_json::from_str(&format!("\"{s}\"")).unwrap();
            assert_eq!(kind, parsed);
        }
    }
}
