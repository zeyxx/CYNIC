//! `cynic-host-exec pve-helper-install` — the organism self-installs its
//! bounded helper on the Proxmox host.
//!
//! Authority model (user law 2026-08-25): cynic (the organism, its
//! root-owned binaries) is NOT the agent (a cortex). The agent never writes
//! install shell lines; instead the organism's own binary renders the exact
//! privileged argv from the declared policy, runs it through `sudo` over SSH,
//! and proves convergence by independent observation. The human grants sudo
//! ONCE to this exact binary behavior — the password prompt travels through
//! `ssh -t`; no human password ever transits a chat.
//!
//! Consent: HumanAdmin class — requires CYNIC_HUMAN_ADMIN_CONSENT=pve-helper-install
//! in the same run, plus --yes.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::Command;

use serde::Serialize;

use crate::host_exec_cli::args;
use crate::host_exec_cli::io::emit_json;
use crate::privilege::{
    DEFAULT_POLICY_PATH, DEFAULT_TRANSPORT_POLICY_PATH, PveHelperPolicy, TransportCapabilities,
};

const PVE_HELPER_INSTALL_REPORT_SCHEMA: &str = "cynic.host.pve_helper_install_report.v1";

#[derive(Debug, Serialize)]
struct InstallStep {
    role: String,
    argv: Vec<String>,
    exit_code: Option<i32>,
    ok: bool,
}

#[derive(Debug, Serialize)]
struct PveHelperInstallReport {
    schema: &'static str,
    status: String,
    endpoint: String,
    helper_target: String,
    sudoers_target: String,
    staged_dir: String,
    steps: Vec<InstallStep>,
    blockers: Vec<String>,
}

/// The exact privileged argv the organism installs under. Rendered from the
/// policy, not hand-written: install helper 0755, install generated sudoers
/// 0440, visudo-validate, PATH shim.
fn render_install_argv(policy: &PveHelperPolicy, staged_dir: &str) -> Vec<String> {
    let remote_sh = format!(
        "set -e; \
         install -m 0755 -o root -g root /tmp/pct-bounded-exec {helper} && \
         install -m 0440 -o root -g root /tmp/pve-pct.sudoers {sudoers} && \
         visudo -cf {sudoers} && \
         ln -sf {pct} /usr/local/bin/pct && \
         echo BOOTSTRAP_DONE",
        helper = policy.helper_path,
        sudoers = policy.sudoers_path,
        pct = policy.pct_path,
    );
    vec![
        "ssh".to_string(),
        "-t".to_string(),
        "sudo".to_string(),
        "sh".to_string(),
        "-c".to_string(),
        remote_sh,
    ]
}

pub(crate) fn execute_pve_helper_install(
    args_slice: &[String],
) -> Result<std::process::ExitCode, String> {
    let mut report = PveHelperInstallReport {
        schema: PVE_HELPER_INSTALL_REPORT_SCHEMA,
        status: "blocked".to_string(),
        endpoint: String::new(),
        helper_target: String::new(),
        sudoers_target: String::new(),
        staged_dir: String::new(),
        steps: Vec::new(),
        blockers: Vec::new(),
    };

    if !args::has_flag(args_slice, "--apply") || !args::has_flag(args_slice, "--yes") {
        report.blockers.push(
            "mutation_requires_explicit_flags: pass both --apply and --yes (consent is not privilege)"
                .to_string(),
        );
        return finish(report);
    }
    let consent = std::env::var("CYNIC_HUMAN_ADMIN_CONSENT").ok();
    if consent.as_deref() != Some("pve-helper-install") {
        report.blockers.push(
            "human_admin_consent_missing: set CYNIC_HUMAN_ADMIN_CONSENT=pve-helper-install \
             in the same run after the human approved this exact behavior"
                .to_string(),
        );
        return finish(report);
    }

    let policy_path =
        args::flag_value(args_slice, "--policy").unwrap_or_else(|| DEFAULT_POLICY_PATH.to_string());
    let Ok(policy) = PveHelperPolicy::load(&policy_path) else {
        report
            .blockers
            .push(format!("policy_unreadable_or_invalid: {policy_path}"));
        return finish(report);
    };
    report.helper_target = policy.helper_path.clone();
    report.sudoers_target = policy.sudoers_path.clone();

    // Endpoint: transport policy overrides > fleet registry > blocked.
    let transport_policy_path = args::flag_value(args_slice, "--transport-policy")
        .unwrap_or_else(|| DEFAULT_TRANSPORT_POLICY_PATH.to_string());
    let endpoint = resolve_pve_endpoint(&transport_policy_path).and_then(|resolved| {
        if resolved.is_empty() {
            None
        } else {
            Some(resolved)
        }
    });
    let Some(endpoint) = endpoint else {
        report.blockers.push(
            "endpoint_unresolved: no pve ssh target from transport policy or registry".to_string(),
        );
        return finish(report);
    };
    report.endpoint = endpoint.clone();

    // Staged artifacts must exist locally (pve-helper-bundle ran first).
    let staged_dir = args::flag_value(args_slice, "--bundle-dir")
        .unwrap_or_else(|| "/tmp/pve-bundle-test".to_string());
    report.staged_dir = staged_dir.clone();
    for artifact in ["pct-bounded-exec", "pve-pct.sudoers"] {
        let path = PathBuf::from(&staged_dir).join(artifact);
        if !path.is_file() {
            report
                .blockers
                .push(format!("missing_staged_artifact: {}", path.display()));
        }
    }
    if !report.blockers.is_empty() {
        return finish(report);
    }

    // Step 1: stage both artifacts via scp (machine identity, unprivileged).
    let stage_argv = vec![
        "scp".to_string(),
        PathBuf::from(&staged_dir)
            .join("pct-bounded-exec")
            .display()
            .to_string(),
        PathBuf::from(&staged_dir)
            .join("pve-pct.sudoers")
            .display()
            .to_string(),
        format!("{endpoint}:/tmp/"),
    ];
    let stage_step = run_step("stage_artifacts", &stage_argv);
    let stage_ok = stage_step.ok;
    report.steps.push(stage_step);
    if !stage_ok {
        report.blockers.push("stage_failed".to_string());
        return finish(report);
    }

    // Step 2: the organism installs itself through sudo over an interactive
    // SSH tunnel; the human's password prompt lives only in their terminal.
    let mut install_argv = render_install_argv(&policy, &staged_dir);
    install_argv.insert(2, endpoint.clone());
    let install_step = run_step_with_stdio("install_via_sudo", &install_argv, true);
    let install_ok = install_step.ok;
    report.steps.push(install_step);
    if !install_ok {
        report.blockers.push("install_failed".to_string());
        return finish(report);
    }

    // Step 3: independent read-back through the bounded membrane (no sudo):
    // probe must succeed now that NOPASSWD exists.
    let probe_argv = vec![
        "ssh".to_string(),
        "-o".to_string(),
        "BatchMode=yes".to_string(),
        endpoint,
        "/usr/local/sbin/pct-bounded-exec list".to_string(),
    ];
    let probe_step = run_step("readback_bounded_list", &probe_argv);
    report.steps.push(probe_step);
    if !report.steps.last().expect("probe step").ok {
        report
            .blockers
            .push("readback_failed: membrane not answering via NOPASSWD".to_string());
        return finish(report);
    }

    report.status = "completed".to_string();
    finish(report)
}

fn resolve_pve_endpoint(transport_policy_path: &str) -> Option<String> {
    if let Ok(policy) = TransportCapabilities::load(transport_policy_path) {
        if let Some(endpoint) = policy.default_endpoint_overrides.get("cynic-host-proxmox") {
            return Some(endpoint.clone());
        }
    }
    let raw = std::fs::read_to_string("infra/registry.json").ok()?;
    let value: serde_json::Value = serde_json::from_str(&raw).ok()?;
    value
        .get("nodes")?
        .as_array()?
        .iter()
        .find(|node| {
            node.get("id").and_then(serde_json::Value::as_str) == Some("cynic-host-proxmox")
        })
        .and_then(|node| node.pointer("/transports/ssh/target"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

fn run_step(role: &str, argv: &[String]) -> InstallStep {
    run_step_with_stdio(role, argv, false)
}

/// `interactive` inherits the parent's stdin/stdout/stderr so an ssh `-t`
/// session can prompt for a sudo password on the real TTY. Piped stdio would
/// make ssh drop PTY allocation ("stdin is not a terminal") and sudo would
/// refuse to ask for a password — the exact failure this surface exists to
/// avoid during HumanAdmin bootstrap.
fn run_step_with_stdio(role: &str, argv: &[String], interactive: bool) -> InstallStep {
    let (program, rest) = argv.split_first().expect("argv non-empty");
    let mut command = Command::new(program);
    command.args(rest);
    if interactive {
        use std::io::IsTerminal;
        if !std::io::stdin().is_terminal() {
            // No real TTY behind us: fail fast with the honest reason instead
            // of letting sudo hang or silently refuse.
            eprintln!(
                "REFUSED: no terminal on stdin; the sudo password prompt needs \
                 your interactive shell (run this command in your terminal)"
            );
            return InstallStep {
                role: role.to_string(),
                argv: argv.to_vec(),
                exit_code: None,
                ok: false,
            };
        }
        command.stdin(std::process::Stdio::inherit());
        command.stdout(std::process::Stdio::inherit());
        command.stderr(std::process::Stdio::inherit());
    } else {
        command.stdin(std::process::Stdio::null());
    }
    let output = command.output();
    match output {
        Ok(output) => {
            if !interactive {
                eprint!("{}", String::from_utf8_lossy(&output.stderr));
                print!("{}", String::from_utf8_lossy(&output.stdout));
            }
            let ok = output.status.success();
            InstallStep {
                role: role.to_string(),
                argv: argv.to_vec(),
                exit_code: output.status.code(),
                ok,
            }
        }
        Err(error) => {
            eprintln!("spawn failed: {error}");
            InstallStep {
                role: role.to_string(),
                argv: argv.to_vec(),
                exit_code: None,
                ok: false,
            }
        }
    }
}

fn finish(report: PveHelperInstallReport) -> Result<std::process::ExitCode, String> {
    let failed = report.status != "completed";
    emit_json(&report, failed)
}
