//! `cynic-audit` — the audit surface that links.
//!
//! WHY THIS BINARY EXISTS. `AGENTS.md` and the `Makefile` both name one
//! validation gate, and both reach it through `cynic`:
//!
//!     $(CYNIC_BIN) audit all --json
//!     cargo run -q -p cynic-cli -- audit all --json
//!
//! Neither can run. `cynic` is a 25 182-line CLI depending on nineteen crates,
//! and it does not link on the pinned toolchain — `target/debug/cynic` does not
//! exist on this host. So the gate the contracts name, the one the divergence
//! audit measured as a divergence in its own right, cannot be executed at all,
//! and every check that lives behind it is unreachable in practice.
//!
//! This binary is the same audit engine with no CLI surface attached. It depends
//! on `cynic-host` only, so it links in seconds and reports the identical report:
//! the rules are shared code, not a second implementation. It is also the
//! answer, in miniature, to the question this repository has been circling — a
//! deployment unit per role, small enough to link on its own.
//!
//! WHAT IT DOES NOT DO. It does not judge. It runs the rules, prints the report
//! as JSON, and exits non-zero when a Critical violation exists, so a CI step is
//! one line and a human reads the same artefact the gate read.
//!
//! IT NEVER PRINTS A SECRET. RUN-001-CREDENTIAL-IN-UNIT reports the unit file and
//! the variable name; the value is not in the engine and therefore cannot be in
//! this output.

#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use serde_json::json;

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("cynic-audit: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let root = workspace_root()?;
    let report = cynic_host::audit_engine::run_workspace_audit(&root);

    // The envelope says whether the host was observed, because a report with no
    // finding in the runtime domain reads exactly like a report where the
    // runtime domain is clean. Those are different facts and the difference is
    // the whole point of the RUN-001 rule.
    let host_observed = host_was_observed(&root);

    let critical: Vec<&cynic_protocol::AuditViolation> = report
        .violations
        .iter()
        .filter(|violation| {
            matches!(violation.severity, cynic_protocol::AuditSeverity::Critical)
        })
        .collect();

    let envelope = json!({
        "schema": "cynic.audit.envelope.v1",
        "root": root.display().to_string(),
        "host_observed": host_observed,
        "evaluated_rules": report.evaluated_rules.len(),
        "violations_total": report.violations.len(),
        "violations_critical": critical.len(),
        "report": report,
    });

    println!(
        "{}",
        serde_json::to_string_pretty(&envelope).map_err(|e| e.to_string())?
    );

    // A gate that always exits zero is a printer. One that exits non-zero on
    // every finding is a blocker nobody can ship against; Critical is the line.
    if critical.is_empty() {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

/// Whether the engine could observe a live host for this run.
///
/// Re-asked here rather than read off the report so the answer is about the
/// machine this ran on, not about a field someone may later forget to update.
fn host_was_observed(root: &std::path::Path) -> bool {
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

fn workspace_root() -> Result<PathBuf, String> {
    if let Ok(explicit) = std::env::var("CYNIC_WORKSPACE_ROOT") {
        let path = PathBuf::from(explicit);
        if path.join("crates").is_dir() && path.join("Cargo.toml").is_file() {
            return Ok(path);
        }
        return Err(format!(
            "CYNIC_WORKSPACE_ROOT={} holds no Cargo.toml and crates/",
            path.display()
        ));
    }
    // CARGO_MANIFEST_DIR is .../crates/cynic-host when cargo runs this, and the
    // working directory is the workspace when it does not. Both are checked, and
    // neither is assumed.
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    // crates/cynic-host -> crates -> workspace root
    for ancestor in manifest.ancestors().skip(1) {
        if ancestor.join("Cargo.toml").is_file() && ancestor.join("crates").is_dir() {
            return Ok(ancestor.to_path_buf());
        }
    }
    let cwd = std::env::current_dir().map_err(|e| e.to_string())?;
    if cwd.join("Cargo.toml").is_file() && cwd.join("crates").is_dir() {
        return Ok(cwd);
    }
    Err(format!(
        "no workspace root: neither {} nor the current directory holds Cargo.toml and crates/",
        manifest.display()
    ))
}
