//! Public API contract discovery endpoints.

use axum::{
    http::header,
    response::{IntoResponse, Response},
};

const OPENAPI_JSON: &str = include_str!("../../../../../docs/api/openapi.json");
const API_TOPOLOGY_JSON: &str = include_str!("../../../../../axioms/api-topology-v1.json");
const API_LAYER_ALIGNMENT_JSON: &str =
    include_str!("../../../../../axioms/api-layer-alignment-v1.json");

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
