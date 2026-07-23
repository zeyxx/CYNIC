//! ModelPublisherPort — agnostic interface for pushing discovered model backends
//! to any target system.
//!
//! Pattern: Discover once, publish to N targets.
//! Adding a new interface (LM Studio, Open WebUI, remote registry, etc.)
//! means implementing this trait — no changes to the discovery or orchestration
//! layers.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::ods_discovery::DiscoveredService;

// ── Publication target result ────────────────────────────────────────────────

/// Result of publishing one model to one target.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PublishResult {
    /// Human-readable name of the publisher (e.g. "inference.toml", "open-webui")
    pub publisher: String,
    /// Model identifier that was published
    pub model_id: String,
    /// Whether this was a new registration (false = already existed / updated)
    pub created: bool,
    /// Optional error — None means success
    pub error: Option<String>,
}

// ── Normalised model descriptor ───────────────────────────────────────────────

/// Engine-agnostic representation of a discovered model, ready to be published.
/// Publishers receive this and decide how to represent it internally.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NormalisedModel {
    /// Stable, safe ID: "<engine>/<node_id>/<model_id_slug>"
    pub backend_id: String,
    /// Engine type string — injected at discovery, never hardcoded in publishers.
    pub engine: String,
    /// OpenAI-compatible base URL (the publisher should use this verbatim).
    pub endpoint: String,
    /// Model identifier as returned by the engine (e.g. "qwen2.5-coder:7b")
    pub model_id: String,
    /// Node where this model was discovered
    pub node_id: String,
    /// True = weights live locally, no cloud egress
    pub sovereign: bool,
    /// Unix timestamp ms of discovery
    pub discovered_at: i64,
}

impl NormalisedModel {
    /// Build from a DiscoveredService + model name.
    pub fn from_discovered(service: &DiscoveredService, model_id: &str, node_id: &str) -> Self {
        let slug = model_id.replace([':', '.', '/'], "-");
        let engine = service.name.to_lowercase().replace(' ', "_");
        let backend_id = format!("{engine}/{node_id}/{slug}");
        let discovered_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        Self {
            backend_id,
            engine,
            endpoint: service.endpoint.clone(),
            model_id: model_id.to_string(),
            node_id: node_id.to_string(),
            sovereign: true,
            discovered_at,
        }
    }
}

// ── Port trait ────────────────────────────────────────────────────────────────

/// Agnostic interface for publishing a discovered model to a target system.
///
/// Each impl represents one "interface" — inference.toml, Open WebUI, a remote
/// registry, SurrealDB, etc. The orchestrator calls all registered publishers
/// without knowing their internals.
#[async_trait]
pub trait ModelPublisherPort: Send + Sync {
    /// Human-readable name, used in logs and proof reports.
    fn name(&self) -> &str;

    /// Publish (upsert) a normalised model to this target.
    /// Must be idempotent — calling twice for the same model_id should not
    /// duplicate entries.
    async fn publish(&self, model: &NormalisedModel) -> PublishResult;

    /// Remove a model registration from this target (called on prune/health failure).
    async fn unpublish(&self, model_id: &str) -> Result<(), String> {
        // Default: no-op — not all publishers support removal.
        let _ = model_id;
        Ok(())
    }

    /// Return all model IDs currently registered at this target.
    /// Used to detect stale registrations on prune runs.
    async fn list_registered(&self) -> Result<Vec<String>, String> {
        Ok(vec![])
    }
}
