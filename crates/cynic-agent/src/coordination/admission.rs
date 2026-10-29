//! Pure domain predicates & rules for agent coordination (Zero-I/O).

use serde_json::Value;

pub fn local_scope_of(kind: &str, target: &str) -> cynic_protocol::agent_coordination::ClaimScope {
    match kind {
        "crate" => cynic_protocol::agent_coordination::ClaimScope::Crate {
            crate_name: target.strip_prefix("crates/").unwrap_or(target).to_string(),
        },
        _ => cynic_protocol::agent_coordination::ClaimScope::File {
            path: target.to_string(),
        },
    }
}

pub fn local_claim_active(claim: &Value, now_s: u64) -> bool {
    if claim.get("status").and_then(|v| v.as_str()).unwrap_or("") != "active" {
        return false;
    }
    let started = claim
        .get("temporal")
        .and_then(|t| t.get("started_at_unix_s"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let ttl = claim
        .get("temporal")
        .and_then(|t| t.get("ttl_seconds"))
        .and_then(|v| v.as_u64())
        .unwrap_or(1800);
    now_s < started.saturating_add(ttl)
}

pub fn refresh_claim_entry(
    doc: &mut Value,
    agent_id: &str,
    target: &str,
    lease_id: &str,
    now_s: u64,
) -> Result<(String, String, u64, u64, u64), String> {
    let list = doc
        .get_mut("claims")
        .and_then(|v| v.as_array_mut())
        .ok_or_else(|| "heartbeat failed: claims store has no claim list".to_string())?;
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
        let kind = claim
            .get("scope")
            .and_then(|s| s.get("kind"))
            .and_then(|v| v.as_str())
            .unwrap_or("file")
            .to_string();
        if !lease_id.is_empty() {
            if cid != lease_id {
                continue;
            }
        } else if holder != agent_id || path != target {
            continue;
        }
        if holder != agent_id && agent_id != "*" {
            return Err(format!("heartbeat failed: claim {cid} is held by {holder}"));
        }
        if !local_claim_active(&*claim, now_s) {
            return Err(format!(
                "heartbeat failed: claim {cid} is expired or released (re-claim instead)"
            ));
        }
        claim["temporal"]["started_at_unix_s"] = serde_json::json!(now_s);
        let ttl = claim
            .get("temporal")
            .and_then(|t| t.get("ttl_seconds"))
            .and_then(|v| v.as_u64())
            .unwrap_or(1800);
        let actions = claim
            .get("budget")
            .and_then(|b| b.get("actions"))
            .and_then(|v| v.as_u64())
            .unwrap_or(50);
        let wall = claim
            .get("budget")
            .and_then(|b| b.get("wall_seconds"))
            .and_then(|v| v.as_u64())
            .unwrap_or(3600);
        return Ok((cid, kind, ttl, actions, wall));
    }
    Err(format!(
        "heartbeat failed: no active local claim for target {target}"
    ))
}
