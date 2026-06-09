#![allow(clippy::unwrap_used, clippy::expect_used)]

use chrono::{Duration, Utc};
use cynic_askesis::log::{LogEntry, LogStore, jsonl::JsonlLog};
use tempfile::TempDir;

#[test]
fn append_then_range_returns_entries() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("log.jsonl");
    let mut store = JsonlLog::new(path).unwrap();

    let entry = LogEntry::new("pas fait de sport aujourd'hui").with_domain("body");
    store.append(entry.clone()).unwrap();

    let now = Utc::now();
    let entries = store
        .range(now - Duration::hours(1), now + Duration::hours(1))
        .unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].content, entry.content);
    assert_eq!(entries[0].domain, entry.domain);
}

#[test]
fn range_filters_by_timestamp() {
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("log.jsonl");
    let mut store = JsonlLog::new(path).unwrap();

    store.append(LogEntry::new("today")).unwrap();

    let future_from = Utc::now() + Duration::days(1);
    let future_to = future_from + Duration::hours(1);
    let entries = store.range(future_from, future_to).unwrap();
    assert_eq!(entries.len(), 0);
}
