use std::path::PathBuf;

use super::*;
use crate::runtime_secret_install::RuntimeSecretInstallSpec;

fn spec() -> ProviderControlEnrollmentSpec {
    ProviderControlEnrollmentSpec {
        desired: cynic_protocol::provider::ProviderControlDesiredState {
            schema: cynic_protocol::provider::PROVIDER_CONTROL_DESIRED_STATE_SCHEMA.to_string(),
            provider: "proxmox".to_string(),
            endpoint: "https://pve.internal:8006".to_string(),
            token_identity: "cynic@pve!runtime".to_string(),
            credential_ref: "/etc/cynic/secrets/proxmox-token".to_string(),
            ca_ref: None,
        },
        token_secret_source: PathBuf::from("/run/cynic-enrollment/proxmox-token"),
        ca_source: None,
        config_target: PathBuf::from("/etc/cynic/proxmox-control.toml"),
        ca_target: PathBuf::from("/etc/cynic/proxmox-ca.pem"),
        secret: RuntimeSecretInstallSpec {
            identity_id: "provider-control:proxmox".to_string(),
            target: PathBuf::from("/etc/cynic/secrets/proxmox-token"),
            owner_id: 0,
            group_id: Some(990),
            mode: 0o640,
        },
        owner_id: 0,
        group_id: 990,
    }
}

fn observation() -> ProviderControlEnrollmentObservation {
    ProviderControlEnrollmentObservation {
        schema: "cynic.provider_control.enrollment_observation.v1",
        provider: "proxmox".to_string(),
        config_path: "/etc/cynic/proxmox-control.toml".to_string(),
        config_present: false,
        config_regular_file: None,
        config_mode: None,
        config_owner_id: None,
        config_group_id: None,
        endpoint_matches: None,
        token_identity_matches: None,
        token_secret_path_matches: None,
        ca_path_matches: None,
        secret_source_present: true,
        secret_source_admissible: true,
        secret_target_present: false,
        secret_target_admissible: false,
        ca_source_present: None,
        ca_source_admissible: None,
        ca_target_present: false,
        ca_target_regular_file: None,
        ca_target_mode: None,
        ca_target_owner_id: None,
        ca_target_group_id: None,
        ca_digest_matches: None,
        secret_exposed: false,
        unknowns: Vec::new(),
    }
}

#[test]
fn first_enrollment_is_planned_without_returning_secret_material() {
    let mut spec = spec();
    spec.secret = RuntimeSecretInstallSpec::provider_control("proxmox").unwrap_or(spec.secret);
    spec.group_id = spec.secret.group_id.unwrap_or(spec.group_id);
    let plan = plan_provider_control_enrollment(&spec, &observation());

    assert_eq!(plan.status, "planned");
    assert_eq!(plan.actions.len(), 2);
    assert!(
        !serde_json::to_string(&plan)
            .unwrap()
            .contains("proxmox-token")
    );
    assert!(!plan.secret_exposed);
}

#[test]
fn partial_existing_identity_blocks_implicit_overwrite() {
    let mut spec = spec();
    spec.secret = RuntimeSecretInstallSpec::provider_control("proxmox").unwrap_or(spec.secret);
    spec.group_id = spec.secret.group_id.unwrap_or(spec.group_id);
    let mut observed = observation();
    observed.secret_target_present = true;

    let plan = plan_provider_control_enrollment(&spec, &observed);

    assert_eq!(plan.status, "blocked");
    assert!(
        plan.blockers
            .contains(&"provider_enrollment_partial_state_requires_recovery".to_string())
    );
}

#[test]
fn endpoint_cannot_embed_credentials() {
    let mut spec = spec();
    spec.secret = RuntimeSecretInstallSpec::provider_control("proxmox").unwrap_or(spec.secret);
    spec.group_id = spec.secret.group_id.unwrap_or(spec.group_id);
    spec.desired.endpoint = "https://root:secret@pve.internal:8006".to_string();

    let plan = plan_provider_control_enrollment(&spec, &observation());

    assert_eq!(plan.status, "blocked");
    assert!(
        plan.blockers
            .contains(&"provider_endpoint_not_credential_free_https_origin".to_string())
    );
}
