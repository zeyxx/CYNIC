mod decision;
mod registry;
mod types;

pub use types::{FrameworkLifecycleAudit, FrameworkLifecycleDimension};

pub fn audit_framework_lifecycle(
    registry_path: Option<&str>,
    role: &str,
) -> Result<FrameworkLifecycleAudit, String> {
    if role.trim().is_empty() {
        return Err("framework lifecycle role is required".to_string());
    }
    let path = crate::runtime_registry::resolve_runtime_registry_path(
        registry_path.map(std::path::Path::new),
    )?;
    let bytes = std::fs::read(&path).map_err(|error| {
        format!(
            "failed to read runtime registry {}: {error}",
            path.display()
        )
    })?;
    let root = serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|error| format!("runtime registry {} is invalid: {error}", path.display()))?;
    let (registry, blockers) = registry::LifecycleRegistry::resolve(&root, role);
    Ok(decision::decide(&registry, blockers))
}

#[cfg(test)]
mod tests;
