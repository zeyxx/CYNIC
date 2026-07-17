//! Update lifecycle CLI — thin adapter over cynic-host::update_lifecycle.
//!
//! Business logic, policy tables, and observation live in the host crate.
//! This surface only: clap parse → inject LocalArgvRunner → print.
//!
//! ```text
//! cynic lifecycle status|plan|gate|promote
//! cynic lifecycle report          # end operational black box
//! cynic lifecycle repair [--yes]  # phase-filtered repairs
//! cynic lifecycle apply [--yes]   # phase-allowed mutations
//! ```

use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::update_lifecycle::{
    ApplyReport, LocalArgvRunner, OpsReport, RepairReport, apply_phase, default_state_path,
    ops_report, plan_report, promote, repair_apply, repair_plan, run_gates, status_report,
};
use serde::Serialize;

fn runner() -> LocalArgvRunner {
    LocalArgvRunner
}

#[derive(Debug, Subcommand)]
pub(crate) enum LifecycleCommand {
    /// Current phase, policy, promotion eligibility.
    Status {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        state: Option<String>,
    },
    /// Planned gates and allowed mutations for current phase.
    Plan {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        state: Option<String>,
    },
    /// Run (or plan) promotion gates for the current phase.
    Gate {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        state: Option<String>,
    },
    /// Promote one phase forward after a green gate.
    Promote {
        #[arg(long)]
        json: bool,
        #[arg(long = "to")]
        to: Option<String>,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        state: Option<String>,
    },
    /// Operational observation report (findings + repair intents) — ends black box.
    Report {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        state: Option<String>,
    },
    /// Plan or apply repairs for open findings (phase-filtered).
    Repair {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        state: Option<String>,
    },
    /// Apply phase-allowed mutations (deps upgrade, inference discover, …).
    Apply {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        state: Option<String>,
    },
}

pub(crate) fn execute_lifecycle(command: LifecycleCommand) -> ExitCode {
    match command {
        LifecycleCommand::Status { json, state } => {
            let path = state_path(state);
            let report = status_report(&path);
            if json {
                return emit(&report);
            }
            print_status_human(&report);
            ExitCode::SUCCESS
        }
        LifecycleCommand::Plan { json, state } => {
            let path = state_path(state);
            let report = plan_report(&path);
            if json {
                return emit(&report);
            }
            print_plan_human(&report);
            ExitCode::SUCCESS
        }
        LifecycleCommand::Gate { json, yes, state } => {
            let path = state_path(state);
            let report = run_gates(&path, yes, &runner());
            if json {
                let code = emit(&report);
                return if report.status == "ok" || report.status == "planned" {
                    code
                } else {
                    ExitCode::FAILURE
                };
            }
            print_gate_human(&report, yes);
            if report.status == "blocked" {
                ExitCode::FAILURE
            } else {
                ExitCode::SUCCESS
            }
        }
        LifecycleCommand::Promote {
            json,
            to,
            yes,
            state,
        } => {
            let path = state_path(state);
            match promote(&path, to.as_deref(), yes) {
                Ok(report) => {
                    if json {
                        let code = emit(&report);
                        return if report.status == "ok" || report.status == "planned" {
                            code
                        } else {
                            ExitCode::FAILURE
                        };
                    }
                    print_promote_human(&report);
                    if report.status == "ok" || report.status == "planned" {
                        ExitCode::SUCCESS
                    } else {
                        ExitCode::FAILURE
                    }
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        LifecycleCommand::Report { json, state } => {
            let path = state_path(state);
            let report = ops_report(&path, &runner());
            if json {
                let code = emit(&report);
                return if report.status == "ok" {
                    code
                } else {
                    ExitCode::FAILURE
                };
            }
            print_ops_human(&report);
            if report.status == "ok" {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        LifecycleCommand::Repair { json, yes, state } => {
            let path = state_path(state);
            let r = runner();
            let report = if yes {
                repair_apply(&path, true, &r)
            } else {
                repair_plan(&path, &r)
            };
            if json {
                let code = emit(&report);
                return exit_for_repair(&report, code);
            }
            print_repair_human(&report, yes);
            exit_for_repair(&report, ExitCode::SUCCESS)
        }
        LifecycleCommand::Apply { json, yes, state } => {
            let path = state_path(state);
            let report = apply_phase(&path, yes, &runner());
            if json {
                let code = emit(&report);
                return exit_for_apply(&report, code);
            }
            print_apply_human(&report, yes);
            exit_for_apply(&report, ExitCode::SUCCESS)
        }
    }
}

fn exit_for_repair(report: &RepairReport, ok_code: ExitCode) -> ExitCode {
    match report.status {
        "ok" | "planned" => ok_code,
        _ => ExitCode::FAILURE,
    }
}

fn exit_for_apply(report: &ApplyReport, ok_code: ExitCode) -> ExitCode {
    match report.status {
        "ok" | "planned" => ok_code,
        _ => ExitCode::FAILURE,
    }
}

fn state_path(override_path: Option<String>) -> std::path::PathBuf {
    override_path
        .map(std::path::PathBuf::from)
        .unwrap_or_else(default_state_path)
}

fn emit(value: &impl Serialize) -> ExitCode {
    match serde_json::to_string_pretty(value) {
        Ok(s) => {
            println!("{s}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("json: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_status_human(r: &cynic_host::update_lifecycle::LifecycleStatusReport) {
    println!("CYNIC update lifecycle");
    println!("  state:  {}", r.state_path);
    println!(
        "  phase:  {}  (authority={:?})",
        r.state.current_phase.as_str(),
        r.policy.harness.authority
    );
    println!(
        "  next:   {}",
        r.next_phase.unwrap_or("(production — cycle complete)")
    );
    println!(
        "  gate:   last_ok={:?}  can_promote={}",
        r.state.last_gate_ok, r.can_promote
    );
    if let Some(b) = &r.promote_blocked_reason {
        println!("  blocked promote: {b}");
    }
    println!("\n  progression:");
    for p in &r.progression {
        let mark = if *p == r.state.current_phase.as_str() {
            "*"
        } else {
            " "
        };
        println!("   {mark} {p}");
    }
    println!("\n  allowed mutations:");
    for m in &r.policy.allowed_mutations {
        println!("   · {m:?}");
    }
    println!("\nNext:");
    println!("  cynic lifecycle report          # open the black box");
    println!("  cynic lifecycle repair          # plan fixes");
    println!("  cynic lifecycle repair --yes    # apply phase-safe fixes");
    println!("  cynic lifecycle gate --yes");
    println!("  cynic lifecycle plan");
    if r.can_promote {
        if let Some(n) = r.next_phase {
            println!("  cynic lifecycle promote --to {n} --yes");
        }
    }
    println!("  JSON: cynic lifecycle status --json");
}

fn print_plan_human(r: &cynic_host::update_lifecycle::LifecyclePlanReport) {
    println!("CYNIC lifecycle plan  phase={}\n", r.phase);
    for a in &r.planned_actions {
        let tag = if a.mutates { "MUTATE" } else { "read  " };
        let yes = if a.requires_yes { " [--yes]" } else { "" };
        println!("  [{tag}] {}{yes}", a.command);
        println!("          id={}", a.id);
    }
    println!("\nFrictionless:");
    for h in &r.human {
        println!("  {h}");
    }
}

fn print_gate_human(r: &cynic_host::update_lifecycle::LifecycleGateReport, executed: bool) {
    println!(
        "CYNIC lifecycle gate  phase={}  status={}\n",
        r.phase, r.status
    );
    if !executed {
        println!("  (plan only — re-run with --yes to execute)\n");
    }
    for g in &r.results {
        let mark = if !g.ran {
            "·"
        } else if g.ok {
            "ok"
        } else {
            "FAIL"
        };
        let req = if g.required { "required" } else { "optional" };
        println!("  [{mark}] {} ({req})  {}", g.id, g.command);
        println!("         {}", g.detail);
    }
    println!("\nNext:");
    for n in &r.next {
        println!("  {n}");
    }
}

fn print_promote_human(r: &cynic_host::update_lifecycle::LifecyclePromoteReport) {
    println!("CYNIC lifecycle promote");
    println!("  status: {}", r.status);
    println!("  {} → {}", r.from, r.to.unwrap_or("(none)"));
    println!("  {}", r.message);
    if r.status == "ok" {
        println!("\n  current phase: {}", r.state.current_phase.as_str());
        println!("  re-gate: cynic lifecycle gate --yes");
    }
}

fn print_ops_human(r: &OpsReport) {
    println!("CYNIC operational report");
    println!(
        "  phase={}  authority={}  status={}  black_box={}",
        r.phase, r.authority, r.status, r.black_box
    );
    println!(
        "  findings: info={} warn={} error={} critical={} repairable={}",
        r.finding_counts.info,
        r.finding_counts.warning,
        r.finding_counts.error,
        r.finding_counts.critical,
        r.finding_counts.repairable
    );
    println!();
    for f in &r.findings {
        if matches!(
            f.severity,
            cynic_host::update_lifecycle::FindingSeverity::Info
        ) && f.repair_command.is_none()
        {
            continue; // keep human view signal-dense
        }
        let sev = format!("{:?}", f.severity).to_lowercase();
        let repair = if f.phase_allows_repair {
            f.repair_command.as_deref().unwrap_or("-")
        } else {
            "(not allowed in phase)"
        };
        println!("  [{sev}] {}  {}", f.surface, f.summary);
        println!("         repair: {repair}");
    }
    println!("\nNext:");
    for n in &r.next {
        println!("  • {n}");
    }
}

fn print_repair_human(r: &RepairReport, applied: bool) {
    println!(
        "CYNIC lifecycle repair  phase={}  status={}\n",
        r.phase, r.status
    );
    println!("  {}\n", r.message);
    if !applied {
        println!("  (plan only — re-run with --yes)\n");
    }
    for a in &r.actions {
        let tag = if a.mutates { "MUTATE" } else { "read  " };
        println!("  [{tag}] {}", a.command);
        println!("          id={}  {}", a.id, a.purpose);
    }
    if !r.results.is_empty() {
        println!("\nResults:");
        for res in &r.results {
            println!(
                "  {}  {}  {}",
                if res.ok { "ok  " } else { "FAIL" },
                res.id,
                res.detail.chars().take(120).collect::<String>()
            );
        }
    }
}

fn print_apply_human(r: &ApplyReport, applied: bool) {
    println!(
        "CYNIC lifecycle apply  phase={}  status={}\n",
        r.phase, r.status
    );
    println!("  {}\n", r.message);
    if !applied {
        println!("  (plan only — re-run with --yes)\n");
    }
    for a in &r.actions {
        let tag = if a.mutates { "MUTATE" } else { "read  " };
        println!("  [{tag}] {}", a.command);
    }
    if !r.results.is_empty() {
        println!("\nResults:");
        for res in &r.results {
            println!(
                "  {}  {}  exit={:?}",
                if res.ok { "ok  " } else { "FAIL" },
                res.id,
                res.exit_code
            );
        }
    }
}
