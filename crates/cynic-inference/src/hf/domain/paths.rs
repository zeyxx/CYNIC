use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Local filesystem locations used by the Hugging Face CLI adapter.
///
/// The adapter does not know about `cynic-inference::ModelLayout`; callers
/// translate their own layout into this explicit value so the provider crate
/// stays dependency-free.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HfLocalPaths {
    /// Destination directory for downloads that should enter the CYNIC vault.
    pub vault_dir: PathBuf,
    /// Observed Hugging Face cache directory.
    pub hf_cache_dir: PathBuf,
    /// Hugging Face CLI binary name or absolute path.
    pub hf_bin: PathBuf,
}

impl HfLocalPaths {
    /// Vault destination for a repository: `vault/hf/<namespace>__<name>/`.
    pub fn vault_download_dir(&self, repo_id: &str) -> PathBuf {
        let slug = repo_id.replace('/', "__");
        self.vault_dir.join("hf").join(slug)
    }

    pub fn from_components(
        vault_dir: impl Into<PathBuf>,
        hf_cache_dir: impl Into<PathBuf>,
        hf_bin: impl Into<PathBuf>,
    ) -> Self {
        Self {
            vault_dir: vault_dir.into(),
            hf_cache_dir: hf_cache_dir.into(),
            hf_bin: hf_bin.into(),
        }
    }
}
