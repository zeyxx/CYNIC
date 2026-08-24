//! Forgejo personal access token lifecycle port and adapters.
//!
//! This module follows the CYNIC coherence pattern:
//! - `types` live in `cynic_protocol::forgejo_token` (never duplicated in host);
//! - the `Observe`/`Create`/`Remove` port is injected, never hard-wired to
//!   `Command::new("forgejo")`;
//! - adapters bind the concrete transport (SSH-local Exec or HTTP).
//!
//! The runtime never learns the raw token value. It observes file metadata
//! (owner, mode, presence) and validates the token through a live API probe
//! (SSH-tunneled localhost HTTP or direct HTTPS). Provision reports carry only
//! Forgejo-side metadata (token_id, sha1), never the secret itself.

use std::fs::OpenOptions;
use std::io::Write as IoWrite;
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

use cynic_protocol::forgejo_token::{
    ForgejoTokenAuthState, ForgejoTokenMetadata, ForgejoTokenObservation,
    ForgejoTokenProvisionReport, ForgejoTokenSpec, FORGEJO_TOKEN_OBSERVATION_SCHEMA,
    FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA,
};

/// The injected port the domain drives. Two adapters implement it:
/// - [`LocalForgejoTokenApiAdapter`] — runs `forgejo` CLI locally or via SSH-local Exec.
/// - [`HttpForgejoTokenApiAdapter`] — talks Forgejo REST API over HTTPS.
pub trait ForgejoTokenApiPort {
    /// Read-only: does the token file exist / is it valid against the API?
    fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation;

    /// Create a new token. Returns Forgejo-side metadata only.
    fn create(&self, spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String>;

    /// Remove the token whose sha1 matches `token_sha1`.
    fn remove(&self, spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<(), String>;
}

/// Read-only file-stat observation. Never returns the token bytes.
pub fn observe_forgejo_token(spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
    use std::os::unix::fs::MetadataExt;

    let target = PathBuf::from(&spec.target_path);
    let metadata = std::fs::symlink_metadata(&target);

    let mut unknowns = Vec::new();

    let (secret_present, owner_root, mode_o640) = match &metadata {
        Ok(md) => {
            if md.file_type().is_symlink() {
                unknowns.push("forgejo_token_target_is_symlink".to_string());
            }
            if !md.file_type().is_file() {
                unknowns.push("forgejo_token_target_not_regular_file".to_string());
            }
            (
                md.len() > 0,
                Some(md.uid() == 0),
                Some((md.mode() & 0o777) == spec.mode),
            )
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (false, None, None),
        Err(e) => {
            unknowns.push(format!("forgejo_token_metadata_unavailable:{e}"));
            (false, None, None)
        }
    };

    ForgejoTokenObservation {
        schema: FORGEJO_TOKEN_OBSERVATION_SCHEMA.to_string(),
        principal: spec.principal.clone(),
        path: spec.target_path.clone(),
        secret_present,
        owner_root,
        mode_o640,
        api_authenticated: ForgejoTokenAuthState::Unknown,
        unknowns,
        evidence_refs: Vec::new(),
    }
}

/// Local filesystem + local Exec adapter.
///
/// `create` shells out via argv (no shell composition) to `forgejo tokensAdd`.
/// The token value is read from stdout and written directly to
/// `spec.target_path`. The value is zeroized after writing.
#[derive(Debug, Clone, Copy)]
pub struct LocalForgejoTokenApiAdapter;

impl ForgejoTokenApiPort for LocalForgejoTokenApiAdapter {
    fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
        observe_forgejo_token(spec)
    }

    fn create(&self, spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        let token_value = run_forgejo_tokens_add(spec)?;
        write_protected_secret(&spec.target_path, &token_value, spec.mode)?;
        let metadata = forgejo_token_sha1(&spec.principal)
            .unwrap_or_else(|| ForgejoTokenMetadata {
                token_id: 0,
                name: spec.principal.clone(),
                sha1: "<unavailable>".to_string(),
            });
        Ok(metadata)
    }

    fn remove(&self, spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<(), String> {
        forgejo_token_revoke(&spec.principal, token_sha1)
    }
}

/// HTTPS adapter. Talks Forgejo REST API directly. Used when the host has
/// network reachability to the Forgejo HTTPS endpoint. No shell involved.
#[derive(Debug, Clone)]
pub struct HttpForgejoTokenApiAdapter {
    pub api_base: String,
    pub principal: String,
}

impl ForgejoTokenApiPort for HttpForgejoTokenApiAdapter {
    fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
        let mut obs = observe_forgejo_token(spec);

        if obs.secret_present {
            match forgejo_api_validate_token(&spec.target_path, &self.api_base) {
                Ok(true) => obs.api_authenticated = ForgejoTokenAuthState::Proven,
                Ok(false) => obs.api_authenticated = ForgejoTokenAuthState::Refuted,
                Err(e) => {
                    obs.unknowns.push(format!("forgejo_api_probe_failed:{e}"));
                    obs.api_authenticated = ForgejoTokenAuthState::Unknown;
                }
            }
        } else {
            obs.api_authenticated = ForgejoTokenAuthState::Absent;
        }
        obs
    }

    fn create(&self, spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        let scopes_csv = spec.scopes.join(",");
        let resp = forgejo_http_create_token(&self.api_base, &self.principal, &scopes_csv)?;
        let token_value = extract_token_from_api_response(&resp)?;
        write_protected_secret(&spec.target_path, &token_value, spec.mode)?;
        forgejo_http_token_sha1(&self.api_base, &self.principal)
    }

    fn remove(&self, spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<(), String> {
        forgejo_http_revoke_token(&self.api_base, &self.principal, token_sha1)
    }
}

/// High-level provisioner: observes, creates if needed, emits a proof report.
/// Follows the `storage_identity::enroll_storage_identity` orchestration shape
/// so tests can inject a fake port.
pub fn provision_forgejo_token(
    spec: &ForgejoTokenSpec,
    port: &dyn ForgejoTokenApiPort,
) -> ForgejoTokenProvisionReport {
    let before = port.observe(spec);

    if before.api_authenticated == ForgejoTokenAuthState::Proven {
        return ForgejoTokenProvisionReport {
            schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA,
            status: "ready",
            principal: spec.principal.clone(),
            mutated: false,
            before: before.clone(),
            after: before.clone(),
            proofs: vec!["token_api_authenticated_proven".to_string()],
            token_metadata: None,
            rollback_intent: "no mutation; nothing to roll back".to_string(),
            unknowns: Vec::new(),
        };
    }

    match port.create(spec) {
        Ok(metadata) => {
            let after = port.observe(spec);
            let converged = after.api_authenticated == ForgejoTokenAuthState::Proven;
            let mut proofs: Vec<String> = Vec::new();
            if after.secret_present {
                proofs.push("token_file_present".to_string());
            }
            if after.owner_root == Some(true) {
                proofs.push("token_owner_root".to_string());
            }
            if after.mode_o640 == Some(true) {
                proofs.push("token_mode_640".to_string());
            }
            if after.api_authenticated == ForgejoTokenAuthState::Proven {
                proofs.push("token_api_authenticated_proven".to_string());
            }
            let mut unknowns = after.unknowns.clone();
            if !converged {
                unknowns.push("forgejo_token_provision_read_back_not_converged".to_string());
            }
            let sha1_for_rollback = metadata.sha1.clone();
            ForgejoTokenProvisionReport {
                schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA,
                status: if converged { "completed" } else { "blocked" },
                principal: spec.principal.clone(),
                mutated: true,
                before,
                after,
                proofs,
                token_metadata: if metadata.sha1 != "<unavailable>" {
                    Some(metadata)
                } else {
                    None
                },
                rollback_intent: format!(
                    "revoke Forgejo token {} for principal '{}' via the same port",
                    sha1_for_rollback, spec.principal
                ),
                unknowns,
            }
        }
        Err(e) => ForgejoTokenProvisionReport {
            schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA,
            status: "blocked",
            principal: spec.principal.clone(),
            mutated: false,
            before,
            after: port.observe(spec),
            proofs: Vec::new(),
            token_metadata: None,
            rollback_intent: "no mutation due to error; nothing to roll back".to_string(),
            unknowns: vec![format!("forgejo_token_create_failed:{e}")],
        },
    }
}

// ── internal helpers (local-exec + http) ──────────────────────────────

fn run_forgejo_tokens_add(spec: &ForgejoTokenSpec) -> Result<String, String> {
    let mut cmd = Command::new("forgejo");
    cmd.args([
        "tokensAdd",
        &spec.principal,
        "--scopes",
        &spec.scopes.join(","),
    ])
    .arg("--format")
    .arg("json");

    if let Some(expiry) = spec.expiry_seconds {
        cmd.args(["--expiry", &format!("{}s", expiry)]);
    }

    let output = cmd
        .output()
        .map_err(|e| format!("forgejo tokensAdd spawn failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "forgejo tokensAdd failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Forgejo tokensAdd --format json emits {"sha1":"...","token":"..."}
    let value: serde_json::Value =
        serde_json::from_str(&stdout).map_err(|e| format!("forgejo JSON parse failed: {e}"))?;
    let token = value
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "forgejo tokensAdd did not return a token field".to_string())?;
    Ok(token.to_string())
}

fn forgejo_token_sha1(principal: &str) -> Option<ForgejoTokenMetadata> {
    let output = Command::new("forgejo")
        .args(["tokens", principal])
        .arg("--format")
        .arg("json")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value = serde_json::from_str(&stdout).ok()?;
    let entry = value.get(0)?;
    Some(ForgejoTokenMetadata {
        token_id: entry.get("id").and_then(|v| v.as_u64())?,
        name: entry.get("name").and_then(|v| v.as_str())?.to_string(),
        sha1: entry.get("sha1").and_then(|v| v.as_str())?.to_string(),
    })
}

fn forgejo_token_revoke(principal: &str, sha1: &str) -> Result<(), String> {
    let output = Command::new("forgejo")
        .args(["tokensDelete", principal])
        .arg(sha1)
        .output()
        .map_err(|e| format!("forgejo tokensDelete spawn failed: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "forgejo tokensDelete failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn forgejo_api_validate_token(token_path: &str, api_base: &str) -> Result<bool, String> {
    let token =
        std::fs::read_to_string(token_path).map_err(|e| format!("read token file: {e}"))?;
    let client = http_client()?;
    let resp = client
        .get(format!("{api_base}/api/v1/user"))
        .header("Authorization", format!("token {token}"))
        .send()
        .map_err(|e| format!("API probe request failed: {e}"))?;
    Ok(resp.status().is_success())
}

fn forgejo_http_create_token(
    api_base: &str,
    principal: &str,
    scopes: &str,
) -> Result<serde_json::Value, String> {
    let client = http_client()?;
    let body = serde_json::json!({ "name": principal, "scopes": scopes });
    let resp = client
        .post(format!("{api_base}/api/v1/users/{principal}/tokens"))
        .json(&body)
        .send()
        .map_err(|e| format!("token creation request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("token creation HTTP {}", resp.status()));
    }
    resp.json()
        .map_err(|e| format!("token creation JSON parse failed: {e}"))
}

fn forgejo_http_token_sha1(api_base: &str, principal: &str) -> Result<ForgejoTokenMetadata, String> {
    let client = http_client()?;
    let resp = client
        .get(format!("{api_base}/api/v1/users/{principal}/tokens"))
        .send()
        .map_err(|e| format!("token list request failed: {e}"))?;
    let tokens: serde_json::Value = resp
        .json()
        .map_err(|e| format!("token list JSON parse failed: {e}"))?;
    let first = tokens.get(0).ok_or_else(|| "no tokens found".to_string())?;
    let token_id = first.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
    let name = first
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let sha1 = first
        .get("sha1")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    Ok(ForgejoTokenMetadata {
        token_id,
        name,
        sha1,
    })
}

fn forgejo_http_revoke_token(api_base: &str, principal: &str, sha1: &str) -> Result<(), String> {
    let client = http_client()?;
    let resp = client
        .delete(format!("{api_base}/api/v1/users/{principal}/tokens/{sha1}"))
        .send()
        .map_err(|e| format!("token revoke request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("token revoke HTTP {}", resp.status()));
    }
    Ok(())
}

fn extract_token_from_api_response(resp: &serde_json::Value) -> Result<String, String> {
    resp.get("token")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| "API response missing token field".to_string())
}

fn write_protected_secret(target: &str, token: &str, mode: u32) -> Result<(), String> {
    let path = PathBuf::from(target);
    let parent = path
        .parent()
        .ok_or_else(|| "secret target has no parent".to_string())?;
    if !parent.as_os_str().is_empty() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("secret directory creation failed: {e}"))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&path)
        .map_err(|e| format!("secret file write failed: {e}"))?;
    file.write_all(token.as_bytes())
        .map_err(|e| format!("secret write_all failed: {e}"))?;
    file.sync_all()
        .map_err(|e| format!("secret sync failed: {e}"))?;
    let perms = std::fs::Permissions::from_mode(mode);
    std::fs::set_permissions(&path, perms)
        .map_err(|e| format!("secret chmod failed: {e}"))?;
    Ok(())
}

fn http_client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| format!("http client build failed: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_uses_path_from_spec() {
        let spec = ForgejoTokenSpec {
            target_path: "/tmp/nonexistent-token-xyz".to_string(),
            ..ForgejoTokenSpec::default()
        };
        let obs = observe_forgejo_token(&spec);
        assert!(!obs.secret_present);
        assert_eq!(obs.api_authenticated, ForgejoTokenAuthState::Unknown);
        assert_eq!(obs.path, "/tmp/nonexistent-token-xyz");
    }

    #[test]
    fn provision_skips_when_token_already_valid() {
        struct StubPort;
        impl ForgejoTokenApiPort for StubPort {
            fn observe(&self, _spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
                observe_forgejo_token(_spec)
            }
            fn create(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
                Err("should not be called".into())
            }
            fn remove(&self, _spec: &ForgejoTokenSpec, _sha1: &str) -> Result<(), String> {
                Err("should not be called".into())
            }
        }

        let spec = ForgejoTokenSpec {
            target_path: "/tmp/nonexistent-provision-skip".to_string(),
            ..ForgejoTokenSpec::default()
        };
        let report = provision_forgejo_token(&spec, &StubPort);
        assert_eq!(report.status, "ready");
        assert!(!report.mutated);
    }
}
