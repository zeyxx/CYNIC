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
        append_claim_event(
            "identify_local",
            agent_id,
            "*",
            serde_json::json!({"credential_kind": "local_unattested"}),
        );
        return Ok(AgentIdentifyReport {
            schema: AGENT_IDENTIFY_REPORT_SCHEMA,
            status: "identified",
            agent_id: agent_id.to_string(),
            agent_kind: agent_kind.unwrap_or("external").to_string(),
            credential_kind: "local_unattested".to_string(),
            attested_at_unix_ms: now_unix_ms(),
        });
    };
    let url = format!("{base}/coord/identify");
    let payload = serde_json::json!({
        "agent_id": agent_id,
        "agent_kind": agent_kind,
    });
    let resp = blocking_kernel_request(move || {
        let client = kernel_client();
        client
            .post(&url)
            .json(&payload)
            .send()
            .map_err(|e| format!("kernel unreachable: {e}"))
    })?;

    let status = resp.status();
    let body: Value = resp.json().map_err(|e| format!("invalid JSON: {e}"))?;

    if !status.is_success() {
        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        return Err(format!("identify failed: {error}"));
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
    let url = format!("{base}/coord/register");
    let payload = serde_json::json!({
        "agent_id": agent_id,
        "intent": intent,
    });
    let resp = blocking_kernel_request(move || {
        let client = kernel_client();
        client
            .post(&url)
            .json(&payload)
            .send()
            .map_err(|e| format!("kernel unreachable: {e}"))
    })?;

    let status = resp.status();
    if is_kernel_auth_refusal(status.as_u16()) {
        warn_kernel_auth_refusal("register");
        return Err(
            "register is a kernel-hosted operation, not a node-local arbitration: the kernel \
             refused credentials and there is no local projection for it (explicit unknown)."
                .to_string(),
        );
    }
    if !status.is_success() {
        let body: Value = resp.json().map_err(|e| format!("invalid JSON: {e}"))?;
        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        return Err(format!("register failed: {error}"));
    }

    append_claim_event(
        "register",
        agent_id,
        "*",
        serde_json::json!({"intent": intent}),
    );
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
    let url = format!("{base}/coord/claim");
    let payload = serde_json::json!({
        "agent_id": agent_id,
        "target": target,
        "claim_type": claim_type,
        "ttl_seconds": ttl_seconds,
        "budget_actions": budget_actions,
        "budget_wall_seconds": budget_wall_seconds,
    });
    let resp = match blocking_kernel_request(move || {
        let client = kernel_client();
        client
            .post(&url)
            .json(&payload)
            .send()
            .map_err(|e| format!("kernel unreachable: {e}"))
    }) {
        Ok(resp) => resp,
        Err(_) => {
            return claim_local();
        }
    };

    let status = resp.status();
    // PROTOCOL EXCEPTION (2026-09-28, operator-admitted): a node-local
    // arbitration call must not be abandoned because the kernel refuses
    // credentials. The kernel authenticates PEERS (federation); it does not
    // authenticate its own local mutations. A 401/403 on `/coord/*` is an
    // explicit unknown about the kernel surface, not a verdict — so it falls
    // back to the local arbiter instead of failing the whole command. Before
    // this, `cynic agent claim|release|status` failed outright, which made the
    // session protocol ("mutate only inside a claimed scope") unexecutable
    // and stranded every live claim.
    if is_kernel_auth_refusal(status.as_u16()) {
        warn_kernel_auth_refusal("claim");
        return claim_local();
    }
    let body: Value = resp.json().map_err(|e| format!("invalid JSON: {e}"))?;

    if !status.is_success() {
        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        return Err(format!("claim failed: {error}"));
    }

    let lease = body.get("lease").cloned().unwrap_or_default();
    let lease_id = lease
        .get("lease_id")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let fencing_token = lease
        .get("fencing_token")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    append_claim_event(
        "claim_kernel",
        agent_id,
        target,
        serde_json::json!({
            "lease_id": lease_id,
            "fencing_token": fencing_token,
            "claim_type": claim_type.unwrap_or("file"),
        }),
    );

    Ok(AgentClaimReport {
        schema: AGENT_CLAIM_REPORT_SCHEMA,
        status: "claimed",
        agent_id: agent_id.to_string(),
        target: target.to_string(),
        claim_type: claim_type.unwrap_or("file").to_string(),
        lease_id,
        fencing_token,
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
    let url = format!("{base}/coord/release");
    let payload = serde_json::json!({
        "agent_id": agent_id,
        "target": target,
        "lease_id": lease_id,
        "fencing_token": fencing_token,
        "proof_refs": proof_refs,
    });
    let resp = match blocking_kernel_request(move || {
        let client = kernel_client();
        client
            .post(&url)
            .json(&payload)
            .send()
            .map_err(|e| format!("kernel unreachable: {e}"))
    }) {
        Ok(resp) => resp,
        // Local file carries no fencing token; the kernel stays the fencing
        // authority whenever it is reachable.
        Err(_) => return execute_release_local(agent_id, target, lease_id, proof_refs),
    };

    let status = resp.status();
    if is_kernel_auth_refusal(status.as_u16()) {
        warn_kernel_auth_refusal("release");
        return execute_release_local(agent_id, target, lease_id, proof_refs);
    }
    if !status.is_success() {
        let body: Value = resp.json().map_err(|e| format!("invalid JSON: {e}"))?;
        let error = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        return Err(format!("release failed: {error}"));
    }

    append_claim_event(
        "release_kernel",
        agent_id,
        target,
        serde_json::json!({
            "lease_id": lease_id,
            "proof_refs": proof_refs,
        }),
    );

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
///
/// RUNTIME SAFETY: the CLI runs under `#[tokio::main]`, so a
/// `reqwest::blocking::Client` must never be created, used, or dropped on a
/// tokio worker thread — dropping its internal runtime from within an async
/// context panics with "Cannot drop a runtime in a context where blocking is
/// not allowed" (tokio-1.53). Mirror the `prove_build_blocking` convention:
/// build the blocking client inside `spawn_blocking` so the entire lifecycle
/// (construction, request, response parsing, and drop) happens on a dedicated
/// blocking thread outside the async runtime.
fn kernel_client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_millis(300))
        .build()
        .unwrap_or_default()
}

/// Perform a blocking kernel HTTP request on a dedicated OS thread.
///
/// WHY: the CLI runs inside a tokio runtime, and `reqwest::blocking::Client`
/// panics when its internal runtime is dropped on or near a tokio worker.
/// `block_in_place` + `spawn_blocking` + `block_on` is NOT sufficient — the
/// hybrid context still trips tokio's blocking shutdown guard. A raw
/// `std::thread::spawn` confines the client's entire lifecycle (construction,
/// request, response, and drop) to a plain OS thread with zero tokio context.
/// Falls back to synchronous execution when no tokio runtime is present
/// (unit tests).
fn blocking_kernel_request<F, T>(f: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String> + Send + 'static,
    T: Send + 'static,
{
    if tokio::runtime::Handle::try_current().is_err() {
        return f();
    }
    let handle = std::thread::spawn(f);
    handle
        .join()
        .map_err(|_| "blocking kernel request panicked on worker thread".to_string())?
}

// ─── Claim event log ───────────────────────────────────────────────────────
//
// WHY: the claim lifecycle must produce durable, timestamped evidence — not
// just a current-state snapshot. A bare `.agent-claims.json` cannot answer
// "why did coordination fail?" after the fact. Every claim, release, TTL
// expiry, and kernel fallback must persist as an append-only record so the
// system observes its own failures through persisted events, not through the
// good behaviour of the agents it coordinates.
//
// HOW: one JSONL file per day at `data_system/vault/captures/agent-claims-YYYY-MM-DD.jsonl`.
// `*.jsonl` is already gitignored (`.gitignore:81`), so the log lives outside
// Git by construction. Each line is a self-contained JSON object validated
// against `contracts/agent-claim-event.json`.

/// Schema identifier for the claim event log.
pub(crate) const CLAIM_EVENT_SCHEMA: &str = "cynic.agent.claim_event.v1";

/// Resolve the event log directory from the workspace root.
///
/// Walks up from the current directory to find the git root, then uses
/// `data_system/vault/captures/`. Returns the captures directory path
/// (the caller composes the dated filename).
fn claim_event_log_dir() -> Option<std::path::PathBuf> {
    // Try git root first; fall back to current directory.
    let root = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| {
            let s = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if s.is_empty() {
                None
            } else {
                Some(s)
            }
        })
        .unwrap_or_else(|| {
            std::env::current_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default()
        });
    if root.is_empty() {
        return None;
    }
    let dir = std::path::PathBuf::from(root)
        .join("data_system")
        .join("vault")
        .join("captures");
    // Best-effort directory creation; the event log must never block a
    // claim or release. If the directory cannot be created, the event
    // is silently dropped (the local claims file still records state).
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir)
}

/// Append one claim event to the daily JSONL log.
///
/// WHY: coordination failures must be observable after the fact. An agent
/// that ignores the claim protocol or forgets to release leaves no trace
/// in a state-only snapshot. The event log is the durable audit trail.
///
/// HOW: append a single JSON line to `agent-claims-<YYYY-MM-DD>.jsonl`.
/// Never reads the file — append-only, O(1) per event, no lock needed
/// for single-writer CLI invocations.
pub(crate) fn append_claim_event(
    event_type: &str,
    agent_id: &str,
    target: &str,
    detail: serde_json::Value,
) {
    let Some(dir) = claim_event_log_dir() else {
        return;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    // Date in UTC from unix seconds (no chrono dependency).
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
        // Best-effort append: never block the caller.
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

/// Convert days-since-epoch to (year, month, day) using the civil calendar
/// algorithm (Howard Hinnant). No chrono dependency needed for the CLI.
fn civil_from_days(z: i64) -> (i64, u64, u64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365; // [0, 399]
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // [0, 365]
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = doy - (153 * mp + 2) / 5 + 1; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 }; // [1, 12]
    (if m <= 2 { y + 1 } else { y }, m, d)
}


/// The kernel guards PEER surfaces with credentials. Node-local arbitration —
/// claim, release, status on this node — is not a peer surface, so an auth
/// refusal there is a boundary defect, not a verdict on the caller.
fn is_kernel_auth_refusal(status: u16) -> bool {
    status == 401 || status == 403
}

/// Say it out loud. A fallback nobody announces is a silent downgrade, which is
/// the same defect class as a silent `None`.
fn warn_kernel_auth_refusal(operation: &str) {
    eprintln!(
        "cynic: kernel refused credentials for /coord/{operation} (401/403) — continuing on the \
         node-local arbiter. The kernel did not rule on this call."
    );
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

/// Liveness: a claim is active only if its status says so AND its lease has
/// not expired. Single definition, reused by every reader (status display,
/// conflict detection, and the worktree cross-check in `branch_audit`): a
/// second copy of this rule is how the active set started lying.
pub(crate) fn local_claim_active(claim: &serde_json::Value, now_s: u64) -> bool {
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
fn execute_release_local(
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
        // Two defects, one loop. (1) A `released` record must not shadow a live
        // one on the same path: matching either kind and breaking on the first
        // made every later release of that scope a silent no-op. (2) claim_id
        // is NOT unique — it is `{agent-slug}_{unix-second}`, so an agent that
        // claims several scopes in the same second produces several entries
        // sharing one lease id, and a lease match then hit whichever came
        // first. Therefore: only an entry that is still held may be flipped.
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
        return Err(format!(
            "release failed: no local claim for target {target}"
        ));
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
    let url = format!("{base}/coord/who?agent_id={agent_id}");
    let resp = blocking_kernel_request(move || {
        let client = kernel_client();
        client
            .get(&url)
            .send()
            .map_err(|e| format!("kernel unreachable: {e}"))
    });

    if resp.is_err() {
        return execute_status_from_claims_file(agent_id);
    }
    let resp = resp.map_err(|e| format!("kernel unreachable: {e}"))?;

    let status = resp.status();
    if is_kernel_auth_refusal(status.as_u16()) {
        warn_kernel_auth_refusal("status");
        return execute_status_from_claims_file(agent_id);
    }
    let body: Value = resp.json().map_err(|e| format!("invalid JSON: {e}"))?;

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

    append_claim_event(
        "status_query",
        agent_id,
        "*",
        serde_json::json!({
            "total_claims": total,
            "source": "kernel",
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
                "provider_control": {"endpoint": "https://pve.service.cynic:8006"}
            })
            .to_string(),
        )
        .expect("fixture");
        path
    }

    #[test]
    /// The REAL registry must resolve, not a fixture that agrees with it.
    ///
    /// WHY: the previous version of this test built its own registry document and
    /// included `"port": 3030` in the kernel service. The real `infra/registry.json`
    /// declared `endpoint_host` and no `port` at all, so `resolve_kernel_base`
    /// returned `None` on the default path and every `cynic agent *` command took
    /// the local file fallback WITHOUT ever attempting HTTP. The route was not dead;
    /// it was never reached. A test that invents the field the production document
    /// lacks is worse than no test, because it reports green about a broken lookup.
    ///
    /// HOW: point at the real registry and assert the base resolves. If the port is
    /// removed from the contract again, this goes red instead of the workspace
    /// silently degrading to file mode.
    #[test]
    fn kernel_base_resolves_from_the_real_registry() {
        let registry = crate::rust_source::workspace_root().join("infra/registry.json");
        let base = resolve_kernel_base(&registry.to_string_lossy(), None);
        assert!(
            base.is_some(),
            "infra/registry.json does not resolve a kernel base: the cynic-kernel service \
             needs both endpoint_host and port, or every `cynic agent` command silently \
             falls back to the local claims file. Resolved: {base:?}"
        );
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
