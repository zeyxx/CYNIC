//! Webhook handler for Forgejo push/PR events.
//!
//! Replaces the bash script `scripts/forgejo-build-webhook` with a native Rust
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

/// Extract exact source commit from payload.
pub fn extract_source_commit(payload: &ForgejoPayload) -> Option<String> {
    if let Some(pr) = &payload.pull_request {
        let sha = pr.head.sha.trim();
        if sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(sha.to_string());
        }
    }
    if let Some(after) = &payload.after {
        let sha = after.trim();
        if sha.len() == 40 && sha.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(sha.to_string());
        }
    }
    None
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
    Triggered { workload_id: String, source_commit: String },
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

#[cfg(test)]
mod tests {
    use super::*;

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
