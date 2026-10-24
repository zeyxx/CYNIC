//! Forgejo personal access token lifecycle port and adapters.

pub mod api;
pub mod provision;

pub use provision::*;

use std::fmt;
use std::path::PathBuf;
use zeroize::Zeroize;

use cynic_types::paths::CynicPaths;
use crate::forgejo_token::{
    FORGEJO_TOKEN_OBSERVATION_SCHEMA, FORGEJO_TOKEN_SPEC_SCHEMA,
    ForgejoTokenAuthState, ForgejoTokenMetadata,
    ForgejoTokenObservation, ForgejoTokenSpec,
};

use self::api::*;

/// Canonical Forgejo token directory (secrets authority).
pub fn forgejo_token_directory() -> PathBuf {
    CynicPaths::secrets_dir()
}

/// Canonical Forgejo token path (secrets authority).
pub fn forgejo_token_path() -> PathBuf {
    CynicPaths::secret_path("forgejo-token")
}

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
    if spec.target_path != forgejo_token_path().display().to_string() {
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
    if spec.target_path != forgejo_token_path().display().to_string() {
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
        let api_url = std::env::var("CYNIC_FORGEJO_API_URL")
            .or_else(|_| std::env::var("FORGEJO_API_URL"))
            .or_else(|_| std::env::var("FORGEJO_URL"))
            .unwrap_or_else(|_| "http://forgejo.service.cynic:3000".to_string());
        ForgejoTokenSpec {
            schema: FORGEJO_TOKEN_SPEC_SCHEMA.to_string(),
            principal,
            api_url,
            scopes: vec![
                "read:organization".to_string(),
                "write:organization".to_string(),
                "write:repository".to_string(),
            ],
            target_path: forgejo_token_path().display().to_string(),
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

