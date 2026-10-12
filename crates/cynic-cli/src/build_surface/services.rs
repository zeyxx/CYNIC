//! Build auxiliary services — webhook receiver and credential provisioning.

use clap::Subcommand;
use cynic_lifecycle::LocalArgvRunner;

/// One workload across the federation.
#[derive(Subcommand, Debug)]
pub(crate) enum BuildWorkloadCommand {
    /// Observe one workload by id (local store + SSH).
    Observe {
        /// Workload id to observe.
        #[arg(long)]
        workload_id: String,
        /// Node state root on remote nodes.
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        /// Restrict to the local store (no SSH federation).
        #[arg(long)]
        local: bool,
        /// Path to the registry JSON for node discovery.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// SSH user for remote nodes.
        #[arg(long, default_value = "cynic")]
        user: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
    /// List workloads known to the federation.
    List {
        /// Node state root on remote nodes.
        #[arg(long, default_value = "/var/lib/cynic")]
        state_root: String,
        /// Restrict to the local store (no SSH federation).
        #[arg(long)]
        local: bool,
        /// Path to the registry JSON for node discovery.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// SSH user for remote nodes.
        #[arg(long, default_value = "cynic")]
        user: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

/// Forgejo push webhook receiver.
#[derive(Subcommand, Debug)]
pub(crate) enum BuildWebhookCommand {
    /// Serve the Forgejo push webhook receiver.
    Serve {
        /// Address to bind (e.g. 127.0.0.1:9147).
        #[arg(long, default_value = "127.0.0.1:9147")]
        bind: String,
        /// Node builder webhook URL. Required: an empty endpoint is a blocked
        /// forwarding configuration, never a guessed runner or registry path.
        #[arg(long)]
        node_webhook_url: String,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

/// Machine credentials derived from the registry.
#[derive(Subcommand, Debug)]
pub(crate) enum BuildCredentialsCommand {
    /// Provision SSH credentials for all machine_enrollment transports.
    Provision {
        /// Path to the registry JSON.
        #[arg(long, default_value = "infra/registry.json")]
        registry: String,
        /// Secrets directory (default: /etc/cynic/secrets).
        #[arg(long, default_value = "/etc/cynic/secrets")]
        secrets_dir: String,
        /// Apply provisioning (generate keys). Without this, only scan.
        #[arg(long)]
        apply: bool,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn serve_webhook(bind: &str, node_webhook_url: &str, json: bool) -> Result<(), String> {
    use cynic_vcs_forgejo::WebhookServerConfig;
    use cynic_vcs_forgejo::forgejo_webhook_server::spawn_webhook_server;

    if node_webhook_url.trim().is_empty() {
        return Err("node webhook URL is required; no forwarding target is guessed".to_string());
    }
    let config = WebhookServerConfig {
        bind: bind.to_string(),
        node_webhook_url: node_webhook_url.to_string(),
        webhook_secret: cynic_secrets::env_var("webhook-secret", "CYNIC_WEBHOOK_SECRET"),
    };

    let handle = spawn_webhook_server(config)
        .map_err(|e| format!("failed to bind webhook server on {bind}: {e}"))?;

    let startup = serde_json::json!({
        "status": "listening",
        "bind": bind,
        "node_webhook_url": node_webhook_url,
        "forwarding": "configured",
        "webhook_secret_present": cynic_secrets::env_var_present("webhook-secret", "CYNIC_WEBHOOK_SECRET"),
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&startup).unwrap());
    } else {
        println!("Forgejo webhook server listening on {bind}");
        println!("  Node webhook: {node_webhook_url}");
        println!(
            "  Webhook secret: {}",
            if cynic_secrets::env_var_present("webhook-secret", "CYNIC_WEBHOOK_SECRET") {
                "configured"
            } else {
                "absent (unsigned pushes are reported as such)"
            }
        );
    }

    let _ = handle.join();
    Ok(())
}

pub(crate) fn provision_credentials(
    registry_path: &str,
    secrets_dir: &str,
    apply: bool,
    json: bool,
) -> Result<(), String> {
    use cynic_host::credential_provisioning::{
        provision_ssh_credential, scan_unprovisioned_enrollment_transports,
    };

    let registry_content = std::fs::read_to_string(registry_path)
        .map_err(|e| format!("failed to read registry: {e}"))?;
    let registry: serde_json::Value = serde_json::from_str(&registry_content)
        .map_err(|e| format!("failed to parse registry: {e}"))?;

    let secrets_path = std::path::Path::new(secrets_dir);
    let transports = scan_unprovisioned_enrollment_transports(&registry, secrets_path)?;

    if !apply {
        let report = serde_json::json!({
            "status": "scan_complete",
            "secrets_dir": secrets_dir,
            "nodes_needing_credentials": transports.iter().map(|t| &t.node_id).collect::<Vec<_>>(),
            "count": transports.len(),
            "apply_requested": false,
        });
        if json {
            println!("{}", serde_json::to_string_pretty(&report).unwrap());
        } else {
            println!("SSH credentials scan:");
            if transports.is_empty() {
                println!("  All transports already provisioned");
            } else {
                for t in &transports {
                    println!(
                        "  - {} ({}) needs SSH key: {}",
                        t.node_id, t.ssh_target, t.credential_ref
                    );
                }
            }
        }
        return Ok(());
    }

    let runner = LocalArgvRunner;
    let mut proofs = Vec::new();
    let mut errors = Vec::new();

    for transport in &transports {
        match provision_ssh_credential(transport, secrets_path, &runner) {
            Ok(proof) => proofs.push(serde_json::json!({
                "node_id": proof.node_id,
                "ssh_key_path": proof.ssh_key_path,
                "public_key_sha256": proof.public_key_sha256,
                "known_hosts_configured": proof.known_hosts_configured,
                "read_back_verified": proof.read_back_verified,
            })),
            Err(e) => errors.push(serde_json::json!({
                "node_id": transport.node_id,
                "error": e,
            })),
        }
    }

    let report = serde_json::json!({
        "status": if errors.is_empty() { "provisioned" } else { "partial_failure" },
        "secrets_dir": secrets_dir,
        "provisioned": proofs,
        "errors": errors,
        "total": transports.len(),
        "succeeded": proofs.len(),
        "failed": errors.len(),
    });

    if json {
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!("SSH credential provisioning:");
        for p in &proofs {
            println!("  + {} → {}", p["node_id"], p["ssh_key_path"]);
        }
        for e in &errors {
            println!("  ✗ {} → {}", e["node_id"], e["error"]);
        }
        println!("  {} provisioned, {} failed", proofs.len(), errors.len());
    }

    if !errors.is_empty() {
        return Err(format!("provisioning failed for {} nodes", errors.len()));
    }

    Ok(())
}
