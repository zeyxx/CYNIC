pub mod model;
pub mod probe;
pub mod render;
pub mod specs;
pub mod upgrade;

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::cli_json::emit_pretty_json;
use crate::dependency::model::{
    DepEntry, DepsAuditFinding, DepsAuditReport, DepsAuditSummary, DepsPlanReport, DepsReport,
    DepsSummary, FrictionlessHints, PinEntry, PlanAction, PrincipalLayoutView, PythonToolingView,
    ToolingAuthorityView, load_pins, specs,
};
use crate::dependency::probe::probe_and_evaluate;
use crate::dependency::render::{print_audit_human, print_deps_human, print_plan_human};
use crate::dependency::upgrade::execute_deps_upgrade_impl;
use cynic_host::domain::ecosystem_package::{
    PrincipalInstallLayout, ProviderChannel, RunAs, ToolingAuthorityFamily,
    ToolingInstallProvenance, channel_auto_applicable, has_auto_handler, observe_python_tooling,
    plan_for_dep, python_cli_policy, resolve_principal_install_layout,
};

/// `cynic dependency` — human by default, `--json` for agents.
pub(crate) fn execute_deps(json: bool) -> ExitCode {
    let (pins_path, pins) = load_pins();
    let report = build_deps_report(&pins_path, pins, now_ms());
    if json {
        return emit_pretty_json(&report);
    }
    print_deps_human(&report);
    ExitCode::SUCCESS
}

/// `cynic dependency plan` — human by default.
pub(crate) fn execute_deps_plan(json: bool) -> ExitCode {
    let (pins_path, pins) = load_pins();
    let deps = build_deps_report(&pins_path, pins, now_ms());
    let plan = build_plan(&deps, now_ms());
    if json {
        return emit_pretty_json(&plan);
    }
    print_plan_human(&plan);
    ExitCode::SUCCESS
}

/// `cynic dependency audit` — prioritized normalization cockpit.
pub(crate) fn execute_deps_audit(json: bool) -> ExitCode {
    let (pins_path, pins) = load_pins();
    let deps = build_deps_report(&pins_path, pins, now_ms());
    let audit = build_deps_audit(&deps);
    if json {
        return emit_pretty_json(&audit);
    }
    print_audit_human(&audit);
    ExitCode::SUCCESS
}

pub(crate) fn execute_deps_upgrade(
    json: bool,
    execute_with_apply: bool,
    yes: bool,
    confirm: Option<&str>,
    only: &[String],
    source: Option<&str>,
) -> ExitCode {
    execute_deps_upgrade_impl(
        json,
        execute_with_apply,
        yes,
        confirm,
        only,
        source,
        now_ms(),
        emit_pretty_json,
    )
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

pub(crate) fn build_deps_report(
    pins_path: &Path,
    pins: Vec<PinEntry>,
    observed_at_unix_ms: u64,
) -> DepsReport {
    let pin_map: BTreeMap<String, PinEntry> =
        pins.into_iter().map(|pin| (pin.id.clone(), pin)).collect();
    let layout = resolve_principal_install_layout();
    let python_tooling = observe_python_tooling(&layout);

    let deps = specs()
        .into_iter()
        .map(|spec| probe_and_evaluate(&spec, pin_map.get(spec.id), &layout))
        .collect::<Vec<_>>();

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

fn summarize(deps: &[DepEntry]) -> DepsSummary {
    let mut summary = DepsSummary {
        observed: 0, missing: 0, stale: 0, desired_upgrade: 0, noncanonical: 0, ok: 0, required_blocked: 0,
    };
    for dep in deps {
        match dep.status.as_str() {
            "missing" => {
                summary.missing += 1;
                if dep.tier == "required" { summary.required_blocked += 1; }
            }
            "stale" => {
                summary.stale += 1;
                if dep.tier == "required" { summary.required_blocked += 1; }
            }
            "desired_upgrade" => summary.desired_upgrade += 1,
            "ok" => summary.ok += 1,
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

fn next_steps(deps: &[DepEntry], summary: &DepsSummary, layout: &PrincipalInstallLayout) -> Vec<String> {
    let mut next = Vec::new();
    if summary.required_blocked > 0 {
        next.push("cynic deps plan  # ordered fix actions".into());
    }
    next.push("cynic deps upgrade --yes  # apply scripted + external providers".into());
    next.push("cynic models start <alias> --yes --json".into());
    if deps.iter().any(|d| d.id == "ods" && d.status == "missing") {
        next.push("Optional ODS: cynic deps upgrade --yes --only ods  then inference-discover --json [--apply]".into());
    }
    if deps.iter().any(|d| d.id == "llama.cpp" && (d.status == "stale" || d.status == "desired_upgrade")) {
        next.push("llama.cpp: cynic deps upgrade --yes --only llama.cpp  (principal-local; newer GGUF arches)".into());
    }
    if !layout.writable {
        next.push(format!("principal `{}` not writable — enable sudo -u {} or run as that user", layout.runtime_user, layout.runtime_user));
    }
    if deps.iter().any(|d| d.install_authority_status == "noncanonical") {
        next.push("Normalize noncanonical install authorities: cynic deps plan".into());
    }
    if deps.iter().any(|d| d.id == "uv" && d.status == "missing") {
        next.push("uv missing: install uv first, then re-run cynic deps and normalize Python CLI tools via uv tool".into());
    }
    for dep in deps {
        if let Some(policy) = python_cli_policy(dep.id) {
            if dep.status != "missing" {
                next.push(format!("{} canonical update authority: {}", dep.id, policy.canonical_update_command));
            }
            if dep.install_authority_status == "noncanonical" {
                next.push(format!("{} authority mismatch: normalize via {}", dep.id, policy.canonical_update_command));
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

fn tooling_authorities(deps: &[DepEntry]) -> Vec<ToolingAuthorityView> {
    deps.iter()
        .filter_map(|dep| {
            let family = dep.tooling_authority_family?;
            Some(ToolingAuthorityView {
                dep_id: dep.id.to_string(),
                family: match family {
                    ToolingAuthorityFamily::Python => "python",
                    ToolingAuthorityFamily::Node => "node",
                    ToolingAuthorityFamily::Brew => "brew",
                    ToolingAuthorityFamily::Rustup => "rustup",
                }.into(),
                observed_provenance: dep.tooling_install_provenance.map(tooling_provenance_name).map(str::to_string),
                expected_provenance: dep.expected_tooling_install_provenance.map(tooling_provenance_name).map(str::to_string),
                status: dep.install_authority_status.clone(),
                path: dep.path.clone(),
                under_principal: dep.under_principal,
            })
        })
        .collect()
}

fn tooling_provenance_name(p: ToolingInstallProvenance) -> &'static str {
    match p {
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
        _ => "unknown",
    }
}

pub(crate) fn build_plan(deps: &DepsReport, observed_at_unix_ms: u64) -> DepsPlanReport {
    let layout = resolve_principal_install_layout();
    let mut actions = Vec::new();
    let mut priority = 10u32;

    for dep in &deps.deps {
        let needs = matches!(dep.status.as_str(), "missing" | "stale" | "desired_upgrade")
            || dep.install_authority_status == "noncanonical";
        if !needs { continue; }

        let action_priority = if dep.tier == "required" { priority } else if dep.tier == "recommended" { priority + 100 } else { priority + 200 };
        priority += 1;

        let reason = match dep.status.as_str() {
            "missing" => format!("{} missing from PATH / principal bin", dep.id),
            "stale" => format!("{} below pin ({:?})", dep.id, dep.pin.as_ref().and_then(|p| p.min_version.clone().or_else(|| p.min_build.map(|b| b.to_string())))),
            "desired_upgrade" => format!("{} at minimum but below desired (e.g. newer GGUF support)", dep.id),
            _ if dep.install_authority_status == "noncanonical" => format!("{} tooling authority is noncanonical ({:?} observed, {:?} expected)", dep.id, dep.tooling_install_provenance, dep.expected_tooling_install_provenance),
            other => other.to_string(),
        };

        let (commands, auto_applicable) = if let Some(plan) = plan_for_dep(dep.id, &reason, &layout) {
            (plan.summary_commands, plan.auto_applicable)
        } else {
            let auto = channel_auto_applicable(dep.channel) && dep.id != "systemd" && has_auto_handler(dep.id);
            (dep.watch.update_commands.clone(), auto)
        };

        actions.push(PlanAction {
            priority: action_priority,
            dep_id: dep.id.to_string(),
            reason,
            commands,
            mutates: true,
            upgrade_mode: dep.upgrade_mode,
            channel: dep.channel,
            install_principal: dep.install_principal,
            auto_applicable,
        });
    }
    actions.sort_by_key(|action| action.priority);

    let mut copy_paste = vec!["set -euo pipefail".into(), "# Prefer: cynic deps upgrade --yes".into()];
    for action in &actions {
        copy_paste.push(format!("# --- {} [{} / {:?}] : {} ---", action.dep_id, match action.channel {
            ProviderChannel::Scripted => "scripted",
            ProviderChannel::External => "external",
            ProviderChannel::DistroManaged => "distro_managed",
        }, action.install_principal, action.reason));
        copy_paste.extend(action.commands.clone());
    }
    copy_paste.push("cynic deps".into());

    DepsPlanReport {
        schema: "cynic.ecosystem_deps_plan.v1",
        surface: "tech_watch",
        observed_at_unix_ms,
        pins_path: deps.pins_path.clone(),
        actions,
        copy_paste,
        frictionless: FrictionlessHints {
            after_cli_change: "cynic deps upgrade --yes --only cynic",
            daily_check: "cynic deps",
            model_start: "cynic models start <alias> --yes --json",
            pins_edit: "edit infra/ecosystem-pins.json then: cynic deps",
        },
    }
}

pub(crate) fn build_deps_audit(report: &DepsReport) -> DepsAuditReport {
    let mut findings = report.deps.iter().filter_map(|dep| {
        let auth = dep.canonical_update_authority.as_ref()?;
        let (class, priority) = if auth.normalizable_now && auth.can_auto_apply {
            ("normalizable_now", 10)
        } else if auth.requires_uv && !auth.normalizable_now {
            ("blocked_missing_uv", 20)
        } else if auth.requires_runtime_principal && !auth.normalizable_now {
            ("blocked_runtime_principal", 30)
        } else if !auth.can_auto_apply {
            ("blocked_non_auto_apply", 40)
        } else {
            ("deferred", 50)
        };
        Some(DepsAuditFinding {
            dep_id: dep.id.to_string(),
            status: dep.status.clone(),
            priority,
            class,
            canonical_update_command: auth.canonical_update_command.clone(),
            observed_payload_provenance: auth.observed_payload_provenance.clone(),
            expected_payload_provenance: auth.expected_payload_provenance.clone(),
            normalizable_now: auth.normalizable_now,
            requires_uv: auth.requires_uv,
            requires_runtime_principal: auth.requires_runtime_principal,
            can_auto_apply: auth.can_auto_apply,
            note: auth.update_authority_note.clone(),
        })
    }).collect::<Vec<_>>();

    findings.sort_by_key(|f| (f.priority, f.dep_id.clone()));

    let summary = DepsAuditSummary {
        total_python_cli_authorities: findings.len(),
        normalizable_now: findings.iter().filter(|f| f.class == "normalizable_now").count(),
        blocked_missing_uv: findings.iter().filter(|f| f.class == "blocked_missing_uv").count(),
        blocked_runtime_principal: findings.iter().filter(|f| f.class == "blocked_runtime_principal").count(),
        blocked_non_auto_apply: findings.iter().filter(|f| f.class == "blocked_non_auto_apply").count(),
    };

    let status = if summary.blocked_missing_uv > 0 || summary.blocked_runtime_principal > 0 {
        "attention_required"
    } else if summary.normalizable_now > 0 {
        "actionable"
    } else {
        "ok"
    };

    DepsAuditReport {
        schema: "cynic.ecosystem_deps_audit.v1",
        surface: "tech_watch",
        status,
        summary,
        findings,
        rules: vec![
            "audit classifies executable normalization state from canonical authority views",
            "payload provenance and update mutator remain distinct facts",
            "normalizable_now means the canonical command can be attempted under current local constraints",
        ],
    }
}
