//! Forensics surface — read-only projection of host crash evidence.
//!
//! Thin surface: parse → delegate to host forensics adapter → emit.
//! No domain policy here. Policy lives in cynic-host/cynic-kernel.

use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::domain::forensics::local::LocalForensicAdapter;
use cynic_host::domain::forensics::port::ForensicObservationPort;
use cynic_host::domain::forensics::probe::parse_boot_table_line;

#[derive(Subcommand)]
pub(crate) enum ForensicsCommand {
    /// Collect forensic evidence from the current boot.
    Evidence {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        boot_id: Option<String>,
    },
    /// Timeline of events for a specific boot.
    Timeline {
        boot_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Post-mortem report: correlate all evidence for a crashed boot.
    PostMortem {
        #[arg(long)]
        boot_id: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Current boot status + last known crash.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Active service restart loops (crash-loops).
    Services {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 100.0)]
        min_rate: f64,
    },
}

pub(crate) fn execute_forensics(command: ForensicsCommand) -> ExitCode {
    let adapter = LocalForensicAdapter::new();
    match command {
        ForensicsCommand::Evidence { json, boot_id } => execute_evidence(&adapter, json, boot_id),
        ForensicsCommand::Timeline { boot_id, json } => execute_timeline(&adapter, &boot_id, json),
        ForensicsCommand::PostMortem { boot_id, json } => {
            execute_post_mortem(&adapter, boot_id, json)
        }
        ForensicsCommand::Status { json } => execute_status(&adapter, json),
        ForensicsCommand::Services { json, min_rate } => execute_services(&adapter, json, min_rate),
    }
}

fn resolve_previous_boot_id() -> String {
    let table = match LocalForensicAdapter::read_boot_table() {
        Some(t) => t,
        None => return "unknown".to_string(),
    };
    let current = LocalForensicAdapter::read_current_boot_id();
    let mut prev: Option<String> = None;
    for line in table.lines() {
        if let Some((bid, _, _)) = parse_boot_table_line(line) {
            if let Some(ref c) = current {
                if &bid == c {
                    return prev.unwrap_or_else(|| "unknown".to_string());
                }
            }
            prev = Some(bid);
        }
    }
    "unknown".to_string()
}

fn execute_evidence(adapter: &LocalForensicAdapter, json: bool, boot_id: Option<String>) -> ExitCode {
    let boot_id = boot_id.unwrap_or_else(|| {
        LocalForensicAdapter::read_current_boot_id()
            .unwrap_or_else(|| "unknown".to_string())
    });
    match adapter.collect_evidence(&boot_id) {
        Ok(evidence) => {
            if json {
                let payload = serde_json::json!({
                    "schema": "cynic.forensics.evidence.v1",
                    "boot_id": boot_id,
                    "evidence_count": evidence.len(),
                    "evidence": evidence,
                });
                crate::cli_json::emit_pretty_json(&payload)
            } else {
                println!("Forensic evidence for boot: {}", boot_id);
                println!("Found {} evidence items:", evidence.len());
                for (i, e) in evidence.iter().enumerate() {
                    println!("  [{}] {} — severity={}", i + 1, e.kind.as_str(), e.severity);
                    println!("      {}", e.snippet.trim());
                }
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("error collecting evidence: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn execute_timeline(adapter: &LocalForensicAdapter, boot_id: &str, json: bool) -> ExitCode {
    match adapter.timeline(boot_id) {
        Ok(timeline) => {
            if json {
                crate::cli_json::emit_pretty_json(&timeline)
            } else {
                println!("Timeline for boot: {}", boot_id);
                println!("{} entries:", timeline.entries.len());
                for entry in &timeline.entries {
                    println!("  [{:>6.1}s] {}", entry.offset_secs, entry.evidence.snippet.trim());
                }
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("error building timeline: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn execute_post_mortem(adapter: &LocalForensicAdapter, boot_id: Option<String>, json: bool) -> ExitCode {
    let boot_id = boot_id.unwrap_or_else(resolve_previous_boot_id);
    match adapter.post_mortem(&boot_id) {
        Ok(report) => {
            if json {
                crate::cli_json::emit_pretty_json(&report)
            } else {
                println!("Post-mortem report for boot: {}", boot_id);
                if let Some(crash) = &report.crash {
                    println!("  Crash kind: {}", crash.as_str());
                } else {
                    println!("  Crash kind: unknown");
                }
                println!("  GPU faults: {}", report.gpu_fault_count);
                println!("  Softlockup traps: {}", report.softlockup_trap_count);
                println!("  Service loops: {}", report.service_loops.len());
                println!("  Core dumps: {}", report.core_dumps.len());
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("error generating post-mortem: {}", e);
            ExitCode::FAILURE
        }
    }
}

fn execute_status(adapter: &LocalForensicAdapter, json: bool) -> ExitCode {
    let current_boot = LocalForensicAdapter::read_current_boot_id();
    let uptime = LocalForensicAdapter::read_uptime_secs();
    let failed_services = adapter.service_loops().unwrap_or_default();
    if json {
        let payload = serde_json::json!({
            "schema": "cynic.forensics.status.v1",
            "current_boot_id": current_boot,
            "uptime_secs": uptime,
            "failed_services_count": failed_services.len(),
        });
        crate::cli_json::emit_pretty_json(&payload)
    } else {
        println!("Forensics status");
        if let Some(ref boot) = current_boot {
            println!("  Current boot: {}", boot);
        }
        if let Some(secs) = uptime {
            println!("  Uptime: {}h {}m {}s", secs / 3600, (secs % 3600) / 60, secs % 60);
        }
        println!("  Failed services: {}", failed_services.len());
        ExitCode::SUCCESS
    }
}

fn execute_services(adapter: &LocalForensicAdapter, json: bool, min_rate: f64) -> ExitCode {
    match adapter.service_loops() {
        Ok(loops) => {
            let filtered: Vec<_> = loops
                .iter()
                .filter(|l| l.rate_per_hour() >= min_rate)
                .collect();
            if json {
                let payload = serde_json::json!({
                    "schema": "cynic.forensics.services.v1",
                    "min_rate_per_hour": min_rate,
                    "total_loops": loops.len(),
                    "filtered_count": filtered.len(),
                    "loops": filtered,
                });
                crate::cli_json::emit_pretty_json(&payload)
            } else {
                println!("Service loops (min rate: {:.0}/h):", min_rate);
                println!("{} of {} loops exceed threshold:", filtered.len(), loops.len());
                for l in &filtered {
                    println!("  {} — {:.0} restarts/h", l.unit, l.rate_per_hour());
                }
                ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("error scanning services: {}", e);
            ExitCode::FAILURE
        }
    }
}
