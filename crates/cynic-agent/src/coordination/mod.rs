pub mod admission;
pub mod events;
pub mod local_store;
pub mod types;

pub use admission::{local_claim_active, local_scope_of, refresh_claim_entry};
pub use events::{civil_from_days, event_matches};
pub use local_store::{
    append_claim_event, claim_event_log_dir, claims_file_path, execute_status_from_claims_file,
    load_claims_doc, now_unix_ms, now_unix_s, store_claims_doc, CLAIM_EVENT_SCHEMA,
};
pub use types::{AgentClaimSummary, AgentStatusReport, ClaimEvent, LocalScope, AGENT_STATUS_REPORT_SCHEMA};

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn claim_entry(status: &str, started_at_unix_s: u64, ttl_seconds: u64) -> serde_json::Value {
        json!({
            "claim_id": "claim_test_1",
            "agent_id": "tester",
            "scope": {"kind": "file", "path": "a/b.rs"},
            "temporal": {"started_at_unix_s": started_at_unix_s, "ttl_seconds": ttl_seconds},
            "status": status,
        })
    }

    #[test]
    fn liveness_requires_active_status_and_unexpired_ttl() {
        let now = 1_790_600_000u64;
        assert!(local_claim_active(&claim_entry("active", now - 100, 1800), now));
        assert!(!local_claim_active(&claim_entry("active", now - 2000, 1800), now));
        assert!(!local_claim_active(&claim_entry("released", now - 100, 1800), now));
        assert!(!local_claim_active(&claim_entry("expired", now - 100, 1800), now));
        assert!(!local_claim_active(&claim_entry("active", now - 1800, 1800), now));
        assert!(local_claim_active(&claim_entry("active", now - 1799, 1800), now));
    }

    #[test]
    fn heartbeat_refreshes_started_at_and_keeps_lease() {
        let now = 1_790_600_000u64;
        let mut doc = json!({
            "schema": "cynic.agent_claims.v2",
            "revision": 3,
            "claims": [claim_entry("active", now - 100, 1800)],
            "merge_queue": [],
        });
        let (id, kind, ttl, actions, wall) =
            refresh_claim_entry(&mut doc, "tester", "a/b.rs", "", now + 500).expect("live claim heartbeats");
        assert_eq!(id, "claim_test_1");
        assert_eq!(kind, "file");
        assert_eq!(ttl, 1800);
        assert_eq!((actions, wall), (50, 3600));
        assert_eq!(doc["claims"][0]["temporal"]["started_at_unix_s"], json!(now + 500));
    }

    #[test]
    fn heartbeat_refuses_expired_claim() {
        let now = 1_790_600_000u64;
        let mut doc = json!({
            "schema": "cynic.agent_claims.v2",
            "revision": 3,
            "claims": [claim_entry("active", now - 2000, 1800)],
            "merge_queue": [],
        });
        let err = refresh_claim_entry(&mut doc, "tester", "a/b.rs", "", now).unwrap_err();
        assert!(err.contains("expired or released"), "unexpected: {err}");
    }

    #[test]
    fn heartbeat_refuses_foreign_lease() {
        let now = 1_790_600_000u64;
        let mut doc = json!({
            "schema": "cynic.agent_claims.v2",
            "revision": 3,
            "claims": [claim_entry("active", now - 100, 1800)],
            "merge_queue": [],
        });
        let err = refresh_claim_entry(&mut doc, "rival", "a/b.rs", "", now).unwrap_err();
        assert!(err.contains("no active local claim"), "unexpected: {err}");
        let err = refresh_claim_entry(&mut doc, "rival", "a/b.rs", "claim_test_1", now).unwrap_err();
        assert!(err.contains("held by tester"), "unexpected: {err}");
    }

    #[test]
    fn event_predicate_filters_by_agent_and_target() {
        let event = json!({
            "schema": "cynic.agent.claim_event.v1",
            "event": "claim_local",
            "agent_id": "hermes",
            "target": "crates/cynic-cli",
            "timestamp_unix_s": 1_791_936_000u64,
            "detail": {},
        });
        assert!(event_matches(&event, "*", "*"));
        assert!(event_matches(&event, "", ""));
        assert!(event_matches(&event, "hermes", "crates/cynic-cli"));
        assert!(!event_matches(&event, "codex", "*"));
        assert!(!event_matches(&event, "*", "crates/cynic-node"));
        let foreign_schema = json!({"schema": "other.v1"});
        assert!(!event_matches(&foreign_schema, "*", "*"));
    }

    #[test]
    fn civil_date_math_matches_known_day() {
        assert_eq!(civil_from_days(1791936000 / 86400), (2026, 10, 14));
    }
}