mod apply;
mod decision;
mod filesystem;
mod observe;
mod registry;
mod rollback;
mod types;

pub(crate) use apply::apply_dns_activation;
pub(crate) use decision::plan_dns_activation;
pub(crate) use observe::observe_dns_activation;
pub(crate) use registry::resolve_dns_activation_spec;

pub(crate) fn apply_dns_activation_value(
    registry: &str,
    bundle_dir: &str,
    yes: bool,
) -> Result<serde_json::Value, String> {
    let spec = resolve_dns_activation_spec(registry, bundle_dir)?;
    serde_json::to_value(apply_dns_activation(
        &spec,
        yes,
        &cynic_host::update_lifecycle::LocalArgvRunner,
    ))
    .map_err(|error| format!("DNS activation apply serialization failed: {error}"))
}

#[cfg(test)]
mod tests;
