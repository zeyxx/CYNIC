//! Registry binding for the sovereign Forgejo provider.
//!
//! Builds a `PrPort` from `infra/registry.json`'s
//! `vcs_provider.sovereign_ssot` block: reads the registry, resolves the
//! machine token (environment → principal file → OS keyring, in that
//! order), and constructs the Forgejo adapter.
//!
//! Fails closed at every step: an unreadable registry, a missing SSOT
//! block, an unknown provider kind, or a missing token all return `Err`
//! with a message naming what was tried. A missing token must never travel
//! to the API as an anonymous call that Forgejo answers with a cryptic
//! 404/400 (observed 2026-10-06).

use std::path::Path;

use cynic_secrets::{
    OsCredentialStoreSecretProvider, SecretProviderKind, SecretProviderPort, SecretReference,
};
use cynic_types::paths::CynicPaths;
use cynic_vcs::find_ssot_in_registry;
use cynic_vcs::pull_request::PrPort;

use crate::pull_request::ForgejoPrAdapter;

/// Resolve the sovereign VCS token from the environment, the principal
/// secret file, or the OS keyring — in that order.
///
/// Pure over injected readers so the refusal path is unit tested: a
/// missing token fails HERE with a message naming all three sources.
pub fn resolve_vcs_token(
    token_env: &str,
    token_file: &str,
    keyring_id: &str,
    read_env: impl Fn(&str) -> Option<String>,
    read_file: impl Fn(&str) -> Option<String>,
    read_keyring: impl Fn(&str) -> Option<String>,
) -> Result<String, String> {
    if let Some(value) = read_env(token_env).filter(|v| !v.is_empty()) {
        return Ok(value);
    }
    if let Some(value) = read_file(token_file)
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        return Ok(value);
    }
    if let Some(value) = read_keyring(keyring_id).filter(|v| !v.is_empty()) {
        return Ok(value);
    }
    Err(format!(
        "no sovereign VCS token: env {token_env} is unset, secret file {token_file} is unreadable, and OS keyring id {keyring_id} is absent; refusing anonymous API call"
    ))
}

/// Read `vcs_provider.sovereign_ssot` from `infra/registry.json` under
/// `root` and return a boxed `PrPort`.
///
/// Fails closed: returns `Err` if the registry is unreadable, the SSOT
/// block is missing, or the provider kind is unknown. Only `forgejo` is
/// implemented; any other kind is refused, never defaulted.
pub fn forgejo_pr_port_from_registry(root: &Path) -> Result<Box<dyn PrPort>, String> {
    let reg_path = root.join("infra/registry.json");
    let raw = std::fs::read_to_string(&reg_path)
        .map_err(|e| format!("registry unreadable at {}: {e}", reg_path.display()))?;
    let reg: serde_json::Value =
        serde_json::from_str(&raw).map_err(|e| format!("registry invalid JSON: {e}"))?;

    let ssot = find_ssot_in_registry(&reg).ok_or_else(|| {
        let nodes_len = reg
            .pointer("/nodes")
            .and_then(|v| v.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        format!("vcs_provider.sovereign_ssot not found in registry (nodes count: {nodes_len})")
    })?;

    let kind = ssot["kind"].as_str().unwrap_or("forgejo");
    let api_url = ssot["api_url"]
        .as_str()
        .unwrap_or("http://forgejo.service.cynic:3000");
    let owner = ssot["owner"].as_str().unwrap_or("jeanterre13");
    let repo = ssot["repo"].as_str().unwrap_or("CYNIC");
    let token_env = ssot["token_env"].as_str().unwrap_or("FORGEJO_TOKEN");
    let token_file = ssot["token_file"]
        .as_str()
        .map(str::to_string)
        .unwrap_or_else(|| {
            CynicPaths::secret_path("forgejo-token")
                .display()
                .to_string()
        });

    // Keyring id mirrors the principal file name (forgejo-token), same
    // convention as the headscale migration: file and keyring address the
    // same secret id in different backends.
    let keyring_id = "forgejo-token";
    let keyring = OsCredentialStoreSecretProvider::default();
    let token = resolve_vcs_token(
        token_env,
        &token_file,
        keyring_id,
        |name| std::env::var(name).ok(),
        |path| std::fs::read_to_string(path).ok(),
        |id| {
            keyring.resolve(&SecretReference::with_class(
                "forgejo",
                id,
                cynic_secrets::SecretClass::RuntimeMachineIdentity,
                SecretProviderKind::OsCredentialStore,
            ))
        },
    )?;

    match kind {
        "forgejo" => Ok(Box::new(ForgejoPrAdapter::new(
            api_url.to_string(),
            owner.to_string(),
            repo.to_string(),
            Some(token),
        ))),
        other => Err(format!(
            "unknown vcs_provider kind '{other}' — only 'forgejo' is implemented"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::resolve_vcs_token;

    #[test]
    fn token_prefers_env_over_file() {
        let got = resolve_vcs_token(
            "FORGEJO_TOKEN",
            "/nonexistent",
            "forgejo-token",
            |_| Some("env-token".to_string()),
            |_| Some("file-token".to_string()),
            |_| Some("keyring-token".to_string()),
        );
        assert_eq!(got, Ok("env-token".to_string()));
    }

    #[test]
    fn token_falls_back_to_file() {
        let got = resolve_vcs_token(
            "FORGEJO_TOKEN",
            "/etc/cynic/secrets/forgejo-token",
            "forgejo-token",
            |_| None,
            |_| Some("  file-token\n".to_string()),
            |_| Some("keyring-token".to_string()),
        );
        assert_eq!(got, Ok("file-token".to_string()));
    }

    #[test]
    fn token_falls_back_to_keyring() {
        let got = resolve_vcs_token(
            "FORGEJO_TOKEN",
            "/etc/cynic/secrets/forgejo-token",
            "forgejo-token",
            |_| None,
            |_| None,
            |_| Some("keyring-token".to_string()),
        );
        assert_eq!(got, Ok("keyring-token".to_string()));
    }

    #[test]
    fn missing_token_fails_closed_naming_all_sources() {
        let err = resolve_vcs_token(
            "FORGEJO_TOKEN",
            "/etc/cynic/secrets/forgejo-token",
            "forgejo-token",
            |_| None,
            |_| None,
            |_| None,
        )
        .expect_err("must refuse anonymous API call");
        assert!(err.contains("FORGEJO_TOKEN"), "names env source: {err}");
        assert!(
            err.contains("/etc/cynic/secrets/forgejo-token"),
            "names file source: {err}"
        );
        assert!(err.contains("forgejo-token"), "names keyring id: {err}");
    }
}
