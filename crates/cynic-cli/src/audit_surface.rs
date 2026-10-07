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
    /// Triage branches by measurement: does a ref still describe a world that
    /// exists, and does it carry patches the base has never seen?
    ///
    /// Named `branch`, not `branch-audit`: no CLI command carries a hyphen
    /// (operator rule, 2026-09-28). It lives here rather than under `dev`
    /// because `cynic audit` is the entry point `make check` runs, and `dev`
    /// is not declared in the crate graph at all — a command the binary cannot
    /// reach is a rule nobody can apply.
    Branch {
        /// Base ref the refs are compared against.
        #[arg(long, default_value = "main")]
        base: String,
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
            let observation =
                cynic_release::adapters::secret_lint::scan_for_secrets(Path::new("."));
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
        AuditCommand::Branch { base, json } => {
            if !json {
                return ExitCode::FAILURE;
            }
            // Deliberately NOT part of `audit all`: a branch being parked is a
            // normal repository state, not a defect of the build. Folding it
            // into the gate would make `make check` red for the wrong reason.
            let report = crate::branch_audit::audit_branches(base);
            let blocked = report.status != "ok";
            emit_json(&report, blocked);
            if blocked {
                return ExitCode::FAILURE;
            }
            ExitCode::SUCCESS
        }
        AuditCommand::Paths { json } => {
            // Projection only. The rule set, the extraction, and the existence
            // observation belong to cynic-audit-engine, which MCP and the
            // workspace audit already consume. A surface projects; it does not own.
            let report =
                cynic_audit_engine::run_path_audit(&crate::rust_source::workspace_root());
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

            // The audit engine is the workspace-wide rule set: every default
            // rule over every crate, with the declared debt ledger applied.
            //
            // It runs HERE because `audit all` is `make audit`, the canonical
            // gate named by AGENTS.md. Until this call existed the engine
            // executed only through a separate binary and the tests --
            // comparators that exist, run, and are never consulted by the gate.
            // That binary is gone; the fact it uniquely carried, `host_observed`,
            // is emitted below. The three checks answer different questions.
            let root = crate::rust_source::workspace_root();
            let engine = cynic_audit_engine::run_workspace_audit(&root);

            let all_passed = crate_report.blocked_count == 0
                && !secret_obs.has_findings()
                && engine.summary.critical_count == 0;

            let unified = serde_json::json!({
                "schema": "cynic.cli.audit_unified.v1",
                "status": if all_passed { "ok" } else { "blocked" },
                // Whether the runtime domain was actually checked. A reader must
                // not have to infer it from the absence of runtime findings.
                "host_observed": host_was_observed(&root),
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
                },
                "engine": {
                    "rules_evaluated": engine.summary.total_rules_evaluated,
                    "passed_rules": engine.summary.passed_rules_count,
                    "critical": engine.summary.critical_count,
                    "warning": engine.summary.warning_count,
                    "debt": engine.summary.debt_count,
                    "maturity_score_percent": engine.summary.maturity_score_percent,
                    "status": engine.summary.status,
                    "violations": engine
                        .violations
                        .iter()
                        .map(|v| serde_json::json!({
                            "code": v.code,
                            "severity": v.severity,
                            "message": v.message,
                            "remediation": v.remediation,
                        }))
                        .collect::<Vec<_>>(),
                }
            });

            if *json {
                emit_json(&unified, !all_passed)
            } else {
                println!("=== CYNIC UNIFIED AUDIT ===");
                println!("Status: {}", if all_passed { "PASS" } else { "FAIL" });
                println!("Host observed: {}", host_was_observed(&root));
                println!(
                    "Crates: {}/{} OK",
                    crate_report.ok_count, crate_report.crate_count
                );
                println!(
                    "Secrets: {} findings ({} files scanned)",
                    secret_obs.findings.len(),
                    secret_obs.files_scanned
                );
                println!(
                    "Engine: {}/{} rules passed, {} critical, {} warning, {} debt, maturity {}%",
                    engine.summary.passed_rules_count,
                    engine.summary.total_rules_evaluated,
                    engine.summary.critical_count,
                    engine.summary.warning_count,
                    engine.summary.debt_count,
                    engine.summary.maturity_score_percent
                );
                for violation in &engine.violations {
                    println!("  [{}] {}", violation.code, violation.message);
                }
                if all_passed {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
        }
    }
}

/// Whether the audit engine could observe a live host on this run.
///
/// Carried from the `cynic-audit` binary this arm absorbed. The fact is not
/// cosmetic: `WorkspaceSourceIndex::host` is an `Option`, and the scanner
/// documents `None` as "not observed here" — an explicit unknown rather than a
/// pass. Without this field a report with no runtime-domain finding reads
/// identically whether the host was observed or not, and a domain that stopped
/// being checked looks identical to a domain that is clean.
///
/// Re-asked here rather than read off the report so the answer is about the
/// machine this ran on, not about a field someone may later forget to update.
fn host_was_observed(root: &Path) -> bool {
    for dir in ["/etc/systemd/system", "/usr/lib/systemd/system"] {
        if std::path::Path::new(dir).is_dir() {
            return true;
        }
    }
    // No systemd here: the honest answer is still whether the scan saw a root
    // it could work with, so a non-host run is reported as not observed.
    root.is_dir()
        && std::env::var("CYNIC_AUDIT_UNIT_DIRS")
            .map(|value| !value.trim().is_empty())
            .unwrap_or(false)
}
