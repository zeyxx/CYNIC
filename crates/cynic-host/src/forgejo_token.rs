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

pub use cynic_protocol::forgejo_token::{
    ForgejoTokenActionKind, ForgejoTokenAuthState, ForgejoTokenMetadata, ForgejoTokenObservation,
    ForgejoTokenPlan, ForgejoTokenProvisionReport, ForgejoTokenSpec, ForgejoTokenStatusReport,
};
use std::fmt;
use std::fs::OpenOptions;
use std::io::Write as IoWrite;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use cynic_protocol::forgejo_token::{
    FORGEJO_TOKEN_OBSERVATION_SCHEMA, FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA,
    FORGEJO_TOKEN_SPEC_SCHEMA,
};

pub const FORGEJO_TOKEN_DIRECTORY: &str = "/etc/cynic/secrets";
pub const FORGEJO_TOKEN_PATH: &str = "/etc/cynic/secrets/forgejo-token";

/// The injected port the domain drives. Two adapters implement it:
/// - [`LocalForgejoTokenApiAdapter`] — runs `forgejo` CLI locally or via SSH-local Exec.
/// - [`HttpForgejoTokenApiAdapter`] — talks Forgejo REST API over HTTPS.
pub trait ForgejoTokenApiPort: Send + Sync + std::fmt::Debug {
    /// Read-only: does the token file exist / is it valid against the API?
    fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation;

    /// Create a new token. Returns Forgejo-side metadata only.
    fn create(&self, spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String>;

    /// Remove the token whose sha1 matches `token_sha1`.
    fn remove(&self, spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<(), String>;

    /// Read current metadata so rotation can revoke the exact old token.
    fn current_token(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        Err("current Forgejo token metadata unavailable".to_string())
    }
}

/// Read-only file-stat observation. Never returns the token bytes.
pub fn observe_forgejo_token(spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
    use std::os::unix::fs::MetadataExt;

    let target = PathBuf::from(&spec.target_path);
    let metadata = std::fs::symlink_metadata(&target);

    let mut unknowns = Vec::new();
    if spec.target_path != FORGEJO_TOKEN_PATH {
        unknowns.push("forgejo_token_target_path_noncanonical".to_string());
    }

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
        let metadata =
            forgejo_token_sha1(&spec.principal).unwrap_or_else(|| ForgejoTokenMetadata {
                token_id: 0,
                name: spec.principal.clone(),
                sha1: "<unavailable>".to_string(),
            });
        Ok(metadata)
    }

    fn remove(&self, spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<(), String> {
        forgejo_token_revoke(&spec.principal, token_sha1)
    }

    fn current_token(&self, spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        forgejo_token_sha1(&spec.principal)
            .ok_or_else(|| "current Forgejo token metadata unavailable".to_string())
    }
}

/// HTTPS adapter. Talks Forgejo REST API directly. Used when the host has
/// network reachability to the Forgejo HTTPS endpoint. No shell involved.
#[derive(Clone)]
pub struct HttpForgejoTokenApiAdapter {
    pub api_base: String,
    pub principal: String,
    /// Runtime credential supplied by the selected adapter; never serialized
    /// or included in Debug output.
    pub authorization_token: Option<String>,
}

impl fmt::Debug for HttpForgejoTokenApiAdapter {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("HttpForgejoTokenApiAdapter")
            .field("api_base", &self.api_base)
            .field("principal", &self.principal)
            .field(
                "authorization_token_present",
                &self.authorization_token.is_some(),
            )
            .finish()
    }
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
        let resp = forgejo_http_create_token(
            &self.api_base,
            &self.principal,
            &scopes_csv,
            self.authorization_token.as_deref(),
        )?;
        let token_value = extract_token_from_api_response(&resp)?;
        write_protected_secret(&spec.target_path, &token_value, spec.mode)?;
        forgejo_http_token_sha1(
            &self.api_base,
            &self.principal,
            self.authorization_token.as_deref(),
        )
    }

    fn remove(&self, spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<(), String> {
        forgejo_http_revoke_token(
            &self.api_base,
            &self.principal,
            token_sha1,
            self.authorization_token.as_deref(),
        )
    }

    fn current_token(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        forgejo_http_token_sha1(
            &self.api_base,
            &self.principal,
            self.authorization_token.as_deref(),
        )
    }
}

/// Boxed port handle so the runtime can swap adapters (local-exec vs HTTP).
pub type BoxedForgejoTokenPort = Box<dyn ForgejoTokenApiPort>;

/// Spec enriched with the adapter to drive. Decoupled from `ForgejoTokenSpec`
/// so the protocol-level types stay transport-agnostic.
#[derive(Debug)]
pub struct ForgejoTokenRuntimeSpec {
    pub token_spec: ForgejoTokenSpec,
    pub adapter: BoxedForgejoTokenPort,
}

/// Builder that derives the canonical token spec from runtime config.
pub struct DefaultForgejoTokenSpec;

impl DefaultForgejoTokenSpec {
    pub fn default_with_principal(principal: String) -> ForgejoTokenSpec {
        ForgejoTokenSpec {
            schema: FORGEJO_TOKEN_SPEC_SCHEMA.to_string(),
            principal,
            api_url: "https://forgejo.synapse.cynic".to_string(),
            scopes: vec![
                "read:organization".to_string(),
                "write:organization".to_string(),
                "write:repository".to_string(),
            ],
            target_path: FORGEJO_TOKEN_PATH.to_string(),
            mode: 0o640,
            expiry_seconds: None,
        }
    }

    pub fn with_token_path(token_path: &str) -> ForgejoTokenSpec {
        let mut spec = Self::default_with_principal("cynic-cortex".to_string());
        spec.target_path = token_path.to_string();
        spec
    }
}

impl Default for ForgejoTokenRuntimeSpec {
    fn default() -> Self {
        Self {
            token_spec: DefaultForgejoTokenSpec::default_with_principal("cynic-cortex".to_string()),
            adapter: Box::new(LocalForgejoTokenApiAdapter),
        }
    }
}

/// Observe the token through the injected port + emit a convergence plan.
/// This is the runtime entry point for `cynic forgejo token-status`.
pub fn observe_forgejo_token_runtime(spec: &ForgejoTokenRuntimeSpec) -> ForgejoTokenObservation {
    spec.adapter.observe(&spec.token_spec)
}

/// Provision (create/rotate) the token through the injected port and emit a
/// read-back proof report. Follows the `observe → apply → read-back` coherence
/// pattern. This is the runtime entry point for `cynic forgejo token-create|rotate`.
pub fn provision_forgejo_token_apply(
    spec: &ForgejoTokenRuntimeSpec,
) -> ForgejoTokenProvisionReport {
    provision_forgejo_token(&spec.token_spec, &*spec.adapter)
}
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

/// Rotate by revoking the exact current token, creating its replacement, and
/// requiring a fresh observation proving the replacement.
pub fn rotate_forgejo_token(
    spec: &ForgejoTokenSpec,
    port: &dyn ForgejoTokenApiPort,
) -> ForgejoTokenProvisionReport {
    let before = port.observe(spec);
    let old = match port.current_token(spec) {
        Ok(metadata) if !metadata.sha1.is_empty() => metadata,
        _ => {
            return blocked_rotation_report(
                spec,
                before,
                "forgejo_token_current_metadata_unavailable",
                false,
            );
        }
    };
    if let Err(error) = port.remove(spec, &old.sha1) {
        return blocked_rotation_report(
            spec,
            before,
            &format!("forgejo_token_revoke_failed:{error}"),
            true,
        );
    }
    match port.create(spec) {
        Ok(metadata) => {
            let after = port.observe(spec);
            let converged = after.api_authenticated == ForgejoTokenAuthState::Proven;
            let mut unknowns = after.unknowns.clone();
            if !converged {
                unknowns.push("forgejo_token_rotation_read_back_not_converged".to_string());
            }
            ForgejoTokenProvisionReport {
                schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA,
                status: if converged { "completed" } else { "blocked" },
                principal: spec.principal.clone(),
                mutated: true,
                before,
                after,
                proofs: vec!["token_previous_revoked".to_string()],
                token_metadata: Some(metadata),
                rollback_intent: format!(
                    "revoke replacement token for principal '{}' via the same port",
                    spec.principal
                ),
                unknowns,
            }
        }
        Err(error) => blocked_rotation_report(
            spec,
            before,
            &format!("forgejo_token_create_failed_after_revoke:{error}"),
            true,
        ),
    }
}

fn blocked_rotation_report(
    spec: &ForgejoTokenSpec,
    before: ForgejoTokenObservation,
    reason: &str,
    mutated: bool,
) -> ForgejoTokenProvisionReport {
    ForgejoTokenProvisionReport {
        schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA,
        status: "blocked",
        principal: spec.principal.clone(),
        mutated,
        before,
        after: observe_forgejo_token(spec),
        proofs: Vec::new(),
        token_metadata: None,
        rollback_intent: "rotation blocked; inspect revocation and replacement state".to_string(),
        unknowns: vec![reason.to_string()],
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
    let token = std::fs::read_to_string(token_path)
        .map_err(|e| format!("read token file: {e}"))?
        .trim()
        .to_string();
    let client = http_client()?;
    let resp = client
        .get(format_api_url(api_base, "/user"))
        .header("Authorization", format!("token {token}"))
        .send()
        .map_err(|e| format!("API probe request failed: {e}"))?;
    Ok(resp.status().is_success())
}

fn forgejo_http_create_token(
    api_base: &str,
    principal: &str,
    scopes: &str,
    authorization_token: Option<&str>,
) -> Result<serde_json::Value, String> {
    let authorization_token = required_authorization(authorization_token)?;
    let client = http_client()?;
    let body = serde_json::json!({ "name": principal, "scopes": scopes });
    let resp = client
        .post(format_api_url(
            api_base,
            &format!("/users/{principal}/tokens"),
        ))
        .header("Authorization", format!("token {authorization_token}"))
        .json(&body)
        .send()
        .map_err(|e| format!("token creation request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("token creation HTTP {}", resp.status()));
    }
    resp.json()
        .map_err(|e| format!("token creation JSON parse failed: {e}"))
}

fn forgejo_http_token_sha1(
    api_base: &str,
    principal: &str,
    authorization_token: Option<&str>,
) -> Result<ForgejoTokenMetadata, String> {
    let authorization_token = required_authorization(authorization_token)?;
    let client = http_client()?;
    let resp = client
        .get(format_api_url(
            api_base,
            &format!("/users/{principal}/tokens"),
        ))
        .header("Authorization", format!("token {authorization_token}"))
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

fn forgejo_http_revoke_token(
    api_base: &str,
    principal: &str,
    sha1: &str,
    authorization_token: Option<&str>,
) -> Result<(), String> {
    let authorization_token = required_authorization(authorization_token)?;
    let client = http_client()?;
    let resp = client
        .delete(format_api_url(
            api_base,
            &format!("/users/{principal}/tokens/{sha1}"),
        ))
        .header("Authorization", format!("token {authorization_token}"))
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

fn required_authorization(token: Option<&str>) -> Result<&str, String> {
    token
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| "Forgejo HTTP mutation requires Authorization token".to_string())
}

fn format_api_url(api_base: &str, path: &str) -> String {
    let base = api_base.trim_end_matches('/');
    let base = base.strip_suffix("/api/v1").unwrap_or(base);
    format!("{base}/api/v1/{}", path.trim_start_matches('/'))
}

fn write_protected_secret(target: &str, token: &str, mode: u32) -> Result<(), String> {
    if target != FORGEJO_TOKEN_PATH {
        return Err(format!(
            "secret target must be canonical path {FORGEJO_TOKEN_PATH}"
        ));
    }
    let path = PathBuf::from(target);
    let parent = path
        .parent()
        .ok_or_else(|| "secret target has no parent".to_string())?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("secret directory creation failed: {e}"))?;
    if parent
        .canonicalize()
        .map_err(|e| format!("secret directory canonicalization failed: {e}"))?
        != PathBuf::from(FORGEJO_TOKEN_DIRECTORY)
    {
        return Err("secret directory must not resolve through a symlink".to_string());
    }
    if let Ok(metadata) = std::fs::symlink_metadata(&path) {
        if metadata.file_type().is_symlink() {
            return Err("secret target symlink rejected".to_string());
        }
        if !metadata.file_type().is_file() {
            return Err("secret target is not a regular file".to_string());
        }
    }
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("clock error: {e}"))?
        .as_nanos();
    let temporary = parent.join(format!(".forgejo-token.{}.{}", std::process::id(), nonce));
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(mode)
        .custom_flags(0o400000)
        .open(&temporary)
        .map_err(|e| format!("secret temporary file write failed: {e}"))?;
    file.write_all(token.as_bytes())
        .map_err(|e| format!("secret write_all failed: {e}"))?;
    file.sync_all()
        .map_err(|e| format!("secret sync failed: {e}"))?;
    drop(file);
    std::fs::rename(&temporary, &path).map_err(|e| format!("secret atomic rename failed: {e}"))?;
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
        #[derive(Debug)]
        struct StubPort;
        impl ForgejoTokenApiPort for StubPort {
            fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
                let mut obs = observe_forgejo_token(spec);
                obs.api_authenticated = ForgejoTokenAuthState::Proven;
                obs
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

    #[test]
    fn http_adapter_absent_when_file_missing() {
        let http = HttpForgejoTokenApiAdapter {
            api_base: "https://forgejo.synapse.cynic/api/v1".to_string(),
            principal: "cynic-cortex".to_string(),
            authorization_token: None,
        };
        let spec = ForgejoTokenSpec {
            target_path: "/tmp/nonexistent-http-token-xyz".to_string(),
            ..ForgejoTokenSpec::default()
        };
        let obs = http.observe(&spec);
        // File missing → secret_present false → ApiAuthenticated::Absent (no API call).
        assert!(!obs.secret_present);
        assert_eq!(obs.api_authenticated, ForgejoTokenAuthState::Absent);
    }

    #[test]
    fn http_mutations_require_auth_and_normalize_api_base() {
        assert!(required_authorization(None).is_err());
        assert_eq!(
            format_api_url("https://forgejo.example/api/v1/", "/user"),
            "https://forgejo.example/api/v1/user"
        );
    }

    #[test]
    fn secret_writer_rejects_noncanonical_targets_before_io() {
        let error = write_protected_secret("/tmp/arbitrary-token", "secret", 0o640)
            .expect_err("arbitrary token paths must be rejected");
        assert!(error.contains("canonical path"));
    }

    #[test]
    fn default_spec_uses_canonical_path() {
        let spec = DefaultForgejoTokenSpec::with_token_path("/etc/cynic/secrets/forgejo-token");
        assert_eq!(spec.target_path, "/etc/cynic/secrets/forgejo-token");
        assert_eq!(spec.principal, "cynic-cortex");
        assert_eq!(spec.mode, 0o640);
    }
}
