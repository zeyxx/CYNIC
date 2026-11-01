use std::path::{Path, PathBuf};

/// Constantes et résolveurs de chemins canoniques pour le système CYNIC.
/// Conforme à la politique normative `infra/directory-tree.json` (schema: cynic.directory_tree.v1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CynicPaths;

impl CynicPaths {
    /// Racine système installée pour la configuration et les secrets (K6/K8 boundary).
    /// Défini par `infra/directory-tree.json:installed_system.root`.
    pub const DEFAULT_SYSTEM_ROOT: &'static str = "/etc/cynic";

    /// Nom canonique du registre de placement runtime.
    pub const DEFAULT_REGISTRY_FILENAME: &'static str = "registry.json";

    /// Nom canonique du fichier de configuration des modèles et backends d'inférence.
    pub const DEFAULT_INFERENCE_FILENAME: &'static str = "inference.toml";

    /// Nom canonique du fichier de configuration du stockage souverain.
    pub const DEFAULT_STORAGE_FILENAME: &'static str = "storage.toml";

    /// Répertoire canonique des secrets machine (permissions 0640 cynic:cynic).
    pub const DEFAULT_SECRETS_DIRNAME: &'static str = "secrets";

    /// Canonical persistent state root (cynic_state).
    /// Defined by `infra/directory-tree.json:cynic_state.root`.
    pub const DEFAULT_STATE_ROOT: &'static str = "/var/lib/cynic";

    /// Canonical log root for runtime logs.
    pub const DEFAULT_LOG_ROOT: &'static str = "/var/log/cynic";

    /// Canonical install directory for runtime binaries.
    pub const DEFAULT_BINARY_DIR: &'static str = "/usr/local/bin";

    /// Canonical file name of the installed CYNIC binary.
    ///
    /// `infra/directory-tree.json:system_binaries.tree.root` declares
    /// `/usr/local/bin/cynic` — the binary itself, not its directory.
    pub const DEFAULT_BINARY_NAME: &'static str = "cynic";

    /// Path prefixes that must be redacted from any text leaving the host.
    ///
    /// These are SENSITIVE DATA, not destinations: a caller redacts them
    /// out of a log, a proof card or a public summary. Naming them here
    /// keeps one list instead of one literal per call site, and keeps the
    /// literal out of the call sites so the path gate reads a table rather
    /// than several competing claims.
    ///
    /// The first two entries are operator homes, the next two are the
    /// canonical CYNIC roots declared in `infra/directory-tree.json`, and
    /// the last is the superuser home.
    pub const REDACTION_PREFIXES: &'static [&'static str] = &[
        "/home/user",
        "/home/cynic",
        Self::DEFAULT_STATE_ROOT,
        "/root",
        Self::DEFAULT_SYSTEM_ROOT,
    ];

    /// Résout la racine de configuration système.
    /// Ordre d'autorité :
    /// 1. Variable d'environnement `CYNIC_CONFIG_DIR` (dérivation explicite en test/dev).
    /// 2. `/etc/cynic` (norme installée en production).
    pub fn system_config_dir() -> PathBuf {
        std::env::var("CYNIC_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(Self::DEFAULT_SYSTEM_ROOT))
    }

    /// Locate the canonical registry path.
    ///
    /// Hierarchy:
    /// 1. `CYNIC_REGISTRY_PATH` environment variable if the file exists.
    /// 2. Walk upwards from the current directory looking for `infra/registry.json`.
    /// 3. `CynicPaths::registry_path()` if the file exists.
    pub fn find_canonical_registry_path() -> Option<PathBuf> {
        if let Ok(path_str) = std::env::var("CYNIC_REGISTRY_PATH") {
            let p = PathBuf::from(path_str);
            if p.is_file() {
                return Some(p);
            }
        }

        if let Ok(mut dir) = std::env::current_dir() {
            loop {
                let candidate = dir.join("infra").join("registry.json");
                if candidate.is_file() {
                    return Some(candidate);
                }
                if !dir.pop() {
                    break;
                }
            }
        }

        let canonical = Self::registry_path();
        if canonical.is_file() {
            return Some(canonical);
        }

        None
    }

    /// Résout le chemin du registre runtime de flotte.
    /// Ordre d'autorité :
    /// 1. `CYNIC_REGISTRY_PATH`
    /// 2. `<system_config_dir>/registry.json`
    pub fn registry_path() -> PathBuf {
        std::env::var("CYNIC_REGISTRY_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Self::system_config_dir().join(Self::DEFAULT_REGISTRY_FILENAME))
    }

    /// Résout le chemin de la configuration d'inférence souveraine.
    /// Ordre d'autorité :
    /// 1. `CYNIC_INFERENCE_CONFIG`
    /// 2. `<system_config_dir>/inference.toml`
    pub fn inference_config_path() -> PathBuf {
        std::env::var("CYNIC_INFERENCE_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Self::system_config_dir().join(Self::DEFAULT_INFERENCE_FILENAME))
    }

    /// Canonical name of the Forgejo bearer-token secret file.
    ///
    /// `infra/directory-tree.json:installed_system.tree.key_paths["secrets/"]`
    /// declares the secrets directory and names this file in it.
    pub const DEFAULT_FORGEJO_TOKEN_SECRET: &'static str = "forgejo-token";

    /// Résout le chemin du répertoire des secrets.
    /// Ordre d'autorité :
    /// 1. `CYNIC_SECRETS_DIR`
    /// 2. `<system_config_dir>/secrets`
    pub fn secrets_dir() -> PathBuf {
        std::env::var("CYNIC_SECRETS_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Self::system_config_dir().join(Self::DEFAULT_SECRETS_DIRNAME))
    }

    /// Résout le chemin d'un secret spécifique par son nom (ex: "forgejo-token", "storage-runtime-password").
    pub fn secret_path(secret_name: &str) -> PathBuf {
        Self::secrets_dir().join(secret_name)
    }

    /// Résout le chemin de configuration du stockage (SurrealDB / local engine).
    pub fn storage_config_path() -> PathBuf {
        std::env::var("CYNIC_STORAGE_CONFIG")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Self::system_config_dir().join(Self::DEFAULT_STORAGE_FILENAME))
    }

    /// Resolve the primary persistent state root (/var/lib/cynic).
    pub fn state_dir() -> PathBuf {
        std::env::var("CYNIC_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(Self::DEFAULT_STATE_ROOT))
    }

    /// Resolve the runtime log root (/var/log/cynic).
    /// Authority order:
    /// 1. `CYNIC_LOG_DIR` environment variable.
    /// 2. `/var/log/cynic` (installed production norm).
    pub fn log_dir() -> PathBuf {
        std::env::var("CYNIC_LOG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(Self::DEFAULT_LOG_ROOT))
    }

    /// Resolve the runtime binary install directory (/usr/local/bin).
    /// Authority order:
    /// 1. `CYNIC_BINARY_DIR` environment variable.
    /// 2. `/usr/local/bin` (installed production norm).
    pub fn binary_dir() -> PathBuf {
        std::env::var("CYNIC_BINARY_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(Self::DEFAULT_BINARY_DIR))
    }

    /// Resolve the installed CYNIC binary itself.
    /// Authority order:
    /// 1. `CYNIC_BIN` environment variable (the operator's override).
    /// 2. `CYNIC_BINARY_DIR` joined with the canonical binary name.
    /// 3. `/usr/local/bin/cynic` (installed production norm, the root
    ///    declared by `infra/directory-tree.json:system_binaries`).
    pub fn binary_path() -> PathBuf {
        std::env::var("CYNIC_BIN")
            .map(PathBuf::from)
            .unwrap_or_else(|_| Self::binary_dir().join(Self::DEFAULT_BINARY_NAME))
    }

    /// Résout le répertoire de configuration utilisateur (~/.config/cynic).
    pub fn user_config_dir() -> Option<PathBuf> {
        std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
            .map(|base| base.join("cynic"))
            .ok()
    }

    /// Résout le répertoire de cache utilisateur (~/.cache/cynic).
    pub fn user_cache_dir() -> PathBuf {
        std::env::var("CYNIC_CACHE_DIR")
            .map(PathBuf::from)
            .or_else(|_| {
                std::env::var("XDG_CACHE_HOME")
                    .map(PathBuf::from)
                    .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".cache")))
            })
            .map(|base| base.join("cynic"))
            .unwrap_or_else(|_| PathBuf::from("/tmp/cynic-cache"))
    }

    /// Preferred system path of the sovereign knowledge cache
    /// (corpus_cache.json). Pure: performs no I/O and creates nothing.
    /// Callers running with I/O rights ensure the parent directory and
    /// fall back to [`Self::user_knowledge_cache_path`] when the system
    /// location is not writable.
    pub fn system_knowledge_cache_path() -> PathBuf {
        Self::state_dir().join("knowledge/corpus_cache.json")
    }

    /// Per-user fallback of the sovereign knowledge cache
    /// (~/.cache/cynic/corpus_cache.json). Pure.
    pub fn user_knowledge_cache_path() -> PathBuf {
        Self::user_cache_dir().join("corpus_cache.json")
    }

    /// Preferred system path of the sovereign rerank score cache (rerank_cache.json). Pure.
    pub fn system_rerank_cache_path() -> PathBuf {
        Self::state_dir().join("knowledge/rerank_cache.json")
    }

    /// Per-user fallback of the sovereign rerank score cache (~/.cache/cynic/rerank_cache.json). Pure.
    pub fn user_rerank_cache_path() -> PathBuf {
        Self::user_cache_dir().join("rerank_cache.json")
    }

    /// Vérifie si un chemin donné pointe vers l'arborescence système cynic.
    pub fn is_under_system_root(path: &Path) -> bool {
        path.starts_with(Self::DEFAULT_SYSTEM_ROOT)
    }

    /// Canonical root for shared build targets: `<state_root>/build-targets`.
    pub fn build_targets_root(state_root: &Path) -> PathBuf {
        state_root.join("build-targets")
    }

    /// Canonical root for shared build workspaces: `<state_root>/build-workspaces`.
    pub fn build_workspaces_root(state_root: &Path) -> PathBuf {
        state_root.join("build-workspaces")
    }

    /// Canonical artifacts directory: `<state_root>/artifacts`.
    pub fn build_artifacts_dir(state_root: &Path) -> PathBuf {
        state_root.join("artifacts")
    }

    /// Canonical evidence directory: `<state_root>/evidence`.
    pub fn build_evidence_dir(state_root: &Path) -> PathBuf {
        state_root.join("evidence")
    }

    /// Canonical per-profile build workspace: `<workspaces_root>/<profile>`.
    pub fn build_workspace_dir(workspaces_root: &Path, profile: &str) -> PathBuf {
        workspaces_root.join(profile)
    }

    /// Scoped build target directory for a checkout.
    ///
    /// When `shared_root` is `Some(root)`: `<root>/<profile>/<checkout_name>`,
    /// where `checkout_name` is the final path component of `workspace_root`.
    /// A hostile/empty/invalid checkout name falls back to `<root>/<profile>`.
    /// When `shared_root` is `None`: `<workspace_root>/target`.
    pub fn scoped_build_target_dir(
        shared_root: Option<&Path>,
        workspace_root: &Path,
        profile: &str,
    ) -> PathBuf {
        match shared_root {
            Some(root) => {
                let profile_root = root.join(profile);
                let checkout_name = workspace_root
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("");
                let name = checkout_name.trim();
                if name.is_empty()
                    || name == "."
                    || name == ".."
                    || name.contains('/')
                    || name.contains('\\')
                    || name.contains('\0')
                {
                    profile_root
                } else {
                    profile_root.join(name)
                }
            }
            None => workspace_root.join("target"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn build_targets_root_joins() {
        let root = PathBuf::from("/var/lib/cynic");
        assert_eq!(CynicPaths::build_targets_root(&root), root.join("build-targets"));
    }

    #[test]
    fn build_workspaces_root_joins() {
        let root = PathBuf::from("/var/lib/cynic");
        assert_eq!(CynicPaths::build_workspaces_root(&root), root.join("build-workspaces"));
    }

    #[test]
    fn build_artifacts_dir_joins() {
        let root = PathBuf::from("/var/lib/cynic");
        assert_eq!(CynicPaths::build_artifacts_dir(&root), root.join("artifacts"));
    }

    #[test]
    fn build_evidence_dir_joins() {
        let root = PathBuf::from("/var/lib/cynic");
        assert_eq!(CynicPaths::build_evidence_dir(&root), root.join("evidence"));
    }

    #[test]
    fn build_workspace_dir_joins_profile() {
        let root = PathBuf::from("/var/lib/cynic/build-workspaces");
        assert_eq!(CynicPaths::build_workspace_dir(&root, "debug"), root.join("debug"));
    }

    #[test]
    fn scoped_build_target_dir_shared() {
        let shared = PathBuf::from("/var/lib/cynic/build-targets");
        let ws = PathBuf::from("/var/lib/cynic/checkouts/cynic-0123456789ab");
        assert_eq!(
            CynicPaths::scoped_build_target_dir(Some(&shared), &ws, "debug"),
            shared.join("debug").join("cynic-0123456789ab")
        );
    }

    #[test]
    fn scoped_build_target_dir_none() {
        let ws = PathBuf::from("/var/lib/cynic/checkouts/cynic-0123456789ab");
        assert_eq!(
            CynicPaths::scoped_build_target_dir(None, &ws, "debug"),
            ws.join("target")
        );
    }

    #[test]
    fn scoped_build_target_dir_fallback_hostile() {
        let shared = PathBuf::from("/var/lib/cynic/build-targets");
        let ws = PathBuf::from("/");
        assert_eq!(
            CynicPaths::scoped_build_target_dir(Some(&shared), &ws, "debug"),
            shared.join("debug")
        );
    }
}
