//! Agent coordination domain services / use cases (Zero-I/O + local state).
//!
//! These functions own the coordination policy that used to live in
//! `cynic_cli::agent_surface::coordination`: claim creation with conflict
//! resolution, local CAS release, the fail-closed mutation gate, and
//! status computation from the local claims document. They are the single
//! source of truth for those rules; `cynic_cli` only dispatches to them.

use cynic_protocol::agent_coordination::{predicates as coord_predicates, ClaimScope};
use serde_json::Value;

use crate::coordination::admission::{local_claim_active, local_scope_of};
use crate::coordination::local_store::{append_claim_event, load_claims_doc, now_unix_ms, now_unix_s, store_claims_doc};
use crate::coordination::types::{
    AgentClaimReport, AgentReleaseReport, AgentScopeReport, AGENT_CLAIM_REPORT_SCHEMA,
    AGENT_RELEASE_REPORT_SCHEMA, AGENT_SCOPE_REPORT_SCHEMA,
};

/// Claim a scope in the local (no kernel) path.
///
/// Pure policy: validation, overlapping-scope conflict detection, and the
/// idempotent re-claim (heartbeat-by-reclaim) are decided here, using the
/// canonical `scopes_overlap` predicate from `cynic_protocol` — never a local
/// copy of the rule. Durable writes happen through `store_claims_doc`.
pub fn execute_claim_local(
    agent_id: &str,
    target: &str,
    claim_type: Option<&str>,
    ttl_seconds: Option<u64>,
    budget_actions: Option<u64>,
    budget_wall_seconds: Option<u64>,
) -> Result<AgentClaimReport, String> {
    if agent_id.trim().is_empty() {
        return Err("claim failed: agent_id is required for local claim".to_string());
    }
    if target.trim().is_empty() {
        return Err("claim failed: target is required for local claim".to_string());
    }
    let kind = claim_type.unwrap_or("file");
    let ttl = ttl_seconds.unwrap_or(1800);
    let actions = budget_actions.unwrap_or(50);
    let wall = budget_wall_seconds.unwrap_or(3600);
    let now_s = now_unix_s();
    let now_ms = now_s * 1000;
    let incoming = local_scope_of(kind, target);

    let mut doc = load_claims_doc()?;
    let claims = doc
        .get("claims")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    for existing_claim in &claims {
        if !local_claim_active(existing_claim, now_s) {
            continue;
        }
        let holder = existing_claim
            .get("agent_id")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let holder_id = existing_claim
            .get("claim_id")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let scope = existing_claim.get("scope");
        let existing_kind = scope
            .and_then(|s| s.get("kind"))
            .and_then(|v| v.as_str())
            .unwrap_or("file");
        let existing_path = scope
            .and_then(|s| s.get("path"))
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if holder == agent_id && existing_kind == kind && existing_path == target {
            let granted_ttl = existing_claim
                .get("temporal")
                .and_then(|t| t.get("ttl_seconds"))
                .and_then(|v| v.as_u64())
                .unwrap_or(ttl);
            let budget_actions_v = existing_claim
                .get("budget")
                .and_then(|b| b.get("actions"))
                .and_then(|v| v.as_u64())
                .unwrap_or(actions);
            let budget_wall_v = existing_claim
                .get("budget")
                .and_then(|b| b.get("wall_seconds"))
                .and_then(|v| v.as_u64())
                .unwrap_or(wall);
            let holder_id_owned = holder_id.to_string();
            // Heartbeat-by-reclaim (2026-09-28): same holder re-claiming the
            // same live scope refreshes started_at instead of stacking a
            // duplicate. Local claims have no heartbeat command; without this
            // every TTL window orphaned its entry and the active set only
            // grew. Dead holders never reclaim, so they decay by TTL.
            if let Some(list) = doc.get_mut("claims").and_then(|v| v.as_array_mut()) {
                if let Some(entry) = list.iter_mut().find(|e| {
                    e.get("claim_id").and_then(|v| v.as_str()) == Some(holder_id_owned.as_str())
                }) {
                    entry["temporal"]["started_at_unix_s"] = serde_json::json!(now_s);
                }
            }
            let revision = store_claims_doc(&mut doc)?;
            append_claim_event(
                "claim_local_heartbeat",
                agent_id,
                target,
                serde_json::json!({
                    "lease_id": holder_id_owned,
                    "revision": revision,
                }),
            );
            return Ok(AgentClaimReport {
                schema: AGENT_CLAIM_REPORT_SCHEMA,
                status: "claimed",
                agent_id: agent_id.to_string(),
                target: target.to_string(),
                claim_type: kind.to_string(),
                lease_id: holder_id_owned,
                fencing_token: revision,
                ttl_seconds: granted_ttl,
                budget_actions: budget_actions_v,
                budget_wall_seconds: budget_wall_v,
                granted_at_unix_ms: now_ms,
                expires_at_unix_ms: now_ms + granted_ttl * 1000,
            });
        }
        if coord_predicates::scopes_overlap(
            &local_scope_of(existing_kind, existing_path),
            &incoming,
        ) {
            return Err(format!(
                "claim failed: scope_conflict with {holder_id} held by {holder}"
            ));
        }
    }

    let slug: String = agent_id
        .chars()
        .filter(|c| c.is_alphanumeric())
        .take(24)
        .collect();
    let claim_id = format!("claim_{slug}_{now_s}");
    let entry = serde_json::json!({
        "claim_id": claim_id,
        "agent_id": agent_id,
        "mode": "baseline",
        "scope": {"kind": kind, "path": target},
        "budget": {"actions": actions, "wall_seconds": wall},
        "consumed": {"actions": 0, "wall_seconds": 0},
        "temporal": {"started_at_unix_s": now_s, "ttl_seconds": ttl},
        "priority": 10,
        "parent_claim_id": null,
        "status": "active",
    });
    match doc.get_mut("claims").and_then(|v| v.as_array_mut()) {
        Some(list) => {
            // Compaction on success path only (2026-09-28): silently expired
            // entries are void for conflicts already; drop them here so the
            // file stops growing with ghosts. Explicit `released` entries are
            // audit trail and stay. A failed claim below must not mutate.
            list.retain(|e| {
                e.get("status").and_then(|v| v.as_str()) == Some("released")
                    || local_claim_active(e, now_s)
            });
            list.push(entry);
        }
        None => doc["claims"] = serde_json::json!([entry]),
    }
    let revision = store_claims_doc(&mut doc)?;

    append_claim_event(
        "claim_local",
        agent_id,
        target,
        serde_json::json!({
            "lease_id": claim_id,
            "revision": revision,
            "claim_type": kind,
            "ttl_seconds": ttl,
        }),
    );

    Ok(AgentClaimReport {
        schema: AGENT_CLAIM_REPORT_SCHEMA,
        status: "claimed",
        agent_id: agent_id.to_string(),
        target: target.to_string(),
        claim_type: kind.to_string(),
        lease_id: claim_id.clone(),
        fencing_token: revision,
        ttl_seconds: ttl,
        budget_actions: actions,
        budget_wall_seconds: wall,
        granted_at_unix_ms: now_ms,
        expires_at_unix_ms: now_ms + ttl * 1000,
    })
}

/// Local CAS release: marks the matching active claim released, revision + 1.
///
/// Two defects, one loop: (1) a `released` record must not shadow a live one
/// on the same path; (2) `claim_id` is NOT unique (`{agent-slug}_{unix-second}
/// `), so a lease match hits whichever entry came first. Therefore only an
/// entry that is still held may be flipped.
pub fn execute_release_local(
    agent_id: &str,
    target: &str,
    lease_id: &str,
    proof_refs: Vec<String>,
) -> Result<AgentReleaseReport, String> {
    let mut doc = load_claims_doc()?;
    let now_ms = now_unix_ms();
    let mut matched_lease = String::new();
    let mut released_record_seen = false;
    if let Some(list) = doc.get_mut("claims").and_then(|v| v.as_array_mut()) {
        for claim in list.iter_mut() {
            let cid = claim
                .get("claim_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let holder = claim
                .get("agent_id")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let path = claim
                .get("scope")
                .and_then(|s| s.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let by_lease = !lease_id.is_empty() && cid == lease_id;
            let by_target = path == target && (holder == agent_id || agent_id == "*");
            if !by_lease && !by_target {
                continue;
            }
            if claim.get("status").and_then(|v| v.as_str()) != Some("active") {
                released_record_seen = true;
                continue;
            }
            if holder != agent_id && agent_id != "*" {
                return Err(format!("release failed: claim {cid} is held by {holder}"));
            }
            claim["status"] = serde_json::json!("released");
            matched_lease = cid;
            break;
        }
    }
    if matched_lease.is_empty() {
        if released_record_seen {
            return Ok(AgentReleaseReport {
                schema: AGENT_RELEASE_REPORT_SCHEMA,
                status: "released",
                agent_id: agent_id.to_string(),
                target: target.to_string(),
                lease_id: lease_id.to_string(),
                proof_refs,
                released_at_unix_ms: now_ms,
            });
        }
        return Err(format!("release failed: no local claim for target {target}"));
    }
    store_claims_doc(&mut doc)?;
    append_claim_event(
        "release_local",
        agent_id,
        target,
        serde_json::json!({
            "lease_id": if lease_id.is_empty() { matched_lease.clone() } else { lease_id.to_string() },
            "proof_refs": proof_refs,
        }),
    );
    Ok(AgentReleaseReport {
        schema: AGENT_RELEASE_REPORT_SCHEMA,
        status: "released",
        agent_id: agent_id.to_string(),
        target: target.to_string(),
        lease_id: if lease_id.is_empty() { matched_lease } else { lease_id.to_string() },
        proof_refs,
        released_at_unix_ms: now_ms,
    })
}

/// Fail-closed mutation gate: every path must be covered by an ACTIVE claim
/// held by this agent, decided by the same `scopes_overlap` predicate as
/// claim admission. Covered = exact or ancestor scope; anything else is
/// uncovered and the caller must claim before mutating. Read-only.
pub fn execute_scope(agent_id: &str, paths: &[String]) -> Result<Value, String> {
    let doc = load_claims_doc()?;
    let now_s = now_unix_s();
    let claims = doc
        .get("claims")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut covered: Vec<String> = Vec::new();
    let mut uncovered: Vec<String> = Vec::new();
    for path in paths {
        let wanted = ClaimScope::File { path: path.clone() };
        let held = claims.iter().any(|claim| {
            claim.get("agent_id")
                .and_then(|v| v.as_str()) == Some(agent_id)
                && local_claim_active(claim, now_s)
                && {
                    let kind = claim
                        .get("scope")
                        .and_then(|s| s.get("kind"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("file");
                    let target = claim
                        .get("scope")
                        .and_then(|s| s.get("path"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    coord_predicates::scopes_overlap(&local_scope_of(kind, target), &wanted)
                }
        });
        if held {
            covered.push(path.clone());
        } else {
            uncovered.push(path.clone());
        }
    }
    Ok(serde_json::json!({
        "schema": AGENT_SCOPE_REPORT_SCHEMA,
        "status": if uncovered.is_empty() { "covered" } else { "uncovered" },
        "agent_id": agent_id,
        "covered": covered,
        "uncovered": uncovered,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim_entry(status: &str, started_at_unix_s: u64, ttl_seconds: u64) -> Value {
        serde_json::json!({
            "claim_id": "claim_test_1",
            "agent_id": "tester",
            "scope": {"kind": "file", "path": "a/b.rs"},
            "temporal": {"started_at_unix_s": started_at_unix_s, "ttl_seconds": ttl_seconds},
            "status": status,
        })
    }

    #[test]
    fn claim_rejects_empty_agent() {
        let err = execute_claim_local("", "a/b.rs", None, None, None, None).unwrap_err();
        assert!(err.contains("agent_id is required"));
    }

    #[test]
    fn claim_rejects_empty_target() {
        let err = execute_claim_local("tester", "", None, None, None, None).unwrap_err();
        assert!(err.contains("target is required"));
    }

    #[test]
    fn claim_creates_unique_entry_and_compacts_expired() {
        let now = 1_790_600_000u64;
        let mut doc = serde_json::json!({
            "schema": "cynic.agent_claims.v2",
            "revision": 3,
            "claims": [
                claim_entry("active", now - 100, 1800),
                claim_entry("released", now - 100, 1800),
            ],
            "merge_queue": [],
        });
        let _ = execute_claim_local("tester", "x/y.rs", None, None, None, None,);
        let claims = doc["claims"].as_array().unwrap();
        assert_eq!(claims.len(), 2);
        assert_eq!(claims[0]["status"], "active");
        assert_eq!(claims[1]["status"], "released");
        assert!(claims[0]["temporal"]["started_at_unix_s"].is_u64());
    }

    #[test]
    fn claim_refreshes_same_holder_without_stack() {
        let now = 1_790_600_000u64;
        let mut doc = serde_json::json!({
            "schema": "cynic.agent_claims.v2",
            "revision": 3,
            "claims": [claim_entry("active", now - 100, 1800)],
            "merge_queue": [],
        });
        let path = crate::coordination::local_store::claims_file_path();
        std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
        let report = execute_claim_local("tester", "a/b.rs", None, None, None, None,).expect("claim");
        assert_eq!(report.status, "claimed");
        let updated_doc: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert_eq!(updated_doc["claims"][0]["temporal"]["started_at_unix_s"], serde_json::json!(now));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn release_fails_when_held_by_other_agent() {
        let now = 1_790_600_000u64;
        let doc = serde_json::json!({
            "schema": "cynic.agent_claims.v2",
            "revision": 1,
            "claims": [claim_entry("active", now - 100, 1800)],
            "merge_queue": [],
        });
        let path = crate::coordination::local_store::claims_file_path();
        std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
        let err = execute_release_local("rival", "a/b.rs", "", vec![]).unwrap_err();
        assert!(err.contains("held by tester"));
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn scope_gated_by_active_and_owner() {
        let now = 1_790_600_000u64;
        let doc = serde_json::json!({
            "schema": "cynic.agent_claims.v2",
            "revision": 1,
            "claims": [claim_entry("active", now - 100, 1800)],
            "merge_queue": [],
        });
        let path = crate::coordination::local_store::claims_file_path();
        std::fs::write(&path, serde_json::to_string_pretty(&doc).unwrap()).unwrap();
        let v = execute_scope("tester", &["a/b.rs".to_string()]).unwrap();
        assert_eq!(v["status"], "covered");
        let uncovered = v["uncovered"].as_array().unwrap();
        assert!(uncovered.is_empty());
        let _ = std::fs::remove_file(&path);
    }
}
