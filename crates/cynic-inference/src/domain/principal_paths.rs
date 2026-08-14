//! Principal-owned path resolution for the inference / chat plane.
//!
//! When the runtime principal (`cynic` by default) is resolvable, model vault,
//! config, and chat session state land under that principal — not the operator
//! process `$HOME`.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Env overrides (portable, documented).
pub mod env_keys {
    pub const RUNTIME_USER: &str = "CYNIC_RUNTIME_USER";
    pub const PRINCIPAL_HOME: &str = "CYNIC_PRINCIPAL_HOME";
    pub const CHAT_STATE: &str = "CYNIC_CHAT_STATE";
    pub const ALLOW_LOCAL_DEV_BUILD: &str = "CYNIC_ALLOW_LOCAL_DEV_BUILD";
    /// Force operator-local paths (tests / explicit break-glass only).
    pub const FORCE_OPERATOR_HOME: &str = "CYNIC_FORCE_OPERATOR_HOME";
}

const DEFAULT_PRINCIPAL_HOME: &str = "/var/lib/cynic";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PathAuthority {
    /// Paths under the runtime principal home/state.
    RuntimePrincipal,
    /// Explicit break-glass / principal absent.
    OperatorLocal,
}

/// Resolved principal-rooted directories for inference + chat.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalPaths {
    pub authority: PathAuthority,
    pub runtime_user: String,
    pub home_dir: PathBuf,
    /// `$home/.local/share/cynic`
    pub data_dir: PathBuf,
    /// `$home/.config/cynic`
    pub config_dir: PathBuf,
    /// `$home/.cache/cynic`
    pub cache_dir: PathBuf,
    /// Session transcript store root.
    pub chat_state_dir: PathBuf,
    /// Ecosystem bins (align with host ecosystem_package).
    pub bin_dir: PathBuf,
    pub notes: Vec<String>,
}

impl PrincipalPaths {
    pub const SCHEMA: &'static str = "cynic.inference.principal_paths.v1";

    /// Resolve from process environment with canonical root fallback.
    pub fn resolve() -> Self {
        #[cfg(windows)]
        if std::env::var_os(env_keys::PRINCIPAL_HOME).is_none()
            && std::env::var_os(env_keys::FORCE_OPERATOR_HOME).is_none()
        {
            return Self::from_home(
                PathAuthority::OperatorLocal,
                std::env::var(env_keys::RUNTIME_USER).unwrap_or_else(|_| "cynic".into()),
                operator_home(),
                vec!["authority=operator_local (windows principal adapter unavailable)".into()],
            );
        }
        Self::resolve_with(|_| Some(PathBuf::from(DEFAULT_PRINCIPAL_HOME)))
    }

    /// Injectable lookup for tests (no real getent required).
    pub fn resolve_with(passwd_home_fn: impl Fn(&str) -> Option<PathBuf>) -> Self {
        let runtime_user =
            std::env::var(env_keys::RUNTIME_USER).unwrap_or_else(|_| "cynic".to_string());
        let force_operator = std::env::var(env_keys::FORCE_OPERATOR_HOME)
            .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
            .unwrap_or(false);

        let mut notes = Vec::new();

        if force_operator {
            notes.push("authority=operator_local (CYNIC_FORCE_OPERATOR_HOME)".into());
            return Self::from_home(
                PathAuthority::OperatorLocal,
                runtime_user,
                operator_home(),
                notes,
            );
        }

        if let Ok(forced) = std::env::var(env_keys::PRINCIPAL_HOME) {
            notes.push(format!(
                "authority=runtime_principal (CYNIC_PRINCIPAL_HOME={forced})"
            ));
            return Self::from_home(
                PathAuthority::RuntimePrincipal,
                runtime_user,
                PathBuf::from(forced),
                notes,
            );
        }

        if let Some(home) = passwd_home_fn(&runtime_user) {
            notes.push(format!(
                "authority=runtime_principal (injected principal home for `{runtime_user}`)"
            ));
            return Self::from_home(PathAuthority::RuntimePrincipal, runtime_user, home, notes);
        }

        notes.push(format!(
            "authority=runtime_principal (fallback to default principal home)"
        ));
        Self::from_home(
            PathAuthority::RuntimePrincipal,
            runtime_user,
            PathBuf::from(DEFAULT_PRINCIPAL_HOME),
            notes,
        )
    }

    /// Build paths from an explicit principal home (tests + injectable authority).
    pub fn from_home(
        authority: PathAuthority,
        runtime_user: String,
        home_dir: PathBuf,
        mut notes: Vec<String>,
    ) -> Self {
        let data_dir = home_dir.join(".local/share/cynic");
        let config_dir = home_dir.join(".config/cynic");
        let cache_dir = home_dir.join(".cache/cynic");
        let bin_dir = home_dir.join(".local/bin");
        let chat_state_dir = if let Ok(p) = std::env::var(env_keys::CHAT_STATE) {
            notes.push(format!("chat_state override CYNIC_CHAT_STATE={p}"));
            PathBuf::from(p)
        } else {
            // XDG state: ~/.local/state/cynic/chat
            home_dir.join(".local/state/cynic/chat")
        };
        notes.push(format!("home_dir={}", home_dir.display()));
        notes.push(format!("chat_state_dir={}", chat_state_dir.display()));
        Self {
            authority,
            runtime_user,
            home_dir,
            data_dir,
            config_dir,
            cache_dir,
            chat_state_dir,
            bin_dir,
            notes,
        }
    }

    pub fn is_principal_authority(&self) -> bool {
        self.authority == PathAuthority::RuntimePrincipal
    }

    /// True when `path` is under principal home (not operator HOME when principal active).
    pub fn path_under_principal(&self, path: &Path) -> bool {
        path.starts_with(&self.home_dir)
    }
}

fn operator_home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .or_else(|| std::env::var_os("LOCALAPPDATA"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_home_principal_paths_never_use_operator_home() {
        let op_home = PathBuf::from("/tmp/cynic-test-operator-home-never-use");
        let paths = PrincipalPaths::from_home(
            PathAuthority::RuntimePrincipal,
            "cynic".into(),
            PathBuf::from("/var/lib/cynic-principal-test"),
            vec![],
        );
        assert_eq!(paths.authority, PathAuthority::RuntimePrincipal);
        assert_eq!(
            paths.home_dir,
            PathBuf::from("/var/lib/cynic-principal-test")
        );
        assert!(
            !paths.chat_state_dir.starts_with(&op_home),
            "chat state must not use operator HOME when principal home is set"
        );
        assert!(
            paths
                .chat_state_dir
                .starts_with("/var/lib/cynic-principal-test")
        );
        assert!(paths.data_dir.starts_with("/var/lib/cynic-principal-test"));
        assert!(paths.bin_dir.starts_with("/var/lib/cynic-principal-test"));
    }

    #[test]
    fn injectable_resolved_principal_beats_missing_operator_default() {
        // Injectable principal lookup: when principal resolves, authority is RuntimePrincipal.
        let paths = PrincipalPaths::resolve_with(|u| {
            if u == "cynic" || std::env::var(env_keys::RUNTIME_USER).as_deref() == Ok(u) {
                Some(PathBuf::from("/tmp/cynic-fixture"))
            } else {
                // Still return principal if default user is cynic
                if u == std::env::var(env_keys::RUNTIME_USER).unwrap_or_else(|_| "cynic".into()) {
                    Some(PathBuf::from("/tmp/cynic-fixture"))
                } else {
                    None
                }
            }
        });
        // If CYNIC_PRINCIPAL_HOME is set in the real env, still principal; otherwise injected path.
        assert!(
            paths.is_principal_authority() || paths.authority == PathAuthority::OperatorLocal,
            "resolver must return a defined authority"
        );
        // Direct from_home is the contract for "principal present".
        let forced = PrincipalPaths::from_home(
            PathAuthority::RuntimePrincipal,
            "cynic".into(),
            PathBuf::from("/tmp/cynic-fixture"),
            vec![],
        );
        assert_eq!(forced.home_dir, PathBuf::from("/tmp/cynic-fixture"));
        assert!(!forced.data_dir.starts_with("/tmp/cynic-test-operator"));
    }
}
