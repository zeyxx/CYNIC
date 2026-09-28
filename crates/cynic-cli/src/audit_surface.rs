use std::path::Path;
use std::process::ExitCode;

use clap::Subcommand;

use crate::{crate_audit, emit_json, fail, node_host_audit};

#[derive(Subcommand)]
pub(crate) enum AuditCommand {
    /// Audit core crates architecture boundary tests and entrypoint line limits.
    Crate {
        #[arg(long)]
        json: bool,
    },
    /// Scan repository for leaked credentials, tokens, or private keys.
    Secret {
        #[arg(long)]
        json: bool,
    },
    /// Audit node-host integration and commit ledger bounds.
    NodeHost {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 80)]
        max_commits: usize,
    },
    /// Audit declared filesystem paths against the observed working tree.
    Paths {
        #[arg(long)]
        json: bool,
    },
    /// Run all dynamic audits (crates, secrets, node-host) and emit unified verdict.
    All {
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_audit(command: &AuditCommand) -> ExitCode {
    match command {
        AuditCommand::Crate { json } => {
            let report = crate_audit::audit_core_crates();
            let is_ok = report.blocked_count == 0;
            if *json {
                emit_json(&report, !is_ok)
            } else {
                println!(
                    "Crate architecture audit: {} crates, {} ok, {} blocked",
                    report.crate_count, report.ok_count, report.blocked_count
                );
                if is_ok {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
        }
        AuditCommand::Secret { json } => {
            let observation = cynic_release::adapters::secret_lint::scan_for_secrets(Path::new("."));
            let is_ok = !observation.has_findings();
            if *json {
                emit_json(&observation, !is_ok)
            } else {
                println!("Secret lint: {}", observation.summary());
                if is_ok {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
        }
        AuditCommand::NodeHost { json, max_commits } => {
            match node_host_audit::build_node_host_audit(*max_commits) {
                Ok(report) => {
                    if *json {
                        emit_json(&report, false)
                    } else {
                        println!("Node host audit: ok");
                        ExitCode::SUCCESS
                    }
                }
                Err(error) => fail(error),
            }
        }
        AuditCommand::Paths { json } => {
            // Projection only. The rule set, the extraction, and the existence
            // observation belong to cynic-host::audit_engine, which MCP and the
            // workspace audit already consume. A surface projects; it does not own.
            let report = cynic_host::audit_engine::run_path_audit(&crate::rust_source::workspace_root());
            let blocked = report.summary.status == "blocked";
            if *json {
                emit_json(&report, blocked)
            } else {
                println!(
                    "Path audit: {} rules, {} critical, {} warning",
                    report.summary.total_rules_evaluated,
                    report.summary.critical_count,
                    report.summary.warning_count
                );
                if blocked {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
        }
        AuditCommand::All { json } => {
            let crate_report = crate_audit::audit_core_crates();
            let secret_obs = cynic_release::adapters::secret_lint::scan_for_secrets(Path::new("."));
            let node_audit = node_host_audit::build_node_host_audit(80).ok();

            let all_passed = crate_report.blocked_count == 0 && !secret_obs.has_findings();

            let unified = serde_json::json!({
                "schema": "cynic.cli.audit_unified.v1",
                "status": if all_passed { "ok" } else { "blocked" },
                "crates": {
                    "total": crate_report.crate_count,
                    "ok": crate_report.ok_count,
                    "blocked": crate_report.blocked_count,
                },
                "secrets": {
                    "files_scanned": secret_obs.files_scanned,
                    "findings_count": secret_obs.findings.len(),
                },
                "node_host": {
                    "available": node_audit.is_some(),
                }
            });

            if *json {
                emit_json(&unified, !all_passed)
            } else {
                println!("=== CYNIC UNIFIED AUDIT ===");
                println!("Status: {}", if all_passed { "PASS" } else { "FAIL" });
                println!(
                    "Crates: {}/{} OK",
                    crate_report.ok_count, crate_report.crate_count
                );
                println!(
                    "Secrets: {} findings ({} files scanned)",
                    secret_obs.findings.len(),
                    secret_obs.files_scanned
                );
                if all_passed {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
        }
    }
}
