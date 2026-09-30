//! `cynic secret use` and `cynic secret audit` — capability-based secret consumption.
//!
//! WHY THIS EXISTS. `cynic secret status/check/resolve/install` answer "is a secret
//! present and where". None of them answers the question an agent actually has:
//! "may I use this secret, for this purpose, under my role — and if so, run this
//! command with it injected, without ever seeing the value". That is the
//! capability-based gatekeeper, and until now it had a policy, a pure decision
//! function and an adapter, but no surface. An authorization nobody can invoke is
//! an authorization that does not exist.
//!
//! WHAT THIS SURFACE DOES. It wires `LocalSecretUseGate` (which reads
//! `infra/agent-roles.json` through `secret_use_policy_loader`) to the CLI. Two
//! verbs:
//!
//! - `use`  — authorize a (role, secret, purpose) request and, for a
//!   value-exposing purpose, execute an argv with the value injected into the
//!   child's environment. The value never appears in this process's stdout,
//!   and the child's own output is scanned for accidental echo and redacted.
//! - `audit` — read back the append-only JSONL audit trail. One line per
//!   request, allowed or denied, never a value.
//!
//! WHAT THIS SURFACE CANNOT DO. It cannot return the value. `SecretUseOutcome`
//! has no field for it, `SecretUseExecutionReport` has no field for it, and
//! `SecretUseAuditEntry` has no field for it. The types are the enforcement, not
//! this file's discipline.
//!
//! THE ROLE IS NOT ASSERTED BY THE CALLER. `--role` is required and is checked
//! against the declared policy. It is not trusted for authority: the policy is
//! the authority, and an undeclared role is denied. The flag exists so the
//! request is attributable, which is what the audit trail needs.

use std::path::PathBuf;
use std::process::ExitCode;

use cynic_host::adapters::local_secret_use_gate::{default_audit_path, resolve_via_composite};
use cynic_host::adapters::secret_use_policy_loader::{
    load_policy_from_file, resolve_declaration_path,
};
use cynic_protocol::secret_reference::{SecretClass, SecretProvider, SecretReference};
use cynic_protocol::secret_use::{SecretUsePurpose, SecretUseRequest};
use cynic_secrets::ports::SecretUseGatePort;
use cynic_secrets::protocol::{SecretUseExecutionReport, SecretUseOutcome};
use serde::Serialize;

use crate::{emit_json, fail};

#[derive(Debug, Serialize)]
struct SecretUseDecisionReport {
    schema: &'static str,
    role: String,
    agent_id: String,
    secret_id: String,
    purpose: String,
    /// "allowed" or "denied" — the verdict, never a value.
    verdict: &'static str,
    /// Machine-readable denial reason when refused.
    denial_reason: Option<String>,
    /// Handle token when allowed. Opaque; carries no value.
    handle_token: Option<String>,
    handle_expires_at_unix_ms: Option<u64>,
    /// Child execution report when `--argv` was given.
    execution: Option<SecretUseExecutionReport>,
}

#[derive(Debug, Serialize)]
struct SecretAuditReport {
    schema: &'static str,
    path: Option<String>,
    total_lines: usize,
    shown: usize,
    entries: Vec<serde_json::Value>,
}

fn parse_class(raw: &str) -> Result<SecretClass, String> {
    match raw {
        "human_admin" => Ok(SecretClass::HumanAdmin),
        "machine_enrollment" => Ok(SecretClass::MachineEnrollment),
        "runtime_machine_identity" => Ok(SecretClass::RuntimeMachineIdentity),
        "recovery_breakglass" => Ok(SecretClass::RecoveryBreakglass),
        other => Err(format!(
            "unknown secret class '{other}'. Expected one of: human_admin, machine_enrollment, \
             runtime_machine_identity, recovery_breakglass"
        )),
    }
}

fn parse_purpose(raw: &str) -> Result<SecretUsePurpose, String> {
    match raw {
        "observe_posture" => Ok(SecretUsePurpose::ObservePosture),
        "inject_process" => Ok(SecretUsePurpose::InjectProcess),
        "rotate" => Ok(SecretUsePurpose::Rotate),
        "install" => Ok(SecretUsePurpose::Install),
        "delete" => Ok(SecretUsePurpose::Delete),
        other => Err(format!(
            "unknown purpose '{other}'. Expected one of: observe_posture, inject_process, \
             rotate, install, delete"
        )),
    }
}

/// Load the declared policy, or the fail-closed empty policy when the
/// declaration cannot be read. A missing declaration is never a licence.
fn load_policy() -> (cynic_protocol::secret_use::SecretUsePolicy, Option<PathBuf>) {
    let declaration = resolve_declaration_path(None);
    match declaration {
        Some(path) => (load_policy_from_file(&path), Some(path)),
        None => (cynic_protocol::secret_use::SecretUsePolicy::deny_all(), None),
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_use(
    role: &str,
    agent: &str,
    id: &str,
    class: &str,
    purpose: &str,
    claim: Option<String>,
    why: Option<String>,
    env_var: Option<String>,
    argv: Vec<String>,
    json: bool,
) -> ExitCode {
    let secret_class = match parse_class(class) {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let use_purpose = match parse_purpose(purpose) {
        Ok(p) => p,
        Err(e) => return fail(e),
    };

    // A value-exposing purpose without an argv would mint a handle nobody
    // consumes. Refuse the shape rather than leave a live handle behind.
    if use_purpose.exposes_value() && argv.is_empty() {
        return fail(
            "purpose inject_process requires a command: pass it after `--`. \
             Use `observe_posture` to authorize without executing.",
        );
    }
    if !argv.is_empty() && env_var.is_none() {
        return fail("--argv requires --env-var <NAME> to name the injected variable");
    }
    if !argv.is_empty() && !use_purpose.exposes_value() {
        return fail(format!(
            "purpose {purpose} does not expose a value; drop --argv or use inject_process"
        ));
    }

    let (policy, declaration_path) = load_policy();

    // The resolver is the composite provider (env -> principal file -> OS store).
    // It is only ever called by `execute_with_secret`, after an Allowed verdict.
    let gate = cynic_host::adapters::local_secret_use_gate::LocalSecretUseGate::new(
        policy,
        resolve_via_composite,
    );

    let reference = SecretReference {
        id: id.to_string(),
        class: secret_class,
        provider: SecretProvider::PrincipalOwnedFile,
        locator: id.to_string(),
    };

    let mut request = SecretUseRequest::new(agent, role, reference, use_purpose);
    if let Some(lease) = claim {
        request = request.with_claim(lease);
    }
    if let Some(text) = why {
        request = request.with_justification(text);
    }

    let outcome = gate.authorize(&request);

    match outcome {
        SecretUseOutcome::Denied { reason } => {
            let report = SecretUseDecisionReport {
                schema: "cynic.secret_use_decision.v1",
                role: role.to_string(),
                agent_id: agent.to_string(),
                secret_id: id.to_string(),
                purpose: purpose.to_string(),
                verdict: "denied",
                denial_reason: Some(format!("{reason:?}")),
                handle_token: None,
                handle_expires_at_unix_ms: None,
                execution: None,
            };
            if json {
                emit_json(&report, true)
            } else {
                eprintln!("DENIED: role '{role}' may not use secret '{id}' for '{purpose}'");
                eprintln!("reason: {reason:?}");
                if declaration_path.is_none() {
                    eprintln!(
                        "note: infra/agent-roles.json was not found; the policy was empty, \
                         which denies everything"
                    );
                }
                ExitCode::FAILURE
            }
        }
        SecretUseOutcome::Allowed { handle } => {
            // With no argv this is a pure authorization check: the handle is
            // reported and dropped, and the value was never resolved.
            let execution = if argv.is_empty() {
                None
            } else {
                let env_name = env_var.unwrap_or_default();
                match gate.execute_with_secret(&handle, &env_name, &argv) {
                    Ok(report) => Some(report),
                    Err(e) => {
                        return fail(format!("authorized but execution failed: {e}"));
                    }
                }
            };

            let failed = execution.as_ref().is_some_and(|e| !e.success);
            let report = SecretUseDecisionReport {
                schema: "cynic.secret_use_decision.v1",
                role: role.to_string(),
                agent_id: agent.to_string(),
                secret_id: id.to_string(),
                purpose: purpose.to_string(),
                verdict: "allowed",
                denial_reason: None,
                handle_token: Some(handle.token.clone()),
                handle_expires_at_unix_ms: Some(handle.expires_at_unix_ms),
                execution,
            };

            if json {
                emit_json(&report, failed)
            } else if failed {
                eprintln!("ALLOWED, but the child command failed.");
                ExitCode::FAILURE
            } else {
                println!(
                    "ALLOWED: role '{}' may use secret '{}' for '{}'",
                    report.role, report.secret_id, report.purpose
                );
                if let Some(exec) = &report.execution {
                    println!("exit_code: {:?}", exec.exit_code);
                    if exec.output_redacted {
                        println!("note: the child echoed the value; it was redacted");
                    }
                    print!("{}", exec.stdout);
                    eprint!("{}", exec.stderr);
                }
                ExitCode::SUCCESS
            }
        }
    }
}

pub(crate) fn execute_audit(limit: usize, json: bool) -> ExitCode {
    let path = default_audit_path();
    let Some(path) = path else {
        let report = SecretAuditReport {
            schema: "cynic.secret_use_audit_readback.v1",
            path: None,
            total_lines: 0,
            shown: 0,
            entries: Vec::new(),
        };
        return if json {
            emit_json(&report, false)
        } else {
            println!("no audit path resolved; no secret use has been recorded on this host");
            ExitCode::SUCCESS
        };
    };

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let report = SecretAuditReport {
                schema: "cynic.secret_use_audit_readback.v1",
                path: Some(path.display().to_string()),
                total_lines: 0,
                shown: 0,
                entries: Vec::new(),
            };
            return if json {
                emit_json(&report, false)
            } else {
                println!("no audit trail yet at {}", path.display());
                ExitCode::SUCCESS
            };
        }
        Err(e) => return fail(format!("cannot read audit trail {}: {e}", path.display())),
    };

    let lines: Vec<&str> = content.lines().filter(|l| !l.trim().is_empty()).collect();
    let total = lines.len();
    let tail: Vec<serde_json::Value> = lines
        .iter()
        .rev()
        .take(limit)
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();

    let report = SecretAuditReport {
        schema: "cynic.secret_use_audit_readback.v1",
        path: Some(path.display().to_string()),
        total_lines: total,
        shown: tail.len(),
        entries: tail,
    };

    if json {
        emit_json(&report, false)
    } else {
        println!("Secret-use audit trail: {}", path.display());
        println!("{total} entr(y/ies) recorded, showing {}", report.shown);
        println!();
        for entry in &report.entries {
            let verdict = entry
                .get("decision")
                .and_then(|d| d.get("verdict"))
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let role = entry.get("role").and_then(|v| v.as_str()).unwrap_or("?");
            let secret = entry.get("secret_id").and_then(|v| v.as_str()).unwrap_or("?");
            let purpose = entry.get("purpose").and_then(|v| v.as_str()).unwrap_or("?");
            let reason = entry
                .get("decision")
                .and_then(|d| d.get("reason"))
                .and_then(|v| v.as_str())
                .map(|r| format!(" ({r})"))
                .unwrap_or_default();
            println!("  {verdict:<8} {role:<22} {secret:<24} {purpose}{reason}");
        }
        ExitCode::SUCCESS
    }
}
