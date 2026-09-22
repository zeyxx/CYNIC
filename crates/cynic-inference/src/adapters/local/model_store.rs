//! Filesystem scan for the centralized model store.

use std::fs;
use std::path::Path;

use crate::adapters::local::vault_inventory::FsLocalModelInventory;
use crate::infra::model_layout::ModelLayout;
use crate::domain::model_store::{
    ModelArtifact, ModelLocationKind, ModelStoreReport, ModelStoreTotals,
};
use crate::ports::local_runtime_port::{LocalModelInventoryPort, LocalRuntimeError};
use crate::ports::model_store_port::ModelStorePort;

#[derive(Debug)]
pub struct FsModelStore {
    layout: ModelLayout,
    inventory: FsLocalModelInventory,
}

impl FsModelStore {
    pub fn from_env() -> Self {
        let layout = ModelLayout::from_env();
        let inventory = FsLocalModelInventory::from_layout(&layout);
        Self { layout, inventory }
    }

    pub fn with_layout(layout: ModelLayout) -> Self {
        let inventory = FsLocalModelInventory::from_layout(&layout);
        Self { layout, inventory }
    }

    fn classify(&self, path: &Path) -> ModelLocationKind {
        let p = path.to_string_lossy();
        if p.starts_with(self.layout.vault_dir.to_string_lossy().as_ref()) {
            ModelLocationKind::Vault
        } else if p.starts_with(self.layout.hf_cache_dir.to_string_lossy().as_ref()) {
            ModelLocationKind::HfCache
        } else if p.starts_with(self.layout.llama_cache_dir.to_string_lossy().as_ref()) {
            ModelLocationKind::LlamaCache
        } else {
            ModelLocationKind::External
        }
    }

    fn scan_gguf_dir(&self, dir: &Path, out: &mut Vec<ModelArtifact>) {
        if !dir.is_dir() {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // one level into vault/hf/<repo>
                self.scan_gguf_dir(&path, out);
                continue;
            }
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if ext != "gguf" && ext != "bin" && ext != "safetensors" {
                continue;
            }
            let meta = fs::metadata(&path).ok();
            let id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("unknown")
                .to_string();
            out.push(ModelArtifact {
                id,
                path: path.display().to_string(),
                location: self.classify(&path),
                kind: ext,
                size_bytes: meta.as_ref().map(|m| m.len()),
                exists: true,
                profile_alias: None,
                hf_repo: None,
                hf_revision: None,
            });
        }
    }
}

impl ModelStorePort for FsModelStore {
    fn scan(&self) -> Result<ModelStoreReport, LocalRuntimeError> {
        let inv = self.inventory.list_profiles()?;
        let mut artifacts: Vec<ModelArtifact> = Vec::new();

        self.scan_gguf_dir(&self.layout.vault_dir, &mut artifacts);
        // Optional observation of llama cache (not vault, but centralized view)
        self.scan_gguf_dir(&self.layout.llama_cache_dir, &mut artifacts);

        // Link profile aliases
        for profile in &inv.models {
            if let Some(art) = artifacts.iter_mut().find(|a| {
                a.path == profile.model_path || Path::new(&a.path) == Path::new(&profile.model_path)
            }) {
                art.profile_alias = Some(profile.alias.clone());
            } else if profile.model_exists {
                artifacts.push(ModelArtifact {
                    id: profile.alias.clone(),
                    path: profile.model_path.clone(),
                    location: self.classify(Path::new(&profile.model_path)),
                    kind: "gguf".into(),
                    size_bytes: profile.size_bytes,
                    exists: true,
                    profile_alias: Some(profile.alias.clone()),
                    hf_repo: None,
                    hf_revision: None,
                });
            } else {
                artifacts.push(ModelArtifact {
                    id: profile.alias.clone(),
                    path: profile.model_path.clone(),
                    location: self.classify(Path::new(&profile.model_path)),
                    kind: "gguf".into(),
                    size_bytes: None,
                    exists: false,
                    profile_alias: Some(profile.alias.clone()),
                    hf_repo: None,
                    hf_revision: None,
                });
            }
        }

        artifacts.sort_by(|a, b| a.path.cmp(&b.path));
        artifacts.dedup_by(|a, b| a.path == b.path);

        let vault_bytes = artifacts
            .iter()
            .filter(|a| a.location == ModelLocationKind::Vault)
            .filter_map(|a| a.size_bytes)
            .sum();
        let missing_files = artifacts.iter().filter(|a| !a.exists).count();

        Ok(ModelStoreReport {
            schema: ModelStoreReport::SCHEMA,
            surface: ModelStoreReport::SURFACE,
            layout: self.layout.report(),
            totals: ModelStoreTotals {
                artifacts: artifacts.len(),
                profiles: inv.models.len(),
                vault_bytes,
                missing_files,
            },
            artifacts,
            profiles: inv.models,
        })
    }

    fn persist_registry(&self, report: &ModelStoreReport) -> Result<(), LocalRuntimeError> {
        if let Some(parent) = self.layout.registry_path.parent() {
            fs::create_dir_all(parent).map_err(|e| {
                LocalRuntimeError::Inventory(format!(
                    "create registry dir {}: {e}",
                    parent.display()
                ))
            })?;
        }
        let json = serde_json::to_string_pretty(report)
            .map_err(|e| LocalRuntimeError::Inventory(format!("serialize model registry: {e}")))?;
        fs::write(&self.layout.registry_path, json).map_err(|e| {
            LocalRuntimeError::Inventory(format!(
                "write registry {}: {e}",
                self.layout.registry_path.display()
            ))
        })?;
        // Restrict permissions best-effort
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(
                &self.layout.registry_path,
                fs::Permissions::from_mode(0o600),
            );
        }
        Ok(())
    }
}
