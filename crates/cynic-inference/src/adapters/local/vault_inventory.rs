//! Filesystem inventory of operator-declared local model profiles.
//!
//! Paths resolve through [`crate::domain::model_layout::ModelLayout`].

use std::fs;
use std::path::{Path, PathBuf};

use crate::domain::local_runtime::{LocalModelInventory, LocalModelProfile, LocalModelSource};
use crate::domain::model_layout::{DEFAULT_UNIT_TEMPLATE, ModelLayout};
use crate::ports::local_runtime_port::{LocalModelInventoryPort, LocalRuntimeError};

#[derive(Debug)]
pub struct FsLocalModelInventory {
    layout: ModelLayout,
}

impl FsLocalModelInventory {
    pub fn from_env() -> Self {
        Self {
            layout: ModelLayout::from_env(),
        }
    }

    pub fn from_layout(layout: &ModelLayout) -> Self {
        Self {
            layout: layout.clone(),
        }
    }

    pub fn with_dirs(vault_dir: PathBuf, profiles_dir: PathBuf) -> Self {
        let mut layout = ModelLayout::from_env();
        layout.vault_dir = vault_dir;
        layout.profiles_dir = profiles_dir;
        layout.unit_template = DEFAULT_UNIT_TEMPLATE.to_string();
        Self { layout }
    }

    fn parse_profile_file(&self, path: &Path) -> Result<LocalModelProfile, LocalRuntimeError> {
        let alias = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| {
                LocalRuntimeError::Inventory(format!("bad profile path: {}", path.display()))
            })?
            .to_string();

        let raw = fs::read_to_string(path)
            .map_err(|e| LocalRuntimeError::Inventory(format!("read {}: {e}", path.display())))?;

        let mut map = std::collections::BTreeMap::<String, String>::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }

        let model_path = map.get("LLAMA_MODEL_PATH").cloned().ok_or_else(|| {
            LocalRuntimeError::Inventory(format!("profile {alias} missing LLAMA_MODEL_PATH"))
        })?;

        let model_pb = PathBuf::from(&model_path);
        let model_exists = model_pb.is_file();
        let size_bytes = if model_exists {
            fs::metadata(&model_pb).ok().map(|m| m.len())
        } else {
            None
        };

        let source = if model_path.starts_with(self.layout.vault_dir.to_string_lossy().as_ref()) {
            LocalModelSource::Vault
        } else {
            LocalModelSource::ExternalPath
        };

        let host = map
            .get("LLAMA_HOST")
            .cloned()
            .unwrap_or_else(|| "127.0.0.1".to_string());
        let port = map
            .get("LLAMA_PORT")
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);
        let ctx_size = map
            .get("LLAMA_CTX_SIZE")
            .and_then(|p| p.parse().ok())
            .unwrap_or(4096);
        let extra_args = map.get("LLAMA_EXTRA_ARGS").cloned().unwrap_or_default();
        let loopback_only = host == "127.0.0.1" || host == "localhost" || host == "::1";

        Ok(LocalModelProfile {
            alias,
            model_path,
            source,
            model_exists,
            size_bytes,
            host,
            port,
            ctx_size,
            extra_args,
            memory_max: map.get("LLAMA_MEMORY_MAX").cloned(),
            cpu_quota: map.get("LLAMA_CPU_QUOTA").cloned(),
            loopback_only,
        })
    }
}

impl LocalModelInventoryPort for FsLocalModelInventory {
    fn list_profiles(&self) -> Result<LocalModelInventory, LocalRuntimeError> {
        let mut models = Vec::new();
        if self.layout.profiles_dir.is_dir() {
            let mut entries: Vec<PathBuf> = fs::read_dir(&self.layout.profiles_dir)
                .map_err(|e| {
                    LocalRuntimeError::Inventory(format!(
                        "read profiles dir {}: {e}",
                        self.layout.profiles_dir.display()
                    ))
                })?
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("env"))
                .collect();
            entries.sort();
            for path in entries {
                match self.parse_profile_file(&path) {
                    Ok(profile) => models.push(profile),
                    Err(e) => {
                        tracing::warn!("skip unreadable model profile {}: {e}", path.display());
                    }
                }
            }
        }

        Ok(LocalModelInventory {
            schema: LocalModelInventory::SCHEMA,
            surface: LocalModelInventory::SURFACE,
            vault_dir: self.layout.vault_dir.display().to_string(),
            profiles_dir: self.layout.profiles_dir.display().to_string(),
            unit_template: self.layout.unit_template.clone(),
            models,
        })
    }

    fn get_profile(&self, alias: &str) -> Result<LocalModelProfile, LocalRuntimeError> {
        let path = self.layout.profiles_dir.join(format!("{alias}.env"));
        if !path.is_file() {
            return Err(LocalRuntimeError::UnknownProfile(alias.to_string()));
        }
        self.parse_profile_file(&path)
    }
}

// Re-export resolvers for callers that imported them from this module.
pub use crate::domain::model_layout::{resolve_profiles_dir, resolve_vault_dir};

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parses_profile_env_and_marks_vault_source() {
        let root = std::env::temp_dir().join(format!("cynic_vault_test_{}", std::process::id()));
        let vault = root.join("vault");
        let profiles = root.join("profiles");
        fs::create_dir_all(&vault).unwrap();
        fs::create_dir_all(&profiles).unwrap();
        let weight = vault.join("tiny.gguf");
        fs::write(&weight, b"gguf").unwrap();
        let mut f = fs::File::create(profiles.join("tiny.env")).unwrap();
        writeln!(f, "LLAMA_MODEL_PATH={}", weight.display()).unwrap();
        writeln!(f, "LLAMA_HOST=127.0.0.1").unwrap();
        writeln!(f, "LLAMA_PORT=8080").unwrap();
        writeln!(f, "LLAMA_CTX_SIZE=2048").unwrap();
        writeln!(f, "LLAMA_EXTRA_ARGS=--parallel 1").unwrap();
        drop(f);

        let inv = FsLocalModelInventory::with_dirs(vault.clone(), profiles);
        let list = inv.list_profiles().unwrap();
        assert_eq!(list.surface, "native");
        assert_eq!(list.models.len(), 1);
        let m = &list.models[0];
        assert_eq!(m.alias, "tiny");
        assert!(m.model_exists);
        assert_eq!(m.source, LocalModelSource::Vault);
        assert!(m.loopback_only);
        assert_eq!(m.ctx_size, 2048);

        let _ = fs::remove_dir_all(root);
    }
}
