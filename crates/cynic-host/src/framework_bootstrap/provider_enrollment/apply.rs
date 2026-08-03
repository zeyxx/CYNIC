use std::fs::OpenOptions;
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;

use crate::runtime_secret_install::{
    install_protected_runtime_file, install_runtime_secret,
    remove_protected_runtime_file_if_matches, remove_runtime_secret_if_matches,
};
use crate::secret_material::SecretMaterial;

use super::decision::plan_provider_control_enrollment;
use super::observe::observe_provider_control_enrollment;
use super::types::{
    PROVIDER_CONTROL_ENROLLMENT_REPORT_SCHEMA, ProviderControlEnrollmentReport,
    ProviderControlEnrollmentSpec,
};

pub fn apply_provider_control_enrollment(
    spec: &ProviderControlEnrollmentSpec,
    yes: bool,
) -> ProviderControlEnrollmentReport {
    let before = observe_provider_control_enrollment(spec);
    let plan = plan_provider_control_enrollment(spec, &before);
    let mut blockers = plan.blockers.clone();
    if !yes {
        blockers.push("explicit_yes_required".to_string());
    }
    if plan.status == "converged" && yes {
        return report(
            spec,
            before.clone(),
            before,
            false,
            true,
            None,
            Vec::new(),
            false,
        );
    }
    if plan.status != "planned" {
        blockers.push("provider_enrollment_plan_not_actionable".to_string());
    }
    if !blockers.is_empty() {
        blockers.sort();
        blockers.dedup();
        return report(
            spec,
            before.clone(),
            before,
            false,
            yes,
            None,
            blockers,
            false,
        );
    }
    let secret = match SecretMaterial::read_file(&spec.token_secret_source) {
        Ok(secret) => secret,
        Err(error) => {
            return report(
                spec,
                before.clone(),
                before,
                false,
                yes,
                None,
                vec![error],
                false,
            );
        }
    };
    let ca_bytes = match spec
        .ca_source
        .as_deref()
        .map(read_public_material)
        .transpose()
    {
        Ok(bytes) => bytes,
        Err(error) => {
            return report(
                spec,
                before.clone(),
                before,
                false,
                yes,
                None,
                vec![format!("provider_ca_source_read_failed:{error}")],
                false,
            );
        }
    };
    let config_bytes = encode_config(spec);
    let secret_install = match install_runtime_secret(&spec.secret, &secret) {
        Ok(report) => report,
        Err(error) => {
            return report(
                spec,
                before.clone(),
                before,
                false,
                yes,
                None,
                vec![error],
                false,
            );
        }
    };
    let mut ca_installed = false;
    if let Some(bytes) = ca_bytes.as_deref() {
        if let Err(error) = install_protected_runtime_file(
            "provider-control-ca:proxmox",
            &spec.ca_target,
            spec.owner_id,
            spec.group_id,
            0o644,
            bytes,
        ) {
            let rolled_back = remove_runtime_secret_if_matches(&spec.secret, &secret).is_ok();
            return report(
                spec,
                before,
                observe_provider_control_enrollment(spec),
                true,
                yes,
                Some(secret_install),
                vec![error],
                rolled_back,
            );
        }
        ca_installed = true;
    }
    if let Err(error) = install_protected_runtime_file(
        "provider-control-config:proxmox",
        &spec.config_target,
        spec.owner_id,
        spec.group_id,
        0o640,
        &config_bytes,
    ) {
        let mut rolled_back = remove_runtime_secret_if_matches(&spec.secret, &secret).is_ok();
        if ca_installed && let Some(bytes) = ca_bytes.as_deref() {
            rolled_back &= remove_protected_runtime_file_if_matches(&spec.ca_target, bytes).is_ok();
        }
        return report(
            spec,
            before,
            observe_provider_control_enrollment(spec),
            true,
            yes,
            Some(secret_install),
            vec![error],
            rolled_back,
        );
    }
    let after = observe_provider_control_enrollment(spec);
    let read_back = plan_provider_control_enrollment(spec, &after);
    if read_back.status == "converged" {
        return report(
            spec,
            before,
            after,
            true,
            yes,
            Some(secret_install),
            Vec::new(),
            false,
        );
    }
    let mut rolled_back =
        remove_protected_runtime_file_if_matches(&spec.config_target, &config_bytes).is_ok();
    if ca_installed && let Some(bytes) = ca_bytes.as_deref() {
        rolled_back &= remove_protected_runtime_file_if_matches(&spec.ca_target, bytes).is_ok();
    }
    rolled_back &= remove_runtime_secret_if_matches(&spec.secret, &secret).is_ok();
    report(
        spec,
        before,
        observe_provider_control_enrollment(spec),
        true,
        yes,
        Some(secret_install),
        vec!["provider_enrollment_read_back_failed".to_string()],
        rolled_back,
    )
}

fn read_public_material(path: &Path) -> Result<Vec<u8>, String> {
    const MAX_PUBLIC_MATERIAL_BYTES: u64 = 1024 * 1024;
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)
        .map_err(|error| format!("source unavailable: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("source metadata unavailable: {error}"))?;
    if !metadata.file_type().is_file()
        || metadata.len() == 0
        || metadata.len() > MAX_PUBLIC_MATERIAL_BYTES
    {
        return Err("source must be a bounded non-empty regular file".to_string());
    }
    let mut bytes = Vec::with_capacity(metadata.len() as usize);
    file.take(MAX_PUBLIC_MATERIAL_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| format!("source read failed: {error}"))?;
    if bytes.len() as u64 > MAX_PUBLIC_MATERIAL_BYTES {
        return Err("source exceeds the bounded size".to_string());
    }
    Ok(bytes)
}

fn encode_config(spec: &ProviderControlEnrollmentSpec) -> Vec<u8> {
    let mut content = format!(
        "endpoint = {}\ntoken_id = {}\ntoken_secret_file = {}\n",
        toml_string(&spec.desired.endpoint),
        toml_string(&spec.desired.token_identity),
        toml_string(&spec.secret.target.display().to_string()),
    );
    if spec.ca_source.is_some() {
        content.push_str(&format!(
            "ca_file = {}\n",
            toml_string(&spec.ca_target.display().to_string())
        ));
    }
    content.into_bytes()
}

fn toml_string(value: &str) -> String {
    serde_json::to_string(value).expect("provider configuration strings are serializable")
}

#[allow(clippy::too_many_arguments)]
fn report(
    spec: &ProviderControlEnrollmentSpec,
    before: super::types::ProviderControlEnrollmentObservation,
    after: super::types::ProviderControlEnrollmentObservation,
    mutates: bool,
    requested_apply: bool,
    secret_install: Option<crate::runtime_secret_install::RuntimeSecretInstallReport>,
    blockers: Vec<String>,
    rollback_performed: bool,
) -> ProviderControlEnrollmentReport {
    ProviderControlEnrollmentReport {
        schema: PROVIDER_CONTROL_ENROLLMENT_REPORT_SCHEMA,
        status: if blockers.is_empty() {
            if mutates {
                "installed_pending_provider_read_back"
            } else {
                "converged_local_enrollment"
            }
        } else {
            "blocked"
        },
        mutates,
        requested_apply,
        provider: spec.desired.provider.clone(),
        before,
        after,
        secret_install,
        proofs: if blockers.is_empty() {
            vec![
                "provider_config_owner_group_mode_read_back",
                "provider_token_presence_without_value",
                "provider_token_owner_group_mode_read_back",
                "provider_configuration_binding_read_back",
                "provider_ca_binding_read_back",
            ]
        } else {
            Vec::new()
        },
        blockers,
        residual_unknowns: vec!["provider_api_authentication_read_back_required".to_string()],
        rollback_performed,
        secret_exposed: false,
    }
}
