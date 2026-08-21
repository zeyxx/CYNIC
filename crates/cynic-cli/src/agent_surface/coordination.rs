//! Coordination execution layer — thin HTTP projection over kernel REST.
//!
//! This module performs coordination operations (claim, release, status,
//! identify) by calling the kernel's REST endpoints. It owns no policy —
//! all decisions happen in the kernel. This is a thin HTTP client.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::agent_surface::types::{
    AgentClaimReport, AgentClaimSummary, AgentIdentifyReport, AgentReleaseReport,
    AgentStatusReport, AGENT_CLAIM_REPORT_SCHEMA, AGENT_IDENTIFY_REPORT_SCHEMA,
    AGENT_RELEASE_REPORT_SCHEMA, AGENT_STATUS_REPORT_SCHEMA,
};

const KERNEL_BASE: &str = "http://localhost:8080";

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Declare agent identity.
pub fn execute_identify(agent_id: &str, agent_kind: Option<&str>) -> Result<AgentIdentifyReport, String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/coord/identify", KERNEL_BASE))
        .json(&serde_json::json!({
            "agent_id": agent_id,
            "agent_kind": agent_kind,
        }))
        .send()
        .map_err(|e| format!("kernel unreachable: {}", e))?;

    let status = resp.status();
    let body: Value = resp.json().map_err(|e| format!("invalid JSON: {}", e))?;

    if !status.is_success() {
        let error = body.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
        return Err(format!("identify failed: {}", error));
    }

    Ok(AgentIdentifyReport {
        schema: AGENT_IDENTIFY_REPORT_SCHEMA,
        status: "identified",
        agent_id: agent_id.to_string(),
        agent_kind: body
            .get("agent_kind")
            .and_then(|v| v.as_str())
            .unwrap_or("external")
            .to_string(),
        credential_kind: body
            .get("credential_kind")
            .and_then(|v| v.as_str())
            .unwrap_or("opaque_reference")
            .to_string(),
        attested_at_unix_ms: body
            .get("attested_at_unix_ms")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| now_unix_ms()),
    })
}

/// Register agent with kernel.
pub fn execute_register(agent_id: &str, intent: &str) -> Result<(), String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/coord/register", KERNEL_BASE))
        .json(&serde_json::json!({
            "agent_id": agent_id,
            "intent": intent,
        }))
        .send()
        .map_err(|e| format!("kernel unreachable: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body: Value = resp.json().map_err(|e| format!("invalid JSON: {}", e))?;
        let error = body.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
        return Err(format!("register failed: {}", error));
    }

    Ok(())
}

/// Claim a scope.
pub fn execute_claim(
    agent_id: &str,
    target: &str,
    claim_type: Option<&str>,
    ttl_seconds: Option<u64>,
    budget_actions: Option<u64>,
    budget_wall_seconds: Option<u64>,
) -> Result<AgentClaimReport, String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/coord/claim", KERNEL_BASE))
        .json(&serde_json::json!({
            "agent_id": agent_id,
            "target": target,
            "claim_type": claim_type,
            "ttl_seconds": ttl_seconds,
            "budget_actions": budget_actions,
            "budget_wall_seconds": budget_wall_seconds,
        }))
        .send()
        .map_err(|e| format!("kernel unreachable: {}", e))?;

    let status = resp.status();
    let body: Value = resp.json().map_err(|e| format!("invalid JSON: {}", e))?;

    if !status.is_success() {
        let error = body.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
        return Err(format!("claim failed: {}", error));
    }

    let lease = body.get("lease").cloned().unwrap_or_default();

    Ok(AgentClaimReport {
        schema: AGENT_CLAIM_REPORT_SCHEMA,
        status: "claimed",
        agent_id: agent_id.to_string(),
        target: target.to_string(),
        claim_type: claim_type.unwrap_or("file").to_string(),
        lease_id: lease
            .get("lease_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        fencing_token: lease.get("fencing_token").and_then(|v| v.as_u64()).unwrap_or(0),
        ttl_seconds: body.get("ttl_seconds").and_then(|v| v.as_u64()).unwrap_or(1800),
        budget_actions: body.get("budget_actions").and_then(|v| v.as_u64()).unwrap_or(50),
        budget_wall_seconds: body
            .get("budget_wall_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600),
        granted_at_unix_ms: now_unix_ms(),
        expires_at_unix_ms: now_unix_ms()
            + body.get("ttl_seconds").and_then(|v| v.as_u64()).unwrap_or(1800) * 1000,
    })
}

/// Release a claim.
pub fn execute_release(
    agent_id: &str,
    target: &str,
    lease_id: &str,
    fencing_token: u64,
    proof_refs: Vec<String>,
) -> Result<AgentReleaseReport, String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/coord/release", KERNEL_BASE))
        .json(&serde_json::json!({
            "agent_id": agent_id,
            "target": target,
            "lease_id": lease_id,
            "fencing_token": fencing_token,
            "proof_refs": proof_refs,
        }))
        .send()
        .map_err(|e| format!("kernel unreachable: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body: Value = resp.json().map_err(|e| format!("invalid JSON: {}", e))?;
        let error = body.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
        return Err(format!("release failed: {}", error));
    }

    Ok(AgentReleaseReport {
        schema: AGENT_RELEASE_REPORT_SCHEMA,
        status: "released",
        agent_id: agent_id.to_string(),
        target: target.to_string(),
        lease_id: lease_id.to_string(),
        proof_refs,
        released_at_unix_ms: now_unix_ms(),
    })
}

/// Query active claims status.
pub fn execute_status(agent_id: &str) -> Result<AgentStatusReport, String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(format!("{}/coord/who?agent_id={}", KERNEL_BASE, agent_id))
        .send()
        .map_err(|e| format!("kernel unreachable: {}", e))?;

    let status = resp.status();
    let body: Value = resp.json().map_err(|e| format!("invalid JSON: {}", e))?;

    if !status.is_success() {
        let error = body.get("error").and_then(|v| v.as_str()).unwrap_or("unknown");
        return Err(format!("status failed: {}", error));
    }

    let claims = body
        .get("claims")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let active_claims: Vec<AgentClaimSummary> = claims
        .iter()
        .filter_map(|c| {
            let granted = c.get("granted_at_unix_ms").and_then(|v| v.as_u64()).unwrap_or(0);
            let expires = c.get("expires_at_unix_ms").and_then(|v| v.as_u64()).unwrap_or(0);
            let remaining = if expires > now_unix_ms() {
                ((expires - now_unix_ms()) / 1000) as i64
            } else {
                0
            };
            Some(AgentClaimSummary {
                claim_id: c
                    .get("claim_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                target: c
                    .get("target")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string(),
                claim_type: c
                    .get("claim_type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("file")
                    .to_string(),
                granted_at_unix_ms: granted,
                expires_at_unix_ms: expires,
                remaining_seconds: remaining,
            })
        })
        .collect();

    let total = active_claims.len();

    Ok(AgentStatusReport {
        schema: AGENT_STATUS_REPORT_SCHEMA,
        status: if total > 0 { "active" } else { "idle" },
        agent_id: agent_id.to_string(),
        active_claims,
        total_claims: total,
    })
}
