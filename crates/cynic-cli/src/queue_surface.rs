#![allow(clippy::print_stdout, clippy::print_stderr)]
//! Thin CLI projection for queue planning and execution — `cynic queue [--apply] [--json]`.
//! Composes branch_graph + discovery + GC via `LocalQueueAdapter::observe` + pure `build_queue_plan`.

use std::process::ExitCode;
use std::time::SystemTime;

use chrono::{DateTime, Utc};
use cynic_host::adapters::local_queue::LocalQueueAdapter;
use cynic_host::process_observation::capture_command;
use cynic_protocol::queue::{
    QUEUE_EXECUTION_RESULT_SCHEMA, QueueExecutionResult, QueueItemExecution,
};

use crate::emit_json;

pub(crate) fn execute_queue(json: bool, apply: bool) -> ExitCode {
    let adapter = LocalQueueAdapter;
    let plan = match adapter.observe() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("queue observation failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    if apply {
        if plan.items.is_empty() {
            if json {
                return emit_json(&plan, false);
            }
            println!("No queued actions to apply — queue is clean.");
            return ExitCode::SUCCESS;
        }

        let current_branch = capture_command("git", &["rev-parse", "--abbrev-ref", "HEAD"])
            .ok()
            .filter(|c| c.success)
            .map(|c| c.stdout.trim().to_string())
            .unwrap_or_default();

        let mut items_executed = Vec::new();
        let mut all_success = true;
        let mut total_reclaimed = 0u64;

        for item in &plan.items {
            if item.action.argv.is_empty() {
                continue;
            }
            // Safety guard 1: Never delete root branch (e.g. main)
            if item.branch == plan.root {
                items_executed.push(QueueItemExecution {
                    id: item.id.clone(),
                    branch: item.branch.clone(),
                    success: false,
                    exit_code: Some(1),
                    stdout: String::new(),
                    stderr: format!(
                        "Safety guard: refusing to delete root branch '{}'",
                        plan.root
                    ),
                });
                all_success = false;
                continue;
            }
            // Safety guard 2: Never delete the active checked-out branch
            if !current_branch.is_empty() && item.branch == current_branch {
                items_executed.push(QueueItemExecution {
                    id: item.id.clone(),
                    branch: item.branch.clone(),
                    success: false,
                    exit_code: Some(1),
                    stdout: String::new(),
                    stderr: format!(
                        "Safety guard: refusing to delete active branch '{}'",
                        current_branch
                    ),
                });
                all_success = false;
                continue;
            }

            let prog = &item.action.argv[0];
            let args: Vec<&str> = item.action.argv[1..].iter().map(String::as_str).collect();
            let res = capture_command(prog, &args);

            let (success, exit_code, stdout, stderr) = match res {
                Ok(c) => (c.success, c.exit_code, c.stdout, c.stderr),
                Err(e) => (false, Some(1), String::new(), e),
            };

            if success {
                if item.id.starts_with("checkout:") {
                    // Reclaimed bytes tracked by GC plan item
                }
            } else {
                all_success = false;
            }

            items_executed.push(QueueItemExecution {
                id: item.id.clone(),
                branch: item.branch.clone(),
                success,
                exit_code,
                stdout,
                stderr,
            });
        }

        if all_success {
            total_reclaimed = plan.total_reclaimable_bytes;
        }

        let timestamp = DateTime::<Utc>::from(SystemTime::now())
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string();

        let result = QueueExecutionResult {
            schema: QUEUE_EXECUTION_RESULT_SCHEMA.to_string(),
            timestamp,
            success: all_success,
            items_executed,
            total_reclaimed_bytes: total_reclaimed,
        };

        if json {
            return emit_json(&result, !all_success);
        }

        print_queue_execution_human(&result);
        if all_success {
            ExitCode::SUCCESS
        } else {
            ExitCode::FAILURE
        }
    } else {
        if json {
            return emit_json(&plan, false);
        }
        print_queue_human(&plan);
        if !plan.items.is_empty() {
            println!();
            println!("To apply: cynic queue --apply");
        }
        ExitCode::SUCCESS
    }
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

fn print_queue_execution_human(result: &QueueExecutionResult) {
    println!("════════════════════════════════════════════════════════════");
    println!("  CYNIC QUEUE EXECUTION — {}", result.timestamp);
    println!("════════════════════════════════════════════════════════════");
    println!("Success: {}", result.success);
    println!("Items executed: {}", result.items_executed.len());
    println!("{:<36} {:<20} {:<10} STATUS", "ID", "BRANCH", "EXIT");
    println!("{}", "-".repeat(80));
    for item in &result.items_executed {
        let status = if item.success { "OK" } else { "FAILED" };
        let code = item
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "-".into());
        println!(
            "{:<36} {:<20} {:<10} {}",
            item.id, item.branch, code, status
        );
        if !item.success && !item.stderr.is_empty() {
            println!("  error: {}", item.stderr.trim());
        }
    }
    if result.total_reclaimed_bytes > 0 {
        println!("Total reclaimed bytes: {}", result.total_reclaimed_bytes);
    }
}
