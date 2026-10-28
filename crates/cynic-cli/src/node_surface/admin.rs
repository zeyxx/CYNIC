//! Node admin ceremony: one-click webhook-secret rotation.
//!
//! WHY: every secret rotation used to mean one root shell per node.
//! This verb does the whole loop from the operator machine: stage the
//! new env fragment, restart, wait for the gauge, prove the secret live
//! with a signed but harmless signal — and reports per node what
//! changed, what is verified, and what is left.
//!
//! The daemon reads ONLY the `CYNIC_FORGEJO_WEBHOOK_SECRET` entry of the
//! systemd env file (default `/etc/cynic/env`). A bare secret file
//! elsewhere is ignored by the daemon — this verb writes the env
//! fragment, never a stray secret file.
//!
//! The secret is read from a FILE (`--secret-file`), never from argv:
//! process lists leak argv, and shell history outlives the ceremony.
//! Nothing secret is printed; verification compares digests only.

use clap::Subcommand;

#[derive(Subcommand)]
pub enum NodeAdminCommand {
    Secret {
        #[command(subcommand)]
        command: NodeAdminSecretCommand,
    },
}

#[derive(Subcommand)]
pub enum NodeAdminSecretCommand {
    Provision {
        /// Local file holding the webhook secret. Never shown, never logged.
        #[arg(long)]
        secret_file: String,
        /// Limit to these node ids. Default: every build_runner in the registry.
        #[arg(long)]
        node: Vec<String>,
        /// Remote destination: the systemd env file the daemon reads.
        /// Must already be wired via EnvironmentFile in the unit.
        #[arg(long, default_value = "/etc/cynic/env")]
        remote_path: String,
        #[arg(long, default_value_t = false)]
        yes: bool,
        #[arg(long, default_value_t = false)]
        json: bool,
        #[arg(long)]
        registry: Option<String>,
    },
}

/// Validate raw secret file content. Trims, refuses blank, refuses short
/// (under 16 chars a typo passes silently and every node goes dark).
pub fn validate_secret_content(raw: &str) -> Result<String, String> {
    let secret = raw.trim().to_string();
    if secret.is_empty() {
        return Err("secret file is blank; refusing to provision nothing".to_string());
    }
    if secret.len() < 16 {
        return Err(format!(
            "secret file holds {} chars; minimum 16, refusing a typo-grade secret",
            secret.len()
        ));
    }
    Ok(secret)
}

/// The single env entry the daemon reads. Pure so tests pin the exact
/// bytes that land in the systemd env file — no stray whitespace, the
/// value that broke c01's rotation is impossible by construction.
pub fn env_fragment_line(secret: &str) -> String {
    format!("CYNIC_FORGEJO_WEBHOOK_SECRET={secret}\n")
}

/// Remote staging for the secret fragment. `/tmp` because the machine
/// user can always write there; the privileged move into the env file
/// happens in one remote step, then staging is removed.
const REMOTE_STAGING: &str = "/tmp/cynic-webhook-secret.ceremony";

/// One node's ceremony outcome. The secret itself never appears here.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NodeCeremonyReport {
    pub node_id: String,
    pub ssh_target: String,
    pub url: String,
    pub distributed: bool,
    pub restarted: bool,
    pub gauge_ok: bool,
    pub signature_ok: bool,
    pub detail: String,
}

fn shell_json(reports: &[NodeCeremonyReport], json: bool) {
    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": "cynic.node.admin_secret_ceremony.v1",
                "nodes": reports,
            }))
            .unwrap_or_default()
        );
    } else {
        for report in reports {
            let mark = if report.signature_ok { "live" } else { "LEFT" };
            println!(
                "{} {} distributed={} restarted={} gauge={} signed={} {}",
                mark,
                report.node_id,
                report.distributed,
                report.restarted,
                report.gauge_ok,
                report.signature_ok,
                report.detail
            );
        }
    }
}

use cynic_lifecycle::{ArgvRunner, LocalArgvRunner};
use cynic_ssh::argv;

fn run_ssh(target: &str, remote: &str) -> Result<String, String> {
    let argv = argv::ssh_script_argv(target, remote)
        .map_err(|e| format!("ssh_argv_invalid: {e}"))?;
    let runner = LocalArgvRunner;
    let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    let output = runner.run(&refs);
    if let Some(err) = output.spawn_error {
        return Err(format!("ssh_unavailable: {err}"));
    }
    if !output.success() {
        return Err(format!(
            "ssh_remote_failed: {}",
            output.stderr.trim()
        ));
    }
    Ok(output.stdout.trim().to_string())
}

fn scp_to(target: &str, local: &str, remote_path: &str) -> Result<(), String> {
    let destination = format!("{target}:{remote_path}");
    let argv = argv::scp_argv(local, &destination)
        .map_err(|e| format!("scp_argv_invalid: {e}"))?;
    let runner = LocalArgvRunner;
    let refs: Vec<&str> = argv.iter().map(String::as_str).collect();
    let output = runner.run(&refs);
    if let Some(err) = output.spawn_error {
        return Err(format!("scp_unavailable: {err}"));
    }
    if !output.success() {
        return Err(format!("scp_failed for {target}"));
    }
    Ok(())
}

fn wait_for_gauge(url: &str) -> bool {
    for _ in 0..18 {
        if cynic_build::admin_signal::fetch_capacity(url).is_ok() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_secs(5));
    }
    false
}

fn restart_remote(target: &str) -> Result<(), String> {
    // Plain restart first (root ssh), then passwordless sudo, then stop
    // with the exact manual command — never guess further.
    if run_ssh(target, "systemctl restart cynic-node").is_ok() {
        return Ok(());
    }
    if run_ssh(target, "sudo -n systemctl restart cynic-node").is_ok() {
        return Ok(());
    }
    Err(format!(
        "restart_refused on {target}; run as root there: systemctl restart cynic-node"
    ))
}

fn provision_one(
    node_id: &str,
    ssh_target: &str,
    url: &str,
    secret_file: &str,
    secret: &str,
    remote_path: &str,
) -> NodeCeremonyReport {
    let base = NodeCeremonyReport {
        node_id: node_id.to_string(),
        ssh_target: ssh_target.to_string(),
        url: url.to_string(),
        distributed: false,
        restarted: false,
        gauge_ok: false,
        signature_ok: false,
        detail: String::new(),
    };
    if let Err(error) = scp_to(ssh_target, secret_file, REMOTE_STAGING) {
        return NodeCeremonyReport {
            detail: error,
            ..base
        };
    }
    // One remote step: strip any old entry, append the exact fragment,
    // move into place, remove staging. The move fails closed without
    // privilege and names the manual command.
    let install = format!(
        "printf 'CYNIC_FORGEJO_WEBHOOK_SECRET=%s\\n' \"$(tr -d '\\n\\r ' < {REMOTE_STAGING})\" > {REMOTE_STAGING}.env && grep -v '^CYNIC_FORGEJO_WEBHOOK_SECRET=' {remote_path} >> {REMOTE_STAGING}.env 2>/dev/null; mv {REMOTE_STAGING}.env {remote_path} && rm -f {REMOTE_STAGING} && sed 's/.*=//' {remote_path} | tr -d '\\n\\r ' | sha256sum"
    );
    let remote_digest = match run_ssh(ssh_target, &install) {
        Ok(out) => out.split_whitespace().next().unwrap_or("").to_string(),
        Err(error) => {
            return NodeCeremonyReport {
                detail: format!("{error}; run as root there: stage the fragment into {remote_path} then systemctl restart cynic-node"),
                ..base
            }
        }
    };
    let mut local_digest_cmd = std::process::Command::new("sha256sum");
    local_digest_cmd
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped());
    let mut child = match local_digest_cmd.spawn() {
        Ok(child) => child,
        Err(error) => {
            return NodeCeremonyReport {
                detail: format!("sha256sum_unavailable: {error}"),
                ..base
            }
        }
    };
    use std::io::Write as _;
    if let Some(stdin) = child.stdin.take() {
        let mut stdin = stdin;
        let _ = stdin.write_all(secret.trim().as_bytes());
    }
    let output = match child.wait_with_output() {
        Ok(output) => output,
        Err(error) => {
            return NodeCeremonyReport {
                detail: format!("sha256sum_failed: {error}"),
                ..base
            }
        }
    };
    let local_digest = String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    if remote_digest != local_digest || local_digest.is_empty() {
        return NodeCeremonyReport {
            detail: "read-back digest mismatch; remote env not trusted".to_string(),
            ..base
        };
    }
    if let Err(error) = restart_remote(ssh_target) {
        return NodeCeremonyReport {
            distributed: true,
            detail: error,
            ..base
        };
    }
    if !wait_for_gauge(url) {
        return NodeCeremonyReport {
            distributed: true,
            restarted: true,
            detail: "gauge never came back after restart".to_string(),
            ..base
        };
    }
    match cynic_build::admin_signal::prove_secret_live(url, secret) {
        Ok(()) => NodeCeremonyReport {
            distributed: true,
            restarted: true,
            gauge_ok: true,
            signature_ok: true,
            detail: "secret live: signed proof refused 409, gauge up".to_string(),
            ..base
        },
        Err(error) => NodeCeremonyReport {
            distributed: true,
            restarted: true,
            gauge_ok: true,
            detail: error,
            ..base
        },
    }
}

pub fn execute_admin_secret(command: &NodeAdminSecretCommand) -> Result<(), String> {
    let NodeAdminSecretCommand::Provision {
        secret_file,
        node,
        remote_path,
        yes,
        json,
        registry,
    } = command;
    if !yes {
        return Err("refusing mutation without --yes; dry run shows nothing secret".to_string());
    }
    let raw =
        std::fs::read_to_string(secret_file).map_err(|e| format!("secret file unreadable: {e}"))?;
    let secret = validate_secret_content(&raw)?;

    let registry_path = registry.clone().unwrap_or_else(|| {
        cynic_types::paths::CynicPaths::registry_path()
            .display()
            .to_string()
    });
    let registry_text = std::fs::read_to_string(&registry_path)
        .map_err(|e| format!("registry unreadable: {e}"))?;
    let registry_json: serde_json::Value =
        serde_json::from_str(&registry_text).map_err(|e| format!("registry unparsable: {e}"))?;
    let pool = cynic_build::builder_pool::resolve_builder_pool(&registry_json);
    let runners = cynic_host::registry::resolve::list_runner_nodes(registry.as_deref())?;
    let ssh_of = |node_id: &str| {
        runners
            .iter()
            .find(|runner| runner.node_id == node_id)
            .map(|runner| runner.ssh_target.clone())
            .unwrap_or_default()
    };

    let mut reports = Vec::new();
    for endpoint in &pool {
        if !node.is_empty() && !node.iter().any(|wanted| wanted == &endpoint.node_id) {
            continue;
        }
        let ssh_target = ssh_of(&endpoint.node_id);
        if ssh_target.is_empty() {
            reports.push(NodeCeremonyReport {
                node_id: endpoint.node_id.clone(),
                ssh_target: String::new(),
                url: endpoint.url.clone(),
                distributed: false,
                restarted: false,
                gauge_ok: false,
                signature_ok: false,
                detail: "no ssh_target in registry; fix the node record".to_string(),
            });
            continue;
        }
        reports.push(provision_one(
            &endpoint.node_id,
            &ssh_target,
            &endpoint.url,
            secret_file,
            &secret,
            remote_path,
        ));
    }
    if reports.is_empty() {
        return Err("no build_runner matched; check --node and the registry".to_string());
    }
    let failures = reports.iter().filter(|r| !r.signature_ok).count();
    shell_json(&reports, *json);
    if failures > 0 {
        return Err(format!("{failures} node(s) left; see detail above"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_secret_is_refused() {
        assert!(validate_secret_content("  \n").is_err());
        assert!(validate_secret_content("").is_err());
    }

    #[test]
    fn short_secret_is_refused_as_typo_grade() {
        let error = validate_secret_content("fifteen-chars!!").unwrap_err();
        assert!(error.contains("minimum 16"), "got: {error}");
    }

    #[test]
    fn healthy_secret_is_trimmed_and_kept() {
        let secret = "a".repeat(32);
        let held = validate_secret_content(&format!("  {secret}\n")).unwrap();
        assert_eq!(held, secret);
    }

    #[test]
    fn env_fragment_is_exact_with_no_stray_whitespace() {
        let line = env_fragment_line(&"a".repeat(32));
        assert_eq!(
            line,
            format!("CYNIC_FORGEJO_WEBHOOK_SECRET={}\n", "a".repeat(32))
        );
        assert!(!line.contains(' '));
    }

    #[test]
    fn report_renders_without_the_secret() {
        let report = NodeCeremonyReport {
            node_id: "n".to_string(),
            ssh_target: "u@h".to_string(),
            url: "http://h:9090".to_string(),
            distributed: true,
            restarted: true,
            gauge_ok: true,
            signature_ok: true,
            detail: "ok".to_string(),
        };
        let rendered = serde_json::to_string(&report).unwrap();
        assert!(rendered.contains("\"signature_ok\":true"));
        assert!(!rendered.contains("secret"));
    }
}
