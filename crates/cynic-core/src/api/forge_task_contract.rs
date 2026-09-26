pub(crate) fn validate_repo_affecting_hermes_task(
    kind: &str,
    domain: &str,
    content: &str,
) -> Result<(), String> {
    cynic_host::lifecycle::forge_task_contract::validate_repo_affecting_hermes_task(
        kind,
        domain,
        content,
        load_registry().as_ref(),
    )
}

fn load_registry() -> Option<serde_json::Value> {
    cynic_host::runtime_registry::load_runtime_registry(None)
}
