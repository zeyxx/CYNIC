#[cfg(test)]
mod command_surfaces_contract_tests {
    use super::*;

    fn base_contract() -> serde_json::Value {
        serde_json::json!({
            "schema": "cynic.command_surfaces.v1",
            "surface_roles": {
                "human_operator": {},
                "interactive_agent": {},
                "machine_fallback": {},
                "validation_dev": {},
                "runtime_daemon": {}
            },
            "commands": [
                {
                    "name": "cynic_lifecycle_dispatch",
                    "role": "interactive_agent",
                    "domain": "lifecycle",
                    "canonical": true,
                    "mutates": true,
                    "mutates_mempool": true,
                    "json_output_required": true,
                    "read_back_required": true
                },
                {
                    "name": "make lint-organ-memory-contract",
                    "role": "validation_dev",
                    "domain": "validation",
                    "canonical": true,
                    "mutates": false,
                    "mutates_mempool": false,
                    "json_output_required": true
                }
            ]
        })
    }

    fn base_exposure() -> serde_json::Value {
        serde_json::json!({
            "mcp_tool_domains": {
                "lifecycle": {
                    "tools": ["cynic_lifecycle_dispatch"]
                }
            }
        })
    }

    #[test]
    fn command_surfaces_contract_accepts_valid_shape() {
        let findings = command_surface_findings(
            "axioms/command-surfaces-v1.json",
            &base_contract(),
            &base_exposure(),
            &std::collections::BTreeSet::from(["lint-organ-memory-contract".to_string()]),
        );

        assert_eq!(findings, Vec::<serde_json::Value>::new());
    }

    #[test]
    fn command_surfaces_contract_rejects_machine_command_without_json() {
        let mut contract = base_contract();
        contract["commands"][0]["json_output_required"] = serde_json::json!(false);

        let findings = command_surface_findings(
            "axioms/command-surfaces-v1.json",
            &contract,
            &base_exposure(),
            &std::collections::BTreeSet::from(["lint-organ-memory-contract".to_string()]),
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("machine_command_without_json_output")
        }));
    }

    #[test]
    fn command_surfaces_contract_rejects_mempool_mutation_without_read_back() {
        let mut contract = base_contract();
        contract["commands"][0]["read_back_required"] = serde_json::json!(false);

        let findings = command_surface_findings(
            "axioms/command-surfaces-v1.json",
            &contract,
            &base_exposure(),
            &std::collections::BTreeSet::from(["lint-organ-memory-contract".to_string()]),
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("mempool_mutation_without_read_back")
        }));
    }

    #[test]
    fn command_surfaces_contract_rejects_canonical_mutating_lifecycle_make_target() {
        let mut contract = base_contract();
        contract["commands"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "name": "make deploy-request",
                "role": "validation_dev",
                "domain": "lifecycle",
                "canonical": true,
                "mutates": true,
                "mutates_mempool": true,
                "json_output_required": false,
                "read_back_required": true
            }));

        let findings = command_surface_findings(
            "axioms/command-surfaces-v1.json",
            &contract,
            &base_exposure(),
            &std::collections::BTreeSet::from([
                "lint-organ-memory-contract".to_string(),
                "deploy-request".to_string(),
            ]),
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("make_lifecycle_mutation_marked_canonical")
        }));
    }
}

