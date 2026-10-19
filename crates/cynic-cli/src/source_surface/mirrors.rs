//! Forgejo push-mirror read-back: list mirrors with last sync + errors.
//!
//! Read-only by construction: GETs the Forgejo REST API only.

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::process::ExitCode;

use crate::cli_json::emit_pretty_json;
use cynic_vcs::registry_binding::find_ssot_in_registry;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct PushMirror {
    #[serde(default, rename = "repo_name")]
    pub repo_name: String,
    #[serde(default, rename = "remote_name")]
    pub remote_name: String,
    #[serde(default, rename = "remote_address")]
    pub remote_address: String,
    #[serde(default, rename = "created")]
    pub created: String,
    #[serde(default, rename = "last_update")]
    pub last_update: String,
    #[serde(default, rename = "last_error")]
    pub last_error: String,
    #[serde(default, rename = "interval")]
    pub interval: String,
    #[serde(default, rename = "sync_on_commit")]
    pub sync_on_commit: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct MirrorsReport {
    pub schema: &'static str,
    pub count: usize,
    pub mirrors: Vec<PushMirror>,
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
                .unwrap_or("/etc/cynic/secrets/forgejo-token");
            let content = std::fs::read_to_string(token_file)
                .map_err(|e| format!("token file unreadable at {token_file}: {e}"))?;
            let t = content.trim().to_string();
            if t.is_empty() {
                return Err(format!("token empty in {token_file}"));
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
    serde_json::from_str(body.trim()).map_err(|e| format!("invalid JSON response: {e}: {body}"))
}

pub(crate) fn execute_mirrors(root: &Path, json: bool) -> ExitCode {
    let (api_url, owner, repo, token) = match resolve_forgejo(root) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("blocked: registry error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let url = format!("{api_url}/api/v1/repos/{owner}/{repo}/push_mirrors");
    let value = match http_get_json(&url, &token) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("blocked: forgejo request failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let mirrors: Vec<PushMirror> = serde_json::from_value(value).unwrap_or_default();

    let report = MirrorsReport {
        schema: "cynic.source.mirrors.v1",
        count: mirrors.len(),
        mirrors,
    };

    if json {
        emit_pretty_json(&report)
    } else {
        println!("mirrors: {}", report.count);
        for m in &report.mirrors {
            println!(
                "  {} -> {} (last_sync={}, error={})",
                m.remote_name, m.remote_address, m.last_update, m.last_error
            );
        }
        ExitCode::SUCCESS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mirror_array() {
        let raw = serde_json::json!([{
            "repo_name":"CYNIC",
            "remote_name":"remote_mirror_abc",
            "remote_address":"https://github.com/zeyxx/CYNIC.git",
            "created":"2026-10-11T18:54:38Z",
            "last_update":"2026-10-19T01:46:24Z",
            "last_error":"",
            "interval":"8h0m0s",
            "sync_on_commit":true
        }]);
        let parsed: Vec<PushMirror> = serde_json::from_value(raw).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].last_error, "");
    }
}
