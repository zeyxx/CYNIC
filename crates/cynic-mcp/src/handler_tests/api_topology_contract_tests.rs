#[cfg(test)]
mod api_topology_contract_tests {
    use super::*;

    #[test]
    fn api_geometry_report_detects_root_split_and_dangerous_operation() {
        let openapi = serde_json::json!({
            "paths": {
                "/legacy/a": {"get": {}},
                "/legacy/b": {"get": {}},
                "/lifecycle/deploy": {"post": {}}
            }
        });
        let topology = serde_json::json!({
            "topology_budget": {"max_root_splits": 0, "max_transitional_operations": 0},
            "operations": {
                "GET /legacy/a": {
                    "layer": "compatibility",
                    "mutation_level": "read",
                    "agent_safe_default": true,
                    "canonical_family": "/canonical-a",
                    "status": "transitional"
                },
                "GET /legacy/b": {
                    "layer": "compatibility",
                    "mutation_level": "read",
                    "agent_safe_default": true,
                    "canonical_family": "/canonical-b",
                    "status": "canonical"
                },
                "POST /lifecycle/deploy": {
                    "layer": "metabolism",
                    "mutation_level": "deployment",
                    "agent_safe_default": false,
                    "canonical_family": "/lifecycle",
                    "status": "canonical"
                }
            }
        });

        let roots: std::collections::BTreeSet<String> = topology
            .get("operations")
            .and_then(|value| value.as_object())
            .unwrap()
            .keys()
            .map(|operation| api_operation_root(operation))
            .collect();

        assert!(roots.contains("/legacy"));
        assert!(openapi_operations_json(&openapi).contains("POST /lifecycle/deploy"));
    }

    #[test]
    fn api_topology_contract_rejects_openapi_operation_without_topology_entry() {
        let openapi = serde_json::json!({
            "openapi": "3.1.0",
            "x-cynic-api-topology-contract": {
                "schema": "cynic.api_topology.v1",
                "path": "axioms/api-topology-v1.json"
            },
            "x-cynic-api-layer-alignment-contract": {
                "schema": "cynic.api_layer_alignment.v1",
                "path": "axioms/api-layer-alignment-v1.json"
            },
            "paths": {
                "/openapi.json": {"get": {"security": []}},
                "/api/topology": {"get": {"security": []}},
                "/api/layer-alignment": {"get": {"security": []}},
                "/unmapped": {"get": {"security": []}}
            }
        });
        let topology = serde_json::json!({
            "schema": "cynic.api_topology.v1",
            "status": "normative",
            "source_of_truth": {"layer_alignment": "axioms/api-layer-alignment-v1.json"},
            "allowed": {
                "layers": ["perception"],
                "mutation_levels": ["read"],
                "statuses": ["canonical"]
            },
            "topology_budget": {
                "max_root_splits": 0,
                "max_transitional_operations": 0,
                "max_legacy_operations": 0,
                "compat_operations": {"allowed": false}
            },
            "public_discovery_endpoints": {
                "GET /openapi.json": {"serves": "docs/api/openapi.json"},
                "GET /api/topology": {"serves": "axioms/api-topology-v1.json"},
                "GET /api/layer-alignment": {"serves": "axioms/api-layer-alignment-v1.json"}
            },
            "operations": {
                "GET /openapi.json": {
                    "layer": "perception",
                    "mutation_level": "read",
                    "agent_safe_default": true,
                    "canonical_family": "/api",
                    "status": "canonical"
                },
                "GET /api/topology": {
                    "layer": "perception",
                    "mutation_level": "read",
                    "agent_safe_default": true,
                    "canonical_family": "/api",
                    "status": "canonical"
                },
                "GET /api/layer-alignment": {
                    "layer": "perception",
                    "mutation_level": "read",
                    "agent_safe_default": true,
                    "canonical_family": "/api",
                    "status": "canonical"
                }
            }
        });
        let alignment = serde_json::json!({
            "schema": "cynic.api_layer_alignment.v1",
            "layers": {"perception": {}}
        });
        let discovery = concat!(
            "const OPENAPI_JSON: &str = include_str!(\"../../../../../docs/api/openapi.json\");\n",
            "const API_TOPOLOGY_JSON: &str = include_str!(\"../../../../../axioms/api-topology-v1.json\");\n",
            "const API_LAYER_ALIGNMENT_JSON: &str = include_str!(\"../../../../../axioms/api-layer-alignment-v1.json\");\n",
            "pub async fn openapi_handler() -> Response { json_contract(OPENAPI_JSON) }\n",
            "pub async fn api_topology_handler() -> Response { json_contract(API_TOPOLOGY_JSON) }\n",
            "pub async fn api_layer_alignment_handler() -> Response { json_contract(API_LAYER_ALIGNMENT_JSON) }\n"
        );
        let middleware = "path == \"/openapi.json\" || path == \"/api/topology\" || path == \"/api/layer-alignment\"\n";

        let findings = api_topology_contract_findings(
            "/tmp", &openapi, &topology, &alignment, discovery, middleware,
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("api_topology_operation_missing")
        }));
    }
}

