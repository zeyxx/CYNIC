//! OpenWebUiPublisher — registers discovered models into a running Open WebUI
//! instance via its REST API (`/api/models`).
//!
//! Open WebUI exposes an OpenAI-compatible connection manager. This publisher
//! POSTs a new connection entry pointing to the discovered engine endpoint.
//!
//! Docs: https://docs.openwebui.com/api
//! Default port: 3000 (configurable via OPEN_WEBUI_URL env var).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::ports::model_publisher_port::{ModelPublisherPort, NormalisedModel, PublishResult};

/// Base URL for the Open WebUI instance. Defaults to http://localhost:3000.
fn open_webui_base() -> String {
    std::env::var("OPEN_WEBUI_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

#[derive(Debug)]
pub struct OpenWebUiPublisher {
    client: reqwest::Client,
}

impl OpenWebUiPublisher {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }
}

impl Default for OpenWebUiPublisher {
    fn default() -> Self {
        Self::new()
    }
}

// ── Open WebUI API shapes ─────────────────────────────────────────────────────

/// POST /api/v1/models/create — create a model connection in Open WebUI.
/// Open WebUI wraps OpenAI-compat backends as "connections".
#[derive(Serialize)]
struct OwiConnectionRequest {
    id: String,
    name: String,
    base_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    api_key: Option<String>,
}

#[derive(Deserialize)]
struct OwiModelEntry {
    id: String,
}

#[derive(Deserialize)]
struct OwiModelList {
    data: Vec<OwiModelEntry>,
}

#[async_trait]
impl ModelPublisherPort for OpenWebUiPublisher {
    fn name(&self) -> &str {
        "open-webui"
    }

    async fn publish(&self, model: &NormalisedModel) -> PublishResult {
        let base = open_webui_base();
        // Open WebUI uses /api/v1/connections to register OpenAI-compat backends.
        let url = format!("{base}/api/v1/connections");

        let connection_id = model.backend_id.replace('/', "_");
        let body = OwiConnectionRequest {
            id: connection_id.clone(),
            name: format!("{} / {}", model.engine, model.model_id),
            base_url: model.endpoint.clone(),
            api_key: None, // sovereign = no key
        };

        match self.client.post(&url).json(&body).send().await {
            Ok(resp) if resp.status().is_success() || resp.status().as_u16() == 409 => {
                // 409 Conflict = already registered — treat as success
                PublishResult {
                    publisher: self.name().to_string(),
                    model_id: model.model_id.clone(),
                    created: resp.status().is_success(),
                    error: None,
                }
            }
            Ok(resp) => PublishResult {
                publisher: self.name().to_string(),
                model_id: model.model_id.clone(),
                created: false,
                error: Some(format!("Open WebUI returned HTTP {}", resp.status())),
            },
            Err(e) => PublishResult {
                publisher: self.name().to_string(),
                model_id: model.model_id.clone(),
                created: false,
                error: Some(format!("Open WebUI unreachable: {e}")),
            },
        }
    }

    async fn list_registered(&self) -> Result<Vec<String>, String> {
        let base = open_webui_base();
        let url = format!("{base}/api/v1/models");
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Open WebUI unreachable: {e}"))?;
        let list: OwiModelList = resp
            .json()
            .await
            .map_err(|e| format!("Failed to parse Open WebUI model list: {e}"))?;
        Ok(list.data.into_iter().map(|m| m.id).collect())
    }
}
