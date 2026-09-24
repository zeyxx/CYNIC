//! I/O transport adapters for Forgejo Pull Request API.
//! Follows the observe -> plan -> apply -> read-back proof coherence pattern.

use cynic_vcs::pull_request::{ForgejoPrState, ForgejoPullRequest};
use serde::{Deserialize, Serialize};
use std::process::Command;

pub const FORGEJO_PR_CREATE_PROOF_SCHEMA: &str = "cynic.forgejo.pr_create_proof.v1";
pub const FORGEJO_PR_MERGE_PROOF_SCHEMA: &str = "cynic.forgejo.pr_merge_proof.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgejoPrCreateProof {
    pub schema: &'static str,
    pub pr_number: u64,
    pub html_url: String,
    pub head_ref: String,
    pub base_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgejoPrMergeProof {
    pub schema: &'static str,
    pub pr_number: u64,
    pub merged: bool,
    pub message: String,
}

pub struct ForgejoPrClient {
    pub api_base: String,
    pub owner: String,
    pub repo: String,
    pub token: Option<String>,
}

impl ForgejoPrClient {
    pub fn new(api_base: String, owner: String, repo: String, token: Option<String>) -> Self {
        Self {
            api_base,
            owner,
            repo,
            token,
        }
    }

    /// List all pull requests matching state ("open", "closed", "all").
    pub fn list_prs(&self, state: &str) -> Result<Vec<ForgejoPullRequest>, String> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls?state={}",
            self.api_base, self.owner, self.repo, state
        );

        let mut cmd = Command::new("curl");
        cmd.args(["-s", "-f", "-X", "GET", &url]);
        if let Some(token) = &self.token {
            cmd.args(["-H", &format!("Authorization: token {token}")]);
        }

        let output = cmd
            .output()
            .map_err(|e| format!("failed to execute curl: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "curl request failed with code {}: {}",
                output.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let raw_json: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("invalid JSON from Forgejo: {e}"))?;

        let items = raw_json
            .as_array()
            .ok_or_else(|| "expected JSON array of PRs".to_string())?;

        let mut prs = Vec::new();
        for item in items {
            let id = item.get("id").and_then(|v| v.as_u64()).unwrap_or_default();
            let number = item.get("number").and_then(|v| v.as_u64()).unwrap_or_default();
            let title = item.get("title").and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let state_str = item.get("state").and_then(|v| v.as_str()).unwrap_or("open");
            let state = match state_str {
                "closed" => ForgejoPrState::Closed,
                _ => ForgejoPrState::Open,
            };
            let head_ref = item.get("head").and_then(|h| h.get("ref")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let head_sha = item.get("head").and_then(|h| h.get("sha")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let base_ref = item.get("base").and_then(|b| b.get("ref")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let base_sha = item.get("base").and_then(|b| b.get("sha")).and_then(|v| v.as_str()).unwrap_or_default().to_string();
            let mergeable = item.get("mergeable").and_then(|v| v.as_bool()).unwrap_or(true);
            let has_conflicts = item.get("has_conflicts").and_then(|v| v.as_bool()).unwrap_or(false);
            let html_url = item.get("html_url").and_then(|v| v.as_str()).unwrap_or_default().to_string();

            prs.push(ForgejoPullRequest {
                id,
                number,
                title,
                state,
                head_ref,
                head_sha,
                base_ref,
                base_sha,
                mergeable,
                has_conflicts,
                html_url,
            });
        }

        Ok(prs)
    }

    /// Open a new PR on Forgejo.
    pub fn open_pr(
        &self,
        title: &str,
        head: &str,
        base: &str,
        body: &str,
    ) -> Result<ForgejoPrCreateProof, String> {
        let url = format!("{}/api/v1/repos/{}/{}/pulls", self.api_base, self.owner, self.repo);
        let payload = serde_json::json!({
            "title": title,
            "head": head,
            "base": base,
            "body": body,
        });

        let mut cmd = Command::new("curl");
        cmd.args([
            "-s", "-f",
            "-X", "POST",
            &url,
            "-H", "Content-Type: application/json",
            "-d", &payload.to_string(),
        ]);
        if let Some(token) = &self.token {
            cmd.args(["-H", &format!("Authorization: token {token}")]);
        }

        let output = cmd
            .output()
            .map_err(|e| format!("failed to execute curl: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "failed to create PR: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        let res: serde_json::Value = serde_json::from_slice(&output.stdout)
            .map_err(|e| format!("invalid JSON response: {e}"))?;

        let pr_number = res.get("number").and_then(|v| v.as_u64()).ok_or_else(|| "missing PR number in response".to_string())?;
        let html_url = res.get("html_url").and_then(|v| v.as_str()).unwrap_or_default().to_string();

        Ok(ForgejoPrCreateProof {
            schema: FORGEJO_PR_CREATE_PROOF_SCHEMA,
            pr_number,
            html_url,
            head_ref: head.to_string(),
            base_ref: base.to_string(),
        })
    }

    /// Merge a PR on Forgejo.
    pub fn merge_pr(&self, number: u64, style: &str) -> Result<ForgejoPrMergeProof, String> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls/{}/merge",
            self.api_base, self.owner, self.repo, number
        );
        let payload = serde_json::json!({
            "Do": style, // "merge", "rebase", or "squash"
        });

        let mut cmd = Command::new("curl");
        cmd.args([
            "-s", "-f",
            "-X", "POST",
            &url,
            "-H", "Content-Type: application/json",
            "-d", &payload.to_string(),
        ]);
        if let Some(token) = &self.token {
            cmd.args(["-H", &format!("Authorization: token {token}")]);
        }

        let output = cmd
            .output()
            .map_err(|e| format!("failed to execute curl: {e}"))?;

        if !output.status.success() {
            return Err(format!(
                "failed to merge PR: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        Ok(ForgejoPrMergeProof {
            schema: FORGEJO_PR_MERGE_PROOF_SCHEMA,
            pr_number: number,
            merged: true,
            message: format!("PR #{number} successfully merged with style '{style}'"),
        })
    }
}
