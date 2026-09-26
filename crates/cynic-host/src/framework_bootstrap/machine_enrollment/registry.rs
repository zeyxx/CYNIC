use std::path::Path;

use cynic_protocol::{NodeEnrollmentDesiredState, validate_node_enrollment_desired_state};
use serde::Deserialize;

use super::types::MachineEnrollmentAuthoritySpec;

pub fn resolve_local_machine_enrollment_authority_spec(
    registry_path: &Path,
    seed_manifest_path: &Path,
    node_id: &str,
    staging_root: &Path,
) -> Result<MachineEnrollmentAuthoritySpec, String> {
    let authority = crate::runtime_authority::default_runtime_authority_paths();
    let owner_id = crate::identity_database::resolve_identity_id("/etc/passwd", "root")
        .ok_or_else(|| "machine_enrollment_root_identity_unresolved".to_string())?;
    let group_id = crate::identity_database::resolve_identity_id(
        "/etc/group",
        &authority.principal.runtime_group,
    )
    .ok_or_else(|| "machine_enrollment_runtime_group_unresolved".to_string())?;
    let now_unix_s = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("machine_enrollment_clock_invalid:{error}"))?
        .as_secs();
    let observed_hostname = crate::runtime_layout::observed_hostname();
    resolve_machine_enrollment_authority_spec(
        registry_path,
        seed_manifest_path,
        node_id,
        &observed_hostname,
        &authority.config_root,
        staging_root,
        owner_id,
        group_id,
        now_unix_s,
    )
}

#[derive(Deserialize)]
struct RegistryDocument {
    node_enrollment: NodeEnrollmentDesiredState,
    nodes: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct SeedManifest {
    schema: String,
    seed_id: String,
    status: String,
    enrollment: SeedEnrollment,
}

#[derive(Deserialize)]
struct SeedEnrollment {
    required: bool,
    target_role: String,
    identity_class: String,
}

#[allow(clippy::too_many_arguments)]
pub fn resolve_machine_enrollment_authority_spec(
    registry_path: &Path,
    seed_manifest_path: &Path,
    node_id: &str,
    observed_hostname: &str,
    config_root: &Path,
    staging_root: &Path,
    owner_id: u32,
    group_id: u32,
    now_unix_s: u64,
) -> Result<MachineEnrollmentAuthoritySpec, String> {
    let registry = read_json::<RegistryDocument>(registry_path, "runtime registry")?;
    let desired_blockers = validate_node_enrollment_desired_state(&registry.node_enrollment);
    if !desired_blockers.is_empty() {
        return Err(desired_blockers.join(","));
    }
    let seed = read_json::<SeedManifest>(seed_manifest_path, "framework seed manifest")?;
    if seed.schema != "cynic.framework_seed.v2"
        || seed.status != "seed_only"
        || !seed.enrollment.required
        || seed.enrollment.identity_class != "MachineEnrollment"
        || !valid_sha256(&seed.seed_id)
    {
        return Err("framework_seed_manifest_not_admissible".to_string());
    }
    if !registry
        .node_enrollment
        .authority_policy
        .allowed_roles
        .contains(&seed.enrollment.target_role)
    {
        return Err("machine_enrollment_role_not_allowed".to_string());
    }
    let requested_capabilities =
        cynic_node_protocol::required_node_enrollment_capabilities(&seed.enrollment.target_role);
    if !requested_capabilities.iter().all(|capability| {
        registry
            .node_enrollment
            .authority_policy
            .allowed_capabilities
            .contains(capability)
    }) {
        return Err("machine_enrollment_capability_not_allowed".to_string());
    }
    let authority_node = super::super::placement::unique_role_node(
        &registry.nodes,
        &registry.node_enrollment.authority_role,
        "machine_enrollment_authority",
    )?;
    super::super::placement::ensure_local_placement(
        &registry.nodes,
        authority_node,
        observed_hostname,
        "machine_enrollment_authority",
    )?;
    let endpoint_count = authority_node
        .get("services")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter(|service| {
            service.get("name").and_then(serde_json::Value::as_str)
                == Some(registry.node_enrollment.endpoint_service.as_str())
        })
        .count();
    if endpoint_count != 1 {
        return Err("machine_enrollment_endpoint_service_not_unique".to_string());
    }
    if node_id.trim().is_empty() {
        return Err("machine_enrollment_node_id_missing".to_string());
    }

    Ok(MachineEnrollmentAuthoritySpec {
        authority_role: registry.node_enrollment.authority_role,
        endpoint_service: registry.node_enrollment.endpoint_service,
        node_id: node_id.to_string(),
        seed_id: seed.seed_id,
        role: seed.enrollment.target_role,
        requested_capabilities,
        policy: registry.node_enrollment.authority_policy,
        config_root: config_root.display().to_string(),
        staging_root: staging_root.display().to_string(),
        owner_id,
        group_id,
        now_unix_s,
    })
}

fn read_json<T: serde::de::DeserializeOwned>(path: &Path, label: &str) -> Result<T, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("failed to read {label} {}: {error}", path.display()))?;
    serde_json::from_slice(&bytes)
        .map_err(|error| format!("failed to decode {label} {}: {error}", path.display()))
}

fn valid_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
    })
}
