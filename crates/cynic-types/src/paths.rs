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

    /// Vérifie si un chemin donné pointe vers l'arborescence système cynic.
    pub fn is_under_system_root(path: &Path) -> bool {
        path.starts_with(Self::DEFAULT_SYSTEM_ROOT)
    }
}
