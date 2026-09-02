//! CLI adapter for build artefact observation and GC (thin projection).
//!
//! Mirrors `runtime_surface::disk` — clap parse happens in `RuntimeCommand`,
//! this file only injects `LocalBuildArtefactAdapter` and prints JSON/human.

use std::process::ExitCode;

use cynic_host::adapters::local_build_artefact::LocalBuildArtefactAdapter;
use cynic_host::domain::build_artefact::port::{
    BuildArtefactGcExecutionPort, BuildArtefactObservationPort,
};

use crate::emit_json;

pub(crate) fn execute_build_artefact(json: bool, apply: bool) -> ExitCode {
    let adapter = LocalBuildArtefactAdapter;
    let report = match adapter.observe_build_artefacts() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Build artefact observation failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    if apply {
        let plan = adapter.plan_gc(&report);
        if plan.items.is_empty() {
            if json {
                return emit_json(&plan, false);
            }
            println!("No reclaimable build artefacts — nothing to GC.");
            return ExitCode::SUCCESS;
        }
        let result = adapter.execute_gc(&plan);
        if json {
            return emit_json(&result, !result.success);
        }
        print_gc_result_human(&result);
        if result.success {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    } else if json {
        // plan view by default when not applying: show plan as JSON so operators
        // can inspect reclaimable set without mutating. `cynic runtime build-artefact --json --apply` for execution.
        // To get raw report, use explicit --full (not yet) — plan is the operator-facing projection.
        let plan = adapter.plan_gc(&report);
        // emit both? For now plan is the thin projection; report stays available via file if needed.
        // Keep single JSON output: plan when items exist, otherwise report.
        if plan.items.is_empty() {
            emit_json(&report, false)
        } else {
            emit_json(&plan, false)
        }
    } else {
        print_report_human(&report);
        let plan = adapter.plan_gc(&report);
        if !plan.items.is_empty() {
            println!();
            print_gc_plan_human(&plan);
            println!();
            println!("To apply: cynic runtime build-artefact --apply");
        }
        ExitCode::SUCCESS
    }
}

fn print_report_human(report: &cynic_protocol::build_artefact::BuildArtefactReport) {
    println!("════════════════════════════════════════════════════════════");
    println!("  CYNIC BUILD ARTEFACT REPORT — {}", report.timestamp);
    println!("════════════════════════════════════════════════════════════");
    println!();
    println!(
        "Total checkouts: {} ({})",
        report.checkouts.len(),
        report.total_checkout_human
    );
    println!(
        "Pressure: {} avail {} (pressured={}) root-owned={}",
        report.pressure.avail_human,
        report.pressure.use_pct,
        report.pressure.is_pressured,
        report.pressure.root_owned_target_count
    );
    println!();
    if report.checkouts.is_empty() {
        println!("(no checkouts)");
    } else {
        println!("{:<45} {:<10} {:<8} REASON", "PATH", "SIZE", "OWNER");
        println!("{}", "-".repeat(90));
        for c in &report.checkouts {
            let reason = if c.is_orphan { "orphan" } else { "live" };
            let owner = format!("{}:{}", c.owner_uid, c.owner_gid);
            println!(
                "{:<45} {:<10} {:<8} {}",
                c.path, c.size_human, owner, reason
            );
        }
    }
    if !report.residual_unknowns.is_empty() {
        println!();
        println!("Residual unknowns: {}", report.residual_unknowns.join(", "));
    }
}

fn print_gc_plan_human(plan: &cynic_protocol::build_artefact::BuildArtefactGcPlan) {
    println!(
        "── GC Plan ({} reclaimable) ─────────────────────────",
        plan.total_reclaimable_human
    );
    println!("{:<30} {:<10} {:<10} REASON", "ID", "SIZE", "PRIV");
    println!("{}", "-".repeat(80));
    for i in &plan.items {
        println!(
            "{:<30} {:<10} {:<10} {}",
            i.id,
            i.size_human,
            if i.requires_privilege { "sudo" } else { "-" },
            i.reason
        );
    }
    println!("Total reclaimable: {}", plan.total_reclaimable_human);
}

fn print_gc_result_human(result: &cynic_protocol::build_artefact::BuildArtefactGcResult) {
    println!("════════════════════════════════════════════════════════════");
    println!("  CYNIC BUILD ARTEFACT GC — {}", result.timestamp);
    println!("════════════════════════════════════════════════════════════");
    for e in &result.items_executed {
        let st = if e.success { "OK" } else { "FAIL" };
        println!(
            "{:<30} {:<6} code={:?} {}",
            e.item_id, st, e.exit_code, e.label
        );
        if !e.success && !e.stderr_tail.is_empty() {
            println!("  stderr: {}", e.stderr_tail);
        }
    }
    println!(
        "Outcome: {} reclaimed {}",
        if result.success { "SUCCESS" } else { "PARTIAL" },
        result.total_reclaimed_human
    );
}
