#[cfg(test)]
mod api_layer_alignment_contract_tests {
    use super::*;

    #[test]
    fn api_layer_alignment_rejects_symbolic_vocabulary_drift() {
        let contract = serde_json::json!({
            "schema": "cynic.api_layer_alignment.v1",
            "status": "normative",
            "symbolic_vocabulary": {"Keter": "source"},
            "guardrails": ["analogy stays below evidence"],
            "layers": {
                "perception": {
                    "engineering_role": "test",
                    "agent_question": "test?",
                    "system_primitives": ["read", "poll", "file descriptor"],
                    "symbolic_alignment": ["Keter"],
                    "must": ["test"],
                    "must_not": ["test"]
                }
            },
            "vertical_alignment_rules": [{"id": "test", "rule": "test", "enforcement": "validator"}]
        });
        let topology = serde_json::json!({
            "schema": "cynic.api_topology.v1",
            "status": "normative",
            "source_of_truth": {"layer_alignment": "axioms/api-layer-alignment-v1.json"},
            "allowed": {"layers": ["perception"]},
            "operations": {}
        });
        let peer = serde_json::json!({
            "schema": "test",
            "status": "normative",
            "symbolic_vocabulary": {
                "Keter": "source",
                "Binah": "bounded form"
            }
        });

        let findings = api_layer_alignment_contract_findings(
            &contract,
            &topology,
            &peer,
            &peer,
            "axioms/api-layer-alignment-v1.json",
            "axioms/api-topology-v1.json",
            "axioms/meta-layer-contract-v1.json",
            "axioms/organ-layer-topology-v1.json",
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("api_layer_symbolic_vocabulary_missing_peer_symbols")
        }));
    }

    #[test]
    fn api_layer_alignment_rejects_missing_system_primitives_strength() {
        let contract = serde_json::json!({
            "schema": "cynic.api_layer_alignment.v1",
            "status": "normative",
            "symbolic_vocabulary": {"Keter": "source"},
            "guardrails": ["analogy stays below evidence"],
            "layers": {
                "perception": {
                    "engineering_role": "test",
                    "agent_question": "test?",
                    "system_primitives": ["read", "poll"],
                    "symbolic_alignment": ["Keter"],
                    "must": ["test"],
                    "must_not": ["test"]
                }
            },
            "vertical_alignment_rules": [{"id": "test", "rule": "test", "enforcement": "validator"}]
        });
        let topology = serde_json::json!({
            "schema": "cynic.api_topology.v1",
            "status": "normative",
            "source_of_truth": {"layer_alignment": "axioms/api-layer-alignment-v1.json"},
            "allowed": {"layers": ["perception"]},
            "operations": {}
        });
        let peer = serde_json::json!({
            "schema": "test",
            "status": "normative",
            "symbolic_vocabulary": {"Keter": "source"}
        });

        let findings = api_layer_alignment_contract_findings(
            &contract,
            &topology,
            &peer,
            &peer,
            "axioms/api-layer-alignment-v1.json",
            "axioms/api-topology-v1.json",
            "axioms/meta-layer-contract-v1.json",
            "axioms/organ-layer-topology-v1.json",
        );

        assert!(findings.iter().any(|finding| {
            finding.get("kind").and_then(|value| value.as_str())
                == Some("api_layer_alignment_system_primitives_too_weak")
        }));
    }
}

