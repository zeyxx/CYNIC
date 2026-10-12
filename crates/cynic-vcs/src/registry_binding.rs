//! Registry binding — pure lookup of the sovereign VCS provider block.
//!
//! The registry (`infra/registry.json`) declares the sovereign source SSOT
//! under `vcs_provider.sovereign_ssot`, nested either inside a node's
//! `source_workspace_layout` or `source_lifecycle` block, or at the document
//! root. The schema has evolved over time, so the lookup scans dynamically
//! instead of pinning one path — a renamed block must not silently break
//! every PR surface.
//!
//! Pure over the parsed JSON value: no I/O, no filesystem, no network.

/// Lookup the JSON pointer for the `vcs_provider` block, dynamically scanning
/// across nodes to be resilient to registry schema evolution.
///
/// Returns `None` when no candidate path carries the block; callers decide
/// how to fail closed on that absence.
pub fn find_ssot_in_registry(reg: &serde_json::Value) -> Option<&serde_json::Value> {
    if let Some(nodes) = reg.get("nodes").and_then(|n| n.as_array()) {
        for node in nodes {
            if let Some(ssot) = node.pointer("/source_workspace_layout/vcs_provider/sovereign_ssot")
            {
                return Some(ssot);
            }
            if let Some(ssot) = node.pointer("/source_lifecycle/vcs_provider/sovereign_ssot") {
                return Some(ssot);
            }
        }
    }
    let top_candidates = [
        "/source_lifecycle/vcs_provider/sovereign_ssot",
        "/vcs_provider/sovereign_ssot",
    ];
    top_candidates.iter().find_map(|ptr| reg.pointer(ptr))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry_with_ssot_at(ptr: &str) -> serde_json::Value {
        let mut reg = serde_json::json!({ "nodes": [] });
        let mut target = serde_json::Value::Object(serde_json::Map::new());
        for (i, segment) in ptr.trim_start_matches('/').split('/').enumerate() {
            if i == ptr.trim_start_matches('/').split('/').count() - 1 {
                target = serde_json::json!({ segment: { "kind": "forgejo" } });
            } else {
                target = serde_json::json!({ segment: target });
            }
        }
        // Merge the nested structure into the registry root.
        merge(&mut reg, &target);
        reg
    }

    fn merge(base: &mut serde_json::Value, overlay: &serde_json::Value) {
        match (base, overlay) {
            (serde_json::Value::Object(base_map), serde_json::Value::Object(overlay_map)) => {
                for (key, value) in overlay_map {
                    merge(base_map.entry(key.clone()).or_insert(serde_json::Value::Null), value);
                }
            }
            (base, overlay) => {
                *base = overlay.clone();
            }
        }
    }

    #[test]
    fn finds_ssot_under_node_source_workspace_layout() {
        let reg = registry_with_ssot_at("/nodes/0/source_workspace_layout/vcs_provider/sovereign_ssot");
        let ssot = find_ssot_in_registry(&reg).expect("ssot found");
        assert_eq!(ssot["kind"], "forgejo");
    }

    #[test]
    fn finds_ssot_under_node_source_lifecycle() {
        let reg = registry_with_ssot_at("/nodes/2/source_lifecycle/vcs_provider/sovereign_ssot");
        let ssot = find_ssot_in_registry(&reg).expect("ssot found");
        assert_eq!(ssot["kind"], "forgejo");
    }

    #[test]
    fn finds_ssot_at_document_root() {
        let reg = registry_with_ssot_at("/source_lifecycle/vcs_provider/sovereign_ssot");
        let ssot = find_ssot_in_registry(&reg).expect("ssot found");
        assert_eq!(ssot["kind"], "forgejo");
    }

    #[test]
    fn returns_none_when_no_candidate_path_carries_the_block() {
        let reg = serde_json::json!({ "nodes": [{ "role": "cortex" }] });
        assert!(find_ssot_in_registry(&reg).is_none());
    }
}
