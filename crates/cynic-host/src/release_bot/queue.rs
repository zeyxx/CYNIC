//! Release bot — queue forgejo commits, gate, and push to github.
//!
//! Architecture:
//!   forgejo/main → [queue: 7 days TTL] → [gate: check + lint + readme] → [bot: release commit] → github/main

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::Path;
use std::time::SystemTime;

const QUEUE_TTL_SECONDS: u64 = 7 * 24 * 60 * 60; // 7 days

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedRelease {
    pub schema: String,
    pub queued_at_unix_s: u64,
    pub source_sha: String,
    pub summary: String,
    pub commit_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseQueue {
    pub schema: String,
    pub releases: VecDeque<QueuedRelease>,
}

impl ReleaseQueue {
    pub fn new() -> Self {
        Self {
            schema: "cynic.host.release_queue.v1".to_string(),
            releases: VecDeque::new(),
        }
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::new());
        }
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("read queue: {e}"))?;
        serde_json::from_str::<Self>(&content)
            .map_err(|e| format!("parse queue: {e}"))
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("serialize queue: {e}"))?;
        std::fs::write(path, json)
            .map_err(|e| format!("write queue: {e}"))
    }

    pub fn enqueue(&mut self, source_sha: String, summary: String, commit_count: u32) {
        self.releases.push_back(QueuedRelease {
            schema: "cynic.host.queued_release.v1".to_string(),
            queued_at_unix_s: SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            source_sha,
            summary,
            commit_count,
        });
    }

    pub fn expire_old(&mut self) {
        let now = SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.releases.retain(|r| {
            now.saturating_sub(r.queued_at_unix_s) < QUEUE_TTL_SECONDS
        });
    }

    pub fn peek(&self) -> Option<&QueuedRelease> {
        self.releases.front()
    }

    pub fn is_empty(&self) -> bool {
        self.releases.is_empty()
    }

    pub fn len(&self) -> usize {
        self.releases.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_new_is_empty() {
        let q = ReleaseQueue::new();
        assert!(q.is_empty());
    }

    #[test]
    fn enqueue_adds_item() {
        let mut q = ReleaseQueue::new();
        q.enqueue("abc123".to_string(), "feat: test".to_string(), 5);
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn expire_old_removes_expired() {
        let mut q = ReleaseQueue::new();
        q.releases.push_back(QueuedRelease {
            schema: "test".to_string(),
            queued_at_unix_s: 0, // very old
            source_sha: "old".to_string(),
            summary: "old".to_string(),
            commit_count: 1,
        });
        q.expire_old();
        assert!(q.is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let mut q = ReleaseQueue::new();
        q.enqueue("abc123".to_string(), "feat: test".to_string(), 5);
        let mut path = std::env::temp_dir();
        path.push("cynic_test_queue.json");
        q.save(&path).unwrap();
        let loaded = ReleaseQueue::load(&path).unwrap();
        assert_eq!(loaded.len(), 1);
        let _ = std::fs::remove_file(&path);
    }
}
