#[cfg(test)]
mod coordination_profile_tests {
    use std::path::Path;

    fn contract_events_required_fields_for_tests(
        protocol: &serde_json::Value,
        profile: Option<&serde_json::Value>,
        event: &str,
    ) -> Vec<String> {
        let mut fields = Vec::new();
        if let Some(obj) = protocol
            .get("coordination_protocol")
            .and_then(|value| value.get("event_contract"))
            .and_then(|value| value.get("required_fields_by_event"))
            .and_then(|value| value.get(event))
        {
            if let Some(list) = obj.as_array() {
                fields.extend(
                    list.iter()
                        .filter_map(|value| value.as_str())
                        .map(String::from),
                );
            }
        }

        if let Some(list) = profile.and_then(|value| value.as_array()) {
            fields.extend(
                list.iter()
                    .filter_map(|value| value.as_str())
                    .map(String::from),
            );
        }

        fields.sort();
        fields.dedup();
        fields
    }

    fn load_coordination_protocol_for_tests() -> serde_json::Value {
        let mut cursor = Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf();
        let protocol_path = loop {
            let candidate = cursor.join("contracts/foundation.json");
            if candidate.is_file() {
                break candidate;
            }
            match cursor.parent() {
                Some(parent) => cursor = parent.to_path_buf(),
                None => break cursor.join("contracts/foundation.json"),
            }
        };
        let path = protocol_path;
        let content = std::fs::read_to_string(&path)
            .expect("coordination protocol fixture should be readable in test workspace");
        serde_json::from_str(&content).expect("coordination protocol fixture should be valid json")
    }

    #[test]
    fn contract_events_required_fields_merges_profile_minimum_fields() {
        let protocol = load_coordination_protocol_for_tests();
        let profile = serde_json::json!(["branch", "owner", "scope"]);

        let required =
            contract_events_required_fields_for_tests(&protocol, Some(&profile), "workstream.started");

        assert_eq!(required, vec!["branch", "owner", "scope"]);
    }

    #[test]
    fn contract_events_required_fields_remains_protocol_only_if_profile_absent() {
        let protocol = load_coordination_protocol_for_tests();
        let required = contract_events_required_fields_for_tests(&protocol, None, "decision.record");

        assert_eq!(required, Vec::<String>::new());
    }
}
