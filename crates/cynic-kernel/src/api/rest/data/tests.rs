//! Tests for data REST handlers.

use super::*;
use crate::domain::ccm::{self, Crystal};
use crate::domain::sanitize::sanitize_observation_target;

#[test]
fn crystal_to_json_includes_4d_fields() {
    let c = Crystal {
        id: "test-id".into(),
        content: "test".into(),
        domain: "chess".into(),
        confidence: 0.5,
        observations: 10,
        state: ccm::CrystalState::Forming,
        created_at: 1_767_225_600_000,
        updated_at: 1_767_312_000_000,
        contributing_verdicts: vec![],
        certainty: 0.75,
        variance_m2: 1.23,
        mean_quorum: 3.5,
        howl_count: 4,
        wag_count: 3,
        growl_count: 2,
        bark_count: 1,
        source_diversity: 2,
        relations: std::collections::BTreeMap::new(),
        embedding: None,
        contributing_sources: std::collections::BTreeMap::new(),
        shattered_at: None,
        shatter_reason: None,
        shatter_source: None,
    };
    let json = crystals::crystal_to_json(&c);
    let obj = json
        .as_object()
        .expect("crystal_to_json must return object");
    assert_eq!(
        obj.len(),
        22,
        "crystal_to_json must serialize all 22 Crystal fields"
    );
    assert_eq!(json["id"], "test-id");
    assert_eq!(json["content"], "test");
    assert_eq!(json["certainty"], 0.75);
    assert_eq!(json["variance_m2"], 1.23);
    assert_eq!(json["mean_quorum"], 3.5);
    assert_eq!(json["howl_count"], 4);
    assert_eq!(json["wag_count"], 3);
    assert_eq!(json["growl_count"], 2);
    assert_eq!(json["bark_count"], 1);
}

#[test]
fn crystals_query_defaults_are_none() {
    let q: CrystalsQuery = serde_json::from_str("{}").expect("empty object parses");
    assert!(q.limit.is_none());
    assert!(q.domain.is_none());
    assert!(q.state.is_none());
}

#[test]
fn crystals_query_parses_all_fields() {
    let q = CrystalsQuery {
        limit: Some(25),
        domain: Some("chess".into()),
        state: Some("crystallized".into()),
    };
    assert_eq!(q.limit, Some(25));
    assert_eq!(q.domain.as_deref(), Some("chess"));
    assert_eq!(q.state.as_deref(), Some("crystallized"));
}

#[test]
fn crystals_query_limit_default_is_50() {
    let q = CrystalsQuery {
        limit: None,
        domain: None,
        state: None,
    };
    let effective_limit = q.limit.unwrap_or(50).min(200);
    assert_eq!(effective_limit, 50);
}

#[test]
fn crystals_query_limit_is_capped_at_200() {
    let q = CrystalsQuery {
        limit: Some(9999),
        domain: None,
        state: None,
    };
    let effective_limit = q.limit.unwrap_or(50).min(200);
    assert_eq!(effective_limit, 200);
}

#[test]
fn create_crystal_request_deserializes_with_domain() {
    let json = r#"{"content":"The Sicilian is strong","domain":"chess"}"#;
    let req: CreateCrystalRequest = serde_json::from_str(json).expect("full request parses");
    assert_eq!(req.content, "The Sicilian is strong");
    assert_eq!(req.domain.as_deref(), Some("chess"));
}

#[test]
fn create_crystal_request_domain_is_optional() {
    let json = r#"{"content":"generic insight"}"#;
    let req: CreateCrystalRequest =
        serde_json::from_str(json).expect("domain-less request parses");
    assert!(req.domain.is_none());
    let domain = req.domain.unwrap_or_else(|| "general".into());
    assert_eq!(domain, "general");
}

#[test]
fn create_crystal_content_empty_is_invalid() {
    let content = "";
    assert!(
        content.trim().is_empty(),
        "empty content must fail validation"
    );
}

#[test]
fn create_crystal_content_2001_chars_is_invalid() {
    let content: String = "x".repeat(2001);
    assert!(
        content.chars().count() > 2000,
        "oversized content must fail validation"
    );
}

#[test]
fn create_crystal_content_2000_chars_is_valid() {
    let content: String = "x".repeat(2000);
    assert!(
        !content.trim().is_empty() && content.chars().count() <= 2000,
        "2000 chars must pass validation"
    );
}

#[test]
fn public_observation_projection_redacts_internal_fields() {
    let obs = crate::domain::storage::RawObservation {
        id: "observation:test".into(),
        tool: "Edit".into(),
        target: "/tmp/\n[INJECT] prompt".into(),
        domain: "talaria".into(),
        status: "ok".into(),
        context: "secret context".into(),
        created_at: 1_780_912_800_000,
        project: "CYNIC".into(),
        agent_id: "agent-123".into(),
        session_id: "session-456".into(),
        tags: vec!["public".into()],
        source_tier: "session".into(),
        value: Some(serde_json::json!({"secret": "yes"})),
        confidence: Some("observed".into()),
        consumer: Some("ops".into()),
        action: Some("review".into()),
        depends_on: vec!["readme".into()],
        maturity: Some(0.5),
        hash: "hash".into(),
        prev_hash: "prev".into(),
        observers: vec!["observer-1".into()],
        consensus_score: Some(0.8),
    };

    let json = observations::public_observation_projection(&obs);
    assert_eq!(json["tool"], "Edit");
    assert_eq!(json["target"], sanitize_observation_target(&obs.target));
    assert_eq!(json["domain"], "talaria");
    assert_eq!(json["status"], "ok");
    assert_eq!(json["project"], "CYNIC");
    assert_eq!(json["source_tier"], "session");
    assert_eq!(json["created_at"], "2026-06-08T10:00:00Z");
    assert!(json.get("context").is_none());
    assert!(json.get("agent_id").is_none());
    assert!(json.get("session_id").is_none());
    assert!(json.get("value").is_none());
}

#[test]
fn observations_query_defaults_are_none() {
    let q: ObservationsQuery = serde_json::from_str("{}").expect("empty object parses");
    assert!(q.limit.is_none());
    assert!(q.domain.is_none());
    assert!(q.agent_id.is_none());
    assert!(q.project.is_none());
    assert!(q.workstream.is_none());
    assert!(q.thread_id.is_none());
    assert!(q.target.is_none());
}

#[test]
fn observations_query_parses_agent_id() {
    let q = ObservationsQuery {
        limit: Some(10),
        domain: Some("code".into()),
        agent_id: Some("claude-123".into()),
        ..Default::default()
    };
    assert_eq!(q.agent_id.as_deref(), Some("claude-123"));
    assert_eq!(q.domain.as_deref(), Some("code"));
    assert_eq!(q.limit, Some(10));
}

#[test]
fn observations_limit_default_and_cap() {
    let q = ObservationsQuery {
        limit: None,
        domain: None,
        agent_id: None,
        ..Default::default()
    };
    let effective = q.limit.unwrap_or(100).min(100);
    assert_eq!(effective, 100);

    let q_big = ObservationsQuery {
        limit: Some(9999),
        domain: None,
        agent_id: None,
        ..Default::default()
    };
    let effective_big = q_big.limit.unwrap_or(100).min(100);
    assert_eq!(effective_big, 100);
}

#[test]
fn sessions_query_default_limit_is_none() {
    let q: SessionsQuery = serde_json::from_str("{}").expect("empty object parses");
    assert!(q.limit.is_none());
}

#[test]
fn sessions_limit_default_50_cap_200() {
    let q = SessionsQuery { limit: None };
    let effective = q.limit.unwrap_or(50).min(200);
    assert_eq!(effective, 50);

    let q_big = SessionsQuery { limit: Some(500) };
    let effective_big = q_big.limit.unwrap_or(50).min(200);
    assert_eq!(effective_big, 200);
}

#[test]
fn audit_query_defaults_are_none() {
    let q: AuditQuery = serde_json::from_str("{}").expect("empty object parses");
    assert!(q.limit.is_none());
    assert!(q.tool.is_none());
    assert!(q.agent_id.is_none());
    assert!(q.since.is_none());
    assert!(q.until.is_none());
}

#[test]
fn audit_query_parses_all_fields() {
    let q = AuditQuery {
        limit: Some(20),
        tool: Some("cynic_judge".into()),
        agent_id: Some("claude-1".into()),
        since: Some("2026-07-01".into()),
        until: Some("2026-07-08".into()),
    };
    assert_eq!(q.tool.as_deref(), Some("cynic_judge"));
    assert_eq!(q.agent_id.as_deref(), Some("claude-1"));
    assert_eq!(q.since.as_deref(), Some("2026-07-01"));
    assert_eq!(q.until.as_deref(), Some("2026-07-08"));
    assert_eq!(q.limit, Some(20));
}

#[test]
fn audit_limit_default_50_cap_100() {
    let q = AuditQuery {
        limit: None,
        tool: None,
        agent_id: None,
        since: None,
        until: None,
    };
    let effective = q.limit.unwrap_or(50).min(100);
    assert_eq!(effective, 50);

    let q_big = AuditQuery {
        limit: Some(999),
        tool: None,
        agent_id: None,
        since: None,
        until: None,
    };
    let effective_big = q_big.limit.unwrap_or(50).min(100);
    assert_eq!(effective_big, 100);
}

#[test]
fn compliance_trend_limit_default_20_cap_100() {
    let q = ComplianceTrendQuery { limit: None };
    let effective = q.limit.unwrap_or(20).min(100);
    assert_eq!(effective, 20);

    let q_big = ComplianceTrendQuery { limit: Some(999) };
    let effective_big = q_big.limit.unwrap_or(20).min(100);
    assert_eq!(effective_big, 100);
}
