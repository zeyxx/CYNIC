//! Node activation commands — status, activate, identity, CSR, cert, audit.
//!
//! Merged from `node_activation_surface.rs` (372 LOC) into the unified node surface.
//! Each function is a thin wrapper around the existing implementation.

use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;

use cynic_types::paths::CynicPaths;

use crate::runtime_paths::{RuntimeContext, resolve_runtime_paths};
use cynic_host::node_activation::NodeActivationPort;
use cynic_host::node_identity_workflow::NodeIdentityWorkflow;
use cynic_node_protocol::{MachineEnrollmentAuthority, NodeEnrollmentRequest};
use sha2::{Digest, Sha256};

fn default_key_path() -> PathBuf {
    CynicPaths::system_config_dir().join("node-identity.key")
}

fn default_cert_path() -> PathBuf {
    CynicPaths::system_config_dir().join("node-identity.cert")
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

#[derive(Serialize)]
struct EnrollmentIssueReport {
    schema: &'static str,
    status: &'static str,
    grant_id: String,
    node_id: String,
    role: String,
    granted_capabilities: Vec<String>,
    authority_key_id: String,
    expires_at_unix_s: u64,
    require_token: bool,
    output_path: Option<String>,
}

fn read_input_file(path: &str) -> Result<Vec<u8>, String> {
    std::fs::read(path).map_err(|error| format!("cannot read {path}: {error}"))
}

/// Admit a `NodeEnrollmentRequest` and emit the signed enrollment grant.
///
/// The authority signing key file holds hex-encoded raw bytes and is loaded
/// read-only: this surface never generates keys. Without `--token-file` the
/// grant is issued with `require_token = false`, which is only legitimate
/// for operator-mediated issuance where the authority holder vouches out of
/// band; the report always states which mode was used.
pub(crate) fn execute_enrollment_issue(
    request: String,
    authority: String,
    token_file: Option<String>,
    signing_key: String,
    output: Option<String>,
    json: bool,
) -> ExitCode {
    let request_bytes = match read_input_file(&request) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let enrollment_request: NodeEnrollmentRequest = match serde_json::from_slice(&request_bytes) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("invalid enrollment request {}: {error}", request);
            return ExitCode::FAILURE;
        }
    };
    let authority_bytes = match read_input_file(&authority) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let enrollment_authority: MachineEnrollmentAuthority =
        match serde_json::from_slice(&authority_bytes) {
            Ok(authority) => authority,
            Err(error) => {
                eprintln!("invalid enrollment authority {}: {error}", authority);
                return ExitCode::FAILURE;
            }
        };
    let (presented_token_digest, require_token) = match token_file {
        Some(path) => {
            let token = match read_input_file(&path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    eprintln!("{error}");
                    return ExitCode::FAILURE;
                }
            };
            let digest = format!(
                "sha256:{}",
                hex::encode(Sha256::digest(
                    String::from_utf8_lossy(&token).trim().as_bytes()
                ))
            );
            (digest, true)
        }
        None => (String::new(), false),
    };
    let key_bytes = match read_input_file(&signing_key) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::FAILURE;
        }
    };
    let key_hex = String::from_utf8_lossy(&key_bytes);
    let authority_key = match hex::decode(key_hex.trim()) {
        Ok(key) if key.len() == 32 => key,
        _ => {
            eprintln!("invalid authority signing key {signing_key}: expected 64 hex chars");
            return ExitCode::FAILURE;
        }
    };
    let workflow = NodeIdentityWorkflow::default();
    let now = current_unix_s();
    let signed = match workflow.issue_signed_enrollment_grant(
        &enrollment_request,
        &enrollment_authority,
        &presented_token_digest,
        require_token,
        &authority_key,
        now,
    ) {
        Ok(signed) => signed,
        Err(error) => {
            eprintln!("enrollment issuance failed: {error:?}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(path) = output.clone() {
        let json_bytes = match serde_json::to_vec_pretty(&signed) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("cannot encode grant: {error}");
                return ExitCode::FAILURE;
            }
        };
        if let Err(error) = std::fs::write(&path, json_bytes) {
            eprintln!("cannot write grant to {path}: {error}");
            return ExitCode::FAILURE;
        }
    }
    let report = EnrollmentIssueReport {
        schema: "cynic.node_enrollment.issue.report.v1",
        status: "issued",
        grant_id: signed.grant.grant_id.clone(),
        node_id: signed.grant.node_id.clone(),
        role: signed.grant.role.clone(),
        granted_capabilities: signed.grant.granted_capabilities.clone(),
        authority_key_id: signed.authority_key_id.clone(),
        expires_at_unix_s: signed.grant.expires_at_unix_s,
        require_token,
        output_path: output,
    };
    if json {
        crate::cli_json::emit_pretty_json(&report)
    } else {
        println!(
            "ISSUED: grant {} for {} (role: {}, token: {})",
            report.grant_id,
            report.node_id,
            report.role,
            if require_token {
                "presented"
            } else {
                "operator-mediated"
            }
        );
        ExitCode::SUCCESS
    }
}
