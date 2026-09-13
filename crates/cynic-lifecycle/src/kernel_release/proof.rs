use sha2::{Digest, Sha256};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

use super::types::KernelReleaseEvent;

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn event_hash(event: &KernelReleaseEvent) -> Result<String, String> {
    let mut unsigned = event.clone();
    unsigned.hash.clear();
    let bytes = serde_json::to_vec(&unsigned).map_err(|error| error.to_string())?;
    Ok(hex::encode(Sha256::digest(bytes)))
}

fn load_and_verify(path: &Path) -> Result<Vec<KernelReleaseEvent>, String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Ok(Vec::new());
    };
    let mut events = Vec::new();
    let mut expected_prev = String::new();
    for (index, line) in raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
    {
        let event: KernelReleaseEvent = serde_json::from_str(line)
            .map_err(|error| format!("invalid release proof line {}: {error}", index + 1))?;
        if event.seq != index as u64 + 1 || event.prev_hash != expected_prev {
            return Err(format!("release proof chain broken at seq={}", event.seq));
        }
        let computed = event_hash(&event)?;
        if event.hash != computed {
            return Err(format!("release proof hash mismatch at seq={}", event.seq));
        }
        expected_prev = event.hash.clone();
        events.push(event);
    }
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
        .unwrap_or_default();
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
    use super::types::{KERNEL_RELEASE_EVENT_SCHEMA, KernelReleaseEvent};
    use super::*;

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
}
