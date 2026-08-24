//! CLI adapter for host disk observation and cleanup.

use std::process::ExitCode;

use cynic_host::adapters::local_build_cache::LocalBuildCacheAdapter;
use cynic_host::adapters::local_disk::LocalDiskAdapter;
use cynic_host::domain::disk::build_runner::{
    load_build_cache_desired_state, reconcile_build_cache_resources,
};
use cynic_host::domain::disk::port::{DiskCleanupExecutionPort, DiskObservationPort};
use cynic_host::domain::disk::{DiskCleanupPlan, DiskCleanupResult, DiskReport};
use cynic_protocol::disk::bytes_to_human;

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

pub(crate) fn execute_disk_clean(
    json: bool,
    apply: bool,
    build_runner: bool,
    registry: &str,
    node: Option<&str>,
    state_root: &str,
) -> ExitCode {
    if build_runner {
        return execute_build_runner_cache(json, apply, registry, node, state_root);
    }
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

fn execute_build_runner_cache(
    json: bool,
    apply: bool,
    registry: &str,
    node: Option<&str>,
    state_root: &str,
) -> ExitCode {
    let observed_hostname = cynic_host::runtime_layout::observed_hostname();
    let node_id = node.unwrap_or(&observed_hostname);
    let temporary_root = std::env::temp_dir();
    let desired = match load_build_cache_desired_state(
        std::path::Path::new(registry),
        node_id,
        std::path::Path::new(state_root),
        &temporary_root,
    ) {
        Ok(desired) => desired,
        Err(error) => {
            let report = serde_json::json!({
                "schema": "cynic.build_runner.cache_report.v1",
                "status": "blocked",
                "mutates": false,
                "blockers": [error]
            });
            if json {
                return emit_json(&report, true);
            }
            eprintln!("Build-runner cache reconciliation blocked: {error}");
            return ExitCode::FAILURE;
        }
    };
    let report = reconcile_build_cache_resources(
        &desired,
        apply,
        &LocalBuildCacheAdapter,
        &LocalBuildCacheAdapter,
    );
    if json {
        return emit_json(&report, report.status == "blocked");
    }
    println!(
        "Build-runner cache: {} (available={:?}, target={})",
        report.status, report.before.available_bytes, report.plan.target_path
    );
    if report.status == "planned" {
        println!("Apply with: cynic runtime disk-clean --build-runner --apply");
    }
    if report.status == "blocked" {
        eprintln!("Blockers: {}", report.blockers.join(", "));
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn print_disk_report_human(report: &DiskReport) {
    println!("════════════════════════════════════════════════════════════");
    println!("  CYNIC DISK REPORT — {}", report.timestamp);
    println!("════════════════════════════════════════════════════════════");
    println!();
    println!("── Physical Disks (lsblk) ─────────────────────────────────");
    if report.physical_disks.is_empty() {
        println!("(none observed)");
    } else {
        println!("{:<14} {:<14} {:<10} {}", "DEVICE", "SIZE", "ROTA?", "MODEL");
        println!("{}", "-".repeat(80));
        for d in &report.physical_disks {
            println!(
                "{:<14} {:<14} {:<10} {}",
                d.device,
                bytes_to_human(d.size_bytes.unwrap_or(0)),
                if d.rotational { "yes" } else { "no" },
                d.model.as_deref().unwrap_or(""),
            );
        }
    }
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
