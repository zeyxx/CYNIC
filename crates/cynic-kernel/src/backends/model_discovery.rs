//! Model discovery — organic awareness of available models on the filesystem.
//!
//! Kernel scans accessible directories for .gguf files at boot, learns what's available,
//! and maps them to backend configurations. If a configured model is missing, backends
//! degrade gracefully rather than failing catastrophically.

use std::collections::HashMap;
use std::path::PathBuf;
use tokio::fs;

/// Discovered model metadata
#[derive(Debug, Clone)]
pub struct DiscoveredModel {
    pub filename: String,
    pub path: PathBuf,
}

/// Model registry — maps model names to their filesystem paths
#[derive(Debug)]
pub struct ModelRegistry {
    pub models: HashMap<String, DiscoveredModel>,
}

impl Default for ModelRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            models: HashMap::new(),
        }
    }

    /// Scan filesystem for available .gguf models
    pub async fn scan(&mut self, search_paths: &[&str]) -> Result<usize, String> {
        let mut count = 0;

        for search_path in search_paths {
            if let Ok(entries) = fs::read_dir(search_path).await {
                let mut dir_entries = entries;
                while let Ok(Some(entry)) = dir_entries.next_entry().await {
                    let path = entry.path();
                    if let Some(filename) = path.file_name()
                        && let Some(name_str) = filename.to_str()
                        && name_str.ends_with(".gguf")
                    {
                        let model_key = name_str.to_string();
                        self.models.insert(
                            model_key.clone(),
                            DiscoveredModel {
                                filename: name_str.to_string(),
                                path: path.clone(),
                            },
                        );
                        tracing::info!("discovered model: {}", path.display());
                        count += 1;
                    }
                }
            }
        }

        Ok(count)
    }

    /// Check if a model is available (exact filename match)
    pub fn has_model(&self, model_name: &str) -> bool {
        self.models.contains_key(model_name)
    }

    /// Get model path if it exists
    pub fn get_model_path(&self, model_name: &str) -> Option<PathBuf> {
        self.models.get(model_name).map(|m| m.path.clone())
    }

    /// Find model by partial name (case-insensitive substring)
    pub fn find_model_by_substring(&self, substring: &str) -> Option<DiscoveredModel> {
        self.models
            .values()
            .find(|m| {
                m.filename
                    .to_lowercase()
                    .contains(&substring.to_lowercase())
            })
            .cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_registry() {
        let registry = ModelRegistry::new();
        assert_eq!(registry.models.len(), 0);
    }
}
