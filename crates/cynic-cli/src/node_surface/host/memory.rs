//! CLI adapter for host memory observation (4-layer: physical/volume/swap/pressure).
//!
//! Thin projection surface — delegates to `LocalMemoryAdapter` in `cynic-host`.

use std::process::ExitCode;

use cynic_host::adapters::local_memory::LocalMemoryAdapter;
use cynic_host::domain::memory::MemoryObservationPort;
use cynic_host::domain::memory::types::MemoryReport;

use crate::emit_json;

pub(crate) fn execute_memory(json: bool) -> ExitCode {
    let adapter = LocalMemoryAdapter::new();
    match adapter.observe_memory() {
        Ok(report) => {
            if json {
                emit_json(&report, false)
            } else {
                print_memory_report_human(&report);
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("Memory observation failed: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_memory_report_human(report: &MemoryReport) {
    println!("════════════════════════════════════════════════════════════");
    println!("  CYNIC MEMORY REPORT — {}", report.timestamp);
    println!("════════════════════════════════════════════════════════════");
    println!();
    println!("── Physical ───────────────────────────────────────────────");
    println!(
        "  Total: {}  Free: {}  Available: {}",
        bytes_to_human(report.physical.total_bytes),
        bytes_to_human(report.physical.free_bytes),
        bytes_to_human(report.physical.available_bytes),
    );
    println!();
    println!("── Volume / Page config ────────────────────────────────────");
    println!("  Page size: {} bytes", report.volume.page_size_bytes);
    if let Some(sw) = report.volume.swappiness {
        println!("  Swappiness: {sw}");
    }
    if let Some(hp) = report.volume.hugepages_reserved {
        println!("  Hugepages reserved: {hp}");
        if let Some(hps) = report.volume.hugepage_size_bytes {
            println!("  Hugepage size: {}", bytes_to_human(hps));
        }
    }
    println!();
    println!("── Swap ────────────────────────────────────────────────────");
    println!(
        "  Total: {}  Free: {}  Used: {}",
        bytes_to_human(report.swap.total_bytes),
        bytes_to_human(report.swap.free_bytes),
        bytes_to_human(report.swap.used_bytes),
    );
    println!();
    if let Some(p) = &report.pressure {
        println!("── Pressure stalls (PSI) ───────────────────────────────────");
        println!(
            "  Some: avg10={} avg60={} avg300={} total_us={}",
            p.some_avg10, p.some_avg60, p.some_avg300, p.some_total_us,
        );
        println!(
            "  Full: avg10={} avg60={} avg300={} total_us={}",
            p.full_avg10, p.full_avg60, p.full_avg300, p.full_total_us,
        );
    } else {
        println!("── Pressure stalls (PSI) ───────────────────────────────────");
        println!("  (unknown — /proc/pressure/memory unavailable)");
    }
    if !report.residual_unknowns.is_empty() {
        println!();
        println!("── Residual unknowns ────────────────────────────────────────");
        for u in &report.residual_unknowns {
            println!("  - {u}");
        }
    }
}

fn bytes_to_human(bytes: u64) -> String {
    use cynic_protocol::disk::bytes_to_human;
    bytes_to_human(bytes)
}
