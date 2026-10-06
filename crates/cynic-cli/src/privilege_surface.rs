//! `cynic privilege` — capability catalog and privileged execution.
//!
//! WHY THIS EXISTS. CYNIC grew five privileged binaries, five sudoers
//! membranes, and three independent implementations of the same `pct` argv
//! (`cynic-lxc::admit` in Rust, `pct-bounded-exec.sh` in shell,
//! `proxmox_clone.rs` in Rust). Three implementations of one policy diverge
//! silently, and the shell one cannot share `admit()` with Rust at all. This
//! surface is the single entry point: one catalog, one admission, one
//! journaled exec.
//!
//! WHAT THIS SURFACE DOES. Three verbs:
//!
//! - `catalog` — read back the declared capability catalog and the installed
//!   set. Read-only, no privilege.
//! - `admit`   — resolve a capability id + parameters into the exact argv
//!   that may run, WITHOUT running it. This is the dry-run an operator or an
//!   audit uses to see what a capability would execute.
//! - `exec`    — admit, journal (capability id, resolved argv, exit code),
//!   then execute. No shell: the argv is built from the catalog template and
//!   passed to the process directly.
//!
//! THE BOUNDARY IS THE ALLOWLIST, NOT THE BINARY. A capability absent from
//! the catalog, absent from the installed set, missing a parameter, carrying
//! an undeclared parameter, or holding a value that fails its kind is
//! refused before any process starts. The sudoers membrane names the exact
//! argv `/usr/local/bin/cynic privilege exec <id> ...`, so the kernel and the
//! catalog agree on what may run as root.

use std::process::ExitCode;

use cynic_host::privilege::capability_catalog::{
    CapabilityView, catalog_path_from_env, installed_path_from_env,
};
use cynic_protocol::privilege::CapabilityDecision;
use serde::Serialize;

use crate::{emit_json, fail};

#[derive(Debug, Serialize)]
struct CatalogReport {
    schema: &'static str,
    status: &'static str,
    catalog_path: String,
    installed_path: String,
    mandatory: Vec<String>,
    capabilities: Vec<CapabilityEntry>,
    blockers: Vec<String>,
}

#[derive(Debug, Serialize)]
struct CapabilityEntry {
    id: String,
    domain: String,
    requires_root: bool,
    mandatory: bool,
    installed: bool,
    argv_template: Vec<String>,
    params: Vec<ParamEntry>,
    purpose: String,
}

#[derive(Debug, Serialize)]
struct ParamEntry {
    name: String,
    kind: String,
}

#[derive(Debug, Serialize)]
struct AdmitReport {
    schema: &'static str,
    status: &'static str,
    capability_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    argv: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    requires_root: Option<bool>,
    blockers: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ExecReport {
    schema: &'static str,
    status: &'static str,
    capability_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    argv: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    exit_code: Option<i32>,
    blockers: Vec<String>,
}

/// Parse `--param name=value` occurrences into a parameter list.
///
/// A malformed `--param` is a refusal, not a skipped argument: silently
/// dropping a parameter would execute a capability with an incomplete argv.
fn parse_params(raw: &[String]) -> Result<Vec<(String, String)>, String> {
    let mut params = Vec::new();
    let mut index = 0;
    while index < raw.len() {
        if raw[index] == "--param" {
            let Some(pair) = raw.get(index + 1) else {
                return Err("--param requires name=value".to_string());
            };
            let Some((name, value)) = pair.split_once('=') else {
                return Err(format!("--param {pair} is not name=value"));
            };
            if name.trim().is_empty() {
                return Err("--param name is empty".to_string());
            }
            params.push((name.to_string(), value.to_string()));
            index += 2;
        } else {
            index += 1;
        }
    }
    Ok(params)
}

/// Extract the capability id: the first argument that is not a `--param` pair.
fn parse_capability_id(raw: &[String]) -> Result<String, String> {
    let mut index = 0;
    while index < raw.len() {
        if raw[index] == "--param" {
            index += 2;
            continue;
        }
        return Ok(raw[index].clone());
    }
    Err("capability id is required".to_string())
}

fn load_view() -> Result<CapabilityView, String> {
    let catalog_path = catalog_path_from_env();
    let installed_path = installed_path_from_env();
    CapabilityView::load(&catalog_path, &installed_path).map_err(|error| error.0)
}

fn execute_catalog(json: bool) -> ExitCode {
    let catalog_path = catalog_path_from_env();
    let installed_path = installed_path_from_env();

    let view = match CapabilityView::load(&catalog_path, &installed_path) {
        Ok(view) => view,
        Err(error) => {
            let report = CatalogReport {
                schema: "cynic.privilege.catalog.v1",
                status: "blocked",
                catalog_path: catalog_path.display().to_string(),
                installed_path: installed_path.display().to_string(),
                mandatory: Vec::new(),
                capabilities: Vec::new(),
                blockers: vec![error.0],
            };
            emit_json(&report, json);
            return ExitCode::FAILURE;
        }
    };

    let capabilities = view
        .catalog
        .capabilities
        .iter()
        .map(|capability| CapabilityEntry {
            id: capability.id.clone(),
            domain: capability.domain.clone(),
            requires_root: capability.requires_root,
            mandatory: capability.mandatory,
            installed: view.installed.contains(&capability.id),
            argv_template: capability.argv_template.clone(),
            params: capability
                .params
                .iter()
                .map(|param| ParamEntry {
                    name: param.name.clone(),
                    kind: format!("{:?}", param.kind),
                })
                .collect(),
            purpose: capability.purpose.clone(),
        })
        .collect();

    let report = CatalogReport {
        schema: "cynic.privilege.catalog.v1",
        status: "ok",
        catalog_path: catalog_path.display().to_string(),
        installed_path: installed_path.display().to_string(),
        mandatory: view.mandatory_ids(),
        capabilities,
        blockers: Vec::new(),
    };
    emit_json(&report, json);
    ExitCode::SUCCESS
}

fn execute_admit(raw: &[String], json: bool) -> ExitCode {
    let id = match parse_capability_id(raw) {
        Ok(id) => id,
        Err(error) => return fail(error),
    };
    let params = match parse_params(raw) {
        Ok(params) => params,
        Err(error) => return fail(error),
    };
    let view = match load_view() {
        Ok(view) => view,
        Err(error) => return fail(error),
    };

    match view.admit(&id, &params) {
        CapabilityDecision::Admitted {
            capability_id,
            argv,
            requires_root,
            ..
        } => {
            let report = AdmitReport {
                schema: "cynic.privilege.admit.v1",
                status: "admitted",
                capability_id,
                argv: Some(argv),
                requires_root: Some(requires_root),
                blockers: Vec::new(),
            };
            emit_json(&report, json);
            ExitCode::SUCCESS
        }
        CapabilityDecision::Blocked { blockers } => {
            let report = AdmitReport {
                schema: "cynic.privilege.admit.v1",
                status: "blocked",
                capability_id: id,
                argv: None,
                requires_root: None,
                blockers,
            };
            emit_json(&report, json);
            ExitCode::FAILURE
        }
    }
}

fn execute_exec(raw: &[String], json: bool) -> ExitCode {
    let id = match parse_capability_id(raw) {
        Ok(id) => id,
        Err(error) => return fail(error),
    };
    let params = match parse_params(raw) {
        Ok(params) => params,
        Err(error) => return fail(error),
    };
    let view = match load_view() {
        Ok(view) => view,
        Err(error) => return fail(error),
    };

    let (argv, requires_root, capability_id) = match view.admit(&id, &params) {
        CapabilityDecision::Admitted {
            capability_id,
            argv,
            requires_root,
            ..
        } => (argv, requires_root, capability_id),
        CapabilityDecision::Blocked { blockers } => {
            let report = ExecReport {
                schema: "cynic.privilege.exec.v1",
                status: "blocked",
                capability_id: id,
                argv: None,
                exit_code: None,
                blockers,
            };
            emit_json(&report, json);
            return ExitCode::FAILURE;
        }
    };

    // Journal before the process starts: an invocation that dies mid-flight
    // must still be attributable to a capability id.
    journal(&capability_id, &argv, requires_root);

    let Some((program, args)) = argv.split_first() else {
        return fail("admitted argv is empty");
    };
    let status = std::process::Command::new(program).args(args).status();

    let (exit_code, succeeded) = match status {
        Ok(status) => (status.code(), status.success()),
        Err(error) => {
            let report = ExecReport {
                schema: "cynic.privilege.exec.v1",
                status: "failed",
                capability_id: capability_id.clone(),
                argv: Some(argv.clone()),
                exit_code: None,
                blockers: vec![format!("spawn_failed:{error}")],
            };
            emit_json(&report, json);
            return ExitCode::FAILURE;
        }
    };

    let report = ExecReport {
        schema: "cynic.privilege.exec.v1",
        status: if succeeded { "executed" } else { "failed" },
        capability_id,
        argv: Some(argv),
        exit_code,
        blockers: Vec::new(),
    };
    emit_json(&report, json);
    if succeeded {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

/// Append one journal line: capability id, resolved argv, whether root was
/// required. The line never carries a secret; the argv is a capability
/// template already validated at admission.
fn journal(capability_id: &str, argv: &[String], requires_root: bool) {
    let line = format!(
        "capability={capability_id} requires_root={requires_root} argv={}",
        argv.join(" ")
    );
    let path = std::env::var_os("CYNIC_PRIVILEGE_JOURNAL")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from("/var/log/cynic/privilege.jsonl"));
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        use std::io::Write;
        let _ = writeln!(file, "{line}");
    }
}

#[derive(Debug, Serialize)]
struct InstallReport {
    schema: &'static str,
    status: &'static str,
    subject: String,
    output_path: String,
    rules: Vec<String>,
    blockers: Vec<String>,
}

#[derive(Debug, clap::Subcommand)]
pub(crate) enum PrivilegeCommand {
    /// Read back the declared capability catalog and the installed set.
    Catalog {
        #[arg(long)]
        json: bool,
    },
    /// Resolve a capability into the exact argv that may run (no execution).
    Admit {
        /// Capability id followed by `--param name=value` occurrences.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Admit, journal, and execute a capability.
    Exec {
        /// Capability id followed by `--param name=value` occurrences.
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
        #[arg(long)]
        json: bool,
    },
    /// Generate the sudoers file from the capability catalog.
    ///
    /// One sudoers line per `requires_root` capability, with `{param}`
    /// placeholders replaced by `*` (sudoers wildcard). The result is written
    /// to `/etc/sudoers.d/cynic` (or `$CYNIC_SUDOERS_PATH`).
    Install {
        /// The user who will be granted sudo access (default: cynic).
        #[arg(long, default_value = "cynic")]
        subject: String,
        /// Output path for the sudoers file.
        #[arg(long, default_value = "/etc/sudoers.d/cynic")]
        output: String,
        /// Machine-readable JSON report.
        #[arg(long)]
        json: bool,
    },
}

fn execute_install(subject: &str, output: &str, json: bool) -> ExitCode {
    let view = match load_view() {
        Ok(view) => view,
        Err(error) => return fail(error),
    };

    let rules = view.catalog.sudoers_rules(subject);
    if rules.is_empty() {
        let report = InstallReport {
            schema: "cynic.privilege.install.v1",
            status: "blocked",
            subject: subject.to_string(),
            output_path: output.to_string(),
            rules: Vec::new(),
            blockers: vec!["no_root_capabilities_in_catalog".to_string()],
        };
        emit_json(&report, json);
        return ExitCode::FAILURE;
    }

    let content = format!("{}\n", rules.join("\n"));
    if let Err(error) = std::fs::write(output, content) {
        let report = InstallReport {
            schema: "cynic.privilege.install.v1",
            status: "failed",
            subject: subject.to_string(),
            output_path: output.to_string(),
            rules,
            blockers: vec![format!("write_failed:{error}")],
        };
        emit_json(&report, json);
        return ExitCode::FAILURE;
    }

    let report = InstallReport {
        schema: "cynic.privilege.install.v1",
        status: "installed",
        subject: subject.to_string(),
        output_path: output.to_string(),
        rules,
        blockers: Vec::new(),
    };
    emit_json(&report, json);
    ExitCode::SUCCESS
}

pub(crate) fn execute_privilege_command(command: PrivilegeCommand) -> ExitCode {
    match command {
        PrivilegeCommand::Catalog { json } => execute_catalog(json),
        PrivilegeCommand::Admit { args, json } => execute_admit(&args, json),
        PrivilegeCommand::Exec { args, json } => execute_exec(&args, json),
        PrivilegeCommand::Install {
            subject,
            output,
            json,
        } => execute_install(&subject, &output, json),
    }
}
