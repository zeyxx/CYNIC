//! Kernel release proof — append-only hash-chained event log.
//!
//! Uses the unified hash-chain primitive (`cynic_protocol::hash_chain`)
//! and digest law (`cynic_protocol::digest` — SHA-256).
//!
//! **Intentional clean break (T13)**: entries sealed with the old
//! SHA3-512 algorithm (pre-T13) do NOT verify under SHA-256. This is an
//! intentional pre-production break with no migration path. Old persisted
//! segments will fail verification on load.

use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use cynic_protocol::hash_chain::HashChain;

use super::types::KernelReleaseEvent;

/// Genesis prev-hash for the unified hash chain.
pub const GENESIS_PREV_HASH: &str = "genesis";

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn event_hash(event: &KernelReleaseEvent) -> Result<String, String> {
    Ok(HashChain::seal(event))
}

fn load_and_verify(path: &Path) -> Result<Vec<KernelReleaseEvent>, String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Ok(Vec::new());
    };
    let mut events = Vec::new();
    for (index, line) in raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
    {
        let event: KernelReleaseEvent = serde_json::from_str(line)
            .map_err(|error| format!("invalid release proof line {}: {error}", index + 1))?;
        if event.seq != index as u64 + 1 {
            return Err(format!("release proof chain broken at seq={}", event.seq));
        }
        events.push(event);
    }
    HashChain::verify(GENESIS_PREV_HASH, &events)
        .map_err(|e| format!("release proof verification failed: {e}"))?;
    Ok(events)
}

pub fn append_event(
    path: &Path,
    mut event: KernelReleaseEvent,
) -> Result<KernelReleaseEvent, String> {
    let events = load_and_verify(path)?;
    event.seq = events.len() as u64 + 1;
    event.prev_hash = events
        .last()
        .map(|last| last.hash.clone())
        .unwrap_or_else(|| GENESIS_PREV_HASH.to_string());
    event.hash = event_hash(&event)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|error| format!("mkdir {}: {error}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| format!("open {}: {error}", path.display()))?;
    let line = serde_json::to_string(&event).map_err(|error| error.to_string())?;
    writeln!(file, "{line}").map_err(|error| format!("append {}: {error}", path.display()))?;
    file.sync_all()
        .map_err(|error| format!("sync {}: {error}", path.display()))?;
    Ok(event)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::kernel_release::types::{KERNEL_RELEASE_EVENT_SCHEMA, KernelReleaseEvent};

    fn event(node: &str) -> KernelReleaseEvent {
        KernelReleaseEvent {
            schema: KERNEL_RELEASE_EVENT_SCHEMA.to_string(),
            seq: 0,
            timestamp_unix_ms: 1,
            artifact_id: "cynic-kernel".to_string(),
            service_id: "cynic-kernel".to_string(),
            node_id: node.to_string(),
            lifecycle_phase: "conformance".to_string(),
            event: "deployment_succeeded".to_string(),
            desired_sha256: "a".repeat(64),
            previous_target: None,
            activated_target: Some("release/a/cynic-kernel".to_string()),
            observed_version: Some("v1".to_string()),
            observed_git_sha: Some("abc".to_string()),
            status: "ok".to_string(),
            prev_hash: String::new(),
            hash: String::new(),
        }
    }

    #[test]
    fn appends_a_verified_hash_chain() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.jsonl");
        let first = append_event(&path, event("a")).unwrap();
        let second = append_event(&path, event("a")).unwrap();
        assert_eq!(first.seq, 1);
        assert_eq!(second.seq, 2);
        assert_eq!(second.prev_hash, first.hash);
        assert_eq!(load_and_verify(&path).unwrap().len(), 2);
    }

    #[test]
    fn hash_uses_sha256_prefix() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.jsonl");
        let first = append_event(&path, event("a")).unwrap();
        assert!(
            first.hash.starts_with("sha256:"),
            "expected sha256: prefix, got: {}",
            &first.hash[..20]
        );
        assert_eq!(first.hash.len(), "sha256:".len() + 64);
    }

    #[test]
    fn old_sha256_genesis_entry_fails_verification() {
        // Pin the intentional clean break: an entry sealed with the old
        // SHA-256 algorithm and old genesis (empty-string prev_hash) must
        // NOT verify under the unified SHA-256 hash chain.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.jsonl");

        // Construct an old-format entry: SHA-256 hash, genesis prev_hash = ""
        let mut old_event = event("a");
        old_event.seq = 1;
        old_event.prev_hash = String::new(); // old genesis: empty string
        let unsigned = serde_json::to_vec(&old_event).unwrap();
        old_event.hash = hex::encode(sha2::Sha256::digest(&unsigned));

        // Write it as a JSONL line
        std::fs::write(&path, serde_json::to_string(&old_event).unwrap() + "\n").unwrap();

        // Verification must fail — the old SHA3-512 hash does not match SHA-256
        let result = load_and_verify(&path);
        assert!(
            result.is_err(),
            "old SHA3-512 entry must fail verification under SHA-256"
        );
    }
}
