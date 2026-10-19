//! Duplicate-submit collision counting (H4 measurement).
//!
//! Pure policy for the scheduler submission surface. Counts idempotency-key
//! / source-commit collisions per 24h window so H4 (same-SHA duplicate builds
//! are common under multi-agent load) can be measured from real traffic.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Trailing window for collision counting: 24 hours in milliseconds.
pub const COLLISION_WINDOW_MS: u64 = 24 * 60 * 60 * 1000;

/// One observed submission event: the idempotency key and source commit of a
/// build submission, with the time it was observed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollisionEvent {
    pub idempotency_key: String,
    pub source_commit: String,
    pub observed_at_ms: u64,
}

/// Count collisions in a trailing window. A collision is an event whose
/// idempotency key OR source commit was already seen within `window_ms`.
///
/// Events are filtered to `[now_ms - window_ms, now_ms]` and processed in
/// time order. The first occurrence of a key (or commit) is not a collision;
/// every repeat is.
pub fn count_collisions(events: &[CollisionEvent], now_ms: u64, window_ms: u64) -> u64 {
    let cutoff = now_ms.saturating_sub(window_ms);
    let mut sorted: Vec<&CollisionEvent> = events
        .iter()
        .filter(|e| e.observed_at_ms >= cutoff && e.observed_at_ms <= now_ms)
        .collect();
    sorted.sort_by_key(|e| e.observed_at_ms);
    let mut seen_keys: HashSet<&str> = HashSet::new();
    let mut seen_commits: HashSet<&str> = HashSet::new();
    let mut collisions = 0u64;
    for event in sorted {
        let key_dup = !seen_keys.insert(event.idempotency_key.as_str());
        let commit_dup = !seen_commits.insert(event.source_commit.as_str());
        if key_dup || commit_dup {
            collisions += 1;
        }
    }
    collisions
}

/// Stateful collision counter for the live submission surface. Records events
/// and answers "how many collisions in the trailing window?".
#[derive(Debug, Default)]
pub struct CollisionCounter {
    events: Vec<CollisionEvent>,
}

impl CollisionCounter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a submission event.
    pub fn record(&mut self, event: CollisionEvent) {
        self.events.push(event);
    }

    /// Number of collisions in the trailing `window_ms` ending at `now_ms`.
    pub fn collisions_in_window(&self, now_ms: u64, window_ms: u64) -> u64 {
        count_collisions(&self.events, now_ms, window_ms)
    }

    /// Convenience: collisions in the standard 24h window.
    pub fn collisions_last_24h(&self, now_ms: u64) -> u64 {
        self.collisions_in_window(now_ms, COLLISION_WINDOW_MS)
    }

    /// Total events recorded (for status surface context).
    pub fn total_events(&self) -> usize {
        self.events.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(key: &str, commit: &str, at_ms: u64) -> CollisionEvent {
        CollisionEvent {
            idempotency_key: key.to_string(),
            source_commit: commit.to_string(),
            observed_at_ms: at_ms,
        }
    }

    #[test]
    fn no_collisions_when_all_keys_and_commits_unique() {
        let events = vec![
            event("key:a", "commit:1", 1_000),
            event("key:b", "commit:2", 2_000),
            event("key:c", "commit:3", 3_000),
        ];
        assert_eq!(count_collisions(&events, 3_000, COLLISION_WINDOW_MS), 0);
    }

    #[test]
    fn repeat_idempotency_key_counts_as_collision() {
        let events = vec![
            event("key:a", "commit:1", 1_000),
            event("key:a", "commit:1", 2_000),
        ];
        assert_eq!(count_collisions(&events, 2_000, COLLISION_WINDOW_MS), 1);
    }

    #[test]
    fn same_commit_different_key_counts_as_collision() {
        let events = vec![
            event("key:a", "commit:1", 1_000),
            event("key:b", "commit:1", 2_000),
        ];
        assert_eq!(count_collisions(&events, 2_000, COLLISION_WINDOW_MS), 1);
    }

    #[test]
    fn three_submissions_same_key_is_two_collisions() {
        let events = vec![
            event("key:a", "commit:1", 1_000),
            event("key:a", "commit:1", 2_000),
            event("key:a", "commit:1", 3_000),
        ];
        assert_eq!(count_collisions(&events, 3_000, COLLISION_WINDOW_MS), 2);
    }

    #[test]
    fn events_outside_window_are_excluded() {
        let events = vec![
            event("key:a", "commit:1", 1_000),
            // More than 24h later — outside the window.
            event("key:a", "commit:1", 1_000 + COLLISION_WINDOW_MS + 1),
        ];
        assert_eq!(
            count_collisions(&events, 1_000 + COLLISION_WINDOW_MS + 1, COLLISION_WINDOW_MS),
            0
        );
    }

    #[test]
    fn window_boundary_is_inclusive() {
        let events = vec![
            event("key:a", "commit:1", 1_000),
            // Exactly at the cutoff: now - window = 1000.
            event("key:a", "commit:1", 1_000 + COLLISION_WINDOW_MS),
        ];
        assert_eq!(
            count_collisions(&events, 1_000 + COLLISION_WINDOW_MS, COLLISION_WINDOW_MS),
            1
        );
    }

    #[test]
    fn counter_records_and_counts() {
        let mut counter = CollisionCounter::new();
        counter.record(event("key:a", "commit:1", 1_000));
        counter.record(event("key:b", "commit:2", 2_000));
        assert_eq!(counter.collisions_last_24h(2_000), 0);
        counter.record(event("key:a", "commit:1", 3_000));
        assert_eq!(counter.collisions_last_24h(3_000), 1);
    }

    #[test]
    fn empty_events_is_zero_collisions() {
        assert_eq!(count_collisions(&[], 0, COLLISION_WINDOW_MS), 0);
    }
}
