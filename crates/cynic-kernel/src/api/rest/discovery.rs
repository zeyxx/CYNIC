//! Public API discovery endpoints for the minimal runtime baseline.

use axum::{
    http::header,
    response::{IntoResponse, Response},
};

const OPENAPI_JSON: &str = r#"{
  "openapi": "3.1.0",
  "info": {
    "title": "CYNIC Kernel API",
    "version": "minimal-runtime"
  },
  "paths": {
    "/health": {},
    "/ready": {},
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
    "/health",
    "/ready",
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

pub async fn openapi_handler() -> Response {
    json_contract(OPENAPI_JSON)
}

pub async fn api_topology_handler() -> Response {
    json_contract(API_TOPOLOGY_JSON)
}

pub async fn api_layer_alignment_handler() -> Response {
    json_contract(API_LAYER_ALIGNMENT_JSON)
}
