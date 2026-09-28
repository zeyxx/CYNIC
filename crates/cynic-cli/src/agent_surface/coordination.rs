//! Coordination execution layer — thin HTTP projection over kernel REST.
//!
//! This module performs coordination operations (claim, release, status,
//! identify) by calling the kernel's REST endpoints. It owns no policy —
//! all decisions happen in the kernel. This is a thin HTTP client.

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::agent_surface::types::{
    AGENT_CLAIM_REPORT_SCHEMA, AGENT_IDENTIFY_REPORT_SCHEMA, AGENT_RELEASE_REPORT_SCHEMA,
    AGENT_STATUS_REPORT_SCHEMA, AgentClaimReport, AgentClaimSummary, AgentIdentifyReport,
    AgentReleaseReport, AgentStatusReport,
};

use cynic_protocol::agent_coordination::{ClaimScope, predicates as coord_predicates};

/// Resolve the kernel REST base URL.
///
/// Precedence: explicit `--kernel-base` > `CYNIC_KERNEL_BASE` env > registry
/// (`nodes[]` with `role == "kernel_primary"`, service `cynic-kernel`,
/// `{scheme}://{endpoint_host}:{port}`). Returns `None` when nothing
/// resolves — callers then use the local claims projection directly,
/// same degraded mode as a kernel transport failure. Never hardcodes
/// a host, IP, or port.
pub(crate) fn resolve_kernel_base(
    registry_path: &str,
    override_base: Option<&str>,
) -> Option<String> {
    if let Some(base) = override_base.filter(|s| !s.trim().is_empty()) {
        return Some(base.trim_end_matches('/').to_string());
    }
    if let Ok(env) = std::env::var("CYNIC_KERNEL_BASE") {
        if !env.trim().is_empty() {
            return Some(env.trim_end_matches('/').to_string());
        }
    }
    let content = std::fs::read_to_string(registry_path).ok()?;
    let doc: Value = serde_json::from_str(&content).ok()?;
    let nodes = doc.get("nodes")?.as_array()?;
    let node = nodes
        .iter()
        .find(|n| n.get("role").and_then(|r| r.as_str()) == Some("kernel_primary"))?;
    let services = node.get("services")?.as_array()?;
    let svc = services
        .iter()
        .find(|s| s.get("name").and_then(|n| n.as_str()) == Some("cynic-kernel"))?;
    let scheme = svc.get("scheme").and_then(|s| s.as_str()).unwrap_or("http");
    if scheme != "http" && scheme != "https" {
        return None;
    }
    let host = svc.get("endpoint_host")?.as_str()?;
    let port = svc.get("port").and_then(|p| p.as_u64())?;
    Some(format!("{scheme}://{host}:{port}"))
}

/// Resolve the build-runner endpoint host from the registry.
///
/// Reads `nodes[]` with `role == "build_runner"`, first service
/// `endpoint_host`. Returns `None` when the registry is unreadable —
/// callers report `unknown`, never a hardcoded IP.
pub(crate) fn resolve_builder_endpoint(registry_path: &str) -> Option<String> {
    let content = std::fs::read_to_string(registry_path).ok()?;
    let doc: Value = serde_json::from_str(&content).ok()?;
    let nodes = doc.get("nodes")?.as_array()?;
    let node = nodes
        .iter()
        .find(|n| n.get("role").and_then(|r| r.as_str()) == Some("build_runner"))?;
    let services = node.get("services")?.as_array()?;
    let host = services
        .iter()
        .filter_map(|s| s.get("endpoint_host").and_then(|h| h.as_str()))
        .find(|h| !h.trim().is_empty())?;
    Some(host.to_string())
}

/// Resolve the git remote from the live repository.
///
/// Runs `git remote get-url origin`; returns `"unknown"` when git is
/// unavailable or has no origin — never a hardcoded URL.
pub(crate) fn resolve_git_remote() -> String {
    std::process::Command::new("git")
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty())
        .unwrap_or("unknown".to_string())
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Declare agent identity.
pub(crate) fn execute_identify(
    agent_id: &str,
    agent_kind: Option<&str>,
    kernel_base: Option<&str>,
) -> Result<AgentIdentifyReport, String> {
    let Some(base) = kernel_base else {
        // No kernel reachable or resolvable: self-attested local identity.
        // credential_kind marks the absence of kernel attestation explicitly.
        return Ok(AgentIdentifyReport {
            schema: AGENT_IDENTIFY_REPORT_SCHEMA,
            status: "identified",
            agent_id: agent_id.to_string(),
            agent_kind: agent_kind.unwrap_or("external").to_string(),
            credential_kind: "local_unattested".to_string(),
            attested_at_unix_ms: now_unix_ms(),
        });
    };
    let client = kernel_client();
    let resp = client
        .post(format!("{base}/coord/identify"))
        .json(&serde_json::json!({
            "agent_id": agent_id,
            "agent_kind": agent_kind,
        }))
        .send()
        .map_err(|e| format!("kernel unreachable: {}", e))?;

    let status = resp.status();
    let body: Value = resp.json().map_err(|e| format!("invalid JSON: {}", e))?;

    if !status.is_success() {
        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
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
pub(crate) fn execute_register(
    agent_id: &str,
    intent: &str,
    kernel_base: Option<&str>,
) -> Result<(), String> {
    let Some(base) = kernel_base else {
        return Err(
            "kernel unreachable: no kernel base resolved (registry or --kernel-base)".to_string(),
        );
    };
    let client = kernel_client();
    let resp = client
        .post(format!("{base}/coord/register"))
        .json(&serde_json::json!({
            "agent_id": agent_id,
            "intent": intent,
        }))
        .send()
        .map_err(|e| format!("kernel unreachable: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let body: Value = resp.json().map_err(|e| format!("invalid JSON: {}", e))?;
        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        return Err(format!("register failed: {}", error));
    }

    Ok(())
}

/// Claim a scope.
pub(crate) fn execute_claim(
    agent_id: &str,
    target: &str,
    claim_type: Option<&str>,
    ttl_seconds: Option<u64>,
    budget_actions: Option<u64>,
    budget_wall_seconds: Option<u64>,
    kernel_base: Option<&str>,
) -> Result<AgentClaimReport, String> {
    let claim_local = || {
        execute_claim_local(
            agent_id,
            target,
            claim_type,
            ttl_seconds,
            budget_actions,
            budget_wall_seconds,
        )
    };
    let Some(base) = kernel_base else {
        return claim_local();
    };
    let client = kernel_client();
    let resp = match client
        .post(format!("{base}/coord/claim"))
        .json(&serde_json::json!({
            "agent_id": agent_id,
            "target": target,
            "claim_type": claim_type,
            "ttl_seconds": ttl_seconds,
            "budget_actions": budget_actions,
            "budget_wall_seconds": budget_wall_seconds,
        }))
        .send()
    {
        Ok(resp) => resp,
        Err(_) => {
            return claim_local();
        }
    };

    let status = resp.status();
    let body: Value = resp.json().map_err(|e| format!("invalid JSON: {}", e))?;

    if !status.is_success() {
        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
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
        fencing_token: lease
            .get("fencing_token")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        ttl_seconds: body
            .get("ttl_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(1800),
        budget_actions: body
            .get("budget_actions")
            .and_then(|v| v.as_u64())
            .unwrap_or(50),
        budget_wall_seconds: body
            .get("budget_wall_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(3600),
        granted_at_unix_ms: now_unix_ms(),
        expires_at_unix_ms: now_unix_ms()
            + body
                .get("ttl_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(1800)
                * 1000,
    })
}

/// Release a claim.
pub(crate) fn execute_release(
    agent_id: &str,
    target: &str,
    lease_id: &str,
    fencing_token: u64,
    proof_refs: Vec<String>,
    kernel_base: Option<&str>,
) -> Result<AgentReleaseReport, String> {
    let Some(base) = kernel_base else {
        return execute_release_local(agent_id, target, lease_id, proof_refs);
    };
    let client = kernel_client();
    let resp = match client
        .post(format!("{base}/coord/release"))
        .json(&serde_json::json!({
            "agent_id": agent_id,
            "target": target,
            "lease_id": lease_id,
            "fencing_token": fencing_token,
            "proof_refs": proof_refs,
        }))
        .send()
    {
        Ok(resp) => resp,
        // Local file carries no fencing token; the kernel stays the fencing
        // authority whenever it is reachable.
        Err(_) => return execute_release_local(agent_id, target, lease_id, proof_refs),
    };

    let status = resp.status();
    if !status.is_success() {
        let body: Value = resp.json().map_err(|e| format!("invalid JSON: {}", e))?;
        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
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

/// WHY: the kernel REST plane (port 8080) is often down in local/dev sessions;
/// claim/release/status must keep working against the local projection
/// (.agent-claims.json) instead of failing with `kernel unreachable`.
/// HOW: short-timeout kernel attempt first; transport failure falls back to a
/// monotone local CAS on the claims file (revision + 1, tmp + rename).
fn kernel_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(300))
        .build()
        .unwrap_or_default()
}

fn now_unix_s() -> u64 {
    now_unix_ms() / 1000
}

fn claims_file_path() -> std::path::PathBuf {
    // Anchor to the enclosing git worktree so a claim from a subdirectory
    // lands in the same projection as every other agent. Fall back to the
    // cwd when outside git (explicit unknown, same file name).
    if let Ok(output) = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
    {
        if output.status.success() {
            let root = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !root.is_empty() {
                return std::path::PathBuf::from(root).join(".agent-claims.json");
            }
        }
    }
    std::path::PathBuf::from(".agent-claims.json")
}

fn load_claims_doc() -> Result<serde_json::Value, String> {
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

/// Atomic monotone write: revision + 1, tmp file + rename.
fn store_claims_doc(doc: &mut serde_json::Value) -> Result<u64, String> {
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

/// Map a CLI (kind, path) pair onto the canonical protocol scope so overlap is
/// decided by `cynic_protocol` predicates, never by a local copy of the rule.
fn local_scope_of(kind: &str, target: &str) -> ClaimScope {
    match kind {
        "crate" => ClaimScope::Crate {
            crate_name: target.strip_prefix("crates/").unwrap_or(target).to_string(),
        },
        _ => ClaimScope::File {
            path: target.to_string(),
        },
    }
}

fn local_claim_active(claim: &serde_json::Value, now_s: u64) -> bool {
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

/// Local CAS claim: first-wins on overlapping active scopes, idempotent
/// re-claim by the same agent on the same scope.
fn execute_claim_local(
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
fn execute_release_local(
    agent_id: &str,
    target: &str,
    lease_id: &str,
    proof_refs: Vec<String>,
) -> Result<AgentReleaseReport, String> {
    let mut doc = load_claims_doc()?;
    let now_ms = now_unix_ms();
    let mut matched_lease = String::new();
    let mut already_released = false;
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
            if by_lease || by_target {
                if holder != agent_id && agent_id != "*" {
                    return Err(format!("release failed: claim {cid} is held by {holder}"));
                }
                matched_lease = cid;
                if claim.get("status").and_then(|v| v.as_str()) == Some("released") {
                    already_released = true;
                } else {
                    claim["status"] = serde_json::json!("released");
                }
                break;
            }
        }
    }
    if matched_lease.is_empty() {
        return Err(format!(
            "release failed: no local claim for target {target}"
        ));
    }
    if !already_released {
        store_claims_doc(&mut doc)?;
    }
    Ok(AgentReleaseReport {
        schema: AGENT_RELEASE_REPORT_SCHEMA,
        status: "released",
        agent_id: agent_id.to_string(),
        target: target.to_string(),
        lease_id: if lease_id.is_empty() {
            matched_lease
        } else {
            lease_id.to_string()
        },
        proof_refs,
        released_at_unix_ms: now_ms,
    })
}

/// Query active claims status.
pub(crate) fn execute_status(
    agent_id: &str,
    kernel_base: Option<&str>,
) -> Result<AgentStatusReport, String> {
    let Some(base) = kernel_base else {
        return execute_status_from_claims_file(agent_id);
    };
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(300))
        .build()
        .unwrap_or_default();
    let resp = client
        .get(format!("{base}/coord/who?agent_id={agent_id}"))
        .send();

    if resp.is_err() {
        return execute_status_from_claims_file(agent_id);
    }
    let resp = resp.map_err(|e| format!("kernel unreachable: {}", e))?;

    let status = resp.status();
    let body: Value = resp.json().map_err(|e| format!("invalid JSON: {}", e))?;

    if !status.is_success() {
        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
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
            let granted = c
                .get("granted_at_unix_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let expires = c
                .get("expires_at_unix_ms")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            // Display honesty (2026-09-28): a claim past its expiry is not
            // active, even if nothing flipped its status string. Listing it
            // with remaining 0 inflated totals and faked liveness.
            if expires <= now_unix_ms() {
                return None;
            };
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

fn execute_status_from_claims_file(agent_id: &str) -> Result<AgentStatusReport, String> {
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
    let json: serde_json::Value = serde_json::from_str(&content)
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
            // Display honesty (2026-09-28): same liveness rule as the
            // conflict path (local_claim_active) — status string alone is
            // not liveness. Expired entries are data for the audit trail,
            // not members of the active set.
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
    Ok(AgentStatusReport {
        schema: AGENT_STATUS_REPORT_SCHEMA,
        status: if total > 0 { "active" } else { "idle" },
        agent_id: agent_id.to_string(),
        active_claims,
        total_claims: total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_registry() -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("cynic-agent-resolve-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("registry.json");
        std::fs::write(
            &path,
            serde_json::json!({
                "nodes": [
                    {
                        "id": "k",
                        "role": "kernel_primary",
                        "services": [
                            {"name": "cynic-kernel", "scheme": "http",
                             "endpoint_host": "192.168.100.2", "port": 3030}
                        ]
                    },
                    {
                        "id": "b",
                        "role": "build_runner",
                        "services": [{"name": "cynic-node", "endpoint_host": "10.10.10.119"}]
                    }
                ],
                "provider_control": {"endpoint": "https://pve.synapse.cynic:8006"}
            })
            .to_string(),
        )
        .expect("fixture");
        path
    }

    #[test]
    fn kernel_base_resolves_from_registry() {
        let path = fixture_registry();
        let base = resolve_kernel_base(path.to_str().unwrap(), None).expect("base");
        assert_eq!(base, "http://192.168.100.2:3030");
    }

    #[test]
    fn explicit_override_wins_over_registry() {
        let path = fixture_registry();
        let base = resolve_kernel_base(path.to_str().unwrap(), Some("http://127.0.0.1:9999/"))
            .expect("base");
        assert_eq!(base, "http://127.0.0.1:9999");
    }

    #[test]
    fn missing_registry_yields_none() {
        assert!(resolve_kernel_base("/nonexistent/registry.json", None).is_none());
    }

    #[test]
    fn builder_endpoint_resolves_from_registry() {
        let path = fixture_registry();
        let host = resolve_builder_endpoint(path.to_str().unwrap()).expect("host");
        assert_eq!(host, "10.10.10.119");
    }

    #[test]
    fn git_remote_never_empty() {
        assert!(!resolve_git_remote().is_empty());
    }

    fn claim_entry(status: &str, started_at_unix_s: u64, ttl_seconds: u64) -> serde_json::Value {
        serde_json::json!({
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
        assert!(local_claim_active(
            &claim_entry("active", now - 100, 1800),
            now
        ));
        // Past TTL with status still "active" is NOT live (the ghost case:
        // status string alone faked liveness across 69 entries).
        assert!(!local_claim_active(
            &claim_entry("active", now - 2000, 1800),
            now
        ));
        assert!(!local_claim_active(
            &claim_entry("released", now - 100, 1800),
            now
        ));
        assert!(!local_claim_active(
            &claim_entry("expired", now - 100, 1800),
            now
        ));
        // Boundary: exactly at expiry is dead (now < started + ttl is strict).
        assert!(!local_claim_active(
            &claim_entry("active", now - 1800, 1800),
            now
        ));
        assert!(local_claim_active(
            &claim_entry("active", now - 1799, 1800),
            now
        ));
    }
}
