use std::path::Path;
use std::process::ExitCode;

use clap::Subcommand;

use crate::{crate_audit, emit_json, external_audit, fail, node_host_audit};

pub(crate) mod host_checks;

#[derive(Subcommand)]
pub(crate) enum AuditNodeCommand {
    /// Audit node/host integration and commit ledger bounds.
    Host {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 80)]
        max_commits: usize,
    },
}

pub(crate) fn execute_audit_node(command: &AuditNodeCommand) -> ExitCode {
    match command {
        AuditNodeCommand::Host { json, max_commits } => {
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
    }
}

#[derive(Subcommand)]
pub(crate) enum AuditGateCommand {
    /// Verify a machine-readable gate receipt artifact (audit the audit).
    Receipt {
        #[arg(long, default_value = "gate-receipt.json")]
        path: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand)]
pub(crate) enum AuditCommand {
    /// Audit core crates architecture boundary tests and entrypoint line limits.
    Crate {
        #[arg(long)]
        json: bool,
    },
    /// Audit workspace crate topology, layer rules (L0-L4), cycles, and declared edges.
    ///
    /// With the `topology_write` feature compiled in, `--write --apply` regenerates
    /// the derived half of the topology SSOT. The default binary carries no write
    /// path at all: the flags do not exist unless the feature is enabled.
    Topology {
        #[arg(long)]
        json: bool,
        #[cfg(feature = "topology_write")]
        #[arg(long)]
        write: bool,
        #[cfg(feature = "topology_write")]
        #[arg(long)]
        apply: bool,
    },
    /// Scan repository for leaked credentials, tokens, or private keys.
    Secret {
        #[arg(long)]
        json: bool,
    },
    /// Audit node/host integration and commit ledger bounds (nested arm).
    Node {
        #[command(subcommand)]
        command: AuditNodeCommand,
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
    /// Audit declared guest boot persistence against the observed provider
    /// state. Catches deploy targets that vanish silently at host reboot.
    Persistence {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        /// Observed guest state as JSON: `{"node-id":"boot_persistent"}`.
        /// Values: `boot_persistent`, `not_boot_persistent`, `unobserved`.
        /// Omit to audit the declared side alone.
        #[arg(long)]
        observed: Option<String>,
    },
    /// Run all dynamic audits (crates, secrets, node host) and emit unified verdict.
    All {
        #[arg(long)]
        json: bool,
    },
    /// Observe host environment conformance against declared capability contracts.
    Conformance {
        #[command(flatten)]
        profile: crate::host_profile_selection::ProfileArgs,
    },
    /// Generate verified runtime metrics from host conformance observation.
    Metrics {
        #[command(flatten)]
        profile: crate::host_profile_selection::ProfileArgs,
        #[arg(long)]
        proof: Option<String>,
    },
    /// Inspect runtime harness catalog and provider execution environments.
    Harness {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        kind: Option<String>,
    },
    /// Read-back of the external (non-CYNIC) project registry (infra/external-project-registry.json, normative).
    External {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
        /// Run the full external project audit orchestration (clone → build → ingest).
        #[arg(long)]
        run: bool,
        /// Project ID to audit (required with --run).
        #[arg(long)]
        project: Option<String>,
        /// Builder URL (defaults to resolved pool URL).
        #[arg(long)]
        builder_url: Option<String>,
        /// Promote/migrate an external project's tier or mutation policy.
        #[arg(long)]
        promote: Option<String>,
        /// Target tier for --promote: served, dev, or contribute.
        #[arg(long)]
        to: Option<String>,
        /// Confirm the mutation (required for --promote).
        #[arg(long)]
        yes: bool,
        /// Forgejo API token (for Forgejo provider).
        #[arg(long)]
        forgejo_token: Option<String>,
        /// GitHub API token (for GitHub provider).
        #[arg(long)]
        github_token: Option<String>,
        /// Evidence store directory (defaults to .cynic/evidence).
        #[arg(long)]
        evidence_dir: Option<String>,
    },
    /// Verify a machine-readable gate receipt artifact (audit the audit).
    Gate {
        #[command(subcommand)]
        command: AuditGateCommand,
    },
}

#[derive(serde::Serialize)]
pub(crate) struct GateReceiptReport {
    pub schema: &'static str,
    pub path: String,
    pub valid: bool,
    pub verdict: Option<String>,
    pub sha: Option<String>,
    pub run_id: Option<String>,
    pub gates: Option<serde_json::Value>,
    pub errors: Vec<String>,
}

pub(crate) fn execute_gate_receipt(path: &str, json: bool) -> ExitCode {
    let raw = match std::fs::read_to_string(path) {
        Ok(r) => r,
        Err(e) => {
            let report = GateReceiptReport {
                schema: "cynic.audit.gate_receipt.v1",
                path: path.to_string(),
                valid: false,
                verdict: None,
                sha: None,
                run_id: None,
                gates: None,
                errors: vec![format!("unreadable: {e}")],
            };
            if json {
                crate::cli_json::emit_pretty_json(&report);
            } else {
                eprintln!("blocked: {}", report.errors.join("; "));
            }
            return ExitCode::FAILURE;
        }
    };

    let value: serde_json::Value = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            let report = GateReceiptReport {
                schema: "cynic.audit.gate_receipt.v1",
                path: path.to_string(),
                valid: false,
                verdict: None,
                sha: None,
                run_id: None,
                gates: None,
                errors: vec![format!("invalid json: {e}")],
            };
            if json {
                crate::cli_json::emit_pretty_json(&report);
            } else {
                eprintln!("blocked: {}", report.errors.join("; "));
            }
            return ExitCode::FAILURE;
        }
    };

    let mut errors = Vec::new();
    for key in ["gate", "verdict", "sha", "run_id", "ts", "gates"] {
        if value.get(key).is_none() {
            errors.push(format!("missing field: {key}"));
        }
    }
    let verdict = value.get("verdict").and_then(|v| v.as_str()).map(str::to_string);
    let sha = value.get("sha").and_then(|v| v.as_str()).map(str::to_string);
    let run_id = value.get("run_id").and_then(|v| v.as_str()).map(str::to_string);
    let gates = value.get("gates").cloned();

    if let Some(v) = &verdict {
        if v != "PASS" && v != "FAIL" {
            errors.push(format!("invalid verdict: {v}"));
        }
    }

    let valid = errors.is_empty();
    let report = GateReceiptReport {
        schema: "cynic.audit.gate_receipt.v1",
        path: path.to_string(),
        valid,
        verdict,
        sha,
        run_id,
        gates,
        errors: errors.clone(),
    };

    if json {
        crate::cli_json::emit_pretty_json(&report);
    } else if valid {
        println!("gate-receipt: valid (verdict={})", report.verdict.clone().unwrap_or_default());
    } else {
        eprintln!("blocked: {}", report.errors.join("; "));
    }

    if valid && report.verdict.as_deref() == Some("PASS") {
        ExitCode::SUCCESS
    } else if valid {
        ExitCode::FAILURE
    } else {
        ExitCode::FAILURE
    }
}

/// Store convergence observation: compares main SHA across source layers.
#[derive(Debug, Clone)]
struct StoreConvergence {
    local_sha: Option<String>,
    origin_sha: Option<String>,
    github_sha: Option<String>,
    converged: bool,
}

impl StoreConvergence {
    fn to_json(&self) -> serde_json::Value {
        serde_json::json!({
            "converged": self.converged,
            "local": self.local_sha,
            "origin": self.origin_sha,
            "github": self.github_sha,
        })
    }
}

fn git_rev_parse(refname: &str) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", refname])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() { None } else { Some(sha) }
}

fn check_store_convergence() -> StoreConvergence {
    let local = git_rev_parse("main");
    let origin = git_rev_parse("origin/main");
    let github = git_rev_parse("github/main");
    let converged = match (&local, &origin, &github) {
        (Some(l), Some(o), Some(g)) => l == o && o == g,
        (Some(l), Some(o), None) => l == o,
        (Some(l), None, None) => true,
        _ => false,
    };
    StoreConvergence {
        local_sha: local,
        origin_sha: origin,
        github_sha: github,
        converged,
    }
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
        AuditCommand::Node { command } => execute_audit_node(command),
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
            let report = cynic_audit_engine::run_path_audit(&crate::rust_source::workspace_root());
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
        #[cfg(feature = "topology_write")]
        AuditCommand::Topology { json, write, apply } => {
            // Compile-time gated writer: the flag pair does not exist in the
            // default binary, so a reader build cannot write even by accident.
            if *write {
                return crate::topology_write::execute(*json, *apply);
            }
            execute_topology_read(*json)
        }
        #[cfg(not(feature = "topology_write"))]
        AuditCommand::Topology { json } => execute_topology_read(*json),
        AuditCommand::Persistence {
            json,
            registry,
            observed,
        } => crate::persistence_audit::execute(*json, registry, observed),
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

            // Store convergence gate: main must agree across all source layers.
            // A divergent main means the audit is measuring a moving target.
            let store_convergence = check_store_convergence();
            let store_ok = store_convergence.converged;

            let all_passed = crate_report.blocked_count == 0
                && !secret_obs.has_findings()
                && engine.summary.critical_count == 0
                && store_ok;

            let unified = serde_json::json!({
                "schema": "cynic.cli.audit_unified.v1",
                "status": if all_passed { "ok" } else { "blocked" },
                // Whether the runtime domain was actually checked. A reader must
                // not have to infer it from the absence of runtime findings.
                "host_observed": host_was_observed(&root),
                "store_convergence": store_convergence.to_json(),
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
                println!(
                    "Store convergence: {} (local={}, origin={}, github={})",
                    if store_ok { "OK" } else { "DIVERGENT" },
                    store_convergence.local_sha.as_deref().unwrap_or("?"),
                    store_convergence.origin_sha.as_deref().unwrap_or("?"),
                    store_convergence.github_sha.as_deref().unwrap_or("?"),
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
        AuditCommand::Conformance { profile } => {
            host_checks::execute_conformance(profile.json, profile)
        }
        AuditCommand::Metrics { profile, proof } => {
            host_checks::execute_metrics(profile.json, profile, proof.as_deref())
        }
        AuditCommand::Harness { json, kind } => {
            host_checks::execute_harness(*json, kind.as_deref())
        }
        AuditCommand::External {
            json,
            registry,
            run,
            project,
            builder_url,
            promote,
            to,
            yes,
            forgejo_token,
            github_token,
            evidence_dir,
        } => {
            if *run {
                external_audit::execute_run(
                    *json,
                    project.as_deref(),
                    builder_url.as_deref(),
                    None,
                    forgejo_token.as_deref(),
                    github_token.as_deref(),
                    evidence_dir.as_deref(),
                )
            } else if let Some(slug) = promote {
                external_audit::promote(
                    *json,
                    slug,
                    to.as_deref(),
                    *yes,
                    registry.as_deref(),
                )
            } else {
                external_audit::execute(*json, registry.as_deref())
            }
        }
        AuditCommand::Gate { command } => match command {
            AuditGateCommand::Receipt { path, json } => execute_gate_receipt(&path, *json),
        },
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

/// Read-only topology audit: ARCH-004/ARCH-005 violations plus SSOT freshness.
///
/// The committed SSOT must match what the manifests and policy keys render. No
/// file is written here; regeneration lives behind the `topology_write` feature
/// in `topology_write::execute`.
fn execute_topology_read(json: bool) -> ExitCode {
    let root = crate::rust_source::workspace_root();
    let engine = cynic_audit_engine::run_workspace_audit(&root);
    let topology_violations: Vec<_> = engine
        .violations
        .into_iter()
        .filter(|v| v.code.starts_with("ARCH-004") || v.code.starts_with("ARCH-005"))
        .collect();

    let committed_result = cynic_audit_engine::topology_generate::committed(&root);
    let rendered_result = cynic_audit_engine::topology_generate::render_text(&root);
    let render_error = committed_result
        .as_ref()
        .err()
        .or(rendered_result.as_ref().err())
        .map(|e| e.to_string());
    let committed_text = committed_result.ok();
    let rendered_text = rendered_result.ok();
    // Freshness is a semantic comparison. A byte mismatch with an empty
    // structural diff (e.g. a trailing newline) is not a stale SSOT.
    let freshness_diff = match (committed_text.as_ref(), rendered_text.as_ref()) {
        (Some(committed), Some(rendered)) => {
            cynic_audit_engine::topology_generate::diff_paths(committed, rendered)
                .unwrap_or_default()
        }
        _ => Vec::new(),
    };
    // Retain an explicit failure when rendering or committing could not be read.
    let render_failed = committed_text.is_none() || rendered_text.is_none();
    let is_fresh = !render_failed && freshness_diff.is_empty();

    let blocked = !topology_violations.is_empty() || !is_fresh || render_failed;

    if json {
        let payload = serde_json::json!({
            "schema": "cynic.cli.audit_topology.v1",
            "status": if blocked { "blocked" } else { "ok" },
            "violations_count": topology_violations.len(),
            "violations": topology_violations,
            "ssot_fresh": is_fresh,
            "freshness_diff_count": freshness_diff.len(),
            "freshness_diff": freshness_diff,
            "render_error": render_error,
        });
        emit_json(&payload, blocked)
    } else {
        println!("=== CYNIC Workspace Topology Audit (ARCH-004 / ARCH-005) ===");
        if !topology_violations.is_empty() {
            println!(
                "Status: BLOCKED ({} violations found)",
                topology_violations.len()
            );
            for v in &topology_violations {
                println!("\n  [{}] {}", v.code, v.message);
                if let Some(ref rem) = v.remediation {
                    println!("    Remediation: {}", rem);
                }
            }
        } else if !is_fresh {
            println!(
                "Status: BLOCKED (SSOT stale, {} differences)",
                freshness_diff.len()
            );
            for diff in freshness_diff.iter().take(10) {
                println!("  {}", diff);
            }
        } else {
            println!(
                "Status: OK (Zero architectural boundary violations, all layers and edges coherent; SSOT fresh)"
            );
        }
        if blocked {
            ExitCode::FAILURE
        } else {
            ExitCode::SUCCESS
        }
    }
}
