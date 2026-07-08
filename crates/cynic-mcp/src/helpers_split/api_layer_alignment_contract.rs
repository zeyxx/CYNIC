fn api_layer_alignment_contract_paths(project_root: &str) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let root = PathBuf::from(project_root);
    (
        root.join("axioms/api-layer-alignment-v1.json"),
        root.join("axioms/api-topology-v1.json"),
        root.join("axioms/meta-layer-contract-v1.json"),
        root.join("axioms/organ-layer-topology-v1.json"),
    )
}

fn validate_api_layer_alignment_contract(project_root: &str) -> Vec<serde_json::Value> {
    let (contract_path, topology_path, meta_layer_path, organ_layer_path) =
        api_layer_alignment_contract_paths(project_root);
    let mut findings = Vec::new();

    let contract = match read_json_contract(&contract_path, "api_layer_alignment_contract", &mut findings) {
        Some(value) => value,
        None => return findings,
    };
    let topology = match read_json_contract(&topology_path, "api_topology_contract", &mut findings) {
        Some(value) => value,
        None => return findings,
    };
    let meta_layer = match read_json_contract(&meta_layer_path, "meta_layer_contract", &mut findings) {
        Some(value) => value,
        None => return findings,
    };
    let organ_layer =
        match read_json_contract(&organ_layer_path, "organ_layer_topology_contract", &mut findings) {
            Some(value) => value,
            None => return findings,
        };

    findings.extend(api_layer_alignment_contract_findings(
        &contract,
        &topology,
        &meta_layer,
        &organ_layer,
        "axioms/api-layer-alignment-v1.json",
        "axioms/api-topology-v1.json",
        "axioms/meta-layer-contract-v1.json",
        "axioms/organ-layer-topology-v1.json",
    ));
    findings
}

fn api_layer_alignment_contract_findings(
    contract: &serde_json::Value,
    topology: &serde_json::Value,
    meta_layer: &serde_json::Value,
    organ_layer: &serde_json::Value,
    contract_path: &str,
    topology_path: &str,
    meta_layer_path: &str,
    organ_layer_path: &str,
) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();

    if contract.get("schema").and_then(|value| value.as_str()) != Some("cynic.api_layer_alignment.v1") {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "unexpected_api_layer_alignment_schema",
            "path": contract_path,
            "detail": format!(
                "expected cynic.api_layer_alignment.v1, got {:?}",
                contract.get("schema").and_then(|value| value.as_str())
            ),
        }));
    }
    if contract.get("status").and_then(|value| value.as_str()) != Some("normative") {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_layer_alignment_not_normative",
            "path": contract_path,
            "detail": "api-layer-alignment-v1 must be normative before agents can use it for API geometry.",
        }));
    }

    if topology
        .get("source_of_truth")
        .and_then(|value| value.get("layer_alignment"))
        .and_then(|value| value.as_str())
        != Some(contract_path)
    {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_topology_layer_alignment_source_drift",
            "path": topology_path,
            "detail": format!("source_of_truth.layer_alignment must be {contract_path:?}."),
        }));
    }

    let (api_vocabulary, vocabulary_findings) =
        symbolic_vocabulary(contract, contract_path, "api_layer_alignment");
    findings.extend(vocabulary_findings);

    for (peer_contract, peer_path, peer_name, kind) in [
        (meta_layer, meta_layer_path, "meta-layer-contract-v1", "meta_layer_contract"),
        (
            organ_layer,
            organ_layer_path,
            "organ-layer-topology-v1",
            "organ_layer_topology_contract",
        ),
    ] {
        let (peer_vocabulary, peer_findings) = symbolic_vocabulary(peer_contract, peer_path, kind);
        findings.extend(peer_findings);
        if !api_vocabulary.is_empty() && !peer_vocabulary.is_empty() {
            findings.extend(validate_symbolic_compatibility(
                &api_vocabulary,
                &peer_vocabulary,
                peer_path,
                peer_name,
                contract_path,
            ));
        }
    }

    let layers = contract.get("layers").and_then(|value| value.as_object());
    if layers.is_none_or(|value| value.is_empty()) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_layer_alignment_layers_missing",
            "path": contract_path,
            "detail": "api-layer-alignment-v1 must declare non-empty layers.",
        }));
    }
    let empty_layers = serde_json::Map::new();
    let layers = layers.unwrap_or(&empty_layers);
    let allowed_layers: std::collections::BTreeSet<String> = topology
        .get("allowed")
        .and_then(|value| value.get("layers"))
        .and_then(|value| value.as_array())
        .into_iter()
        .flatten()
        .filter_map(|value| value.as_str().map(str::to_string))
        .collect();
    let alignment_layers: std::collections::BTreeSet<String> = layers.keys().cloned().collect();
    for layer in allowed_layers.difference(&alignment_layers) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_layer_alignment_layer_missing",
            "path": contract_path,
            "detail": format!("missing alignment for API topology layer {layer:?}."),
        }));
    }
    for layer in alignment_layers.difference(&allowed_layers) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_layer_alignment_layer_unknown",
            "path": contract_path,
            "detail": format!("alignment declares layer {layer:?}, absent from api topology allowed.layers."),
        }));
    }

    let required_fields = [
        "engineering_role",
        "agent_question",
        "system_primitives",
        "symbolic_alignment",
        "must",
        "must_not",
    ];
    for (layer_name, layer) in layers {
        let Some(layer) = layer.as_object() else {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_layer_alignment_layer_invalid",
                "path": layer_name,
                "detail": "alignment layers must be objects.",
            }));
            continue;
        };
        for field in required_fields {
            let present = match layer.get(field) {
                Some(serde_json::Value::Array(items)) => !items.is_empty(),
                Some(serde_json::Value::String(text)) => !text.trim().is_empty(),
                Some(serde_json::Value::Null) | None => false,
                Some(_) => true,
            };
            if !present {
                findings.push(serde_json::json!({
                    "severity": "critical",
                    "kind": "api_layer_alignment_metadata_missing",
                    "path": layer_name,
                    "detail": format!("alignment layer must declare {field}."),
                }));
            }
        }
        for field in ["system_primitives", "symbolic_alignment", "must", "must_not"] {
            let values = layer.get(field).and_then(|value| value.as_array());
            if values.is_none_or(|values| values.is_empty()) {
                findings.push(serde_json::json!({
                    "severity": "critical",
                    "kind": "api_layer_alignment_list_metadata_invalid",
                    "path": layer_name,
                    "detail": format!("alignment field {field} must be a non-empty list."),
                }));
                continue;
            }
            if values.is_some_and(|values| {
                values
                    .iter()
                    .any(|value| value.as_str().is_none_or(|value| value.trim().is_empty()))
            }) {
                findings.push(serde_json::json!({
                    "severity": "critical",
                    "kind": "api_layer_alignment_list_metadata_invalid",
                    "path": layer_name,
                    "detail": format!("alignment field {field} entries must be non-empty strings."),
                }));
            }
        }
        if layer
            .get("system_primitives")
            .and_then(|value| value.as_array())
            .map(|values| values.len())
            .unwrap_or(0)
            < 3
        {
            findings.push(serde_json::json!({
                "severity": "critical",
                "kind": "api_layer_alignment_system_primitives_too_weak",
                "path": layer_name,
                "detail": "system_primitives must name at least three concrete OS or engineering primitives.",
            }));
        }
        if let Some(symbols) = layer.get("symbolic_alignment").and_then(|value| value.as_array()) {
            for symbol in symbols.iter().filter_map(|value| value.as_str()) {
                if !api_vocabulary.is_empty() && !api_vocabulary.contains_key(symbol) {
                    findings.push(serde_json::json!({
                        "severity": "critical",
                        "kind": "api_layer_alignment_unknown_symbol",
                        "path": layer_name,
                        "detail": format!("symbolic_alignment references {symbol:?}, absent from symbolic_vocabulary."),
                    }));
                }
            }
        }
    }

    if contract.get("guardrails").and_then(|value| value.as_array()).is_none_or(|value| value.is_empty()) {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_layer_alignment_guardrails_missing",
            "path": contract_path,
            "detail": "api-layer-alignment-v1 must declare guardrails that keep analogy below executable evidence.",
        }));
    }
    if contract
        .get("vertical_alignment_rules")
        .and_then(|value| value.as_array())
        .is_none_or(|value| value.is_empty())
    {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_layer_alignment_vertical_rules_missing",
            "path": contract_path,
            "detail": "api-layer-alignment-v1 must declare vertical_alignment_rules.",
        }));
    }

    findings
}

fn symbolic_vocabulary(
    contract: &serde_json::Value,
    path: &str,
    kind: &str,
) -> (std::collections::BTreeMap<String, String>, Vec<serde_json::Value>) {
    let mut findings = Vec::new();
    let Some(vocabulary) = contract.get("symbolic_vocabulary").and_then(|value| value.as_object()) else {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": format!("{kind}_symbolic_vocabulary_missing"),
            "path": path,
            "detail": format!("{path} must declare symbolic_vocabulary."),
        }));
        return (std::collections::BTreeMap::new(), findings);
    };
    if vocabulary.is_empty() {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": format!("{kind}_symbolic_vocabulary_missing"),
            "path": path,
            "detail": format!("{path} must declare symbolic_vocabulary."),
        }));
        return (std::collections::BTreeMap::new(), findings);
    }
    let invalid: Vec<String> = vocabulary
        .iter()
        .filter_map(|(key, value)| {
            let valid = !key.trim().is_empty()
                && value
                    .as_str()
                    .is_some_and(|value| !value.trim().is_empty());
            if valid { None } else { Some(key.clone()) }
        })
        .collect();
    if !invalid.is_empty() {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": format!("{kind}_symbolic_vocabulary_invalid"),
            "path": path,
            "detail": format!("symbolic_vocabulary contains invalid entries: {invalid:?}."),
        }));
        return (std::collections::BTreeMap::new(), findings);
    }
    let result = vocabulary
        .iter()
        .map(|(key, value)| (key.clone(), value.as_str().unwrap_or_default().to_string()))
        .collect();
    (result, findings)
}

fn validate_symbolic_compatibility(
    api_vocabulary: &std::collections::BTreeMap<String, String>,
    peer_vocabulary: &std::collections::BTreeMap<String, String>,
    peer_path: &str,
    peer_name: &str,
    contract_path: &str,
) -> Vec<serde_json::Value> {
    let mut findings = Vec::new();
    let api_symbols: std::collections::BTreeSet<&String> = api_vocabulary.keys().collect();
    let peer_symbols: std::collections::BTreeSet<&String> = peer_vocabulary.keys().collect();
    let missing_from_api: Vec<&String> = peer_symbols.difference(&api_symbols).copied().collect();
    let missing_from_peer: Vec<&String> = api_symbols.difference(&peer_symbols).copied().collect();
    if !missing_from_api.is_empty() {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_layer_symbolic_vocabulary_missing_peer_symbols",
            "path": contract_path,
            "detail": format!("API layer alignment is missing symbols from {peer_name}: {missing_from_api:?}."),
        }));
    }
    if !missing_from_peer.is_empty() {
        findings.push(serde_json::json!({
            "severity": "critical",
            "kind": "api_layer_symbolic_vocabulary_peer_missing_symbols",
            "path": peer_path,
            "detail": format!("{peer_name} is missing API layer symbols: {missing_from_peer:?}."),
        }));
    }
    findings
}
