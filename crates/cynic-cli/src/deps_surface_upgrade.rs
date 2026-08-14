use std::path::PathBuf;
use std::process::ExitCode;

use crate::deps_surface_model::{
    HumanValidation, UpgradeCandidate, UpgradeMode, UpgradeReport, UpgradeResult,
};
use crate::deps_surface_pins::load_pins;
use crate::deps_surface_plan::build_plan;
use crate::deps_surface_render::print_upgrade_human;
use crate::deps_surface_report::build_deps_report;
use cynic_host::domain::ecosystem_package::{
    apply_for_dep, has_auto_handler, resolve_principal_install_layout,
};

pub(crate) fn execute_deps_upgrade(
    json: bool,
    execute_with_apply: bool,
    yes: bool,
    confirm: Option<&str>,
    only: &[String],
    observed_at_unix_ms: u64,
    emit_json: impl Fn(&UpgradeReport) -> ExitCode,
) -> ExitCode {
    let (pins_path, pins) = load_pins();
    let deps = build_deps_report(&pins_path, pins, observed_at_unix_ms);
    let plan = build_plan(&deps, observed_at_unix_ms);
    let mut candidates: Vec<UpgradeCandidate> = plan
        .actions
        .iter()
        .map(|action| UpgradeCandidate {
            dep_id: action.dep_id.clone(),
            status: deps
                .deps
                .iter()
                .find(|dep| dep.id == action.dep_id)
                .map(|dep| dep.status.clone())
                .unwrap_or_else(|| "unknown".into()),
            upgrade_mode: action.upgrade_mode,
            auto_applicable: action.auto_applicable,
            reason: action.reason.clone(),
            commands: action.commands.clone(),
        })
        .collect();

    if !only.is_empty() {
        let mut selected = Vec::new();
        for id in only {
            if let Some(candidate) = candidates.iter().find(|candidate| candidate.dep_id == *id) {
                selected.push(candidate.clone());
                continue;
            }
            if let Some(dep) = deps.deps.iter().find(|dep| dep.id == id.as_str())
                && (has_auto_handler(dep.id) || dep.id == "cynic")
            {
                selected.push(UpgradeCandidate {
                    dep_id: dep.id.to_string(),
                    status: dep.status.clone(),
                    upgrade_mode: dep.upgrade_mode,
                    auto_applicable: true,
                    reason: format!("operator --only refresh ({})", dep.status),
                    commands: dep.watch.update_commands.clone(),
                });
            }
        }
        candidates = selected;
    }

    let token = confirm_token_for(&candidates, observed_at_unix_ms);
    let human = HumanValidation {
        steps: vec![
            "One-click (recommended):  cynic deps upgrade --yes",
            "Or two-step:              cynic deps upgrade   # note token",
            "                          cynic deps upgrade --apply --confirm <token>",
            "Then verify:              cynic deps",
        ],
        apply_command_template: "cynic deps upgrade --yes".to_string(),
    };

    let do_apply = yes || execute_with_apply;
    if !do_apply {
        let report = UpgradeReport {
            schema: "cynic.ecosystem_deps_upgrade.v1",
            surface: "tech_watch",
            mutates: false,
            requested_apply: false,
            confirm_token: token.clone(),
            confirm_required: true,
            human_validation: human,
            candidates,
            results: vec![],
            after: vec![
                "No changes applied.".into(),
                "One-click: cynic deps upgrade --yes".into(),
                format!("Or:        cynic deps upgrade --apply --confirm {token}"),
            ],
        };
        if json {
            return emit_json(&report);
        }
        print_upgrade_human(&report, false);
        return ExitCode::SUCCESS;
    }

    if !yes {
        let Some(provided) = confirm else {
            eprintln!("Need human validation: use --yes  OR  --apply --confirm <token>");
            return ExitCode::FAILURE;
        };
        if provided != token {
            eprintln!("confirm token mismatch — run: cynic deps upgrade");
            return ExitCode::FAILURE;
        }
    }

    if !json {
        println!(
            "Applying scripted upgrades (human consent: {})…\n",
            if yes { "--yes" } else { "--confirm" }
        );
    }

    let mut results = Vec::new();
    let mut any_fail = false;
    let mut any_scripted = false;
    for candidate in &candidates {
        if !candidate.auto_applicable {
            if !json {
                println!(
                    "  skip  {}  ({:?} — not auto-applicable; see cynic deps plan)",
                    candidate.dep_id, candidate.upgrade_mode
                );
            }
            results.push(UpgradeResult {
                dep_id: candidate.dep_id.clone(),
                attempted: false,
                ok: false,
                argv: vec![],
                exit_code: None,
                stdout_tail: String::new(),
                stderr_tail: String::new(),
                message: format!("channel_not_auto:{:?}", candidate.upgrade_mode),
            });
            continue;
        }

        any_scripted = true;
        if !json {
            println!("  run   {}  …", candidate.dep_id);
        }
        match run_scripted_upgrade(&candidate.dep_id) {
            Ok(result) => {
                if !json {
                    println!("        → {}", if result.ok { "ok" } else { "FAILED" });
                }
                if !result.ok {
                    any_fail = true;
                }
                results.push(result);
            }
            Err(error) => {
                any_fail = true;
                if !json {
                    println!("        → error: {error}");
                }
                results.push(UpgradeResult {
                    dep_id: candidate.dep_id.clone(),
                    attempted: true,
                    ok: false,
                    argv: vec![],
                    exit_code: None,
                    stdout_tail: String::new(),
                    stderr_tail: error,
                    message: "execution error".into(),
                });
            }
        }
    }

    if !any_scripted && !json {
        println!(
            "Nothing auto-applicable (distro_managed only, blocked principal, or already ok)."
        );
    }

    let report = UpgradeReport {
        schema: "cynic.ecosystem_deps_upgrade.v1",
        surface: "tech_watch",
        mutates: true,
        requested_apply: true,
        confirm_token: token,
        confirm_required: true,
        human_validation: human,
        candidates,
        results,
        after: vec!["cynic deps".into(), "cynic inference status".into()],
    };
    if json {
        let code = emit_json(&report);
        if any_fail { ExitCode::FAILURE } else { code }
    } else {
        print_upgrade_human(&report, true);
        if any_fail {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }
}

fn confirm_token_for(candidates: &[UpgradeCandidate], observed_at_unix_ms: u64) -> String {
    let mut material = String::new();
    for candidate in candidates {
        material.push_str(&candidate.dep_id);
        material.push(':');
        material.push_str(&candidate.status);
        material.push(':');
        material.push_str(match candidate.upgrade_mode {
            UpgradeMode::Scripted => "scripted",
            UpgradeMode::External => "external",
            UpgradeMode::DistroManaged => "distro_managed",
        });
        material.push('|');
    }
    let day = observed_at_unix_ms / 86_400_000;
    material.push_str(&format!("day:{day}"));
    format!("cynic-up-{:016x}", fnv1a64(material.as_bytes()))
}

fn fnv1a64(data: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for byte in data {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

fn run_scripted_upgrade(dep_id: &str) -> Result<UpgradeResult, String> {
    if dep_id == "cynic" {
        return upgrade_cynic_self();
    }

    let layout = resolve_principal_install_layout();
    let Some(result) = apply_for_dep(dep_id, "deps upgrade --yes", &layout) else {
        return Ok(UpgradeResult {
            dep_id: dep_id.into(),
            attempted: false,
            ok: false,
            argv: vec![],
            exit_code: None,
            stdout_tail: String::new(),
            stderr_tail: String::new(),
            message: "no provider handler".into(),
        });
    };
    let last_argv = result
        .steps_run
        .last()
        .map(|step| step.argv.clone())
        .unwrap_or_default();
    let stdout = result
        .steps_run
        .iter()
        .map(|step| step.stdout_tail.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let stderr = result
        .steps_run
        .iter()
        .map(|step| format!("{}: {}", step.role, step.stderr_tail))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(UpgradeResult {
        dep_id: dep_id.into(),
        attempted: result.attempted,
        ok: result.ok,
        argv: last_argv,
        exit_code: result.steps_run.last().and_then(|step| step.exit_code),
        stdout_tail: tail(&stdout, 1500),
        stderr_tail: tail(&stderr, 1500),
        message: result.message,
    })
}

fn upgrade_cynic_self() -> Result<UpgradeResult, String> {
    let source = std::env::current_exe().map_err(|error| format!("current_exe: {error}"))?;
    let target = default_user_cli_target();
    let report = cynic_host::framework_bootstrap::install_standalone_cli(
        Some(source.to_str().unwrap_or("")),
        Some(target.to_str().unwrap_or("")),
        true,
    )?;
    let ok = matches!(report.status, "installed" | "already_installed");
    Ok(UpgradeResult {
        dep_id: "cynic".into(),
        attempted: true,
        ok,
        argv: report.command,
        exit_code: report.exit_code,
        stdout_tail: report.stdout,
        stderr_tail: report.stderr,
        message: report.status.to_string(),
    })
}

fn default_user_cli_target() -> PathBuf {
    if let Ok(path) = std::env::var("CYNIC_STANDALONE_CLI") {
        return PathBuf::from(path);
    }
    PathBuf::from("/usr/local/bin/cynic")
}

fn tail(s: &str, max: usize) -> String {
    let trimmed = s.trim();
    if trimmed.len() <= max {
        trimmed.to_string()
    } else {
        trimmed[trimmed.len().saturating_sub(max)..].to_string()
    }
}
