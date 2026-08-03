mod apply;
mod decision;
mod observe;
mod types;

use std::path::{Path, PathBuf};

pub use apply::apply_provider_control_enrollment;
pub use decision::plan_provider_control_enrollment;
pub use observe::observe_provider_control_enrollment;
pub use types::{
    ProviderControlEnrollmentAction, ProviderControlEnrollmentObservation,
    ProviderControlEnrollmentPlan, ProviderControlEnrollmentReport, ProviderControlEnrollmentSpec,
};

pub fn resolve_local_provider_control_enrollment_spec(
    provider: &str,
    endpoint: &str,
    token_id: &str,
    token_secret_source: &Path,
    ca_source: Option<&Path>,
) -> Result<ProviderControlEnrollmentSpec, String> {
    let authority = crate::runtime_authority::default_runtime_authority_paths();
    let owner_id = crate::identity_database::resolve_identity_id("/etc/passwd", "root")
        .ok_or_else(|| "provider_enrollment_root_identity_unresolved".to_string())?;
    let group_id = crate::identity_database::resolve_identity_id(
        "/etc/group",
        &authority.principal.runtime_group,
    )
    .ok_or_else(|| "provider_enrollment_runtime_group_unresolved".to_string())?;
    let secret =
        crate::runtime_secret_install::RuntimeSecretInstallSpec::provider_control(provider)?;
    let config_target = authority.config_root.join("proxmox-control.toml");
    let ca_target = authority.config_root.join("proxmox-ca.pem");
    let desired = cynic_protocol::provider::ProviderControlDesiredState {
        schema: cynic_protocol::provider::PROVIDER_CONTROL_DESIRED_STATE_SCHEMA.to_string(),
        provider: provider.to_string(),
        endpoint: endpoint.to_string(),
        token_identity: token_id.to_string(),
        credential_ref: secret.target.display().to_string(),
        ca_ref: ca_source.map(|_| ca_target.display().to_string()),
        credential_lease_metadata_ref: None,
        credential_authority_public_key_ref: None,
    };
    Ok(ProviderControlEnrollmentSpec {
        desired,
        token_secret_source: token_secret_source.to_path_buf(),
        ca_source: ca_source.map(PathBuf::from),
        config_target,
        ca_target,
        secret,
        owner_id,
        group_id,
    })
}

pub fn resolve_local_provider_control_enrollment_spec_from_registry(
    registry_path: &Path,
    token_secret_source: &Path,
    ca_source: Option<&Path>,
) -> Result<ProviderControlEnrollmentSpec, String> {
    let bytes = std::fs::read(registry_path).map_err(|error| {
        format!(
            "failed to read runtime registry {}: {error}",
            registry_path.display()
        )
    })?;
    let root: serde_json::Value = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "failed to parse runtime registry {}: {error}",
            registry_path.display()
        )
    })?;
    let desired: cynic_protocol::provider::ProviderControlDesiredState = serde_json::from_value(
        root.get("provider_control")
            .cloned()
            .ok_or_else(|| "provider_control_desired_state_missing".to_string())?,
    )
    .map_err(|error| format!("provider_control_desired_state_invalid:{error}"))?;
    let blockers = cynic_protocol::provider::validate_provider_control_desired_state(&desired);
    if !blockers.is_empty() {
        return Err(blockers.join(","));
    }
    let spec = resolve_local_provider_control_enrollment_spec(
        &desired.provider,
        &desired.endpoint,
        &desired.token_identity,
        token_secret_source,
        ca_source,
    )?;
    if spec.desired != desired {
        return Err("provider_control_desired_state_runtime_authority_mismatch".to_string());
    }
    Ok(spec)
}

#[cfg(test)]
mod tests;
