//! `cynic secret` — sovereign runtime secret posture, observation, and bounded installation.
//!
//! Conforms to `contracts/foundation.json` and foundation invariants:
//! - Secrets are never printed in cleartext, logs, or reports.
//! - Resolution order: ProcessEnvironment -> PrincipalOwnedFile -> OsCredentialStore.
//! - Read-only observation with machine-readable JSON support.
//! - Bounded installation enforces strict 0600 permissions and immediate read-back proof.

use std::io::Read;
use std::process::ExitCode;

use clap::Subcommand;
use cynic_host::secret_presence::{
    RuntimeSecretPresenceReport, RuntimeSecretPresenceSpec, observe_runtime_secret_presence,
};
use cynic_secrets::{
    CompositeSecretProvider, EncryptedFileSecretProvider, SecretReference, SecretStorePort,
};
use serde::Serialize;

use crate::{emit_json, fail};

#[derive(Subcommand)]
pub(crate) enum SecretCommand {
    /// Observe presence and provenance of all declared runtime secrets.
    Status {
        #[arg(long)]
        json: bool,
    },
    /// Conformance check: fail-closed if any critical runtime secret is missing.
    Check {
        #[arg(long)]
        json: bool,
    },
    /// Verify resolution for a specific secret identifier without revealing its value.
    Resolve {
        /// Secret identifier (e.g. proxmox_control, storage_runtime, cloudflare, etc.)
        id: String,
        #[arg(long)]
        json: bool,
    },
    /// Securely install a secret to principal storage (~/.config/cynic/secrets/<id>) with strict 0600 permissions.
    Install {
        /// Secret identifier (e.g. huggingface, openai, anthropic, cloudflare, etc.)
        id: String,
        /// Take value from an existing environment variable
        #[arg(long = "from-env")]
        from_env: Option<String>,
        /// Take value from an existing file
        #[arg(long)]
        file: Option<String>,
        /// Read value interactively from stdin
        #[arg(long)]
        stdin: bool,
        /// Explicit consent to persist the secret
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        json: bool,
    },
    /// Authorize a secret use under a declared role, and optionally run argv
    /// with the value injected. The value never enters this process's output.
    Use {
        /// The declared role the agent acts under. Checked against
        /// `infra/agent-roles.json`; an undeclared role is denied.
        #[arg(long)]
        role: String,
        /// Agent identity, for attribution in the audit trail.
        #[arg(long)]
        agent: String,
        /// Secret identifier (e.g. forgejo, proxmox_control).
        #[arg(long)]
        id: String,
        /// Secret class: human_admin | machine_enrollment | runtime_machine_identity | recovery_breakglass.
        #[arg(long)]
        class: String,
        /// Purpose: observe_posture | inject_process | rotate | install | delete.
        #[arg(long)]
        purpose: String,
        /// Optional CYNIC claim lease id, required when the grant demands one.
        #[arg(long)]
        claim: Option<String>,
        /// Optional justification, recorded in the audit trail.
        #[arg(long)]
        why: Option<String>,
        /// Environment variable name the child receives. Required with `--argv`.
        #[arg(long = "env-var")]
        env_var: Option<String>,
        /// Command to execute with the secret injected. Everything after `--`.
        #[arg(last = true)]
        argv: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Read back the append-only secret-use audit trail.
    Audit {
        /// Number of trailing entries to show.
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Serialize)]
struct SecretPostureReport {
    schema: &'static str,
    status: &'static str,
    total_specs: usize,
    present_count: usize,
    absent_count: usize,
    secrets: Vec<RuntimeSecretPresenceReport>,
}

#[derive(Debug, Serialize)]
struct SecretResolveReport {
    schema: &'static str,
    id: String,
    resolved: bool,
    source: Option<String>,
}

#[derive(Debug, Serialize)]
struct SecretInstallReport {
    schema: &'static str,
    status: &'static str,
    id: String,
    target_path: String,
    mode: &'static str,
    mutated: bool,
    verified_present: bool,
}

pub(crate) fn execute_secret_command(command: SecretCommand) -> ExitCode {
    match command {
        SecretCommand::Status { json } => execute_status(json),
        SecretCommand::Check { json } => execute_check(json),
        SecretCommand::Resolve { id, json } => execute_resolve(&id, json),
        SecretCommand::Install {
            id,
            from_env,
            file,
            stdin,
            yes,
            json,
        } => execute_install(&id, from_env, file, stdin, yes, json),
        SecretCommand::Use {
            role,
            agent,
            id,
            class,
            purpose,
            claim,
            why,
            env_var,
            argv,
            json,
        } => crate::secret_use_surface::execute_use(
            &role, &agent, &id, &class, &purpose, claim, why, env_var, argv, json,
        ),
        SecretCommand::Audit { limit, json } => {
            crate::secret_use_surface::execute_audit(limit, json)
        }
    }
}

fn execute_status(json: bool) -> ExitCode {
    let specs = RuntimeSecretPresenceSpec::all_runtime_specs();
    let reports: Vec<_> = specs.iter().map(observe_runtime_secret_presence).collect();

    let present_count = reports.iter().filter(|r| r.present).count();
    let unknown_count = reports
        .iter()
        .filter(|r| r.presence == cynic_host::secret_presence::RuntimeSecretPresence::Unknown)
        .count();
    let absent_count = reports.len() - present_count - unknown_count;

    let overall_status = if absent_count == 0 && unknown_count == 0 {
        "all_present"
    } else if unknown_count > 0 && present_count == 0 && absent_count == 0 {
        "all_unknown_behind_permission_boundary"
    } else if present_count > 0 {
        "partially_present"
    } else {
        "all_absent"
    };

    let report = SecretPostureReport {
        schema: "cynic.secret_posture.v1",
        status: overall_status,
        total_specs: reports.len(),
        present_count,
        absent_count,
        secrets: reports,
    };

    if json {
        emit_json(&report, false)
    } else {
        println!("CYNIC Runtime Secrets Posture");
        println!("============================");
        println!(
            "Status: {} ({}/{} present)",
            report.status, report.present_count, report.total_specs
        );
        println!();
        for s in &report.secrets {
            let status_mark = if s.present {
                "[OK]"
            } else if s.presence == cynic_host::secret_presence::RuntimeSecretPresence::Unknown {
                "[UNKNOWN]"
            } else {
                "[MISSING]"
            };
            let source_str = s
                .source
                .map(|src| format!("{src:?}"))
                .unwrap_or_else(|| "none".to_string());
            println!(
                "  {:<10} {:<25} source={:<20}",
                status_mark, s.identity_id, source_str
            );
        }
        ExitCode::SUCCESS
    }
}

fn execute_check(json: bool) -> ExitCode {
    let specs = RuntimeSecretPresenceSpec::all_runtime_specs();
    let reports: Vec<_> = specs.iter().map(observe_runtime_secret_presence).collect();

    let missing_ids: Vec<String> = reports
        .iter()
        .filter(|r| !r.present)
        .map(|r| r.identity_id.clone())
        .collect();
    let failed = !missing_ids.is_empty();

    let report = SecretPostureReport {
        schema: "cynic.secret_posture_check.v1",
        status: if failed {
            "blocked_missing_secrets"
        } else {
            "ok"
        },
        total_specs: reports.len(),
        present_count: reports.len() - missing_ids.len(),
        absent_count: missing_ids.len(),
        secrets: reports,
    };

    if json {
        emit_json(&report, failed)
    } else if failed {
        eprintln!(
            "FAIL: {} secret(s) missing for runtime posture:",
            missing_ids.len()
        );
        for m in &missing_ids {
            eprintln!("  - {m}");
        }
        ExitCode::FAILURE
    } else {
        println!(
            "PASS: All {} runtime secrets verified present.",
            report.total_specs
        );
        ExitCode::SUCCESS
    }
}

fn execute_resolve(id: &str, json: bool) -> ExitCode {
    let specs = RuntimeSecretPresenceSpec::all_runtime_specs();
    let matched = specs.iter().find(|s| s.identity_id == id);

    let Some(spec) = matched else {
        if json {
            let report = SecretResolveReport {
                schema: "cynic.secret_resolve.v1",
                id: id.to_string(),
                resolved: false,
                source: None,
            };
            return emit_json(&report, true);
        } else {
            return fail(format!("unknown secret identity '{id}'"));
        }
    };

    let observation = observe_runtime_secret_presence(spec);
    let report = SecretResolveReport {
        schema: "cynic.secret_resolve.v1",
        id: id.to_string(),
        resolved: observation.present,
        source: observation.source.map(|s| format!("{s:?}")),
    };

    if json {
        emit_json(&report, !observation.present)
    } else if observation.present {
        println!(
            "Secret '{}' is resolved via {:?}",
            id,
            observation.source.unwrap()
        );
        ExitCode::SUCCESS
    } else {
        eprintln!("Secret '{}' could NOT be resolved.", id);
        ExitCode::FAILURE
    }
}

fn map_filename(id: &str) -> &str {
    match id {
        "storage_runtime" => "storage-runtime-password",
        "proxmox_control" => "proxmox-token",
        "cloudflare" => "cloudflare-api-token",
        "porkbun" => "porkbun-api-key",
        "forgejo" => "forgejo-token",
        "headscale_key" | "headscale" => "headscale-api-key",
        "telegram_bot" => "telegram-bot-token",
        "telegram_chat" => "telegram-chat-id",
        "huggingface" => "hf-token",
        "openai" => "openai-api-key",
        "anthropic" => "anthropic-api-key",
        other => other,
    }
}

fn execute_install(
    id: &str,
    from_env: Option<String>,
    file: Option<String>,
    stdin: bool,
    yes: bool,
    json: bool,
) -> ExitCode {
    // The sealed vault resolves its own store location (system principal
    // store when present, user store otherwise). Shown in plan mode.
    let planned_file = {
        use cynic_secrets::EncryptedFileSecretProvider;
        EncryptedFileSecretProvider::default()
            .base_dir
            .join(map_filename(id))
    };

    if !yes {
        let report = SecretInstallReport {
            schema: "cynic.secret_install.v1",
            status: "plan_only",
            id: id.to_string(),
            target_path: planned_file.display().to_string(),
            mode: "0600",
            mutated: false,
            verified_present: false,
        };
        if json {
            return emit_json(&report, false);
        } else {
            println!(
                "Plan: install secret '{}' sealed to {}",
                id,
                planned_file.display()
            );
            println!("Permissions: mode 0600 (owner read-only), AES-256-GCM sealed");
            println!("Provide --yes to apply.");
            return ExitCode::SUCCESS;
        }
    }

    // Resolve input value
    let mut secret_bytes = Vec::new();
    if let Some(var_name) = from_env {
        match std::env::var(&var_name) {
            Ok(v) if !v.trim().is_empty() => secret_bytes = v.trim().as_bytes().to_vec(),
            _ => {
                return fail(format!(
                    "environment variable '{var_name}' is empty or absent"
                ));
            }
        }
    } else if let Some(file_path) = file {
        match std::fs::read(&file_path) {
            Ok(bytes) if !bytes.is_empty() => secret_bytes = bytes,
            _ => return fail(format!("source file '{file_path}' is empty or unreadable")),
        }
    } else if stdin {
        let mut buffer = Vec::new();
        if let Err(e) = std::io::stdin().take(65536).read_to_end(&mut buffer) {
            return fail(format!("failed to read secret from stdin: {e}"));
        }
        while buffer.last() == Some(&b'\n') || buffer.last() == Some(&b'\r') {
            buffer.pop();
        }
        if buffer.is_empty() {
            return fail("empty secret received on stdin");
        }
        secret_bytes = buffer;
    } else {
        return fail("must specify source for secret: --from-env <VAR>, --file <PATH>, or --stdin");
    }

    // The sealed vault owns its directories; nothing to pre-create here.
    // The report path below names where the vault stores this id.
    let target_file = EncryptedFileSecretProvider::default()
        .base_dir
        .join(map_filename(id));

    // Write through the sealed vault: the value is encrypted under a fresh
    // random data key before it touches disk. Never store plaintext.
    let vault = CompositeSecretProvider::default();
    let reference = SecretReference::with_class(
        id,
        map_filename(id),
        cynic_secrets::SecretClass::RuntimeMachineIdentity,
        cynic_secrets::SecretProviderKind::PrincipalOwnedFile,
    );
    let write_res = vault.store(&reference, &String::from_utf8_lossy(&secret_bytes));

    secret_bytes.fill(0);

    if let Err(e) = write_res {
        return fail(format!(
            "failed to write secret to {}: {e}",
            target_file.display()
        ));
    }

    // Read-back verification
    let spec = RuntimeSecretPresenceSpec::new(id, &[], map_filename(id));
    let observation = observe_runtime_secret_presence(&spec);

    let report = SecretInstallReport {
        schema: "cynic.secret_install.v1",
        status: if observation.present {
            "installed"
        } else {
            "failed_read_back"
        },
        id: id.to_string(),
        target_path: target_file.display().to_string(),
        mode: "0600",
        mutated: true,
        verified_present: observation.present,
    };

    if json {
        emit_json(&report, !observation.present)
    } else if observation.present {
        println!(
            "SUCCESS: Secret '{}' securely installed at {} (mode 0600).",
            id,
            target_file.display()
        );
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "ERROR: Secret '{}' written but read-back observation failed.",
            id
        );
        ExitCode::FAILURE
    }
}
