//! Forensics surface — read-only projection of host crash evidence.
//!
//! Thin surface: parse → delegate to host forensics adapter → emit.
//! No domain policy here. Policy lives in cynic-host/cynic-kernel.

use std::process::ExitCode;

use clap::Subcommand;

#[derive(Subcommand)]
pub(crate) enum ForensicsCommand {
    /// Collect forensic evidence from the current boot.
    Evidence {
        /// Machine-readable JSON (default is human summary).
        #[arg(long)]
        json: bool,
        /// Boot id to inspect (default: current boot from /proc).
        #[arg(long)]
        boot_id: Option<String>,
    },
    /// Timeline of events for a specific boot.
    Timeline {
        /// Boot id to inspect.
        boot_id: String,
        /// Machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Post-mortem report: correlate all evidence for a crashed boot.
    PostMortem {
        /// Boot id to inspect (default: previous boot).
        #[arg(long)]
        boot_id: Option<String>,
        /// Machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Current boot status + last known crash.
    Status {
        /// Machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// Active service restart loops (crash-loops).
    Services {
        /// Machine-readable JSON.
        #[arg(long)]
        json: bool,
        /// Minimum restart rate per hour to report.
        #[arg(long, default_value_t = 100.0)]
        min_rate: f64,
    },
}

pub(crate) fn execute_forensics(command: ForensicsCommand) -> ExitCode {
    match command {
        ForensicsCommand::Evidence { json, boot_id } => execute_evidence(json, boot_id),
        ForensicsCommand::Timeline { boot_id, json } => execute_timeline(&boot_id, json),
        ForensicsCommand::PostMortem { boot_id, json } => execute_post_mortem(boot_id, json),
        ForensicsCommand::Status { json } => execute_status(json),
        ForensicsCommand::Services { json, min_rate } => execute_services(json, min_rate),
    }
}

fn execute_evidence(json: bool, boot_id: Option<String>) -> ExitCode {
    // TODO: wire to cynic_host::domain::forensics::LocalForensicAdapter
    let boot_id = boot_id.unwrap_or_else(|| "<current>".to_string());
    if json {
        let payload = serde_json::json!({
            "schema": "cynic.forensics.evidence.v1",
            "boot_id": boot_id,
            "status": "wired_in_slice_4_placeholder",
            "note": "Host adapter wiring deferred — cynic-host forensics port ready."
        });
        crate::cli_json::emit_pretty_json(&payload)
    } else {
        println!("Forensic evidence for boot: {}", boot_id);
        println!("[placeholder — host adapter wiring pending]");
        ExitCode::SUCCESS
    }
}

fn execute_timeline(boot_id: &str, json: bool) -> ExitCode {
    if json {
        let payload = serde_json::json!({
            "schema": "cynic.forensic_timeline.v1",
            "boot_id": boot_id,
            "entries": [],
            "status": "wired_in_slice_4_placeholder",
        });
        crate::cli_json::emit_pretty_json(&payload)
    } else {
        println!("Timeline for boot: {}", boot_id);
        println!("[placeholder — host adapter wiring pending]");
        ExitCode::SUCCESS
    }
}

fn execute_post_mortem(boot_id: Option<String>, json: bool) -> ExitCode {
    let target = boot_id.unwrap_or_else(|| "<previous>".to_string());
    if json {
        let payload = serde_json::json!({
            "schema": "cynic.post_mortem_report.v1",
            "boot_id": target,
            "status": "wired_in_slice_4_placeholder",
        });
        crate::cli_json::emit_pretty_json(&payload)
    } else {
        println!("Post-mortem report for boot: {}", target);
        println!("[placeholder — host adapter wiring pending]");
        ExitCode::SUCCESS
    }
}

fn execute_status(json: bool) -> ExitCode {
    if json {
        let payload = serde_json::json!({
            "schema": "cynic.forensics.status.v1",
            "status": "wired_in_slice_4_placeholder",
        });
        crate::cli_json::emit_pretty_json(&payload)
    } else {
        println!("Forensics status: [host adapter wiring pending]");
        ExitCode::SUCCESS
    }
}

fn execute_services(json: bool, min_rate: f64) -> ExitCode {
    if json {
        let payload = serde_json::json!({
            "schema": "cynic.forensics.services.v1",
            "min_rate_per_hour": min_rate,
            "loops": [],
            "status": "wired_in_slice_4_placeholder",
        });
        crate::cli_json::emit_pretty_json(&payload)
    } else {
        println!("Service loops (min rate: {:.0}/h):", min_rate);
        println!("[placeholder — host adapter wiring pending]");
        ExitCode::SUCCESS
    }
}
