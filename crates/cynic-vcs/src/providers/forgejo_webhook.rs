//! Webhook handler for Forgejo push/PR events.
//!
//! Replaces the retired bash build-webhook script with a native Rust
//! surface that respects CYNIC principles:
//!   - declarative desired state (registry-driven node discovery)
//!   - observation before action
//!   - bounded mutation (--yes/--apply gate)
//!   - read-back proof (signed pipeline_build_proof)
//!
//! Design:
//!   forgejo push/PR → HTTP POST /webhook/forgejo → extract commit → submit
//!   BuildAgentRequest → orchestrator (admission + grant + submission)
//!
//! No shell, no SSH direct — the builder observes the workload and executes it
//! through the delivery loop.

use serde::{Deserialize, Serialize};

/// Forgejo webhook payload (minimal — only what we need).
#[derive(Debug, Clone, Deserialize)]
pub struct ForgejoPayload {
    #[serde(rename = "ref")]
    pub branch_ref: Option<String>,
    pub after: Option<String>,
    pub pull_request: Option<ForgejoPullRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ForgejoPullRequest {
    pub head: ForgejoPrHead,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ForgejoPrHead {
    pub sha: String,
}

/// Extract exact source commit (validated 40-char hex SHA) from payload.
pub fn extract_source_commit(payload: &ForgejoPayload) -> Option<String> {
    if let Some(pr) = &payload.pull_request {
        let sha = pr.head.sha.trim();
        if validate_source_commit(sha) {
            return Some(sha.to_string());
        }
    }
    if let Some(after) = &payload.after {
        let sha = after.trim();
        if validate_source_commit(sha) {
            return Some(sha.to_string());
        }
    }
    None
}

/// Validate a git commit SHA: exactly 40 hex characters.
pub fn validate_source_commit(sha: &str) -> bool {
    sha.len() == 40 && sha.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Extract branch name from ref.
pub fn extract_branch(payload: &ForgejoPayload) -> Option<String> {
    payload
        .branch_ref
        .as_ref()
        .map(|r| r.replace("refs/heads/", ""))
}

/// Result of processing a forgejo webhook.
#[derive(Debug, Clone, Serialize)]
pub enum ForgejoHandlerResult {
    /// Build triggered, contains workload id.
    Triggered {
        workload_id: String,
        source_commit: String,
    },
    /// Skipped (no commit, wrong branch).
    Skipped { reason: String },
}

/// Configuration for the forgejo webhook handler.
#[derive(Debug, Clone)]
pub struct ForgejoHandlerConfig {
    /// Only build this branch (None = any).
    pub require_branch: Option<String>,
    /// Default build phase.
    pub default_phase: String,
}

impl Default for ForgejoHandlerConfig {
    fn default() -> Self {
        Self {
            require_branch: Some("main".to_string()),
            default_phase: "check".to_string(),
        }
    }
}

/// Process a forgejo webhook payload.
pub fn handle_forgejo_payload(
    payload: &ForgejoPayload,
    config: &ForgejoHandlerConfig,
) -> ForgejoHandlerResult {
    let source_commit = match extract_source_commit(payload) {
        Some(sha) => sha,
        None => {
            return ForgejoHandlerResult::Skipped {
                reason: "no_commit_sha_found".to_string(),
            };
        }
    };

    if let Some(required) = &config.require_branch {
        let branch = extract_branch(payload);
        if branch.as_deref() != Some(required) {
            return ForgejoHandlerResult::Skipped {
                reason: format!("branch_mismatch: required={required}"),
            };
        }
    }

    let workload_id = format!("forgejo-{}", &source_commit[..24]);

    ForgejoHandlerResult::Triggered {
        workload_id,
        source_commit,
    }
}

use serde_json::Value;

/// Parse a Forgejo push webhook JSON body into the canonical `VcsPushEvent`.
pub fn parse_forgejo_push(body: &[u8]) -> Result<cynic_protocol::vcs::VcsPushEvent, String> {
    let value: Value = serde_json::from_slice(body).map_err(|e| format!("invalid JSON: {e}"))?;

    // commit_sha: validated 40 hex chars.
    let commit_sha = extract_commit_sha(&value).ok_or_else(|| "missing commit sha".to_string())?;
    if !validate_source_commit(&commit_sha) {
        return Err(format!(
            "invalid commit SHA: expected 40 hex chars, got {}",
            commit_sha.len()
        ));
    }

    let ref_name = extract_ref_name(&value).unwrap_or_default();
    let pusher = extract_pusher(&value).unwrap_or_else(|| "forgejo".to_string());
    let repository = extract_repository(&value);
    let pushed_at_unix_ms = extract_pushed_at(&value);

    Ok(cynic_protocol::vcs::VcsPushEvent {
        provider: "forgejo".to_string(),
        commit_sha,
        ref_name,
        pusher,
        repository,
        pushed_at_unix_ms,
    })
}

fn as_str(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn extract_commit_sha(v: &Value) -> Option<String> {
    // after
    if let Some(s) = v.get("after").and_then(as_str) {
        let s = s.trim().to_string();
        if !s.is_empty() {
            return Some(s);
        }
    }
    // reref / ref object with id
    for key in ["reref", "ref"] {
        if let Some(obj) = v.get(key) {
            if let Some(id) = obj.get("id").and_then(as_str) {
                let id = id.trim().to_string();
                if !id.is_empty() && validate_source_commit(&id) {
                    return Some(id);
                }
            }
            // if "ref" is actually a sha string? handle below
        }
    }
    // head_commit
    if let Some(hc) = v.get("head_commit") {
        if let Some(id) = hc.get("id").or_else(|| hc.get("sha")).and_then(as_str) {
            let id = id.trim().to_string();
            if !id.is_empty() {
                return Some(id);
            }
        }
    }
    // pull_request head
    if let Some(pr) = v.get("pull_request") {
        if let Some(head) = pr.get("head") {
            if let Some(sha) = head.get("sha").and_then(as_str) {
                let sha = sha.trim().to_string();
                if !sha.is_empty() {
                    return Some(sha);
                }
            }
        }
    }
    // commits last
    if let Some(commits) = v.get("commits").and_then(|c| c.as_array()) {
        if let Some(last) = commits.last() {
            if let Some(id) = last.get("id").or_else(|| last.get("sha")).and_then(as_str) {
                let id = id.trim().to_string();
                if !id.is_empty() {
                    return Some(id);
                }
            }
        }
    }
    // direct fields
    for key in ["commit_sha", "source_commit", "sha"] {
        if let Some(s) = v.get(key).and_then(as_str) {
            let s = s.trim().to_string();
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    None
}

fn extract_ref_name(v: &Value) -> Option<String> {
    // direct
    for key in ["ref_name", "branch_ref", "refname"] {
        if let Some(s) = v.get(key).and_then(as_str) {
            if !s.is_empty() {
                return Some(s);
            }
        }
    }
    // ref string
    if let Some(Value::String(s)) = v.get("ref") {
        if !s.is_empty() {
            return Some(s.clone());
        }
    }
    // reref/ref object id (only if not a commit sha)
    for key in ["reref", "ref"] {
        if let Some(obj) = v.get(key) {
            if let Some(id) = obj.get("id").and_then(as_str) {
                if !id.is_empty() && !validate_source_commit(&id) {
                    return Some(id);
                }
            }
        }
    }
    // branch
    if let Some(s) = v.get("branch").and_then(as_str) {
        if !s.is_empty() {
            return Some(s);
        }
    }
    None
}

fn extract_pusher(v: &Value) -> Option<String> {
    if let Some(p) = v.get("pusher") {
        match p {
            Value::String(s) => {
                if !s.is_empty() {
                    return Some(s.clone());
                }
            }
            Value::Object(_) => {
                for key in ["login", "name", "username", "full_name"] {
                    if let Some(s) = p.get(key).and_then(as_str) {
                        if !s.is_empty() {
                            return Some(s);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(s) = v.get("pusher_name").and_then(as_str) {
        if !s.is_empty() {
            return Some(s);
        }
    }
    if let Some(sender) = v.get("sender") {
        for key in ["login", "name", "username"] {
            if let Some(s) = sender.get(key).and_then(as_str) {
                if !s.is_empty() {
                    return Some(s);
                }
            }
        }
    }
    None
}

fn extract_repository(v: &Value) -> Option<String> {
    if let Some(repo) = v.get("repository") {
        match repo {
            Value::String(s) => {
                if !s.is_empty() {
                    return Some(s.clone());
                }
            }
            Value::Object(_) => {
                for key in ["full_name", "name", "path", "id"] {
                    if let Some(s) = repo.get(key).and_then(as_str) {
                        if !s.is_empty() {
                            return Some(s);
                        }
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(s) = v.get("repository_name").and_then(as_str) {
        if !s.is_empty() {
            return Some(s);
        }
    }
    None
}

fn parse_u64_value(val: &Value, key_hint_ms: bool) -> Option<u64> {
    match val {
        Value::Number(n) => n.as_u64().map(|n| {
            if key_hint_ms {
                n
            } else if n < 1_000_000_000_000 {
                n * 1000
            } else {
                n
            }
        }),
        Value::String(s) => {
            let s = s.trim();
            if let Ok(n) = s.parse::<u64>() {
                Some(if key_hint_ms || n >= 1_000_000_000_000 {
                    n
                } else {
                    n * 1000
                })
            } else {
                None
            }
        }
        _ => None,
    }
}

fn extract_pushed_at(v: &Value) -> Option<u64> {
    // direct ms
    for key in ["pushed_at_unix_ms", "pushed_at_ms", "timestamp_ms"] {
        if let Some(val) = v.get(key) {
            if let Some(n) = parse_u64_value(val, true) {
                return Some(n);
            }
        }
    }
    // seconds-ish
    for key in ["pushed_at", "timestamp", "pushed_at_unix", "created_at"] {
        if let Some(val) = v.get(key) {
            if let Some(n) = parse_u64_value(val, false) {
                return Some(n);
            }
        }
    }
    // commits last timestamp
    if let Some(commits) = v.get("commits").and_then(|c| c.as_array()) {
        if let Some(last) = commits.last() {
            for key in ["pushed_at_unix_ms", "pushed_at_ms", "timestamp_ms"] {
                if let Some(val) = last.get(key) {
                    if let Some(n) = parse_u64_value(val, true) {
                        return Some(n);
                    }
                }
            }
            for key in ["pushed_at", "timestamp", "created_at"] {
                if let Some(val) = last.get(key) {
                    if let Some(n) = parse_u64_value(val, false) {
                        return Some(n);
                    }
                }
            }
        }
    }
    // head_commit
    if let Some(hc) = v.get("head_commit") {
        for key in ["pushed_at_unix_ms", "pushed_at_ms", "timestamp_ms", "pushed_at", "timestamp"] {
            if let Some(val) = hc.get(key) {
                let ms = key.ends_with("_ms") || key == "pushed_at_unix_ms";
                if let Some(n) = parse_u64_value(val, ms) {
                    return Some(n);
                }
            }
        }
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_forgejo_push_basic() {
        let body = br#"{"ref":"refs/heads/main","after":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","pusher":{"login":"zeyxx"},"repository":{"full_name":"owner/cynic"},"pushed_at_unix_ms":12345}"#;
        let event = parse_forgejo_push(body).unwrap();
        assert_eq!(event.provider, "forgejo");
        assert_eq!(event.commit_sha, "a".repeat(40));
        assert_eq!(event.ref_name, "refs/heads/main");
        assert_eq!(event.pusher, "zeyxx");
        assert_eq!(event.repository, Some("owner/cynic".to_string()));
        assert_eq!(event.pushed_at_unix_ms, Some(12345));
    }

    #[test]
    fn parse_forgejo_push_invalid_sha() {
        let body = br#"{"ref":"refs/heads/main","after":"short","pusher":"zeyxx"}"#;
        assert!(parse_forgejo_push(body).is_err());
    }

    #[test]
    fn parse_forgejo_push_string_pusher_repo() {
        let body = br#"{"ref":"refs/heads/main","after":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","pusher":"alice","repository":"cynic","pushed_at":5}"#;
        let event = parse_forgejo_push(body).unwrap();
        assert_eq!(event.pusher, "alice");
        assert_eq!(event.repository, Some("cynic".to_string()));
        // 5 seconds -> 5000 ms
        assert_eq!(event.pushed_at_unix_ms, Some(5000));
    }

    #[test]
    fn extract_commit_from_push() {
        let payload = ForgejoPayload {
            branch_ref: Some("refs/heads/main".to_string()),
            after: Some("a".repeat(40)),
            pull_request: None,
        };
        assert_eq!(extract_source_commit(&payload), Some("a".repeat(40)));
    }

    #[test]
    fn extract_commit_from_pr() {
        let payload = ForgejoPayload {
            branch_ref: None,
            after: None,
            pull_request: Some(ForgejoPullRequest {
                head: ForgejoPrHead {
                    sha: "b".repeat(40),
                },
            }),
        };
        assert_eq!(extract_source_commit(&payload), Some("b".repeat(40)));
    }

    #[test]
    fn skip_invalid_commit() {
        let payload = ForgejoPayload {
            branch_ref: None,
            after: Some("short".to_string()),
            pull_request: None,
        };
        assert_eq!(extract_source_commit(&payload), None);
    }

    #[test]
    fn handler_skips_wrong_branch() {
        let payload = ForgejoPayload {
            branch_ref: Some("refs/heads/dev".to_string()),
            after: Some("a".repeat(40)),
            pull_request: None,
        };
        let config = ForgejoHandlerConfig {
            require_branch: Some("main".to_string()),
            default_phase: "check".to_string(),
        };
        let result = handle_forgejo_payload(&payload, &config);
        match result {
            ForgejoHandlerResult::Skipped { reason } => assert!(reason.contains("branch_mismatch")),
            _ => panic!("expected Skipped"),
        }
    }

    #[test]
    fn handler_triggers_on_main() {
        let payload = ForgejoPayload {
            branch_ref: Some("refs/heads/main".to_string()),
            after: Some("a".repeat(40)),
            pull_request: None,
        };
        let config = ForgejoHandlerConfig::default();
        let result = handle_forgejo_payload(&payload, &config);
        match result {
            ForgejoHandlerResult::Triggered { source_commit, .. } => {
                assert_eq!(source_commit, "a".repeat(40));
            }
            _ => panic!("expected Triggered"),
        }
    }
}
