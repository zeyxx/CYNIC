use crate::deps_surface_model::{
    DepsReport, DepsSummary, PinEntry, PrincipalLayoutView, PythonToolingView, ToolingAuthorityView,
};
use crate::deps_surface_probe::probe_and_evaluate;
use crate::deps_surface_specs::specs;
use cynic_host::domain::ecosystem_package::{
    PrincipalInstallLayout, RunAs, ToolingAuthorityFamily, ToolingInstallProvenance,
    observe_python_tooling, python_cli_policy, resolve_principal_install_layout,
};
use std::collections::BTreeMap;
use std::path::Path;

pub(crate) fn build_deps_report(
    pins_path: &Path,
    pins: Vec<PinEntry>,
    observed_at_unix_ms: u64,
) -> DepsReport {
    let pin_map: BTreeMap<String, PinEntry> =
        pins.into_iter().map(|pin| (pin.id.clone(), pin)).collect();
    let layout = resolve_principal_install_layout();
    let python_tooling = observe_python_tooling(&layout);

    let mut deps = Vec::new();
    for spec in specs() {
        deps.push(probe_and_evaluate(&spec, pin_map.get(spec.id), &layout));
    }

    let summary = summarize(&deps);
    let next = next_steps(&deps, &summary, &layout);

    DepsReport {
        schema: "cynic.ecosystem_deps.v1",
        surface: "tech_watch",
        observed_at_unix_ms,
        pins_path: pins_path.display().to_string(),
        pins_loaded: !pin_map.is_empty() || pins_path.exists(),
        host_path_note: format!(
            "principal bin {} should be on PATH before /usr/local/bin",
            layout.bin_dir.display()
        ),
        runtime_layout: principal_view(&layout),
        python_tooling: PythonToolingView::from(python_tooling),
        tooling_authorities: tooling_authorities(&deps),
        deps,
        summary,
        next,
    }
}

fn summarize(deps: &[crate::deps_surface_model::DepEntry]) -> DepsSummary {
    let mut summary = DepsSummary {
        observed: 0,
        missing: 0,
        stale: 0,
        desired_upgrade: 0,
        noncanonical: 0,
        ok: 0,
        required_blocked: 0,
    };

    for dep in deps {
        match dep.status.as_str() {
            "missing" => {
                summary.missing += 1;
                if dep.tier == "required" {
                    summary.required_blocked += 1;
                }
            }
            "stale" => {
                summary.stale += 1;
                if dep.tier == "required" {
                    summary.required_blocked += 1;
                }
            }
            "desired_upgrade" => summary.desired_upgrade += 1,
            "ok" => summary.ok += 1,
            "observed" | "unknown" => summary.observed += 1,
            _ => summary.observed += 1,
        }
        if dep.install_authority_status == "noncanonical" {
            summary.noncanonical += 1;
            if dep.tier == "required" || dep.tier == "recommended" {
                summary.required_blocked += 1;
            }
        }
    }

    summary
}

fn next_steps(
    deps: &[crate::deps_surface_model::DepEntry],
    summary: &DepsSummary,
    layout: &PrincipalInstallLayout,
) -> Vec<String> {
    let mut next = Vec::new();
    if summary.required_blocked > 0 {
        next.push("cynic deps plan  # ordered fix actions".into());
    }
    next.push("cynic deps upgrade --yes  # apply scripted + external providers".into());
    next.push(
        "cynic inference-start <alias>  # or: cynic runtime inference-start <alias> --json".into(),
    );
    if deps
        .iter()
        .any(|dep| dep.id == "ods" && dep.status == "missing")
    {
        next.push(
            "Optional ODS: cynic deps upgrade --yes --only ods  then inference-discover --json [--apply]".into(),
        );
    }
    if deps.iter().any(|dep| {
        dep.id == "llama.cpp" && (dep.status == "stale" || dep.status == "desired_upgrade")
    }) {
        next.push(
            "llama.cpp: cynic deps upgrade --yes --only llama.cpp  (principal-local; newer GGUF arches)".into(),
        );
    }
    if !layout.writable {
        next.push(format!(
            "principal `{}` not writable — enable sudo -u {} or run as that user",
            layout.runtime_user, layout.runtime_user
        ));
    }
    if deps
        .iter()
        .any(|dep| dep.install_authority_status == "noncanonical")
    {
        next.push("Normalize noncanonical install authorities: cynic deps plan".into());
    }
    if deps
        .iter()
        .any(|dep| dep.id == "uv" && dep.status == "missing")
    {
        next.push(
            "uv missing: install uv first, then re-run cynic deps and normalize Python CLI tools via uv tool".into(),
        );
    }
    for dep in deps {
        if let Some(policy) = python_cli_policy(dep.id) {
            if dep.status != "missing" {
                next.push(format!(
                    "{} canonical update authority: {}",
                    dep.id, policy.canonical_update_command
                ));
            }
            if dep.install_authority_status == "noncanonical" {
                next.push(format!(
                    "{} authority mismatch: normalize via {}",
                    dep.id, policy.canonical_update_command
                ));
            }
        }
    }
    next.sort();
    next.dedup();
    next
}

fn principal_view(layout: &PrincipalInstallLayout) -> PrincipalLayoutView {
    PrincipalLayoutView {
        runtime_user: layout.runtime_user.clone(),
        home_dir: layout.home_dir.display().to_string(),
        bin_dir: layout.bin_dir.display().to_string(),
        src_dir: layout.src_dir.display().to_string(),
        writable: layout.writable,
        run_as: match &layout.run_as {
            RunAs::Current => "current".into(),
            RunAs::SudoUser(user) => format!("sudo -u {user}"),
        },
        notes: layout.notes.clone(),
    }
}

fn tooling_authorities(deps: &[crate::deps_surface_model::DepEntry]) -> Vec<ToolingAuthorityView> {
    deps.iter()
        .filter_map(|dep| {
            let family = dep.tooling_authority_family?;
            Some(ToolingAuthorityView {
                dep_id: dep.id.to_string(),
                family: tooling_family_name(family).into(),
                observed_provenance: dep
                    .tooling_install_provenance
                    .map(tooling_provenance_name)
                    .map(str::to_string),
                expected_provenance: dep
                    .expected_tooling_install_provenance
                    .map(tooling_provenance_name)
                    .map(str::to_string),
                status: dep.install_authority_status.clone(),
                path: dep.path.clone(),
                under_principal: dep.under_principal,
            })
        })
        .collect()
}

fn tooling_family_name(family: ToolingAuthorityFamily) -> &'static str {
    match family {
        ToolingAuthorityFamily::Python => "python",
        ToolingAuthorityFamily::Node => "node",
        ToolingAuthorityFamily::Brew => "brew",
        ToolingAuthorityFamily::Rustup => "rustup",
    }
}

fn tooling_provenance_name(provenance: ToolingInstallProvenance) -> &'static str {
    match provenance {
        ToolingInstallProvenance::UvTool => "uv_tool",
        ToolingInstallProvenance::UvVenv => "uv_venv",
        ToolingInstallProvenance::VirtualEnv => "virtual_env",
        ToolingInstallProvenance::Pipx => "pipx",
        ToolingInstallProvenance::PipUserSite => "pip_user_site",
        ToolingInstallProvenance::SystemPython => "system_python",
        ToolingInstallProvenance::NpmGlobal => "npm_global",
        ToolingInstallProvenance::HomebrewCellar => "brew_cellar",
        ToolingInstallProvenance::HomebrewCask => "brew_cask",
        ToolingInstallProvenance::RustupToolchain => "rustup_toolchain",
        ToolingInstallProvenance::Unknown => "unknown",
    }
}
