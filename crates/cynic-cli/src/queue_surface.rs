#![allow(clippy::print_stdout, clippy::print_stderr)]
//! Thin CLI projection for queue planning — `cynic queue --json`.
//! Composes branch_graph + discovery + GC via `LocalQueueAdapter::observe` + pure `build_queue_plan`.

use std::process::ExitCode;

use cynic_host::adapters::local_queue::LocalQueueAdapter;

use crate::emit_json;

pub(crate) fn execute_queue(json: bool) -> ExitCode {
    let adapter = LocalQueueAdapter;
    let plan = match adapter.observe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("queue observation failed: {e}");
            return ExitCode::FAILURE;
        }
    };
    if json {
        return emit_json(&plan, false);
    }
    print_queue_human(&plan);
    ExitCode::SUCCESS
}

fn print_queue_human(plan: &cynic_protocol::queue::QueuePlan) {
    println!("════════════════════════════════════════════════════════════");
    println!(
        "  CYNIC QUEUE — root: {} ({} items)",
        plan.root,
        plan.items.len()
    );
    println!("════════════════════════════════════════════════════════════");
    if plan.items.is_empty() {
        println!("(empty — no queued branches or reclaimable artefacts)");
        println!("reclaimable bytes: {}", plan.total_reclaimable_bytes);
        return;
    }
    println!("{:<36} {:<20} {:<22} PRIV", "ID", "BRANCH", "REASON");
    println!("{}", "-".repeat(90));
    for item in &plan.items {
        let priv_s = if item.action.requires_privilege {
            "sudo"
        } else {
            "-"
        };
        println!(
            "{:<36} {:<20} {:<22?} {}",
            item.id, item.branch, item.reason, priv_s
        );
        println!("  recipe={} argv={:?}", item.recipe_id, item.action.argv);
    }
    println!("total_reclaimable_bytes: {}", plan.total_reclaimable_bytes);
}
