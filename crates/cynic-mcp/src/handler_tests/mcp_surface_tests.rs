#[cfg(test)]
mod mcp_surface_tests {
    use super::*;

    fn test_identity() -> crate::session_identity::SessionIdentity {
        crate::session_identity::SessionIdentity {
            agent_id: "test-agent".to_string(),
            agent_type: "test".to_string(),
            source: crate::session_identity::SessionIdentitySource::GeneratedProcess,
            stable_for: "mcp_process",
            secret: false,
        }
    }

    fn exposure_fixture() -> serde_json::Value {
        serde_json::json!({
            "constitutional_execution": {
                "execution_contract": "cynic.agent_workload_envelope.v1"
            },
            "mcp_tool_domains": {
                "organism_status": {
                    "description": "status",
                    "tools": ["cynic_api_contracts", "cynic_mcp_surface"],
                    "operator_tools": []
                },
                "lifecycle": {
                    "description": "lifecycle",
                    "tools": ["cynic_lifecycle_status", "cynic_lifecycle_dispatch"],
                    "operator_tools": ["cynic_lifecycle_status", "cynic_lifecycle_dispatch"]
                }
            },
            "mcp_servers": [
                {
                    "name": "cynic",
                    "tools": [
                        {"name": "cynic_api_contracts"},
                        {"name": "cynic_mcp_surface"},
                        {"name": "cynic_lifecycle_status"},
                        {"name": "cynic_lifecycle_dispatch"}
                    ]
                }
            ]
        })
    }

    #[test]
    fn mcp_surface_findings_accepts_exposed_tools_classified_by_domain() {
        let exposure = exposure_fixture();

        assert_eq!(
            mcp_surface_findings(&exposure),
            Vec::<serde_json::Value>::new()
        );
    }

    #[test]
    fn mcp_surface_findings_rejects_exposed_tool_without_domain() {
        let mut exposure = exposure_fixture();
        exposure["mcp_servers"][0]["tools"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({"name": "unclassified_tool"}));

        let findings = mcp_surface_findings(&exposure);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("exposed_tool_missing_domain")
                && finding.get("tool").and_then(|value| value.as_str()) == Some("unclassified_tool")
        }));
    }

    #[test]
    fn mcp_surface_findings_rejects_domain_tool_not_exposed() {
        let mut exposure = exposure_fixture();
        exposure["mcp_tool_domains"]["coordination"] = serde_json::json!({
            "tools": ["cynic_coord_claim"],
            "operator_tools": ["cynic_coord_claim"]
        });

        let findings = mcp_surface_findings(&exposure);

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str()) == Some("domain_tool_not_exposed")
                && finding.get("tool").and_then(|value| value.as_str()) == Some("cynic_coord_claim")
        }));
    }

    #[test]
    fn mcp_domain_surface_sorts_domains_for_stable_agent_reads() {
        let exposure = exposure_fixture();
        let domains = mcp_domain_surface(&exposure);
        let names: Vec<&str> = domains
            .iter()
            .filter_map(|domain| domain.get("name").and_then(|value| value.as_str()))
            .collect();

        assert_eq!(names, vec!["lifecycle", "organism_status"]);
    }

    #[test]
    fn disconnected_surface_retains_only_explicit_read_only_projection() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root");
        let report = mcp_surface_manifest(
            &root.display().to_string(),
            &test_identity(),
            false,
            false,
            "unknown",
        )
        .expect("surface manifest");

        assert_eq!(report["runtime_status"], "degraded_disconnected");
        assert_eq!(report["kernel_session"]["active"], false);
        assert_eq!(
            report["agent_shell_fallback"]["status"],
            "available_degraded"
        );
        assert_eq!(
            report["agent_shell_fallback"]["authority"],
            "local_source_projection_only"
        );
        let blocked = report["agent_shell_fallback"]["blocked_without_mcp"]
            .as_array()
            .expect("blocked capability list");
        assert!(blocked.iter().any(|item| item == "fleet_mutation"));
        assert!(blocked.iter().any(|item| item == "build_runner_dispatch"));
    }
}
