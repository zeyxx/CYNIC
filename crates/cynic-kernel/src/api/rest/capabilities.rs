//! Proactive capability declaration endpoint.
//!
//! `GET /capabilities` — machine-readable declaration of what this CYNIC node
//! can do RIGHT NOW. Designed for LLM agents and tool-bearing clients.
//!
//! Unlike `/health` (passive proprioception), `/capabilities` answers:
//!   "What actions can I perform against this runtime?"
//!
//! Schema: cynic.kernel.capabilities.v1

use axum::{extract::State, http::StatusCode, response::Json};
use serde::Serialize;
use std::sync::Arc;

use super::types::AppState;

const CAPABILITIES_SCHEMA: &str = "cynic.kernel.capabilities.v1";

/// Top-level capabilities response.
#[derive(Debug, Serialize)]
pub struct CapabilitiesResponse {
    schema: &'static str,
    service: &'static str,
    node_id: String,
    pid: u32,
    version: &'static str,
    git_sha: &'static str,
    profile: &'static str,
    role: String,
    routes: Vec<RouteCapability>,
    inference: InferenceCapability,
    coordination: CoordinationCapability,
    discovery: DiscoveryCapability,
    #[serde(skip_serializing_if = "Option::is_none")]
    identity: Option<IdentityCapability>,
}

#[derive(Debug, Serialize)]
pub struct RouteCapability {
    path: String,
    method: &'static str,
    auth_required: bool,
    description: &'static str,
}

#[derive(Debug, Serialize)]
pub struct InferenceCapability {
    available: bool,
    backends: u32,
    sovereign_backends: u32,
    models_endpoint: &'static str,
}

#[derive(Debug, Serialize)]
pub struct CoordinationCapability {
    available: bool,
    active_sessions: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct DiscoveryCapability {
    fleet_endpoint: &'static str,
    topology_endpoint: &'static str,
    layer_alignment_endpoint: &'static str,
}

#[derive(Debug, Serialize)]
pub struct IdentityCapability {
    bearer_auth: bool,
    node_enrollment: bool,
}

/// Build the capabilities response from runtime state.
fn build_capabilities(state: &AppState) -> CapabilitiesResponse {
    let inference_backends = state.backend_configs.len() as u32;
    let sovereign_backends = state.backend_configs.iter().filter(|c| c.sovereign).count() as u32;

    let bearer_auth = state.role_keys.has_any_authentication_authority();

    CapabilitiesResponse {
        schema: CAPABILITIES_SCHEMA,
        service: "cynic-kernel",
        node_id: super::discovery::observed_node_id(),
        pid: std::process::id(),
        version: env!("CARGO_PKG_VERSION"),
        git_sha: env!("CYNIC_GIT_SHA"),
        profile: env!("CYNIC_BUILD_PROFILE"),
        role: "kernel".to_string(),
        routes: build_routes(bearer_auth),
        inference: InferenceCapability {
            available: inference_backends > 0,
            backends: inference_backends + 1, // +1 for deterministic-dog
            sovereign_backends,
            models_endpoint: "/models",
        },
        coordination: CoordinationCapability {
            available: true,
            active_sessions: None, // populated by coord port if available
        },
        discovery: DiscoveryCapability {
            fleet_endpoint: "/fleet",
            topology_endpoint: "/api/topology",
            layer_alignment_endpoint: "/api/layer-alignment",
        },
        identity: Some(IdentityCapability {
            bearer_auth,
            node_enrollment: true,
        }),
    }
}

/// Static route registry — mirrors the actual router in mod.rs.
fn build_routes(bearer_auth: bool) -> Vec<RouteCapability> {
    vec![
        RouteCapability {
            path: "/live".into(),
            method: "GET",
            auth_required: false,
            description: "Liveness probe (200 if alive)",
        },
        RouteCapability {
            path: "/ready".into(),
            method: "GET",
            auth_required: false,
            description: "Readiness probe (200 if can serve)",
        },
        RouteCapability {
            path: "/health".into(),
            method: "GET",
            auth_required: bearer_auth,
            description: "Full system topology and diagnostics",
        },
        RouteCapability {
            path: "/capabilities".into(),
            method: "GET",
            auth_required: false,
            description: "This endpoint — machine-readable capability declaration",
        },
        RouteCapability {
            path: "/identity".into(),
            method: "GET",
            auth_required: false,
            description: "Unauthenticated process identity",
        },
        RouteCapability {
            path: "/openapi.json".into(),
            method: "GET",
            auth_required: false,
            description: "OpenAPI 3.1.0 specification",
        },
        RouteCapability {
            path: "/api/topology".into(),
            method: "GET",
            auth_required: false,
            description: "Static route topology",
        },
        RouteCapability {
            path: "/api/layer-alignment".into(),
            method: "GET",
            auth_required: false,
            description: "Layer alignment contract",
        },
        RouteCapability {
            path: "/models".into(),
            method: "GET",
            auth_required: bearer_auth,
            description: "List available inference models (dogs)",
        },
        RouteCapability {
            path: "/inference/route".into(),
            method: "POST",
            auth_required: bearer_auth,
            description: "Route a stimulus to the best available inference backend",
        },
        RouteCapability {
            path: "/coord/claim".into(),
            method: "POST",
            auth_required: bearer_auth,
            description: "Claim a coordination scope (agent session)",
        },
        RouteCapability {
            path: "/coord/release".into(),
            method: "POST",
            auth_required: bearer_auth,
            description: "Release a coordination scope",
        },
        RouteCapability {
            path: "/coord/who".into(),
            method: "GET",
            auth_required: bearer_auth,
            description: "List active coordination sessions",
        },
        RouteCapability {
            path: "/observe".into(),
            method: "POST",
            auth_required: bearer_auth,
            description: "Submit an observation to the organism",
        },
        RouteCapability {
            path: "/agent-tasks".into(),
            method: "GET",
            auth_required: bearer_auth,
            description: "List agent tasks",
        },
        RouteCapability {
            path: "/node-enrollment".into(),
            method: "POST",
            auth_required: bearer_auth,
            description: "Enroll a new node into the fleet",
        },
    ]
}

/// GET /capabilities — proactive capability declaration.
pub async fn capabilities_handler(
    State(state): State<Arc<AppState>>,
) -> Json<CapabilitiesResponse> {
    Json(build_capabilities(&state))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_schema_versioned() {
        let routes = build_routes(false);
        assert!(!routes.is_empty());
        // /capabilities must be in the route list
        assert!(
            routes.iter().any(|r| r.path == "/capabilities"),
            "/capabilities missing from route registry"
        );
    }

    #[test]
    fn capabilities_routes_have_methods() {
        let routes = build_routes(true);
        for route in &routes {
            assert!(!route.method.is_empty());
            assert!(!route.path.is_empty());
        }
    }

    #[test]
    fn capabilities_auth_flag_respected() {
        let with_auth = build_routes(true);
        let without_auth = build_routes(false);
        // /health should reflect auth state
        let health_with = with_auth.iter().find(|r| r.path == "/health").unwrap();
        let health_without = without_auth.iter().find(|r| r.path == "/health").unwrap();
        assert!(health_with.auth_required);
        assert!(!health_without.auth_required);
    }
}
