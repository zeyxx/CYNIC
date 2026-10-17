//! Node activation commands — status, activate, identity, CSR, cert, audit.
//!
//! Merged from `node_activation_surface.rs` (372 LOC) into the unified node surface.
//! Each function is a thin wrapper around the existing implementation.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Subcommand;
use serde::Serialize;

use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};
use cynic_host::node_activation::NodeActivationPort;
use cynic_host::node_identity_workflow::NodeIdentityWorkflow;

pub(crate) const DEFAULT_REGISTRY_PATH: &str = "/etc/cynic/registry.json";

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

pub(crate) fn execute_status(node_id: String, json: bool) -> ExitCode {
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

pub(crate) fn execute_activate(node_id: String, yes: bool, json: bool) -> ExitCode {
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

pub(crate) fn execute_identity_init(key_path: Option<String>, json: bool) -> ExitCode {
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

pub(crate) fn execute_csr_create(
    node_id: String,
    role: String,
    key_path: Option<String>,
    json: bool,
) -> ExitCode {
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

pub(crate) fn execute_cert_inspect(cert_path: Option<String>, json: bool) -> ExitCode {
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

pub(crate) fn execute_cert_verify(
    kernel_pubkey_hex: String,
    cert_path: Option<String>,
    json: bool,
) -> ExitCode {
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

pub(crate) fn execute_host_audit(json: bool, max_commits: usize) -> ExitCode {
    if !json {
        return ExitCode::FAILURE;
    }
    match crate::node_host_audit::build_node_host_audit(max_commits) {
        Ok(report) => crate::emit_json(&report, false),
        Err(error) => crate::fail(error),
    }
}

/// Node certificate arms: inspect a local enrollment certificate, or verify
/// one against a kernel authority key.
#[derive(Subcommand)]
pub enum NodeCertCommand {
    /// Inspect a local Node Enrollment Certificate.
    Inspect {
        #[arg(long)]
        cert_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Cryptographically verify a Node Enrollment Certificate against a Kernel authority public key.
    Verify {
        #[arg(long)]
        kernel_pubkey_hex: String,
        #[arg(long)]
        cert_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Issue a Node Enrollment Certificate for a CSR, signed by an offline
    /// authority key. OPERATOR-RUN ONLY: the authority secret must never
    /// transit agent-reachable disk. Requires explicit `--yes` consent.
    Issue {
        /// Path to the CSR JSON file (as produced by `node csr create --json`).
        #[arg(long)]
        csr_path: String,
        /// Path to the authority secret (fleet hex, raw 32B, or OpenSSH PEM).
        #[arg(long)]
        authority_key_path: String,
        /// Human-readable authority identifier recorded on the certificate.
        #[arg(long)]
        authority_id: String,
        /// Certificate lifetime in seconds.
        #[arg(long, default_value_t = 31_536_000)]
        ttl_seconds: u64,
        /// Where to write the issued certificate JSON.
        #[arg(long)]
        out: String,
        /// Explicit consent: issuing a certificate mints fleet trust.
        #[arg(long, default_value_t = false)]
        yes: bool,
        #[arg(long)]
        json: bool,
    },
}

/// Node CSR arms: create a self-signed Certificate Signing Request.
#[derive(Subcommand)]
pub enum NodeCsrCommand {
    /// Create a self-signed Certificate Signing Request (CSR) for node enrollment.
    Create {
        #[arg(long)]
        node_id: String,
        #[arg(long)]
        role: String,
        #[arg(long)]
        key_path: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

pub(crate) fn execute_node_cert(command: NodeCertCommand) -> ExitCode {
    match command {
        NodeCertCommand::Inspect { cert_path, json } => execute_cert_inspect(cert_path, json),
        NodeCertCommand::Verify {
            kernel_pubkey_hex,
            cert_path,
            json,
        } => execute_cert_verify(kernel_pubkey_hex, cert_path, json),
        NodeCertCommand::Issue {
            csr_path,
            authority_key_path,
            authority_id,
            ttl_seconds,
            out,
            yes,
            json,
        } => execute_cert_issue(
            csr_path,
            authority_key_path,
            authority_id,
            ttl_seconds,
            out,
            yes,
            json,
        ),
    }
}

pub(crate) fn execute_node_csr(command: NodeCsrCommand) -> ExitCode {
    match command {
        NodeCsrCommand::Create {
            node_id,
            role,
            key_path,
            json,
        } => execute_csr_create(node_id, role, key_path, json),
    }
}

#[derive(Serialize)]
struct CertIssueReport {
    schema: &'static str,
    certificate_id: String,
    node_id: String,
    role: String,
    authority_id: String,
    issued_at_unix_s: u64,
    expires_at_unix_s: u64,
    out_path: String,
    status: &'static str,
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_cert_issue(
    csr_path: String,
    authority_key_path: String,
    authority_id: String,
    ttl_seconds: u64,
    out: String,
    yes: bool,
    json: bool,
) -> ExitCode {
    if !yes {
        if json {
            crate::cli_json::emit_pretty_json(&serde_json::json!({
                "schema": "cynic.node_certificate.issue.report.v1",
                "status": "blocked",
                "blockers": ["issue_consent_required:--yes mints fleet trust, re-run with explicit consent"],
            }));
        } else {
            eprintln!("BLOCKED: --yes consent required (issuing a certificate mints fleet trust)");
        }
        return ExitCode::from(2);
    }
    let csr_bytes = match std::fs::read(&csr_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("Failed to read CSR file {csr_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let csr: cynic_node_protocol::NodeCsr = match serde_json::from_slice(&csr_bytes) {
        Ok(csr) => csr,
        Err(error) => {
            eprintln!("Failed to parse CSR file {csr_path}: {error}");
            return ExitCode::FAILURE;
        }
    };
    let seed = match cynic_host::one_shot_authority::load_ed25519_seed(std::path::Path::new(
        &authority_key_path,
    )) {
        Ok(seed) => seed,
        Err(error) => {
            eprintln!("Failed to load authority key: {error}");
            return ExitCode::FAILURE;
        }
    };
    let workflow = NodeIdentityWorkflow::default();
    let now = current_unix_s();
    match workflow.issue_certificate(&csr, &authority_id, &seed, ttl_seconds, now) {
        Ok(cert) => {
            if let Err(error) = workflow.install_certificate(&cert, std::path::Path::new(&out)) {
                eprintln!("Failed to write certificate to {out}: {error}");
                return ExitCode::FAILURE;
            }
            // Seed bytes drop here; never printed, never logged.
            let report = CertIssueReport {
                schema: "cynic.node_certificate.issue.report.v1",
                certificate_id: cert.certificate_id.clone(),
                node_id: cert.node_id.clone(),
                role: cert.role.clone(),
                authority_id: cert.kernel_authority_id.clone(),
                issued_at_unix_s: cert.issued_at_unix_s,
                expires_at_unix_s: cert.expires_at_unix_s,
                out_path: out,
                status: "issued",
            };
            if json {
                crate::cli_json::emit_pretty_json(&report)
            } else {
                println!(
                    "Certificate issued for {} (role {}) by {} -> {}",
                    report.node_id, report.role, report.authority_id, report.out_path
                );
                ExitCode::SUCCESS
            }
        }
        Err(error) => {
            eprintln!("Failed to issue certificate: {error}");
            ExitCode::FAILURE
        }
    }
}
