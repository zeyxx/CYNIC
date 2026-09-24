//! Forgejo adapter implementing the sovereign `PrPort`.
//!
//! Follows observe -> plan -> apply -> read-back proof coherence pattern.
//! Transport: curl CLI (no reqwest at this layer).

use std::process::Command;

use cynic_vcs::pull_request::{
    PR_CREATE_PROOF_SCHEMA, PR_MERGE_PROOF_SCHEMA, PrCreateProof, PrMergeProof, PrPort,
    PrState, PullRequest,
};

pub struct ForgejoPrAdapter {
    pub api_base: String,
    pub owner: String,
    pub repo: String,
    pub token: Option<String>,
}

impl ForgejoPrAdapter {
    pub fn new(api_base: String, owner: String, repo: String, token: Option<String>) -> Self {
        Self { api_base, owner, repo, token }
    }

    fn auth_header(&self) -> Option<String> {
        self.token.as_ref().map(|t| format!("Authorization: token {t}"))
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
        let mut cmd = Command::new("curl");
        cmd.args(["-s", "-f", "-X", "POST", url,
                  "-H", "Content-Type: application/json", "-d", body]);
        if let Some(h) = self.auth_header() {
            cmd.args(["-H", &h]);
        }
        let out = cmd.output().map_err(|e| format!("curl exec failed: {e}"))?;
        if !out.status.success() {
            return Err(format!(
                "POST {} failed ({}): {}",
                url,
                out.status.code().unwrap_or(-1),
                String::from_utf8_lossy(&out.stderr)
            ));
        }
        serde_json::from_slice(&out.stdout).map_err(|e| format!("invalid JSON: {e}"))
    }
}

fn parse_pr(item: &serde_json::Value) -> PullRequest {
    let str_path = |path: &[&str]| -> String {
        let mut v = item;
        for &k in path { v = &v[k]; }
        v.as_str().unwrap_or_default().to_string()
    };
    PullRequest {
        number:        item["number"].as_u64().unwrap_or_default(),
        title:         str_path(&["title"]),
        state: match item["state"].as_str().unwrap_or("open") {
            "closed" => PrState::Closed,
            _ => PrState::Open,
        },
        head_ref:      str_path(&["head", "ref"]),
        head_sha:      str_path(&["head", "sha"]),
        base_ref:      str_path(&["base", "ref"]),
        base_sha:      str_path(&["base", "sha"]),
        mergeable:     item["mergeable"].as_bool().unwrap_or(true),
        has_conflicts: item["has_conflicts"].as_bool().unwrap_or(false),
        html_url:      str_path(&["html_url"]),
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

    fn open(&self, title: &str, head: &str, base: &str, body: &str) -> Result<PrCreateProof, String> {
        let url = format!("{}/api/v1/repos/{}/{}/pulls", self.api_base, self.owner, self.repo);
        let payload = serde_json::json!({"title": title, "head": head, "base": base, "body": body});
        let res = self.curl_post(&url, &payload.to_string())?;
        let pr_number = res["number"].as_u64().ok_or("missing PR number in response")?;
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
        cmd.args(["-s", "-f", "-o", "/dev/null", "-X", "POST", &url,
                  "-H", "Content-Type: application/json", "-d", &payload.to_string()]);
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
}
