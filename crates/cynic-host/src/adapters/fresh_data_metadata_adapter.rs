//! Fresh-data metadata adapter — loads model metadata from a JSON file.
//!
//! Belongs to the Host Substrate layer (outbound adapter). Never imported by
//! the pure core domain.

use std::fs;
use std::path::Path;

/// Metadata for a single model.
#[derive(Debug, Clone)]
pub struct ModelMetadata {
    pub model_id: String,
    pub provider: String,
    pub benchmarks: Vec<String>,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
    pub role: String,
    pub data_policy: String,
    pub pricing: String,
    pub cost_on_plan: String,
}

/// Adapter for loading model metadata from a JSON file.
pub struct MetadataAdapter {
    pub path: String,
}

impl Default for MetadataAdapter {
    fn default() -> Self {
        Self {
            path: "/home/user/Bureau/SOLANA/asdf-forge/zeyxx/CYNIC/infra/model-metadata.json".to_string(),
        }
    }
}

impl MetadataAdapter {
    /// Load model metadata from the configured JSON file.
    pub fn load(&self) -> Result<Vec<ModelMetadata>, String> {
        let content = fs::read_to_string(&self.path)
            .map_err(|e| format!("failed to read metadata file: {e}"))?;

        let json: serde_json::Value = serde_json::from_str(&content)
            .map_err(|e| format!("failed to parse metadata JSON: {e}"))?;

        let models = json
            .get("models")
            .and_then(|v| v.as_object())
            .ok_or_else(|| "missing 'models' object in metadata".to_string())?;

        let mut result = Vec::new();
        for (model_id, meta) in models {
            let provider = meta
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let benchmarks = meta
                .get("benchmarks")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let strengths = meta
                .get("strengths")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let weaknesses = meta
                .get("weaknesses")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                .unwrap_or_default();

            let role = meta
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let data_policy = meta
                .get("data_policy")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let pricing = meta
                .get("pricing")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            let cost_on_plan = meta
                .get("cost_on_plan")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string();

            result.push(ModelMetadata {
                model_id: model_id.clone(),
                provider,
                benchmarks,
                strengths,
                weaknesses,
                role,
                data_policy,
                pricing,
                cost_on_plan,
            });
        }

        Ok(result)
    }

    /// Check if the metadata file exists.
    pub fn exists(&self) -> bool {
        Path::new(&self.path).exists()
    }
}