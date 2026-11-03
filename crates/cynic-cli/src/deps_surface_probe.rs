use crate::deps_surface_model::{
    DepEntry, DepWatch, PinEntry, PinView, PythonCliUpdateAuthorityView,
};
use cynic_pkg::{
    EcosystemPackageExecutionPort, LocalEcosystemPackageExecution, ProviderChannel,
    observe_python_install_authority, observe_tooling_authority, plan_for_dep, python_cli_policy,
    resolve_binary,
};
use cynic_pkg::evaluate::{
    channel_for_dep, cmp_loose_version, evaluate_status, extract_build_number,
    format_python_provenance, mode_from_channel, parse_version_triple, principal_for_dep,
    python_cli_can_auto_apply, python_cli_normalizable_now, python_cli_requires_runtime_principal,
    python_cli_requires_uv, python_cli_uv_in_runtime_layout,
};

pub(crate) fn probe_and_evaluate(
    spec: &cynic_pkg::specs::DepSpec,
    pin: Option<&PinEntry>,
    layout: &cynic_pkg::types::PrincipalInstallLayout,
) -> DepEntry {
    let executor = LocalEcosystemPackageExecution;
    probe_and_evaluate_with_executor(spec, pin, layout, &executor)
}

fn probe_and_evaluate_with_executor(
    spec: &cynic_pkg::specs::DepSpec,
    pin: Option<&PinEntry>,
    layout: &cynic_pkg::types::PrincipalInstallLayout,
    executor: &dyn EcosystemPackageExecutionPort,
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
        commit: entry.commit.clone(),
        notes: entry.notes.clone(),
    });

    let Some(path) = path else {
        let optional = pin.map(|entry| entry.required == Some(false)).unwrap_or(false);
        let python_install_authority =
            observe_python_install_authority(spec.python_distribution, None, spec.expected_python_provenance);
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
                    requires_runtime_principal: python_cli_requires_runtime_principal(layout, policy.dep_id),
                    can_auto_apply: python_cli_can_auto_apply(layout, policy.dep_id),
                }
            }),
            pin: pin_view,
            watch,
        };
    };

    let version = read_version(spec, &path, executor);
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
                requires_runtime_principal: python_cli_requires_runtime_principal(layout, policy.dep_id),
                can_auto_apply: python_cli_can_auto_apply(layout, policy.dep_id),
            }
        }),
        pin: pin_view,
        watch,
    }
}

fn read_version(
    spec: &cynic_pkg::specs::DepSpec,
    path: &str,
    executor: &dyn EcosystemPackageExecutionPort,
) -> Option<String> {
    let argv = std::iter::once(path.to_string())
        .chain(spec.version_args.iter().map(|arg| (*arg).to_string()))
        .collect::<Vec<_>>();
    let output = executor.execute(&argv, None);
    if output.spawn_error.is_some() {
        return None;
    }
    let version = match spec.version_from {
        cynic_pkg::specs::VersionFrom::HelpPresence => {
            let stdout = output.stdout.as_str();
            if stdout.contains("inference-start") {
                Some("cynic-cli+inference-start".into())
            } else if output.success() {
                Some("present-old-surface".into())
            } else {
                Some("present".into())
            }
        }
        cynic_pkg::specs::VersionFrom::StdoutFirstLine => {
            let line = output
                .stdout
                .lines()
                .next()
                .or_else(|| output.stderr.lines().next())
                .unwrap_or("")
                .trim();
            if line.is_empty() {
                None
            } else {
                Some(line.chars().take(120).collect())
            }
        }
        cynic_pkg::specs::VersionFrom::JsonField(field) => serde_json::from_str::<serde_json::Value>(&output.stdout)
            .ok()
            .and_then(|value| value.get(field)?.as_str().map(|s| s.to_string()))
            .or_else(|| {
                output
                    .stdout
                    .lines()
                    .next()
                    .map(|line| line.trim().to_string())
            }),
    };
    // Commit-pinned native binaries (llama-server --version reports
    // "unknown"): the installer writes `<binary>.commit` next to the
    // binary; the pinned commit travels inside the version string.
    if spec.id == "llama.cpp" {
        if let Ok(raw) = std::fs::read_to_string(format!("{path}.commit")) {
            let commit = raw.trim().to_string();
            if !commit.is_empty() {
                let base = version.unwrap_or_else(|| "unknown".to_string());
                return Some(format!("{base} commit:{commit}"));
            }
        }
    }
    version
}
