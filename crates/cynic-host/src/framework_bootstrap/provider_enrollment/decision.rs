use super::types::{
    PROVIDER_CONTROL_ENROLLMENT_PLAN_SCHEMA, ProviderControlEnrollmentAction,
    ProviderControlEnrollmentObservation, ProviderControlEnrollmentPlan,
    ProviderControlEnrollmentSpec,
};

pub fn plan_provider_control_enrollment(
    spec: &ProviderControlEnrollmentSpec,
    observed: &ProviderControlEnrollmentObservation,
) -> ProviderControlEnrollmentPlan {
    let mut blockers = validate_spec(spec);
    blockers.extend(observed.unknowns.iter().cloned());
    let config_converged = observed.config_present
        && observed.config_regular_file == Some(true)
        && observed.config_mode == Some(0o640)
        && observed.config_owner_id == Some(spec.owner_id)
        && observed.config_group_id == Some(spec.group_id)
        && observed.endpoint_matches == Some(true)
        && observed.token_identity_matches == Some(true)
        && observed.token_secret_path_matches == Some(true)
        && observed.ca_path_matches == Some(true);
    let ca_converged = match spec.ca_source.as_ref() {
        Some(_) => {
            observed.ca_target_present
                && observed.ca_target_regular_file == Some(true)
                && observed.ca_target_mode == Some(0o644)
                && observed.ca_target_owner_id == Some(spec.owner_id)
                && observed.ca_target_group_id == Some(spec.group_id)
                && observed.ca_digest_matches == Some(true)
        }
        None => !observed.ca_target_present,
    };
    let converged = config_converged && observed.secret_target_admissible && ca_converged;
    let partial =
        observed.config_present || observed.secret_target_present || observed.ca_target_present;
    if !converged && partial {
        blockers.push("provider_enrollment_partial_state_requires_recovery".to_string());
    }
    if !converged && !observed.secret_source_admissible {
        blockers.push("provider_token_secret_source_unavailable".to_string());
    }
    if !converged && spec.ca_source.is_some() && observed.ca_source_admissible != Some(true) {
        blockers.push("provider_ca_source_unavailable".to_string());
    }
    blockers.sort();
    blockers.dedup();
    let status = if converged {
        "converged"
    } else if blockers.is_empty() {
        "planned"
    } else {
        "blocked"
    };
    let actions = if status == "planned" {
        let mut actions = vec![ProviderControlEnrollmentAction::InstallRuntimeMachineSecret];
        if spec.ca_source.is_some() {
            actions.push(ProviderControlEnrollmentAction::InstallProviderCa);
        }
        actions.push(ProviderControlEnrollmentAction::InstallProviderConfiguration);
        actions
    } else {
        Vec::new()
    };
    ProviderControlEnrollmentPlan {
        schema: PROVIDER_CONTROL_ENROLLMENT_PLAN_SCHEMA,
        status,
        mutates: false,
        provider: spec.desired.provider.clone(),
        endpoint: spec.desired.endpoint.clone(),
        token_identity: spec.desired.token_identity.clone(),
        actions,
        blockers,
        required_proofs: vec![
            "provider_config_owner_group_mode_read_back",
            "provider_token_presence_without_value",
            "provider_token_owner_group_mode_read_back",
            "provider_configuration_binding_read_back",
            "provider_ca_binding_read_back",
            "provider_api_authentication_read_back",
        ],
        rollback_intent: "remove only files created by this first-enrollment transaction when their exact material still matches; credential rotation is a separate transaction",
        secret_exposed: false,
    }
}

fn validate_spec(spec: &ProviderControlEnrollmentSpec) -> Vec<String> {
    let mut blockers =
        cynic_protocol::provider::validate_provider_control_desired_state(&spec.desired);
    if spec.desired.provider != "proxmox" {
        blockers.push("provider_enrollment_adapter_unavailable".to_string());
    }
    let authority = crate::runtime_authority::default_runtime_authority_paths();
    let canonical_secret =
        crate::runtime_secret_install::RuntimeSecretInstallSpec::provider_control("proxmox").ok();
    if spec.config_target != authority.config_root.join("proxmox-control.toml")
        || spec.ca_target != authority.config_root.join("proxmox-ca.pem")
        || canonical_secret.as_ref() != Some(&spec.secret)
        || spec.desired.credential_ref != spec.secret.target.display().to_string()
        || spec.desired.ca_ref
            != spec
                .ca_source
                .as_ref()
                .map(|_| spec.ca_target.display().to_string())
        || spec.owner_id != 0
        || Some(spec.group_id) != spec.secret.group_id
    {
        blockers.push("provider_enrollment_runtime_authority_mismatch".to_string());
    }
    if !spec.token_secret_source.is_absolute()
        || spec
            .ca_source
            .as_ref()
            .is_some_and(|path| !path.is_absolute())
    {
        blockers.push("provider_enrollment_source_path_not_absolute".to_string());
    }
    blockers
}
