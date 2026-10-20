#![forbid(unsafe_code)]

//! Cynic Evidence — append-only proof and history log.
//!
//! Domain pure: no I/O, no chrono, no FS. Types, hashes, verification.
//! Adapters live in other crates (kernel, ...).
//!
//! Invariants:
//! - Every entry has a content-addressed hash (SHA-256 of canonical JSON).
//! - Hash chain: entry N includes hash(N-1) — tamper-evident.
//! - No deletion, no mutation, no silent overwrite.

pub mod entry;
pub mod log;
pub mod writer;

pub use entry::{EVIDENCE_ENTRY_SCHEMA, EvidenceEntry, EvidenceKind, GENESIS_HASH};
pub use log::{EVIDENCE_SCHEMA, EvidenceLog};
pub use writer::EvidenceWriter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn genesis_entry_hashes_correctly() {
        let entry = EvidenceEntry::new(
            EvidenceKind::CourtAudit,
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
        let mut log = EvidenceLog::new("test-log");
        log.append(
            EvidenceKind::CourtAudit,
            1700000000,
            "court-cycle-1",
            serde_json::json!({"violations": 0}),
        );
        log.append(
            EvidenceKind::Forensics,
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
        let mut log = EvidenceLog::new("test");
        log.append(
            EvidenceKind::CourtAudit,
            1700000000,
            "cycle-1",
            serde_json::json!({"ok": true}),
        );

        // Tamper with evidence
        log.entries[0].evidence = serde_json::json!({"ok": false});
        assert!(!log.verify_chain());
    }

    #[test]
    fn evidence_kind_roundtrip() {
        for kind in [
            EvidenceKind::CourtAudit,
            EvidenceKind::Forensics,
            EvidenceKind::ReleaseProof,
            EvidenceKind::AgentEvidence,
            EvidenceKind::Custom,
        ] {
            let s = kind.as_str();
            let parsed: EvidenceKind = serde_json::from_str(&format!("\"{s}\"")).unwrap();
            assert_eq!(kind, parsed);
        }
    }

    #[test]
    fn entry_hash_uses_sha256_prefix() {
        let entry = EvidenceEntry::new(
            EvidenceKind::CourtAudit,
            1700000000,
            1,
            GENESIS_HASH,
            "test",
            serde_json::json!({"ok": true}),
        );
        assert!(
            entry.entry_hash.starts_with("sha256:"),
            "expected sha256: prefix, got: {}",
            &entry.entry_hash[..20]
        );
        assert_eq!(entry.entry_hash.len(), "sha256:".len() + 64);
    }

    #[test]
    fn old_sha256_genesis_entry_fails_verification() {
        // Pin the intentional clean break: an entry sealed with the old
        // SHA-256 algorithm and old genesis (64-zero prev_hash) must NOT
        // verify under the unified SHA-256 hash chain.
        let mut entry = EvidenceEntry::new(
            EvidenceKind::CourtAudit,
            1700000000,
            1,
            GENESIS_HASH,
            "test",
            serde_json::json!({"ok": true}),
        );
        // Simulate old format: prev_hash = 64 zeros, hash = SHA-256 (unprefixed)
        entry.prev_hash = "0".repeat(64);
        entry.entry_hash = "0".repeat(64);
        assert!(
            !entry.verify(),
            "old SHA-256 entry must fail verification under SHA-256 (T13 flip: same algo, different genesis)"
        );
    }
}
