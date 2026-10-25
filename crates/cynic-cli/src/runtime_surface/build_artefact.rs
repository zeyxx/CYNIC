//! CLI adapter for build artefact observation and GC (thin projection).
//!
//! Uses the retention trio: `RetentionObserver` → `RetentionPlanner` → `RetentionExecutor`.

use std::process::ExitCode;

use cynic_host::adapters::local_build_artefact::LocalBuildArtefactAdapter;
use cynic_host::domain::build_artefact::{RetentionExecutor, RetentionObserver, RetentionPlanner};
use cynic_protocol::retention::RetentionPolicy;

use crate::emit_json;

pub(crate) fn execute_build_artefact_status(json: bool) -> ExitCode {
    let adapter = LocalBuildArtefactAdapter;
    let obs = match adapter.observe() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Build artefact observation failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    if json {
        emit_json(&obs, false)
    } else {
        print_observation_human(&obs);
        ExitCode::SUCCESS
    }
}

pub(crate) fn execute_build_artefact(json: bool, apply: bool) -> ExitCode {
    let adapter = LocalBuildArtefactAdapter;
    let obs = match adapter.observe() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Build artefact observation failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    if apply {
        let policy = build_artefact_policy();
        let plan = adapter.plan(&obs, &policy);
        if plan.items.is_empty() {
            if json {
                return emit_json(&plan, false);
            }
            println!("No reclaimable build artefacts — nothing to GC.");
            return ExitCode::SUCCESS;
        }
        let result = adapter.execute(&plan);
        match result {
            Ok(result) => {
                if json {
                    return emit_json(&result, !result.success);
                }
                print_gc_result_human(&result);
                if result.success {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(e) => {
                eprintln!("Build artefact GC execution failed: {e}");
                ExitCode::FAILURE
            }
        }
    } else if json {
        emit_json(&obs, false)
    } else {
        print_observation_human(&obs);
        let policy = build_artefact_policy();
        let plan = adapter.plan(&obs, &policy);
        if !plan.items.is_empty() {
            println!();
            print_gc_plan_human(&plan);
            println!();
            println!("To apply: cynic runtime build artefact --apply");
        }
        ExitCode::SUCCESS
    }
}

fn build_artefact_policy() -> RetentionPolicy {
    cynic_protocol::retention::default_build_artefact_policy()
}

fn print_observation_human(obs: &cynic_protocol::retention::RetentionObservation) {
    println!("════════════════════════════════════════════════════════════");
    println!("  CYNIC BUILD ARTEFACT OBSERVATION — {}", obs.timestamp);
    println!("════════════════════════════════════════════════════════════");
    println!();
    println!("Total items: {}", obs.items.len());
    if let Some(pressure) = &obs.pressure {
        println!(
            "Pressure: {}% avail {} (pressured={})",
            pressure.use_pct,
            bytes_to_human(pressure.avail_bytes),
            pressure.is_pressured
        );
    }
    println!();
    if obs.items.is_empty() {
        println!("(no items)");
    } else {
        println!("{:<45} {:<10} {:<8} REASON", "PATH", "SIZE", "OWNER");
        println!("{}", "-".repeat(90));
        for item in &obs.items {
            let owner = format!(
                "{}:{}",
                item.owner_uid.unwrap_or(999),
                item.owner_gid.unwrap_or(990)
            );
            let reason = if item.is_orphan == Some(true) {
                "orphan"
            } else {
                "live"
            };
            println!(
                "{:<45} {:<10} {:<8} {}",
                item.path,
                bytes_to_human(item.size_bytes),
                owner,
                reason
            );
        }
    }
}

fn print_gc_plan_human(plan: &cynic_protocol::retention::RetentionPlan) {
    println!(
        "── GC Plan ({} reclaimable) ─────────────────────────",
        bytes_to_human(plan.total_reclaimable_bytes)
    );
    println!("{:<30} {:<10} {:<10} REASON", "ID", "SIZE", "PRIV");
    println!("{}", "-".repeat(80));
    for i in &plan.items {
        println!(
            "{:<30} {:<10} {:<10} {}",
            i.id,
            bytes_to_human(i.size_bytes),
            if i.requires_privilege { "sudo" } else { "-" },
            i.reason
        );
    }
    println!(
        "Total reclaimable: {}",
        bytes_to_human(plan.total_reclaimable_bytes)
    );
}

fn print_gc_result_human(result: &cynic_protocol::retention::RetentionResult) {
    println!("════════════════════════════════════════════════════════════");
    println!("  CYNIC BUILD ARTEFACT GC — {}", result.timestamp);
    println!("════════════════════════════════════════════════════════════");
    for e in &result.items_executed {
        let st = if e.success { "OK" } else { "FAIL" };
        println!("{:<30} {:<6} code={:?}", e.item_id, st, e.exit_code);
        if !e.success && !e.stderr_tail.is_empty() {
            println!("  stderr: {}", e.stderr_tail);
        }
    }
    println!(
        "Outcome: {} reclaimed {}",
        if result.success { "SUCCESS" } else { "PARTIAL" },
        bytes_to_human(result.total_reclaimed_bytes)
    );
}

fn bytes_to_human(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes}B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1}K", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1}M", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1}G", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}
