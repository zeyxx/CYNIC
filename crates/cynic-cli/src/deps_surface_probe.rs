use std::process::Command;

use crate::deps_surface_model::{
    DepEntry, DepSpec, DepWatch, PinEntry, PinView, PythonCliUpdateAuthorityView, VersionFrom,
};
use cynic_host::domain::ecosystem_package::{
    InstallPrincipalKind, PrincipalInstallLayout, ProviderChannel,
    observe_python_install_authority, observe_tooling_authority, plan_for_dep, provider_spec,
    python_cli_policy, resolve_binary,
};

pub(crate) fn probe_and_evaluate(
    spec: &DepSpec,
    pin: Option<&PinEntry>,
    layout: &PrincipalInstallLayout,
) -> DepEntry {
    let channel = channel_for_dep(spec.id);
    let install_principal = principal_for_dep(spec.id);
    let upgrade_mode = mode_from_channel(channel);

    let mut path = None;
    let mut under_principal = false;
    for bin in spec.bins {
        let observation = resolve_binary(bin, layout);
        if let Some(resolved_path) = observation.path {
            under_principal = observation.under_principal;
            path = Some(resolved_path);
            break;
        }
    }
    let watch = DepWatch {
        upstream: spec.upstream,
        check_hint: spec.check_hint,
        update_hint: spec.update_hint,
        update_commands: if let Some(plan) = plan_for_dep(spec.id, "refresh", layout) {
            plan.summary_commands
        } else {
            spec.update_commands
                .iter()
                .map(|command| (*command).to_string())
                .collect()
        },
    };

    let pin_view = pin.map(|entry| PinView {
        min_version: entry.min_version.clone(),
        min_build: entry.min_build,
        desired_build: entry.desired_build,
        min_capability: entry.min_capability.clone(),
        notes: entry.notes.clone(),
    });

    let Some(path) = path else {
        let optional = pin
            .map(|entry| entry.required == Some(false))
            .unwrap_or(false);
        let python_install_authority = observe_python_install_authority(
            spec.python_distribution,
            None,
            spec.expected_python_provenance,
        );
        return DepEntry {
            id: spec.id,
            role: spec.role,
            category: spec.category,
            tier: if optional { "optional" } else { spec.tier },
            path: None,
            version: None,
            status: "missing".into(),
            upgrade_mode,
            channel,
            install_principal,
            under_principal: false,
            tooling_authority_family: spec.tooling_authority_family,
            tooling_install_provenance: None,
            expected_tooling_install_provenance: spec.expected_tooling_provenance,
            python_install_provenance: python_install_authority.observed_provenance,
            expected_python_install_provenance: python_install_authority.expected_provenance,
            install_authority_status: python_install_authority.status,
            canonical_update_authority: python_cli_policy(spec.id).map(|policy| {
                PythonCliUpdateAuthorityView {
                    observed_payload_provenance: None,
                    expected_payload_provenance: python_install_authority
                        .expected_provenance
                        .map(format_python_provenance),
                    canonical_update_command: policy.canonical_update_command.to_string(),
                    update_authority_note: policy.update_authority_note.to_string(),
                    normalizable_now: python_cli_normalizable_now(layout, policy.dep_id),
                    requires_uv: python_cli_requires_uv(policy.dep_id),
                    requires_runtime_principal: python_cli_requires_runtime_principal(
                        layout,
                        policy.dep_id,
                    ),
                    can_auto_apply: python_cli_can_auto_apply(layout, policy.dep_id),
                }
            }),
            pin: pin_view,
            watch,
        };
    };

    let version = read_version(spec, &path);
    let status = evaluate_status(spec.id, version.as_deref(), pin);
    let tooling_authority = spec.tooling_authority_family.map(|family| {
        observe_tooling_authority(
            family,
            spec.python_distribution,
            Some(&path),
            spec.expected_tooling_provenance,
        )
    });
    let python_install_authority = observe_python_install_authority(
        spec.python_distribution,
        Some(&path),
        spec.expected_python_provenance,
    );

    DepEntry {
        id: spec.id,
        role: spec.role,
        category: spec.category,
        tier: spec.tier,
        path: Some(path),
        version,
        status,
        upgrade_mode,
        channel,
        install_principal,
        under_principal,
        tooling_authority_family: spec.tooling_authority_family,
        tooling_install_provenance: tooling_authority
            .as_ref()
            .and_then(|a| a.observed_provenance),
        expected_tooling_install_provenance: tooling_authority
            .as_ref()
            .and_then(|a| a.expected_provenance)
            .or(spec.expected_tooling_provenance),
        python_install_provenance: python_install_authority.observed_provenance,
        expected_python_install_provenance: python_install_authority.expected_provenance,
        install_authority_status: tooling_authority
            .map(|a| a.status)
            .unwrap_or(python_install_authority.status),
        canonical_update_authority: python_cli_policy(spec.id).map(|policy| {
            PythonCliUpdateAuthorityView {
                observed_payload_provenance: python_install_authority
                    .observed_provenance
                    .map(format_python_provenance),
                expected_payload_provenance: python_install_authority
                    .expected_provenance
                    .map(format_python_provenance),
                canonical_update_command: policy.canonical_update_command.to_string(),
                update_authority_note: policy.update_authority_note.to_string(),
                normalizable_now: python_cli_normalizable_now(layout, policy.dep_id),
                requires_uv: python_cli_requires_uv(policy.dep_id),
                requires_runtime_principal: python_cli_requires_runtime_principal(
                    layout,
                    policy.dep_id,
                ),
                can_auto_apply: python_cli_can_auto_apply(layout, policy.dep_id),
            }
        }),
        pin: pin_view,
        watch,
    }
}

fn python_cli_normalizable_now(layout: &PrincipalInstallLayout, dep_id: &str) -> bool {
    if !python_cli_requires_uv(dep_id) {
        return false;
    }
    if !python_cli_uv_in_runtime_layout(layout) {
        return false;
    }
    if python_cli_requires_runtime_principal(layout, dep_id) {
        return layout.writable;
    }
    true
}

fn python_cli_uv_in_runtime_layout(layout: &PrincipalInstallLayout) -> bool {
    let uv = resolve_binary("uv", layout);
    uv.path.is_some() && uv.under_principal
}

fn python_cli_requires_uv(dep_id: &str) -> bool {
    python_cli_policy(dep_id)
        .map(|policy| policy.canonical_update_argv.first() == Some(&"uv"))
        .unwrap_or(false)
}

fn python_cli_requires_runtime_principal(layout: &PrincipalInstallLayout, dep_id: &str) -> bool {
    python_cli_policy(dep_id)
        .map(|policy| {
            plan_for_dep(dep_id, "policy-check", layout)
                .map(|plan| {
                    plan.steps.iter().any(|step| step.as_principal)
                        || matches!(plan.principal, InstallPrincipalKind::RuntimeUser)
                })
                .unwrap_or(false)
                || policy.dep_id == "huggingface-cli"
        })
        .unwrap_or(false)
}

fn python_cli_can_auto_apply(layout: &PrincipalInstallLayout, dep_id: &str) -> bool {
    python_cli_policy(dep_id)
        .and_then(|_| plan_for_dep(dep_id, "policy-check", layout))
        .map(|plan| plan.auto_applicable)
        .unwrap_or(false)
}

fn format_python_provenance(
    provenance: cynic_host::domain::ecosystem_package::PythonInstallProvenance,
) -> String {
    match provenance {
        cynic_host::domain::ecosystem_package::PythonInstallProvenance::UvTool => "uv_tool",
        cynic_host::domain::ecosystem_package::PythonInstallProvenance::UvVenv => "uv_venv",
        cynic_host::domain::ecosystem_package::PythonInstallProvenance::VirtualEnv => "virtual_env",
        cynic_host::domain::ecosystem_package::PythonInstallProvenance::Pipx => "pipx",
        cynic_host::domain::ecosystem_package::PythonInstallProvenance::PipUserSite => {
            "pip_user_site"
        }
        cynic_host::domain::ecosystem_package::PythonInstallProvenance::SystemPython => {
            "system_python"
        }
        cynic_host::domain::ecosystem_package::PythonInstallProvenance::Unknown => "unknown",
    }
    .to_string()
}

fn mode_from_channel(channel: ProviderChannel) -> crate::deps_surface_model::UpgradeMode {
    match channel {
        ProviderChannel::Scripted => crate::deps_surface_model::UpgradeMode::Scripted,
        ProviderChannel::External => crate::deps_surface_model::UpgradeMode::External,
        ProviderChannel::DistroManaged => crate::deps_surface_model::UpgradeMode::DistroManaged,
    }
}

fn channel_for_dep(id: &str) -> ProviderChannel {
    provider_spec(id)
        .map(|spec| spec.channel)
        .unwrap_or(ProviderChannel::External)
}

fn principal_for_dep(id: &str) -> InstallPrincipalKind {
    provider_spec(id)
        .map(|spec| spec.principal)
        .unwrap_or(InstallPrincipalKind::OperatorLocal)
}

fn evaluate_status(id: &str, version: Option<&str>, pin: Option<&PinEntry>) -> String {
    let Some(pin) = pin else {
        return if version.is_some() {
            "observed".into()
        } else {
            "unknown".into()
        };
    };

    if let Some(capability) = &pin.min_capability {
        return match version {
            Some(value) if value.contains(capability) || value.contains("inference-start") => {
                "ok".into()
            }
            Some(_) => "stale".into(),
            None => "unknown".into(),
        };
    }

    if let Some(min_build) = pin.min_build {
        let Some(value) = version else {
            return "unknown".into();
        };
        let Some(observed_build) = extract_build_number(value) else {
            return "unknown".into();
        };
        if observed_build < min_build {
            return "stale".into();
        }
        if let Some(desired_build) = pin.desired_build {
            if observed_build < desired_build {
                return "desired_upgrade".into();
            }
        }
        return "ok".into();
    }

    if let Some(min_version) = &pin.min_version {
        let Some(value) = version else {
            return "unknown".into();
        };
        return match cmp_loose_version(value, min_version) {
            Some(std::cmp::Ordering::Less) => "stale".into(),
            Some(_) => "ok".into(),
            None => "unknown".into(),
        };
    }

    let _ = id;
    if version.is_some() {
        "ok".into()
    } else {
        "observed".into()
    }
}

fn extract_build_number(s: &str) -> Option<u64> {
    for part in s.split(|c: char| !c.is_ascii_digit()) {
        if part.len() >= 3 && part.len() <= 6 {
            if let Ok(value) = part.parse::<u64>() {
                if value >= 100 {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn cmp_loose_version(observed: &str, min: &str) -> Option<std::cmp::Ordering> {
    let observed = parse_version_triple(observed)?;
    let min = parse_version_triple(min)?;
    Some(observed.cmp(&min))
}

fn parse_version_triple(s: &str) -> Option<(u64, u64, u64)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            let slice = &s[start..i];
            let mut parts = slice.split('.').filter_map(|part| part.parse::<u64>().ok());
            let major = parts.next()?;
            let minor = parts.next().unwrap_or(0);
            let patch = parts.next().unwrap_or(0);
            return Some((major, minor, patch));
        }
        i += 1;
    }
    None
}

fn read_version(spec: &DepSpec, path: &str) -> Option<String> {
    match spec.version_from {
        VersionFrom::HelpPresence => {
            let output = Command::new(path).args(spec.version_args).output().ok()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("inference-start") {
                Some("cynic-cli+inference-start".into())
            } else if output.status.success() {
                Some("present-old-surface".into())
            } else {
                Some("present".into())
            }
        }
        VersionFrom::StdoutFirstLine => {
            let output = Command::new(path).args(spec.version_args).output().ok()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            let line = stdout
                .lines()
                .next()
                .or_else(|| stderr.lines().next())
                .unwrap_or("")
                .trim();
            if line.is_empty() {
                None
            } else {
                Some(line.chars().take(120).collect())
            }
        }
        VersionFrom::JsonField(field) => {
            let output = Command::new(path).args(spec.version_args).output().ok()?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            serde_json::from_str::<serde_json::Value>(&stdout)
                .ok()
                .and_then(|value| value.get(field)?.as_str().map(|s| s.to_string()))
                .or_else(|| stdout.lines().next().map(|line| line.trim().to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        PrincipalInstallLayout, python_cli_normalizable_now, python_cli_uv_in_runtime_layout,
    };
    use cynic_host::domain::ecosystem_package::InstallPrincipalKind;
    use std::fs::File;
    use std::path::PathBuf;

    fn runtime_layout(home: PathBuf, writable: bool) -> PrincipalInstallLayout {
        PrincipalInstallLayout {
            principal: InstallPrincipalKind::RuntimeUser,
            runtime_user: "cynic".into(),
            home_dir: home.clone(),
            bin_dir: home.join(".local/bin"),
            src_dir: home.join(".local/share/cynic/src"),
            share_dir: home.join(".local/share/cynic"),
            run_as: cynic_host::domain::ecosystem_package::RunAs::Current,
            writable,
            notes: vec![],
        }
    }

    #[test]
    fn python_cli_uv_in_runtime_layout_checks_principal_uv_binary() {
        let home = std::env::temp_dir().join("cynic-test-uv-runtime-home-1");
        let _ = std::fs::remove_dir_all(&home);
        let layout = runtime_layout(home.clone(), true);
        std::fs::create_dir_all(&layout.bin_dir).unwrap();
        assert!(!python_cli_uv_in_runtime_layout(&layout));
        File::create(layout.bin_dir.join("uv")).unwrap();
        assert!(python_cli_uv_in_runtime_layout(&layout));
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn python_cli_normalizable_now_for_hermes_requires_writable_and_runtime_uv() {
        let home = std::env::temp_dir().join("cynic-test-uv-runtime-home-2");
        let _ = std::fs::remove_dir_all(&home);
        let mut layout = runtime_layout(home.clone(), true);
        std::fs::create_dir_all(&layout.bin_dir).unwrap();
        File::create(layout.bin_dir.join("uv")).unwrap();
        assert!(python_cli_normalizable_now(&layout, "hermes-agent"));

        layout.writable = false;
        assert!(!python_cli_normalizable_now(&layout, "hermes-agent"));
        let _ = std::fs::remove_dir_all(&home);
    }
}
