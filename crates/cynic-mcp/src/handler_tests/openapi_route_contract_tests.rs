#[cfg(test)]
mod openapi_route_contract_tests {
    use super::*;

    #[test]
    fn openapi_route_contract_rejects_router_path_missing_from_openapi() {
        let router_text =
            "Router::new().route(\"/health\", get(health)).route(\"/missing\", get(missing));\n";
        let openapi = serde_json::json!({
            "openapi": "3.1.0",
            "paths": {
                "/health": {"get": {}}
            }
        });

        let findings = openapi_route_contract_findings("/tmp", router_text, &openapi);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("openapi_route_missing")
        }));
    }

    #[test]
    fn openapi_route_contract_rejects_stale_openapi_path() {
        let router_text = "Router::new().route(\"/health\", get(health));\n";
        let openapi = serde_json::json!({
            "openapi": "3.1.0",
            "paths": {
                "/health": {"get": {}},
                "/stale": {"get": {}}
            }
        });

        let findings = openapi_route_contract_findings("/tmp", router_text, &openapi);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("openapi_route_stale")
        }));
    }
}

