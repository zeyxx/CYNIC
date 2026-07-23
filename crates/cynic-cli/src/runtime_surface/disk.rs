//! CLI adapter for host disk observation and cleanup.

use std::process::ExitCode;

use cynic_host::adapters::local_disk::LocalDiskAdapter;
use cynic_host::domain::disk::port::{DiskCleanupExecutionPort, DiskObservationPort};
use cynic_host::domain::disk::{DiskCleanupPlan, DiskCleanupResult, DiskReport};

use crate::emit_json;

pub(crate) fn execute_disk(json: bool) -> ExitCode {
    let adapter = LocalDiskAdapter;
    match adapter.observe_disk() {
        Ok(report) => {
            if json {
                emit_json(&report, false)
            } else {
                print_disk_report_human(&report);
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("Disk observation failed: {e}");
            ExitCode::FAILURE
        }
    }
}

pub(crate) fn execute_disk_clean(json: bool, apply: bool) -> ExitCode {
    let adapter = LocalDiskAdapter;
    let report = match adapter.observe_disk() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Disk observation failed: {e}");
            return ExitCode::FAILURE;
        }
    };

    let plan = adapter.plan_cleanup(&report);

    if apply {
        let result = adapter.execute_cleanup(&plan);
        if json {
            emit_json(&result, !result.success)
        } else {
            print_cleanup_result_human(&result);
            if result.success {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
    } else {
        if json {
            emit_json(&plan, false)
        } else {
            print_cleanup_plan_human(&plan);
            ExitCode::SUCCESS
        }
    }
}

fn print_disk_report_human(report: &DiskReport) {
    println!("════════════════════════════════════════════════════════════");
    println!("  CYNIC DISK REPORT — {}", report.timestamp);
    println!("════════════════════════════════════════════════════════════");
    println!();
    println!("── Filesystems ──────────────────────────────────────────────");
    println!(
        "{:<25} {:<10} {:<10} {:<10} {:<10} {:<6} MOUNT",
        "FILESYSTEM", "TYPE", "SIZE", "USED", "AVAIL", "USE%"
    );
    println!("{}", "-".repeat(80));
    for fs in &report.filesystems {
        println!(
            "{:<25} {:<10} {:<10} {:<10} {:<10} {:<6} {}",
            fs.source,
            fs.fstype,
            fs.size_human,
            fs.used_human,
            fs.avail_human,
            fs.use_pct,
            fs.mount
        );
    }
    println!();

    println!("── Known Caches & Build Artifacts ──────────────────────────");
    println!("{:<30} {:<12} PATH", "LABEL", "SIZE");
    println!("{}", "-".repeat(80));
    for cache in &report.known_caches {
        println!(
            "{:<30} {:<12} {}",
            cache.label, cache.size_human, cache.path
        );
    }
    println!();

    println!("── Large Files (>100 MB) ────────────────────────────────────");
    if report.large_files.is_empty() {
        println!("(none found)");
    } else {
        println!("{:<12} PATH", "SIZE");
        println!("{}", "-".repeat(80));
        for file in &report.large_files {
            println!("{:<12} {}", file.size_human, file.path);
        }
    }
    println!();
    println!("Tip: Run `cynic runtime disk-clean` to view potential savings.");
}

fn print_cleanup_plan_human(plan: &DiskCleanupPlan) {
    println!("════════════════════════════════════════════════════════════");
    println!("  CYNIC DISK CLEANUP PLAN");
    println!("════════════════════════════════════════════════════════════");
    println!();
    if plan.items.is_empty() {
        println!("No cleanable items found. Your system is tidy!");
        return;
    }

    println!("Proposed cleanup tasks:");
    println!(
        "{:<30} {:<12} {:<10} DESCRIPTION",
        "ID", "EST. RECLAIM", "PRIVILEGE"
    );
    println!("{}", "-".repeat(80));
    for item in &plan.items {
        println!(
            "{:<30} {:<12} {:<10} {}",
            item.id,
            item.size_human,
            if item.requires_privilege { "Yes" } else { "No" },
            item.label
        );
    }
    println!();
    println!("Total reclaimable space: {}", plan.total_reclaimable_human);
    println!();
    println!("> To apply this cleanup, run: cynic runtime disk-clean --apply");
}

fn print_cleanup_result_human(result: &DiskCleanupResult) {
    println!("════════════════════════════════════════════════════════════");
    println!("  CYNIC DISK CLEANUP EXECUTION — {}", result.timestamp);
    println!("════════════════════════════════════════════════════════════");
    println!();
    println!("Execution Details:");
    println!("{:<30} {:<10} {:<10} MESSAGE", "TASK ID", "STATUS", "CODE");
    println!("{}", "-".repeat(80));
    for exec in &result.items_executed {
        let status_str = if exec.success { "SUCCESS" } else { "FAILED" };
        let code_str = exec
            .exit_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "-".to_string());
        let msg = if exec.success {
            "Reclaimed space successfully".to_string()
        } else {
            exec.stderr_tail.clone()
        };
        println!(
            "{:<30} {:<10} {:<10} {}",
            exec.item_id, status_str, code_str, msg
        );
    }
    println!();
    println!(
        "Cleanup outcome: {}",
        if result.success {
            "SUCCESS"
        } else {
            "PARTIAL FAILURE"
        }
    );
    println!("Total space reclaimed: {}", result.total_reclaimed_human);
}
