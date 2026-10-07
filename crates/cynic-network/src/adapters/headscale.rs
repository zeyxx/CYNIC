//! Headscale adapter — implements `FleetMeshPort` for the Headscale API.
//!
//! I/O adapter: HTTP REST calls to the Headscale server.
//! The API key is resolved via `cynic-secrets` (SecretProviderPort).
//! The key value never leaves this module.

use serde::Deserialize;

use cynic_core::fleet_mesh::{FleetMeshPort, MeshError, MeshNode, MeshRoute};

/// Headscale API response for a node.
#[derive(Debug, Deserialize)]
struct HeadscaleNode {
    id: String,
    name: String,
    ip_addresses: Vec<String>,
    online: bool,
    last_seen: Option<String>,
}

/// Headscale API response for a route.
#[derive(Debug, Deserialize)]
struct HeadscaleRoute {
    id: String,
    prefix: String,
    enabled: bool,
    node_id: String,
}

/// Headscale API response wrapper for node list.
#[derive(Debug, Deserialize)]
struct HeadscaleNodeList {
    nodes: Vec<HeadscaleNode>,
}

/// Headscale API response wrapper for route list.
#[derive(Debug, Deserialize)]
struct HeadscaleRouteList {
    routes: Vec<HeadscaleRoute>,
}

/// Configuration for the Headscale client.
#[derive(Debug, Clone)]
pub struct HeadscaleConfig {
    pub base_url: String,
    pub api_key_id: String,
    pub api_key_env_var: String,
}

impl HeadscaleConfig {
    /// Create a new Headscale configuration.
    pub fn new(
        base_url: impl Into<String>,
        api_key_id: impl Into<String>,
        api_key_env_var: impl Into<String>,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            api_key_id: api_key_id.into(),
            api_key_env_var: api_key_env_var.into(),
        }
    }
}

/// Headscale client — implements `FleetMeshPort`.
pub struct HeadscaleClient {
    config: HeadscaleConfig,
    api_key: String,
}

impl HeadscaleClient {
    /// Create a new Headscale client.
    ///
    /// The API key is resolved via `cynic-secrets` using the configured
    /// `api_key_id` and `api_key_env_var`. The key is never logged.
    pub fn new(config: HeadscaleConfig) -> Result<Self, MeshError> {
        let api_key = cynic_secrets::resolve_secret(&config.api_key_id, &config.api_key_env_var)
            .ok_or(MeshError::Unauthorized)?;
        Ok(Self { config, api_key })
    }

    /// Make an authenticated GET request to the Headscale API.
    fn get<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, MeshError> {
        let url = format!("{}/api/v1/{}", self.config.base_url, path);
        let response = reqwest::blocking::Client::new()
            .get(&url)
            .bearer_auth(&self.api_key)
            .send()
            .map_err(|e| MeshError::ApiError(e.to_string()))?;
        response
            .json::<T>()
            .map_err(|e| MeshError::ApiError(e.to_string()))
    }

    /// Make an authenticated POST request to the Headscale API.
    fn post<T: for<'de> Deserialize<'de>>(&self, path: &str, body: &str) -> Result<T, MeshError> {
        let url = format!("{}/api/v1/{}", self.config.base_url, path);
        let response = reqwest::blocking::Client::new()
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .body(body.to_string())
            .send()
            .map_err(|e| MeshError::ApiError(e.to_string()))?;
        response
            .json::<T>()
            .map_err(|e| MeshError::ApiError(e.to_string()))
    }

    /// Make an authenticated DELETE request to the Headscale API.
    fn delete<T: for<'de> Deserialize<'de>>(&self, path: &str) -> Result<T, MeshError> {
        let url = format!("{}/api/v1/{}", self.config.base_url, path);
        let response = reqwest::blocking::Client::new()
            .delete(&url)
            .bearer_auth(&self.api_key)
            .send()
            .map_err(|e| MeshError::ApiError(e.to_string()))?;
        response
            .json::<T>()
            .map_err(|e| MeshError::ApiError(e.to_string()))
    }
}

impl FleetMeshPort for HeadscaleClient {
    fn list_nodes(&self) -> Result<Vec<MeshNode>, MeshError> {
        let response: HeadscaleNodeList = self.get("node/")?;
        Ok(response
            .nodes
            .into_iter()
            .map(|n| MeshNode {
                id: n.id,
                name: n.name,
                ip: n.ip_addresses.first().cloned().unwrap_or_default(),
                online: n.online,
                last_seen: n.last_seen,
            })
            .collect())
    }

    fn enroll_node(&self, node: &MeshNode) -> Result<(), MeshError> {
        let body = serde_json::json!({
            "name": node.name,
            "ip": node.ip,
        });
        let _: serde_json::Value = self.post("node/", &body.to_string())?;
        Ok(())
    }

    fn remove_node(&self, node_id: &str) -> Result<(), MeshError> {
        let _: serde_json::Value = self.delete(&format!("node/{}", node_id))?;
        Ok(())
    }

    fn list_routes(&self) -> Result<Vec<MeshRoute>, MeshError> {
        let response: HeadscaleRouteList = self.get("routes/")?;
        Ok(response
            .routes
            .into_iter()
            .map(|r| MeshRoute {
                id: r.id,
                prefix: r.prefix,
                enabled: r.enabled,
                node_id: r.node_id,
            })
            .collect())
    }

    fn enable_route(&self, route_id: &str) -> Result<(), MeshError> {
        let _: serde_json::Value = self.post(&format!("routes/{}/enable", route_id), "{}")?;
        Ok(())
    }

    fn disable_route(&self, route_id: &str) -> Result<(), MeshError> {
        let _: serde_json::Value = self.post(&format!("routes/{}/disable", route_id), "{}")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headscale_config_is_pure_data() {
        let config = HeadscaleConfig::new(
            "http://10.10.10.40:8080",
            "headscale-api-key",
            "CYNIC_HEADSCALE_API_KEY",
        );
        assert_eq!(config.base_url, "http://10.10.10.40:8080");
        assert_eq!(config.api_key_id, "headscale-api-key");
        assert_eq!(config.api_key_env_var, "CYNIC_HEADSCALE_API_KEY");
    }

    #[test]
    fn headscale_client_requires_api_key() {
        let config = HeadscaleConfig::new(
            "http://10.10.10.40:8080",
            "headscale-api-key",
            "CYNIC_HEADSCALE_API_KEY",
        );
        // Without the env var set, this must fail with Unauthorized
        let result = HeadscaleClient::new(config);
        assert!(matches!(result, Err(MeshError::Unauthorized)));
    }
}
