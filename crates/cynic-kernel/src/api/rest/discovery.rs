//! Public API discovery endpoints for the minimal runtime baseline.

use axum::{
    Json,
    http::header,
    response::{IntoResponse, Response},
};
use serde::Serialize;

const OPENAPI_JSON: &str = r#"{
  "openapi": "3.1.0",
  "info": {
    "title": "CYNIC Kernel API",
    "version": "minimal-runtime"
  },
  "paths": {
      "/identity": {},
      "/health": {},
      "/ready": {},
      "/capabilities": {},
      "/coord/register": {},
    "/coord/who": {},
    "/coord/claim": {},
    "/coord/release": {},
    "/observe": {},
    "/agent-tasks": {}
  }
}"#;

const API_TOPOLOGY_JSON: &str = r#"{
  "schema": "cynic.api_topology.v1.minimal",
  "status": "runtime_embedded",
  "authority": "crates/cynic-kernel",
  "routes": [
    "/identity",
    "/health",
    "/ready",
    "/capabilities",
    "/coord/register",
    "/coord/who",
    "/coord/claim",
    "/coord/release",
    "/observe",
    "/agent-tasks"
  ]
}"#;

const API_LAYER_ALIGNMENT_JSON: &str = r#"{
  "schema": "cynic.api_layer_alignment.v1.minimal",
  "status": "runtime_embedded",
  "authority": "infra/runtime-authority.json",
  "layers": {
    "kernel": "crates/cynic-kernel",
    "mcp": "crates/cynic-mcp",
    "node": "crates/cynic-node",
    "host": "crates/cynic-host",
    "cli": "crates/cynic-cli"
  }
}"#;

fn json_contract(body: &'static str) -> Response {
    (
        [(header::CONTENT_TYPE, "application/json; charset=utf-8")],
        body,
    )
        .into_response()
}

#[derive(Debug, Serialize)]
pub struct KernelBuildIdentity {
    version: &'static str,
    git_sha: &'static str,
    profile: &'static str,
}

#[derive(Debug, Serialize)]
pub struct KernelIdentity {
    schema: &'static str,
    service: &'static str,
    node_id: String,
    hostname: String,
    pid: u32,
    build: KernelBuildIdentity,
}

fn observed_hostname() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|hostname| !hostname.trim().is_empty())
        .or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .ok()
                .map(|hostname| hostname.trim().to_string())
                .filter(|hostname| !hostname.is_empty())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

pub(crate) fn observed_node_id() -> String {
    std::env::var("CYNIC_NODE_ID")
        .ok()
        .filter(|node_id| !node_id.trim().is_empty())
        .unwrap_or_else(observed_hostname)
}

/// Minimal unauthenticated process identity for bounded service discovery.
pub async fn identity_handler() -> Json<KernelIdentity> {
    let hostname = observed_hostname();
    let node_id = observed_node_id();
    Json(KernelIdentity {
        schema: "cynic.kernel.identity.v1",
        service: "cynic-kernel",
        node_id,
        hostname,
        pid: std::process::id(),
        build: KernelBuildIdentity {
            version: env!("CYNIC_VERSION"),
            git_sha: env!("CYNIC_GIT_SHA"),
            profile: env!("CYNIC_BUILD_PROFILE"),
        },
    })
}

pub async fn openapi_handler() -> Response {
    json_contract(OPENAPI_JSON)
}

pub async fn api_topology_handler() -> Response {
    json_contract(API_TOPOLOGY_JSON)
}

pub async fn api_layer_alignment_handler() -> Response {
    json_contract(API_LAYER_ALIGNMENT_JSON)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn identity_is_versioned_and_process_bound() {
        let Json(identity) = identity_handler().await;
        assert_eq!(identity.schema, "cynic.kernel.identity.v1");
        assert_eq!(identity.service, "cynic-kernel");
        assert!(!identity.node_id.is_empty());
        assert!(!identity.hostname.is_empty());
        assert_eq!(identity.pid, std::process::id());
        assert!(!identity.build.version.is_empty());
        assert!(!identity.build.git_sha.is_empty());
    }
}
