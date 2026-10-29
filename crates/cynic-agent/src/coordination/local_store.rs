//! Local persistence adapter for claims document and event log (I/O).

use std::time::{SystemTime, UNIX_EPOCH};
use serde_json::Value;

use crate::coordination::events::civil_from_days;
use crate::coordination::types::{AgentClaimSummary, AgentStatusReport, AGENT_STATUS_REPORT_SCHEMA};
use cynic_process::ArgvRunner;

pub const CLAIM_EVENT_SCHEMA: &str = "cynic.agent.claim_event.v1";

pub fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub fn now_unix_s() -> u64 {
    now_unix_ms() / 1000
}

pub fn claims_file_path() -> std::path::PathBuf {
    let output = cynic_process::LocalArgvRunner.run(&["git", "rev-parse", "--show-toplevel"]);
    if output.success() && !output.stdout.trim().is_empty() {
        return std::path::PathBuf::from(output.stdout.trim()).join(".agent-claims.json");
    }
    std::path::PathBuf::from(".agent-claims.json")
}

pub fn load_claims_doc() -> Result<Value, String> {
    let path = claims_file_path();
    if !path.exists() {
        return Ok(serde_json::json!({
            "schema": "cynic.agent_claims.v2",
            "revision": 0,
            "claims": [],
            "merge_queue": [],
        }));
    }
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read .agent-claims.json: {e}"))?;
    serde_json::from_str(&content).map_err(|e| format!("failed to parse .agent-claims.json: {e}"))
}

pub fn store_claims_doc(doc: &mut Value) -> Result<u64, String> {
    let revision = doc.get("revision").and_then(|v| v.as_u64()).unwrap_or(0) + 1;
    doc["revision"] = serde_json::json!(revision);
    let content = serde_json::to_string_pretty(doc)
        .map_err(|e| format!("failed to serialize .agent-claims.json: {e}"))?;
    let path = claims_file_path();
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, content).map_err(|e| format!("failed to write claims tmp file: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("failed to commit claims file: {e}"))?;
    Ok(revision)
}

pub fn claim_event_log_dir() -> Option<std::path::PathBuf> {
    let output = cynic_process::LocalArgvRunner.run(&["git", "rev-parse", "--show-toplevel"]);
    let root = if output.success() && !output.stdout.trim().is_empty() {
        output.stdout.trim().to_string()
    } else {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    };
    if root.is_empty() {
        return None;
    }
    let dir = std::path::PathBuf::from(root)
        .join("data_system")
        .join("vault")
        .join("captures");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

pub fn append_claim_event(event_type: &str, agent_id: &str, target: &str, detail: Value) {
    let Some(dir) = claim_event_log_dir() else {
        return;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days_since_epoch = now / 86_400;
    let (y, m, d) = civil_from_days(days_since_epoch as i64);
    let filename = format!("agent-claims-{y:04}-{m:02}-{d:02}.jsonl");
    let path = dir.join(filename);

    let event = serde_json::json!({
        "schema": CLAIM_EVENT_SCHEMA,
        "event": event_type,
        "agent_id": agent_id,
        "target": target,
        "timestamp_unix_s": now,
        "detail": detail,
    });
    if let Ok(line) = serde_json::to_string(&event) {
        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "{line}")
            });
    }
}

pub fn execute_status_from_claims_file(agent_id: &str) -> Result<AgentStatusReport, String> {
    use crate::coordination::admission::local_claim_active;
    let path = std::path::Path::new(".agent-claims.json");
    if !path.exists() {
        return Ok(AgentStatusReport {
            schema: AGENT_STATUS_REPORT_SCHEMA,
            status: "idle",
            agent_id: agent_id.to_string(),
            active_claims: Vec::new(),
            total_claims: 0,
        });
    }

    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read .agent-claims.json: {e}"))?;
    let json: Value = serde_json::from_str(&content)
        .map_err(|e| format!("failed to parse .agent-claims.json: {e}"))?;

    let claims = json
        .get("claims")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    let now_ms = now_unix_ms();
    let active_claims: Vec<AgentClaimSummary> = claims
        .iter()
        .filter_map(|c| {
            let cid = c.get("claim_id")?.as_str()?.to_string();
            let aid = c.get("agent_id")?.as_str()?;
            if !local_claim_active(c, now_ms / 1000) {
                return None;
            }
            if !agent_id.is_empty() && aid != agent_id && agent_id != "*" {
                return None;
            }

            let scope = c.get("scope");
            let target = scope
                .and_then(|s| s.get("path"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let claim_type = scope
                .and_then(|s| s.get("kind"))
                .and_then(|v| v.as_str())
                .unwrap_or("file")
                .to_string();

            let temporal = c.get("temporal");
            let started_s = temporal
                .and_then(|t| t.get("started_at_unix_s"))
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let ttl_s = temporal
                .and_then(|t| t.get("ttl_seconds"))
                .and_then(|v| v.as_u64())
                .unwrap_or(1800);

            let granted_ms = started_s * 1000;
            let expires_ms = (started_s + ttl_s) * 1000;
            let remaining = if expires_ms > now_ms {
                ((expires_ms - now_ms) / 1000) as i64
            } else {
                0
            };

            Some(AgentClaimSummary {
                claim_id: cid,
                target,
                claim_type,
                granted_at_unix_ms: granted_ms,
                expires_at_unix_ms: expires_ms,
                remaining_seconds: remaining,
            })
        })
        .collect();

    let total = active_claims.len();
    append_claim_event(
        "status_query",
        agent_id,
        "*",
        serde_json::json!({
            "total_claims": total,
            "source": "local_file",
        }),
    );
    Ok(AgentStatusReport {
        schema: AGENT_STATUS_REPORT_SCHEMA,
        status: if total > 0 { "active" } else { "idle" },
        agent_id: agent_id.to_string(),
        active_claims,
        total_claims: total,
    })
}
