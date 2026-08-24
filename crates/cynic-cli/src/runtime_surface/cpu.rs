//! CLI adapter for host CPU observation (4-layer: physical/volume/usage/pressure).
//!
//! Thin projection surface — delegates to `LocalCpuAdapter` in `cynic-host`.

use std::process::ExitCode;

use cynic_host::adapters::local_cpu::LocalCpuAdapter;
use cynic_host::domain::cpu::CpuReport;
use cynic_host::domain::cpu::port::CpuObservationPort;

use crate::emit_json;

pub(crate) fn execute_cpu(json: bool) -> ExitCode {
    let adapter = LocalCpuAdapter::new();
    match adapter.observe_cpu() {
        Ok(report) => {
            if json {
                emit_json(&report, false)
            } else {
                print_cpu_report_human(&report);
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("CPU observation failed: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_cpu_report_human(report: &CpuReport) {
    println!("════════════════════════════════════════════════════════════");
    println!("  CYNIC CPU REPORT — {}", report.timestamp);
    println!("════════════════════════════════════════════════════════════");
    println!();
    println!("── Physical ───────────────────────────────────────────────");
    println!("  Model: {}", report.physical.model_name);
    println!("  Logical CPUs: {}", report.physical.logical_cpus);
    if let Some(c) = report.physical.cores {
        println!("  Cores: {c}");
    }
    if let Some(s) = report.physical.sockets {
        println!("  Sockets: {s}");
    }
    if let Some(f) = report.physical.max_bogomips_or_hz {
        println!("  BogoMIPS/MaxHz: {f}");
    }
    println!();
    println!("── Volume / Policy ─────────────────────────────────────────");
    println!("  Online CPUs: {}", report.volume.online_cpus);
    println!("  Possible CPUs: {}", report.volume.possible_cpus);
    if let Some(g) = &report.volume.governor {
        println!("  Governor: {g}");
    }
    if let Some(m) = &report.volume.isolated_mask {
        println!("  Isolated: {m}");
    }
    println!();
    println!("── Usage (jiffies) ─────────────────────────────────────────");
    println!(
        "  user={:<10} system={:<10} idle={:<10} iowait={:<10}",
        report.usage.user,
        report.usage.system,
        report.usage.idle,
        report.usage.iowait,
    );
    println!(
        "  softirq={:<8} steal={:<10}",
        report.usage.softirq,
        report.usage.steal,
    );
    if let Some(u) = report.usage.util_pct {
        println!("  Utilisation: {u:.1}%");
    }
    println!();
    if let Some(p) = &report.pressure {
        println!("── Pressure stalls (PSI) ───────────────────────────────────");
        println!(
            "  Some avg10={}",
            p.some_avg10.unwrap_or(0.0),
        );
        if let Some(softirq) = p.softirq_steal {
            println!("  PSI total (softirq/steal proxy): {softirq}");
        }
    } else {
        println!("── Pressure stalls (PSI) ───────────────────────────────────");
        println!("  (unknown — /proc/pressure/cpu unavailable)");
    }
    if !report.residual_unknowns.is_empty() {
        println!();
        println!("── Residual unknowns ────────────────────────────────────────");
        for u in &report.residual_unknowns {
            println!("  - {u}");
        }
    }
}
