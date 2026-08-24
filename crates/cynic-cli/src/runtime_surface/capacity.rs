//! CLI adapter for host capacity observation (metabolic state + headroom).
//!
//! Thin projection surface — delegates to `LocalCapacityAdapter` in `cynic-host`
//! and the pure `cynic_protocol::capacity` decision. Read-only.
//!
//! Policy provenance: the metabolic policy is node-declared data (HITL Q3).
//! Default resolution order:
//!   1. `CYNIC_METABOLIC_POLICY` env var — path to a JSON MetabolicPolicy file
//!   2. legacy defaults (85/90/40/60, same semantics as the former host port)
//! Secrets are never read; thresholds are not secrets.

use std::process::ExitCode;

use cynic_host::adapters::local_capacity::LocalCapacityAdapter;
use cynic_protocol::capacity::MetabolicPolicy;

use crate::emit_json;

pub(crate) fn execute_capacity(json: bool) -> ExitCode {
    let policy = match load_policy() {
        Ok(policy) => policy,
        Err(error) => {
            eprintln!("Metabolic policy error: {error}");
            return ExitCode::FAILURE;
        }
    };

    let adapter = LocalCapacityAdapter::new();
    match adapter.assess_capacity(&policy) {
        Ok(assessment) => {
            if json {
                emit_json(&assessment, false)
            } else {
                print_capacity_human(&assessment);
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("Capacity assessment failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn load_policy() -> Result<MetabolicPolicy, String> {
    match std::env::var("CYNIC_METABOLIC_POLICY") {
        Ok(path) => {
            let raw = std::fs::read_to_string(&path)
                .map_err(|error| format!("policy file {path} unreadable: {error}"))?;
            serde_json::from_str(&raw)
                .map_err(|error| format!("policy file {path} invalid: {error}"))
        }
        Err(_) => Ok(MetabolicPolicy::with_legacy_defaults("node-default")),
    }
}

fn print_capacity_human(a: &cynic_protocol::capacity::CapacityAssessmentV1) {
    println!("════════════════════════════════════════════════════════════");
    println!(
        "  CYNIC CAPACITY / METABOLIC STATE — policy {}",
        a.policy_id
    );
    println!("════════════════════════════════════════════════════════════");
    println!();
    println!("  State: {:?}", a.state);
    match a.cpu_util_pct {
        Some(pct) => println!("  CPU used: {pct:.1}%"),
        None => println!("  CPU used: (unknown)"),
    }
    match a.mem_used_pct {
        Some(pct) => println!("  RAM used: {pct:.1}%"),
        None => println!("  RAM used: (unknown)"),
    }
    println!("  RAM available: {} bytes", a.available_ram_bytes);
    if !a.residual_unknowns.is_empty() {
        println!();
        println!("── Residual unknowns ────────────────────────────────────────");
        for u in &a.residual_unknowns {
            println!("  - {u}");
        }
    }
}
