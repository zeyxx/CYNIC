//! Build auxiliary services — webhook receiver and credential provisioning.

use clap::Subcommand;
use cynic_types::paths::CynicPaths;
use cynic_process::LocalArgvRunner;

/// One workload across the federation.
#[derive(Subcommand, Debug)]
pub(crate) enum BuildWorkloadCommand {
    /// Observe one workload by id (local store + SSH).
    Observe {
        /// Workload id to observe.
        #[arg(long)]
        workload_id: String,
        /// Node state root on remote nodes.
        #[arg(long, default_value_t = CynicPaths::state_dir().display().to_string())]
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
        #[arg(long, default_value_t = CynicPaths::state_dir().display().to_string())]
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
    /// Read the output a builder sealed for a failed workload (native HTTP, no SSH).
    Output {
        /// Workload id, as printed by `build submit`.
        #[arg(long)]
        workload_id: String,
        /// Builder HTTP endpoint (resolved from the registry pool if omitted).
        #[arg(long)]
        builder_url: Option<String>,
        /// Number of trailing lines kept per stream.
        #[arg(long, default_value_t = 60)]
        tail: usize,
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
        /// Secrets directory (default: the system config secrets dir).
        #[arg(long, default_value_t = CynicPaths::secrets_dir().display().to_string())]
        secrets_dir: String,
        /// Apply provisioning (generate keys). Without this, only scan.
        #[arg(long)]
        apply: bool,
        /// Emit machine-readable JSON.
        #[arg(long)]
        json: bool,
    },
}


pub(crate) fn provision_credentials(
    registry_path: &str,
    secrets_dir: &str,
    apply: bool,
    json: bool,
) -> Result<(), String> {
    use cynic_secrets::credential::provisioning::{
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
