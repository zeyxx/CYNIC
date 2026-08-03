use super::types::{
    DnsActivationAction, DnsActivationObservation, DnsActivationPlan, DnsActivationSpec,
};

pub(crate) fn plan_dns_activation(
    spec: &DnsActivationSpec,
    observation: &DnsActivationObservation,
) -> DnsActivationPlan {
    let mut blockers = spec.blockers.clone();
    let mut unknowns = observation.unknowns.clone();
    if spec.schema != "cynic.dns_activation.spec.v1" {
        blockers.push("dns_activation_spec_schema_mismatch".to_string());
    }
    if observation.schema != "cynic.dns_activation.observation.v1" {
        unknowns.push("dns_activation_observation_schema_mismatch".to_string());
    }
    if spec.provider.is_empty()
        || spec.provider_package.is_empty()
        || !spec.provider_provenance_proved
    {
        blockers.push("dns_activation_provider_provenance_unproved".to_string());
    }
    if spec.service_manager_kind != "systemd" {
        blockers.push("dns_activation_service_manager_adapter_unsupported".to_string());
    }
    if spec.files.len() != 4 {
        blockers.push("dns_activation_file_set_incomplete".to_string());
    }
    if spec.files.iter().any(|file| {
        !safe_absolute_path(std::path::Path::new(&file.source))
            || !safe_absolute_path(std::path::Path::new(&file.target))
            || file.sha256.strip_prefix("sha256:").is_none_or(|digest| {
                digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            })
    }) {
        blockers.push("dns_activation_file_contract_invalid".to_string());
    }
    let files_converged = spec.files.iter().all(|desired| {
        observation.files.iter().any(|observed| {
            observed.target == desired.target
                && observed.regular
                && observed.sha256.as_deref() == Some(desired.sha256.as_str())
                && observed.mode == Some(desired.mode)
                && observed.owner_uid == Some(0)
                && observed.owner_gid == Some(0)
        })
    });
    let converged = files_converged
        && observation.service_active == Some(true)
        && observation.service_enabled == Some(true)
        && observation.probe_resolves == Some(true);
    blockers.sort();
    blockers.dedup();
    unknowns.sort();
    unknowns.dedup();
    let status = if !blockers.is_empty() {
        "blocked"
    } else if !unknowns.is_empty() {
        "unknown"
    } else if converged {
        "converged"
    } else {
        "planned"
    };
    let mut actions = Vec::new();
    if status == "planned" {
        actions.extend(
            spec.files
                .iter()
                .map(|file| DnsActivationAction::InstallFile {
                    source: file.source.clone(),
                    target: file.target.clone(),
                }),
        );
        actions.extend(
            activation_argv(spec)
                .into_iter()
                .map(|argv| DnsActivationAction::ExecuteArgv { argv }),
        );
        actions.push(DnsActivationAction::ObserveReadBack);
    }
    DnsActivationPlan {
        schema: "cynic.dns_activation.plan.v1",
        status,
        mutates: status == "planned",
        zone: spec.zone.clone(),
        provider: spec.provider.clone(),
        actions,
        blockers,
        unknowns,
        evidence_refs: observation.evidence_refs.clone(),
        rollback_intent: "restore previous files, created directories, service enablement, and service activity; then read back or retain residual unknowns",
    }
}

fn safe_absolute_path(path: &std::path::Path) -> bool {
    path.is_absolute()
        && path.components().all(|component| {
            matches!(
                component,
                std::path::Component::RootDir | std::path::Component::Normal(_)
            )
        })
}

pub(super) fn activation_argv(spec: &DnsActivationSpec) -> Vec<Vec<String>> {
    vec![
        vec![
            spec.service_manager_bin.clone(),
            "daemon-reload".to_string(),
        ],
        vec![
            spec.service_manager_bin.clone(),
            "enable".to_string(),
            "--now".to_string(),
            spec.service_unit.clone(),
        ],
        vec![
            spec.service_manager_bin.clone(),
            "restart".to_string(),
            spec.resolver_unit.clone(),
        ],
    ]
}
