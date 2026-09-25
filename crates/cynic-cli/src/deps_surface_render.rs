use crate::deps_surface_audit::DepsAuditReport;
use crate::deps_surface_model::{DepsPlanReport, DepsReport, UpgradeMode, UpgradeReport};
use cynic_host::domain::ecosystem_package::{
    ProviderChannel, ToolingAuthorityFamily, ToolingInstallProvenance,
};

pub(crate) fn print_deps_human(report: &DepsReport) {
    println!("CYNIC ecosystem deps");
    println!(
        "  pins: {} ({})",
        report.pins_path,
        if report.pins_loaded {
            "loaded"
        } else {
            "missing"
        }
    );
    let principal = &report.runtime_layout;
    println!(
        "  principal: user={}  bin={}  writable={}  run_as={}",
        principal.runtime_user, principal.bin_dir, principal.writable, principal.run_as
    );
    let summary = &report.summary;
    println!(
        "  summary: ok={}  stale={}  desired_upgrade={}  missing={}  noncanonical={}  blocked_required={}\n",
        summary.ok,
        summary.stale,
        summary.desired_upgrade,
        summary.missing,
        summary.noncanonical,
        summary.required_blocked
    );
    println!(
        "  {:16} {:<12} {:<14} {:<18} {:<10} {:<16} {:<16} VERSION",
        "STATUS", "TIER", "CHANNEL", "ID", "FAMILY", "OBSERVED", "EXPECTED"
    );
    println!("  {}", "-".repeat(144));
    for dep in &report.deps {
        let mark = match dep.status.as_str() {
            "ok" => " ",
            "desired_upgrade" => "~",
            "stale" | "missing" => "!",
            _ => "?",
        };
        let channel = match dep.channel {
            ProviderChannel::Scripted => "scripted",
            ProviderChannel::External => "external",
            ProviderChannel::DistroManaged => "distro",
        };
        let family = dep
            .tooling_authority_family
            .map(format_tooling_family)
            .unwrap_or("-");
        let observed = dep
            .tooling_install_provenance
            .map(format_tooling_provenance)
            .or_else(|| dep.python_install_provenance.map(format_python_provenance))
            .unwrap_or("-");
        let expected = dep
            .expected_tooling_install_provenance
            .map(format_tooling_provenance)
            .or_else(|| {
                dep.expected_python_install_provenance
                    .map(format_python_provenance)
            })
            .unwrap_or("-");
        println!(
            "{mark} {:16} {:12} {:14} {:18} {:10} {:16} {:16} {}",
            dep.status,
            dep.tier,
            channel,
            dep.id,
            family,
            observed,
            expected,
            dep.version.as_deref().unwrap_or("-")
        );
        if let Some(authority) = &dep.canonical_update_authority {
            println!(
                "  ↳ update_authority payload={} expected={} normalizable_now={} requires_uv={} requires_runtime_principal={} can_auto_apply={} cmd={}",
                authority
                    .observed_payload_provenance
                    .as_deref()
                    .unwrap_or("-"),
                authority
                    .expected_payload_provenance
                    .as_deref()
                    .unwrap_or("-"),
                authority.normalizable_now,
                authority.requires_uv,
                authority.requires_runtime_principal,
                authority.can_auto_apply,
                authority.canonical_update_command
            );
        }
    }
    if !report.next.is_empty() {
        println!("\nNext:");
        for next in &report.next {
            println!("  • {next}");
        }
    }
    println!("\nOne-click upgrades:  cynic deps upgrade --yes");
    println!("Full plan:           cynic deps plan");
    println!("JSON:                cynic deps --json");
}

pub(crate) fn print_plan_human(plan: &DepsPlanReport) {
    println!("CYNIC deps upgrade plan\n");
    if plan.actions.is_empty() {
        println!("  (nothing to do — all pins satisfied or optional missing)");
    }
    for action in &plan.actions {
        let mode = if action.auto_applicable {
            "ONE-CLICK"
        } else {
            match action.upgrade_mode {
                UpgradeMode::DistroManaged => "DISTRO  ",
                UpgradeMode::External => "EXTERNAL",
                UpgradeMode::Scripted => "BLOCKED ",
            }
        };
        println!(
            "  [{mode}] {} ({:?}) — {}",
            action.dep_id, action.install_principal, action.reason
        );
        for command in &action.commands {
            println!("           {command}");
        }
        println!();
    }
    println!("Frictionless:");
    println!("  {}", plan.frictionless.daily_check);
    println!("  {}", plan.frictionless.after_cli_change);
    println!("  cynic deps upgrade --yes   # apply all ONE-CLICK providers");
}

pub(crate) fn print_upgrade_human(report: &UpgradeReport, applied: bool) {
    if !applied {
        println!("CYNIC deps upgrade (preview)\n");
        println!("  confirm_token: {}", report.confirm_token);
        println!("  one-click:     cynic deps upgrade --yes\n");
        for candidate in &report.candidates {
            let tag = if candidate.auto_applicable {
                "ONE-CLICK"
            } else {
                "PLAN    "
            };
            println!(
                "  [{tag}] {} ({}) — {}",
                candidate.dep_id, candidate.status, candidate.reason
            );
        }
        if report.candidates.is_empty() {
            println!("  (no pending upgrades)");
        }
        println!();
        for step in &report.human_validation.steps {
            println!("  {step}");
        }
        return;
    }
    println!("\nCYNIC deps upgrade results\n");
    for result in &report.results {
        if !result.attempted {
            continue;
        }
        println!(
            "  {}  {}  {}",
            if result.ok { "ok  " } else { "FAIL" },
            result.dep_id,
            result.message
        );
    }
    println!("\nVerify:  cynic deps");
}

pub(crate) fn print_audit_human(report: &DepsAuditReport) {
    println!(
        "CYNIC deps audit: {} (python_cli={} actionable={} missing_uv={} runtime_principal={} non_auto_apply={})",
        report.status,
        report.summary.total_python_cli_authorities,
        report.summary.normalizable_now,
        report.summary.blocked_missing_uv,
        report.summary.blocked_runtime_principal,
        report.summary.blocked_non_auto_apply,
    );
    for finding in &report.findings {
        println!(
            "- {}: class={} priority={} status={} payload={:?}->{:?} uv={} runtime_principal={} auto_apply={} cmd={}",
            finding.dep_id,
            finding.class,
            finding.priority,
            finding.status,
            finding.observed_payload_provenance,
            finding.expected_payload_provenance,
            finding.requires_uv,
            finding.requires_runtime_principal,
            finding.can_auto_apply,
            finding.canonical_update_command
        );
    }
}

fn format_tooling_family(family: ToolingAuthorityFamily) -> &'static str {
    match family {
        ToolingAuthorityFamily::Python => "python",
        ToolingAuthorityFamily::Node => "node",
        ToolingAuthorityFamily::Brew => "brew",
        ToolingAuthorityFamily::Rustup => "rustup",
    }
}

fn format_tooling_provenance(provenance: ToolingInstallProvenance) -> &'static str {
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
<<<<<<< HEAD
        _ => "unknown",
=======
>>>>>>> github/main
    }
}

fn format_python_provenance(
    provenance: cynic_host::domain::ecosystem_package::PythonInstallProvenance,
) -> &'static str {
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
}
