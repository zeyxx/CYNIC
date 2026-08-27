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
use zeroize::Zeroize;

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

    /// Read current metadata only when the adapter can bind it to the installed
    /// token through an explicit, trustworthy contract. Principal/name matching
    /// is never sufficient.
    fn current_token(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        Err("current Forgejo token metadata unavailable".to_string())
    }

    /// Prove that a previously revoked token is no longer listed remotely.
    fn is_revoked(&self, _spec: &ForgejoTokenSpec, _token_sha1: &str) -> Result<bool, String> {
        Err("token revocation read-back unavailable".to_string())
    }
}

/// Read-only file-stat observation. Never returns the token bytes.
pub fn observe_forgejo_token(spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
    if spec.target_path != FORGEJO_TOKEN_PATH {
        return ForgejoTokenObservation {
            schema: FORGEJO_TOKEN_OBSERVATION_SCHEMA.to_string(),
            principal: spec.principal.clone(),
            path: spec.target_path.clone(),
            secret_present: false,
            owner_root: None,
            mode_o640: None,
            api_authenticated: ForgejoTokenAuthState::Unknown,
            unknowns: vec!["forgejo_token_target_path_noncanonical".to_string()],
            evidence_refs: Vec::new(),
        };
    }
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
        let (mut token_value, metadata) = run_forgejo_tokens_add(spec)?;
        let write_result = write_protected_secret(&spec.target_path, &token_value, spec.mode);
        token_value.zeroize();
        write_result?;
        Ok(metadata)
    }

    fn remove(&self, spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<(), String> {
        forgejo_token_revoke(&spec.principal, token_sha1)
    }

    fn current_token(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        Err("current Forgejo token identity is not bound to the installed secret".to_string())
    }

    fn is_revoked(&self, spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<bool, String> {
        forgejo_token_is_revoked(&spec.principal, token_sha1)
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
        let (mut token_value, metadata) = extract_api_token_material(&resp)?;
        let write_result = write_protected_secret(&spec.target_path, &token_value, spec.mode);
        token_value.zeroize();
        write_result?;
        Ok(metadata)
    }

    fn remove(&self, _spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<(), String> {
        forgejo_http_revoke_token(
            &self.api_base,
            &self.principal,
            token_sha1,
            self.authorization_token.as_deref(),
        )
    }

    fn current_token(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        Err("current Forgejo token identity is not bound to the installed secret".to_string())
    }

    fn is_revoked(&self, _spec: &ForgejoTokenSpec, token_sha1: &str) -> Result<bool, String> {
        forgejo_http_token_is_revoked(
            &self.api_base,
            &self.principal,
            token_sha1,
            self.authorization_token.as_deref(),
        )
    }
}

/// Adapter used when runtime reachability and credential injection have not
/// been proven. It never probes or mutates, so the surface fails closed.
#[derive(Debug, Clone, Copy)]
pub struct UnavailableForgejoTokenApiAdapter;

impl ForgejoTokenApiPort for UnavailableForgejoTokenApiAdapter {
    fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
        let mut observation = observe_forgejo_token(spec);
        observation.api_authenticated = ForgejoTokenAuthState::Unknown;
        observation
            .unknowns
            .push("forgejo_token_adapter_selection_unproven".to_string());
        observation
    }

    fn create(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
        Err("Forgejo token adapter selection is unproven".to_string())
    }

    fn remove(&self, _spec: &ForgejoTokenSpec, _token_sha1: &str) -> Result<(), String> {
        Err("Forgejo token adapter selection is unproven".to_string())
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
            adapter: Box::new(UnavailableForgejoTokenApiAdapter),
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
    if spec.target_path != FORGEJO_TOKEN_PATH {
        let observation = port.observe(spec);
        return ForgejoTokenProvisionReport {
            schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA,
            status: "blocked",
            principal: spec.principal.clone(),
            mutated: false,
            before: observation.clone(),
            after: observation,
            proofs: Vec::new(),
            token_metadata: None,
            rollback_intent: "no mutation; non-canonical token path".to_string(),
            unknowns: vec!["forgejo_token_target_path_noncanonical".to_string()],
        };
    }
    let before = port.observe(spec);

    if before.api_authenticated == ForgejoTokenAuthState::Proven {
        return ForgejoTokenProvisionReport {
            schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA,
            status: "ready",
            principal: spec.principal.clone(),
            mutated: false,
            before: before.clone(),
            after: before.clone(),
            // Proofs describe completed post-action evidence, not readiness.
            proofs: Vec::new(),
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
                proofs: if converged { proofs } else { Vec::new() },
                token_metadata: if converged && metadata.sha1 != "<unavailable>" {
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
    if spec.target_path != FORGEJO_TOKEN_PATH {
        let observation = port.observe(spec);
        return ForgejoTokenProvisionReport {
            schema: FORGEJO_TOKEN_PROVISION_REPORT_SCHEMA,
            status: "blocked",
            principal: spec.principal.clone(),
            mutated: false,
            before: observation.clone(),
            after: observation,
            proofs: Vec::new(),
            token_metadata: None,
            rollback_intent: "no mutation; non-canonical token path".to_string(),
            unknowns: vec!["forgejo_token_target_path_noncanonical".to_string()],
        };
    }
    let before = port.observe(spec);
    let old = match port.current_token(spec) {
        Ok(metadata) if validate_token_identity(&metadata.sha1).is_ok() => metadata,
        _ => {
            return blocked_rotation_report(
                spec,
                port,
                before,
                "forgejo_token_current_metadata_unavailable",
                false,
            );
        }
    };
    if let Err(error) = port.remove(spec, &old.sha1) {
        return blocked_rotation_report(
            spec,
            port,
            before,
            &format!("forgejo_token_revoke_failed:{error}"),
            true,
        );
    }
    match port.is_revoked(spec, &old.sha1) {
        Ok(true) => {}
        Ok(false) => {
            return blocked_rotation_report(
                spec,
                port,
                before,
                "forgejo_token_revocation_read_back_refuted",
                true,
            );
        }
        Err(error) => {
            return blocked_rotation_report(
                spec,
                port,
                before,
                &format!("forgejo_token_revocation_read_back_unknown:{error}"),
                true,
            );
        }
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
                proofs: if converged {
                    vec!["token_previous_revoked".to_string()]
                } else {
                    Vec::new()
                },
                token_metadata: if converged { Some(metadata) } else { None },
                rollback_intent: format!(
                    "revoke replacement token for principal '{}' via the same port",
                    spec.principal
                ),
                unknowns,
            }
        }
        Err(error) => blocked_rotation_report(
            spec,
            port,
            before,
            &format!("forgejo_token_create_failed_after_revoke:{error}"),
            true,
        ),
    }
}

fn blocked_rotation_report(
    spec: &ForgejoTokenSpec,
    port: &dyn ForgejoTokenApiPort,
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
        after: port.observe(spec),
        proofs: Vec::new(),
        token_metadata: None,
        rollback_intent: "rotation blocked; inspect revocation and replacement state".to_string(),
        unknowns: vec![reason.to_string()],
    }
}

// ── internal helpers (local-exec + http) ──────────────────────────────

fn run_forgejo_tokens_add(
    spec: &ForgejoTokenSpec,
) -> Result<(String, ForgejoTokenMetadata), String> {
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
    let metadata = extract_token_metadata(&value)?;
    Ok((token.to_string(), metadata))
}

fn forgejo_token_is_revoked(principal: &str, token_sha1: &str) -> Result<bool, String> {
    let output = Command::new("forgejo")
        .args(["tokens", principal, "--format", "json"])
        .output()
        .map_err(|e| format!("forgejo token list spawn failed: {e}"))?;
    if !output.status.success() {
        return Err("forgejo token list failed during revocation read-back".to_string());
    }
    let value: serde_json::Value = serde_json::from_slice(&output.stdout)
        .map_err(|e| format!("forgejo token list JSON parse failed: {e}"))?;
    let entries = value
        .as_array()
        .ok_or_else(|| "forgejo token list returned a non-array".to_string())?;
    Ok(!entries
        .iter()
        .any(|entry| entry.get("sha1").and_then(|v| v.as_str()) == Some(token_sha1)))
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
    if token_path != FORGEJO_TOKEN_PATH {
        return Err("token API probe requires the canonical token path".to_string());
    }
    let mut token =
        std::fs::read_to_string(FORGEJO_TOKEN_PATH).map_err(|e| format!("read token file: {e}"))?;
    let token_value = token.trim().to_string();
    token.zeroize();
    let client = http_client()?;
    let result = client
        .get(format_api_url(api_base, "/user"))
        .header("Authorization", format!("token {token_value}"))
        .send()
        .map(|resp| resp.status().is_success())
        .map_err(|e| format!("API probe request failed: {e}"));
    drop(token_value);
    result
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

fn forgejo_http_token_is_revoked(
    api_base: &str,
    principal: &str,
    token_sha1: &str,
    authorization_token: Option<&str>,
) -> Result<bool, String> {
    let authorization_token = required_authorization(authorization_token)?;
    let client = http_client()?;
    let resp = client
        .get(format_api_url(
            api_base,
            &format!("/users/{principal}/tokens"),
        ))
        .header("Authorization", format!("token {authorization_token}"))
        .send()
        .map_err(|e| format!("token revocation read-back request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("token revocation read-back HTTP {}", resp.status()));
    }
    let tokens: serde_json::Value = resp
        .json()
        .map_err(|e| format!("token revocation read-back JSON parse failed: {e}"))?;
    let entries = tokens
        .as_array()
        .ok_or_else(|| "token revocation read-back returned a non-array".to_string())?;
    Ok(!entries
        .iter()
        .any(|entry| entry.get("sha1").and_then(|v| v.as_str()) == Some(token_sha1)))
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

fn extract_api_token_material(
    resp: &serde_json::Value,
) -> Result<(String, ForgejoTokenMetadata), String> {
    let mut token = extract_token_from_api_response(resp)?;
    let metadata = match extract_token_metadata(resp) {
        Ok(metadata) => metadata,
        Err(error) => {
            token.zeroize();
            return Err(error);
        }
    };
    Ok((token, metadata))
}

fn validate_token_identity(sha1: &str) -> Result<(), String> {
    if sha1.len() != 40 {
        return Err("Forgejo token identity must be exactly 40 hexadecimal characters".to_string());
    }
    if !sha1.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("Forgejo token identity must be hexadecimal".to_string());
    }
    Ok(())
}

fn extract_token_metadata(resp: &serde_json::Value) -> Result<ForgejoTokenMetadata, String> {
    Ok(ForgejoTokenMetadata {
        token_id: resp
            .get("id")
            .and_then(|value| value.as_u64())
            .ok_or_else(|| "token response missing exact id".to_string())?,
        name: resp
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or_else(|| "token response missing exact name".to_string())?
            .to_string(),
        sha1: resp
            .get("sha1")
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| "token response missing exact sha1".to_string())?
            .to_string(),
    })
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
    let mut file = match OpenOptions::new()
        .create_new(true)
        .write(true)
        .mode(mode)
        .custom_flags(0o400000)
        .open(&temporary)
    {
        Ok(file) => file,
        Err(e) => {
            let _ = std::fs::remove_file(&temporary);
            return Err(format!("secret temporary file write failed: {e}"));
        }
    };
    if let Err(e) = file.write_all(token.as_bytes()) {
        drop(file);
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("secret write_all failed: {e}"));
    }
    if let Err(e) = file.sync_all() {
        drop(file);
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("secret sync failed: {e}"));
    }
    drop(file);
    if let Err(e) = std::fs::rename(&temporary, &path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(format!("secret atomic rename failed: {e}"));
    }
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
            target_path: FORGEJO_TOKEN_PATH.to_string(),
            ..ForgejoTokenSpec::default()
        };
        let report = provision_forgejo_token(&spec, &StubPort);
        assert_eq!(report.status, "ready");
        assert!(!report.mutated);
        assert!(report.proofs.is_empty());
    }

    #[test]
    fn concrete_adapters_fail_closed_without_installed_identity_binding() {
        let spec = ForgejoTokenSpec::default();
        assert!(LocalForgejoTokenApiAdapter.current_token(&spec).is_err());
        let http = HttpForgejoTokenApiAdapter {
            api_base: "https://forgejo.example".to_string(),
            principal: spec.principal.clone(),
            authorization_token: None,
        };
        assert!(http.current_token(&spec).is_err());
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
    fn noncanonical_paths_observe_through_injected_port() {
        #[derive(Debug)]
        struct StubPort;
        impl ForgejoTokenApiPort for StubPort {
            fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
                let mut observation = observe_forgejo_token(spec);
                observation
                    .unknowns
                    .push("injected_observation".to_string());
                observation
            }
            fn create(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
                Err("must not create".to_string())
            }
            fn remove(&self, _spec: &ForgejoTokenSpec, _sha1: &str) -> Result<(), String> {
                Err("must not remove".to_string())
            }
        }

        let spec = ForgejoTokenSpec {
            target_path: "/tmp/noncanonical-token".to_string(),
            ..ForgejoTokenSpec::default()
        };
        let provision = provision_forgejo_token(&spec, &StubPort);
        let rotation = rotate_forgejo_token(&spec, &StubPort);
        assert!(
            provision
                .before
                .unknowns
                .iter()
                .any(|u| u == "injected_observation")
        );
        assert!(
            rotation
                .before
                .unknowns
                .iter()
                .any(|u| u == "injected_observation")
        );
    }

    #[test]
    fn api_material_requires_metadata_before_secret_write() {
        let response = serde_json::json!({"token": "raw-secret"});
        let error = extract_api_token_material(&response).expect_err("metadata is required first");
        assert!(error.contains("missing exact id"));
    }

    #[test]
    fn rotation_identity_validation_rejects_placeholders_before_remove() {
        #[derive(Debug)]
        struct InvalidIdentityPort;
        impl ForgejoTokenApiPort for InvalidIdentityPort {
            fn observe(&self, spec: &ForgejoTokenSpec) -> ForgejoTokenObservation {
                observe_forgejo_token(spec)
            }
            fn create(&self, _spec: &ForgejoTokenSpec) -> Result<ForgejoTokenMetadata, String> {
                panic!("create must not run")
            }
            fn remove(&self, _spec: &ForgejoTokenSpec, _sha1: &str) -> Result<(), String> {
                panic!("remove must not run for an invalid identity")
            }
            fn current_token(
                &self,
                _spec: &ForgejoTokenSpec,
            ) -> Result<ForgejoTokenMetadata, String> {
                Ok(ForgejoTokenMetadata {
                    token_id: 1,
                    name: "cynic".to_string(),
                    sha1: "<unavailable>".to_string(),
                })
            }
        }

        let spec = ForgejoTokenSpec {
            target_path: FORGEJO_TOKEN_PATH.to_string(),
            ..ForgejoTokenSpec::default()
        };
        let report = rotate_forgejo_token(&spec, &InvalidIdentityPort);
        assert_eq!(report.status, "blocked");
        assert!(!report.mutated);
        assert_eq!(
            report.unknowns[0],
            "forgejo_token_current_metadata_unavailable"
        );
    }

    #[test]
    fn rotation_identity_validation_rejects_placeholders_and_non_hex() {
        assert!(validate_token_identity("").is_err());
        assert!(validate_token_identity("<unavailable>").is_err());
        assert!(validate_token_identity(&"a".repeat(39)).is_err());
        assert!(validate_token_identity(&"g".repeat(40)).is_err());
        assert!(validate_token_identity(&"a".repeat(40)).is_ok());
    }

    #[test]
    fn runtime_default_is_unavailable() {
        let runtime = ForgejoTokenRuntimeSpec::default();
        let observation = observe_forgejo_token_runtime(&runtime);
        assert!(
            observation
                .unknowns
                .iter()
                .any(|unknown| unknown == "forgejo_token_adapter_selection_unproven")
        );
        assert!(runtime.adapter.create(&runtime.token_spec).is_err());
    }

    #[test]
    fn default_spec_uses_canonical_path() {
        let spec = DefaultForgejoTokenSpec::with_token_path("/etc/cynic/secrets/forgejo-token");
        assert_eq!(spec.target_path, "/etc/cynic/secrets/forgejo-token");
        assert_eq!(spec.principal, "cynic-cortex");
        assert_eq!(spec.mode, 0o640);
    }
}
