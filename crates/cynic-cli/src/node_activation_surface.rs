use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Subcommand;
use serde::Serialize;

use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};
use cynic_host::node_activation::NodeActivationPort;
use cynic_host::node_identity_workflow::NodeIdentityWorkflow;

#[derive(Subcommand)]
pub(crate) enum NodeActivationCommand {
    /// Observe node activation state.
    Status {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Activate a node (bounded action with read-back proof).
    Activate {
        #[arg(long)]
        node_id: String,
        /// Explicit consent — required for mutation.
        #[arg(long, default_value_t = false)]
        yes: bool,
        #[arg(long, default_value_t = false)]
        json: bool,
    },
    /// Initialize or display the local node Ed25519 signing keypair.
    IdentityInit {
        #[arg(long)]
        key_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Create a self-signed Certificate Signing Request (CSR) for node enrollment.
    CsrCreate {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        role: String,
        #[arg(long)]
        key_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Inspect a local Node Enrollment Certificate.
    CertInspect {
        #[arg(long)]
        cert_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Cryptographically verify a Node Enrollment Certificate against a Kernel authority public key.
    CertVerify {
        #[arg(long)]
        kernel_pubkey_hex: String,
        #[arg(long)]
        cert_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Audit host commits, divergence, and node health state.
    HostAudit {
        #[arg(long)]
        json: bool,
        #[arg(long, default_value_t = 80)]
        max_commits: usize,
    },
    /// Host crash forensics, boot timeline, and post-mortem evidence.
    Forensics {
        #[command(subcommand)]
        command: crate::forensics_surface::ForensicsCommand,
    },
}

#[derive(Serialize)]
struct IdentityInitReport {
    schema: &'static str,
    key_path: String,
    public_key_hex: String,
    digest: String,
    status: &'static str,
}

#[derive(Serialize)]
struct CertInspectReport {
    schema: &'static str,
    certificate_id: String,
    node_id: String,
    role: String,
    subject_public_key_hex: String,
    kernel_authority_id: String,
    issued_at_unix_s: u64,
    expires_at_unix_s: u64,
    is_currently_valid: bool,
    remaining_seconds: i64,
}

#[derive(Serialize)]
struct CertVerifyReport {
    schema: &'static str,
    certificate_id: String,
    node_id: String,
    verified: bool,
    is_currently_valid: bool,
    status: &'static str,
}

fn default_key_path() -> PathBuf {
    PathBuf::from("/etc/cynic/node-identity.key")
}

fn default_cert_path() -> PathBuf {
    PathBuf::from("/etc/cynic/node-identity.cert")
}

fn current_unix_s() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub(crate) fn execute_node_activation(command: NodeActivationCommand) -> ExitCode {
    match command {
        NodeActivationCommand::Status { node_id, json } => {
            if !json {
                return ExitCode::FAILURE;
            }
            let registry_path =
                resolve_runtime_paths(&RuntimeContext::from_env(None)).registry_path;
            let adapter = cynic_host::node_activation::NodeActivationAdapter::new(
                registry_path.to_string_lossy().to_string(),
            );
            let verdict = adapter.observe(&node_id);
            crate::cli_json::emit_pretty_json(&verdict)
        }
        NodeActivationCommand::Activate { node_id, yes, json } => {
            let registry_path =
                resolve_runtime_paths(&RuntimeContext::from_env(None)).registry_path;
            let adapter = cynic_host::node_activation::NodeActivationAdapter::new(
                registry_path.to_string_lossy().to_string(),
            );
            let report = adapter.activate(&node_id, yes);
            if json {
                crate::cli_json::emit_pretty_json(&report)
            } else {
                match &report.result {
                    Some(Ok(proof)) => match proof {
                        cynic_node_protocol::node_activation::ActivationProof::Activated {
                            evidence_refs,
                            ..
                        } => {
                            println!(
                                "ACTIVATED: {} (evidence: {})",
                                node_id,
                                evidence_refs.join(", ")
                            );
                        }
                        cynic_node_protocol::node_activation::ActivationProof::Failed {
                            blockers,
                            ..
                        } => {
                            println!("FAILED: {} (blockers: {})", node_id, blockers.join(", "));
                            return ExitCode::FAILURE;
                        }
                        cynic_node_protocol::node_activation::ActivationProof::RolledBack {
                            rollback_evidence_refs,
                            ..
                        } => {
                            println!(
                                "ROLLED BACK: {} (evidence: {})",
                                node_id,
                                rollback_evidence_refs.join(", ")
                            );
                            return ExitCode::FAILURE;
                        }
                    },
                    Some(Err(e)) => {
                        println!("BLOCKED: {}", e);
                        return ExitCode::FAILURE;
                    }
                    None => {
                        println!("BLOCKED: --yes consent required (mutates host)");
                        return ExitCode::FAILURE;
                    }
                }
                ExitCode::SUCCESS
            }
        }
        NodeActivationCommand::IdentityInit { key_path, json } => {
            let path = key_path.map(PathBuf::from).unwrap_or_else(default_key_path);
            let workflow = NodeIdentityWorkflow::default();
            match workflow.load_or_generate_keypair(&path) {
                Ok(material) => {
                    use sha2::{Digest, Sha256};
                    let digest = format!(
                        "sha256:{}",
                        hex::encode(Sha256::digest(&material.public_key))
                    );
                    let report = IdentityInitReport {
                        schema: "cynic.node_identity.init.report.v1",
                        key_path: path.to_string_lossy().to_string(),
                        public_key_hex: material.public_key_hex.clone(),
                        digest: digest.clone(),
                        status: "converged",
                    };
                    if json {
                        crate::cli_json::emit_pretty_json(&report)
                    } else {
                        println!("Node identity initialized at: {}", path.display());
                        println!("Public key: {}", material.public_key_hex);
                        println!("Digest:     {}", digest);
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("Failed to initialize node identity: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        NodeActivationCommand::CsrCreate {
            node_id,
            role,
            key_path,
            json,
        } => {
            let path = key_path.map(PathBuf::from).unwrap_or_else(default_key_path);
            let workflow = NodeIdentityWorkflow::default();
            let material = match workflow.load_or_generate_keypair(&path) {
                Ok(m) => m,
                Err(e) => {
                    eprintln!("Failed to load node private key: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let now = current_unix_s();
            match workflow.create_csr(&node_id, &role, &material, now) {
                Ok(csr) => {
                    if json {
                        crate::cli_json::emit_pretty_json(&csr)
                    } else {
                        println!("Node CSR created for: {} (role: {})", node_id, role);
                        println!("Signature: {}", csr.csr_signature_hex);
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("Failed to create node CSR: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        NodeActivationCommand::CertInspect { cert_path, json } => {
            let path = cert_path
                .map(PathBuf::from)
                .unwrap_or_else(default_cert_path);
            let workflow = NodeIdentityWorkflow::default();
            match workflow.load_certificate(&path) {
                Ok(cert) => {
                    let now = current_unix_s();
                    let valid = cert.is_valid_at(now);
                    let remaining = (cert.expires_at_unix_s as i64) - (now as i64);
                    let report = CertInspectReport {
                        schema: "cynic.node_certificate.inspect.report.v1",
                        certificate_id: cert.certificate_id.clone(),
                        node_id: cert.node_id.clone(),
                        role: cert.role.clone(),
                        subject_public_key_hex: cert.subject_public_key_hex.clone(),
                        kernel_authority_id: cert.kernel_authority_id.clone(),
                        issued_at_unix_s: cert.issued_at_unix_s,
                        expires_at_unix_s: cert.expires_at_unix_s,
                        is_currently_valid: valid,
                        remaining_seconds: remaining,
                    };
                    if json {
                        crate::cli_json::emit_pretty_json(&report)
                    } else {
                        println!("Certificate ID: {}", cert.certificate_id);
                        println!("Node ID:        {}", cert.node_id);
                        println!("Role:           {}", cert.role);
                        println!("Authority:      {}", cert.kernel_authority_id);
                        println!("Valid:          {} (remaining: {}s)", valid, remaining);
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("Failed to load certificate from {}: {e}", path.display());
                    ExitCode::FAILURE
                }
            }
        }
        NodeActivationCommand::CertVerify {
            kernel_pubkey_hex,
            cert_path,
            json,
        } => {
            let path = cert_path
                .map(PathBuf::from)
                .unwrap_or_else(default_cert_path);
            let workflow = NodeIdentityWorkflow::default();
            let cert = match workflow.load_certificate(&path) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!("Failed to load certificate: {e}");
                    return ExitCode::FAILURE;
                }
            };
            let pubkey_bytes = match hex::decode(kernel_pubkey_hex.trim()) {
                Ok(b) if b.len() == 32 => b,
                _ => {
                    eprintln!("Invalid kernel_pubkey_hex: must be 64 hex characters (32 bytes)");
                    return ExitCode::FAILURE;
                }
            };
            let now = current_unix_s();
            match workflow.verify_certificate(&cert, &pubkey_bytes, now) {
                Ok(verified) => {
                    let valid = cert.is_valid_at(now);
                    let report = CertVerifyReport {
                        schema: "cynic.node_certificate.verify.report.v1",
                        certificate_id: cert.certificate_id.clone(),
                        node_id: cert.node_id.clone(),
                        verified,
                        is_currently_valid: valid,
                        status: if verified && valid {
                            "verified"
                        } else {
                            "rejected"
                        },
                    };
                    if json {
                        crate::cli_json::emit_pretty_json(&report)
                    } else {
                        if verified && valid {
                            println!(
                                "VERIFIED: Certificate {} is authentic and currently valid.",
                                cert.certificate_id
                            );
                            ExitCode::SUCCESS
                        } else {
                            println!(
                                "REJECTED: Certificate signature or temporal validity check failed."
                            );
                            ExitCode::FAILURE
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Verification error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        NodeActivationCommand::HostAudit { json, max_commits } => {
            if !json {
                return ExitCode::FAILURE;
            }
            match crate::node_host_audit::build_node_host_audit(max_commits) {
                Ok(report) => crate::emit_json(&report, false),
                Err(error) => crate::fail(error),
            }
        }
        NodeActivationCommand::Forensics { command } => {
            crate::forensics_surface::execute_forensics(command)
        }
    }
}
