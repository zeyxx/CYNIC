//! Forgejo adapter implementing the sovereign `PrPort`.
//!
//! Follows observe -> plan -> apply -> read-back proof coherence pattern.
//! Transport: curl CLI (no reqwest at this layer).

use std::process::Command;

use cynic_vcs::pull_request::{
    PR_CLOSE_PROOF_SCHEMA, PR_CREATE_PROOF_SCHEMA, PR_MERGE_PROOF_SCHEMA, PrCloseProof,
    PrCreateProof, PrMergeProof, PrPort, PrState, PullRequest,
};

pub struct ForgejoPrAdapter {
    pub api_base: String,
    pub owner: String,
    pub repo: String,
    pub token: Option<String>,
}

impl ForgejoPrAdapter {
    pub fn new(api_base: String, owner: String, repo: String, token: Option<String>) -> Self {
        Self {
            api_base,
            owner,
            repo,
            token,
        }
    }

    fn auth_header(&self) -> Option<String> {
        self.token
            .as_ref()
            .map(|t| format!("Authorization: token {t}"))
    }

    fn curl_get(&self, url: &str) -> Result<serde_json::Value, String> {
        let mut cmd = Command::new("curl");
        cmd.args(["-s", "-f", "-X", "GET", url]);
        if let Some(h) = self.auth_header() {
            cmd.args(["-H", &h]);
        }
        let out = cmd.output().map_err(|e| format!("curl exec failed: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "GET {} failed ({}): {}",
                url,
                out.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        serde_json::from_slice(&out.stdout).map_err(|e| format!("invalid JSON: {e}"))
    }

    fn curl_post(&self, url: &str, body: &str) -> Result<serde_json::Value, String> {
        self.curl_json("POST", url, Some(body))
    }

    fn curl_patch(&self, url: &str, body: &str) -> Result<serde_json::Value, String> {
        self.curl_json("PATCH", url, Some(body))
    }

    fn curl_delete(&self, url: &str) -> Result<(), String> {
        self.curl_json("DELETE", url, None).map(|_| ())
    }

    /// Fetch a single PR by number (adapter-internal read-back helper).
    pub fn get(&self, number: u64) -> Result<PullRequest, String> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls/{}",
            self.api_base, self.owner, self.repo, number
        );
        let json = self.curl_get(&url)?;
        Ok(parse_pr(&json))
    }

    fn curl_json(
        &self,
        method: &str,
        url: &str,
        body: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let mut cmd = Command::new("curl");
        cmd.args([
            "-s",
            "-f",
            "-X",
            method,
            url,
            "-H",
            "Content-Type: application/json",
        ]);
        if let Some(b) = body {
            cmd.args(["-d", b]);
        }
        if let Some(h) = self.auth_header() {
            cmd.args(["-H", &h]);
        }
        let out = cmd.output().map_err(|e| format!("curl exec failed: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "{method} {} failed ({}): {}",
                url,
                out.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        if out.stdout.is_empty() {
            return Ok(serde_json::Value::Null);
        }
        serde_json::from_slice(&out.stdout).map_err(|e| format!("invalid JSON: {e}"))
    }
}

fn parse_pr(item: &serde_json::Value) -> PullRequest {
    let str_path = |path: &[&str]| -> String {
        let mut v = item;
        for &k in path {
            v = &v[k];
        }
        v.as_str().unwrap_or_default().to_string()
    };
    PullRequest {
        number: item["number"].as_u64().unwrap_or_default(),
        title: str_path(&["title"]),
        state: match item["state"].as_str().unwrap_or("open") {
            "closed" => PrState::Closed,
            _ => PrState::Open,
        },
        head_ref: str_path(&["head", "ref"]),
        head_sha: str_path(&["head", "sha"]),
        base_ref: str_path(&["base", "ref"]),
        base_sha: str_path(&["base", "sha"]),
        mergeable: item["mergeable"].as_bool().unwrap_or(true),
        has_conflicts: item["has_conflicts"].as_bool().unwrap_or(false),
        html_url: str_path(&["html_url"]),
    }
}

impl PrPort for ForgejoPrAdapter {
    fn list(&self, state: &str) -> Result<Vec<PullRequest>, String> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls?state={}&limit=50",
            self.api_base, self.owner, self.repo, state
        );
        let json = self.curl_get(&url)?;
        let items = json.as_array().ok_or("expected JSON array")?;
        Ok(items.iter().map(parse_pr).collect())
    }

    fn open(
        &self,
        title: &str,
        head: &str,
        base: &str,
        body: &str,
    ) -> Result<PrCreateProof, String> {
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls",
            self.api_base, self.owner, self.repo
        );
        let payload = serde_json::json!({"title": title, "head": head, "base": base, "body": body});
        let res = self.curl_post(&url, &payload.to_string())?;
        let pr_number = res["number"]
            .as_u64()
            .ok_or("missing PR number in response")?;
        let html_url = res["html_url"].as_str().unwrap_or_default().to_string();
        Ok(PrCreateProof {
            schema: PR_CREATE_PROOF_SCHEMA,
            pr_number,
            html_url,
            head_ref: head.to_string(),
            base_ref: base.to_string(),
        })
    }

    fn merge(&self, number: u64, style: &str) -> Result<PrMergeProof, String> {
        // Forgejo merge returns 204 No Content — discard body, check exit code only.
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls/{}/merge",
            self.api_base, self.owner, self.repo, number
        );
        let payload = serde_json::json!({"Do": style});
        let mut cmd = Command::new("curl");
        cmd.args([
            "-s",
            "-f",
            "-o",
            "/dev/null",
            "-X",
            "POST",
            &url,
            "-H",
            "Content-Type: application/json",
            "-d",
            &payload.to_string(),
        ]);
        if let Some(h) = self.auth_header() {
            cmd.args(["-H", &h]);
        }
        let out = cmd.output().map_err(|e| format!("curl exec failed: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "merge PR#{number} failed ({}): {}",
                out.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        Ok(PrMergeProof {
            schema: PR_MERGE_PROOF_SCHEMA,
            pr_number: number,
            merged: true,
            message: format!("PR #{number} merged via Forgejo (style: {style})"),
        })
    }

    fn close(&self, number: u64, delete_branch: bool) -> Result<PrCloseProof, String> {
        // Observe: refuse close of a PR that is not open.
        let current = self.get(number)?;
        if current.state != PrState::Open {
            return Err(format!("pr_not_open:state_{:?}", current.state));
        }
        // Apply: Forgejo closes via PATCH state=closed.
        let url = format!(
            "{}/api/v1/repos/{}/{}/pulls/{}",
            self.api_base, self.owner, self.repo, number
        );
        self.curl_patch(&url, &serde_json::json!({"state": "closed"}).to_string())?;
        // Read-back: the PR must now read closed.
        let closed = self.get(number)?;
        if closed.state == PrState::Open {
            return Err(format!("close PR#{number} not reflected on read-back"));
        }
        let branch_deleted = if delete_branch {
            let branch_url = format!(
                "{}/api/v1/repos/{}/{}/branches/{}",
                self.api_base, self.owner, self.repo, closed.head_ref
            );
            match self.curl_delete(&branch_url) {
                Ok(()) => true,
                Err(e) => {
                    return Ok(PrCloseProof {
                        schema: PR_CLOSE_PROOF_SCHEMA,
                        pr_number: number,
                        closed: true,
                        branch_deleted: false,
                        message: format!("PR #{number} closed, branch delete failed: {e}"),
                    });
                }
            }
        } else {
            false
        };
        Ok(PrCloseProof {
            schema: PR_CLOSE_PROOF_SCHEMA,
            pr_number: number,
            closed: true,
            branch_deleted,
            message: format!("PR #{number} closed via Forgejo"),
        })
    }
}
