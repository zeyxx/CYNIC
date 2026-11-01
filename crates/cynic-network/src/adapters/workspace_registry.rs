/// Resolve service endpoints from the workspace registry.
///
/// Walks up from the current directory to find `infra/registry.json`, parses it,
/// calls `service_endpoints_from_registry`, and returns the endpoint strings.
/// Returns `None` when no registry resolves or no endpoints are found.
pub fn endpoints_from_workspace_registry(service_name: &str) -> Option<Vec<String>> {
    let candidate = cynic_types::CynicPaths::find_canonical_registry_path()?;
    let data = std::fs::read_to_string(&candidate).ok()?;
    let registry: serde_json::Value = serde_json::from_str(&data).ok()?;
    let services = crate::topology::discovery::service_endpoints_from_registry(&registry, service_name);
    let endpoints: Vec<String> = services.into_iter().map(|s| s.endpoint).collect();
    if endpoints.is_empty() {
        return None;
    }
    Some(endpoints)
}

