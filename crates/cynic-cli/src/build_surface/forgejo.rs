//! Forgejo Actions read-back verbs: runner registration token + recent runs.
//!
//! Read-only by construction: GETs the Forgejo REST API only. No enrollment,
//! no workflow dispatch, no mutation. Hosts and tokens resolve from
//! `infra/registry.json` (sovereign SSOT) + token env/file, never hardcoded.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::ExitCode;

use crate::cli_json::emit_pretty_json;
use cynic_vcs::registry_binding::find_ssot_in_registry;

#[derive(Debug, Serialize)]
pub(crate) struct RunnerTokenReport {
    pub schema: &'static str,
    pub owner: String,
    pub repo: String,
    pub api_url: String,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct WorkflowRun {
    pub id: i64,
    pub name: String,
    #[serde(default)]
    pub head_branch: String,
    #[serde(default)]
    pub head_sha: String,
    #[serde(default)]
    pub run_number: i64,
    #[serde(default)]
    pub event: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub workflow_id: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
}

#[derive(Debug, Deserialize)]
struct RunsResponse {
    #[serde(default)]
    workflow_runs: Vec<WorkflowRun>,
}

#[derive(Debug, Serialize)]
pub(crate) struct RunsReport {
    pub schema: &'static str,
    pub count: usize,
    pub runs: Vec<WorkflowRun>,
}

fn resolve_forgejo(root: &Path) -> Result<(String, String, String, String), String> {
    let registry_path = root.join("infra/registry.json");
    let raw = std::fs::read_to_string(&registry_path)
        .map_err(|e| format!("registry unreadable at {}: {e}", registry_path.display()))?;
    let reg: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("registry invalid JSON: {e}"))?;
    let ssot = find_ssot_in_registry(&reg)
        .ok_or_else(|| "vcs_provider.sovereign_ssot not found in registry".to_string())?;
    let api_url = ssot
        .get("api_url")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "ssot api_url missing".to_string())?
        .trim_end_matches('/')
        .to_string();
    let owner = ssot
        .get("owner")
        .and_then(|v| v.as_str())
        .unwrap_or("jeanterre13")
        .to_string();
    let repo = ssot
        .get("repo")
        .and_then(|v| v.as_str())
        .unwrap_or("CYNIC")
        .to_string();

    let token_env = ssot
        .get("token_env")
        .and_then(|v| v.as_str())
        .unwrap_or("FORGEJO_TOKEN");
    let token = match std::env::var(token_env) {
        Ok(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => {
            let token_file = ssot
                .get("token_file")
                .and_then(|v| v.as_str())
                .map(std::path::PathBuf::from)
                .unwrap_or_else(|| {
                    cynic_types::paths::CynicPaths::secret_path(
                        cynic_types::paths::CynicPaths::DEFAULT_FORGEJO_TOKEN_SECRET,
                    )
                });
            let content = std::fs::read_to_string(&token_file)
                .map_err(|e| format!("token file unreadable at {}: {e}", token_file.display()))?;
            let t = content.trim().to_string();
            if t.is_empty() {
                return Err(format!("token empty in {}", token_file.display()));
            }
            t
        }
    };

    Ok((api_url, owner, repo, token))
}

fn http_get_json(url: &str, token: &str) -> Result<serde_json::Value, String> {
    // reqwest (not curl subprocess): the token travels in a header object,
    // never in argv, so it cannot leak via /proc to other local users.
    // Client is built AND dropped inside block_in_place: reqwest::blocking
    // owns a runtime handle that panics if dropped in async context.
    let body = tokio::task::block_in_place(|| {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("http client: {e}"))?;
        client
            .get(url)
            .header("Authorization", format!("token {token}"))
            .header("Accept", "application/json")
            .send()
            .map_err(|e| format!("GET {url} failed: {e}"))?
            .text()
            .map_err(|e| format!("read body: {e}"))
    })?;
    let value: serde_json::Value =
        serde_json::from_str(body.trim()).map_err(|e| format!("invalid JSON response: {e}: {body}"))?;
    if let Some(msg) = value.get("message").and_then(|v| v.as_str()) {
        // Forgejo returns {"message":"..."} on errors.
        if value.get("token").is_none() && value.get("workflow_runs").is_none() && value.as_array().is_none() {
            return Err(format!("forgejo error: {msg}"));
        }
    }
    Ok(value)
}

pub(crate) fn execute_runner_token(root: &Path, json: bool) -> ExitCode {
    let (api_url, owner, repo, token) = match resolve_forgejo(root) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("blocked: registry error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let url = format!("{api_url}/api/v1/repos/{owner}/{repo}/actions/runners/registration-token");
    let value = match http_get_json(&url, &token) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("blocked: forgejo request failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    let token_str = match value.get("token").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => {
            eprintln!("blocked: no token in response");
            return ExitCode::FAILURE;
        }
    };

    let report = RunnerTokenReport {
        schema: "cynic.build.runner_token.v1",
        owner,
        repo,
        api_url,
        token: token_str,
    };

    if json {
        emit_pretty_json(&report)
    } else {
        println!("runner-token: {}", report.token);
        ExitCode::SUCCESS
    }
}

pub(crate) fn execute_runs(root: &Path, json: bool) -> ExitCode {
    let (api_url, owner, repo, token) = match resolve_forgejo(root) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("blocked: registry error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let url = format!("{api_url}/api/v1/repos/{owner}/{repo}/actions/tasks");
    let value = match http_get_json(&url, &token) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("blocked: forgejo request failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let runs: Vec<WorkflowRun> = value
        .get("workflow_runs")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let report = RunsReport {
        schema: "cynic.build.runs.v1",
        count: runs.len(),
        runs,
    };

    if json {
        emit_pretty_json(&report)
    } else {
        println!("runs: {}", report.count);
        for r in &report.runs {
            println!(
                "  #{} {} {} {} {}",
                r.run_number, r.name, r.status, r.head_branch, r.head_sha
            );
        }
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_runs_response_parses() {
        let raw = serde_json::json!({
            "workflow_runs": [
                {"id":1,"name":"proof","head_branch":"main","head_sha":"abc","run_number":1,"event":"push","status":"success","workflow_id":"proof.yml","url":"u","created_at":"t","updated_at":"t"}
            ],
            "total_count":1
        });
        let parsed: RunsResponse = serde_json::from_value(raw).unwrap();
        assert_eq!(parsed.workflow_runs.len(), 1);
        assert_eq!(parsed.workflow_runs[0].status, "success");
    }
}
