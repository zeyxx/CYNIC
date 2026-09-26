//! Machine Enrollment Grant issuer and renewer for CYNIC node runtime.
//!
//! Generates or reads authority keys, binds machine identity,
//! signs a canonical SignedNodeEnrollmentGrant, and updates identity files.

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use cynic_protocol::node_enrollment::{
    node_enrollment_grant_signing_bytes, NodeEnrollmentGrant, NodeEnrollmentRequest,
    SignedNodeEnrollmentGrant, NODE_ENROLLMENT_GRANT_SCHEMA, NODE_ENROLLMENT_REQUEST_SCHEMA,
    NODE_ENROLLMENT_SIGNATURE_NAMESPACE, SIGNED_NODE_ENROLLMENT_GRANT_SCHEMA,
};
use ed25519_dalek::{Signer, SigningKey};
use sha2::{Digest, Sha256};

#[derive(Parser, Debug)]
#[command(name = "cynic-node-enroll", about = "Renew or issue CYNIC node enrollment grants")]
struct Args {
    /// Node identifier
    #[arg(long, default_value = "cynic-builder")]
    node_id: String,

    /// Node role
    #[arg(long, default_value = "build_runner")]
    role: String,

    /// Identity storage directory
    #[arg(long, default_value = "/var/lib/cynic/identity")]
    identity_dir: PathBuf,

    /// Config directory for public trust root
    #[arg(long, default_value = "/etc/cynic")]
    config_dir: PathBuf,

    /// Validity in days
    #[arg(long, default_value = "365")]
    ttl_days: u64,

    /// Emit JSON report
    #[arg(long)]
    json: bool,
}

fn read_or_create_hex_signing_key(path: &Path) -> Result<SigningKey, String> {
    if path.exists() {
        let content = fs::read_to_string(path)
            .map_err(|e| format!("read {}: {e}", path.display()))?;
        let bytes: [u8; 32] = hex::decode(content.trim())
            .map_err(|e| format!("decode hex {}: {e}", path.display()))?
            .try_into()
            .map_err(|_| format!("key {} is not 32 bytes", path.display()))?;
        Ok(SigningKey::from_bytes(&bytes))
    } else {
        let mut f = File::open("/dev/urandom")
            .map_err(|e| format!("open /dev/urandom: {e}"))?;
        let mut bytes = [0u8; 32];
        f.read_exact(&mut bytes)
            .map_err(|e| format!("read /dev/urandom: {e}"))?;
        let key = SigningKey::from_bytes(&bytes);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(path, hex::encode(key.to_bytes()))
            .map_err(|e| format!("write {}: {e}", path.display()))?;
        Ok(key)
    }
}

fn main() -> ExitCode {
    let args = Args::parse();

    let node_signing_key_path = args.identity_dir.join("node-signing.key");
    let node_key = match read_or_create_hex_signing_key(&node_signing_key_path) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("blocked: node signing key error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let machine_public_key_bytes = node_key.verifying_key().to_bytes();
    let machine_public_key_hex = hex::encode(machine_public_key_bytes);
    let machine_identity_digest = format!(
        "sha256:{}",
        hex::encode(Sha256::digest(machine_public_key_bytes))
    );

    // Authority signing key (stored in identity_dir/enrollment-authority.key)
    let authority_key_path = args.identity_dir.join("enrollment-authority.key");
    let authority_key = match read_or_create_hex_signing_key(&authority_key_path) {
        Ok(k) => k,
        Err(e) => {
            eprintln!("blocked: authority key error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let authority_pub_bytes = authority_key.verifying_key().to_bytes();
    let authority_pub_hex = hex::encode(authority_pub_bytes);
    let authority_key_id = format!("sha256:{}", hex::encode(Sha256::digest(authority_pub_bytes)));

    let now_unix_s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let ttl_s = args.ttl_days * 86400;
    let expires_at_unix_s = now_unix_s + ttl_s;

    let capabilities = vec![
        "node_control".to_string(),
        "build_runner".to_string(),
        "kernel_to_node_workload_delivery_read_back".to_string(),
        "scheduler_claim_renewal".to_string(),
        "scheduler_cancellation_delivery".to_string(),
        "scheduler_crash_recovery".to_string(),
        "build_runner_signature_and_live_workload_verifier".to_string(),
        "workspace_claim_store_wiring_and_conformance_proof".to_string(),
        "build_completion_and_artifact_read_back".to_string(),
        "node_workspace_observer_wiring_and_proof".to_string(),
        "node_owned_build_grant_machine_signer".to_string(),
        "scheduler_to_build_runner_execution_transport".to_string(),
        "bounded_source_change_apply_adapter".to_string(),
        "post_apply_source_read_back".to_string(),
    ];

    let request_id = format!("enroll:{}", hex::encode(&now_unix_s.to_be_bytes()));
    let nonce = format!("nonce:{}", hex::encode(&now_unix_s.to_be_bytes()));

    let seed_id = format!(
        "sha256:{}",
        hex::encode(Sha256::digest(args.node_id.as_bytes()))
    );

    let request = NodeEnrollmentRequest {
        schema: NODE_ENROLLMENT_REQUEST_SCHEMA.to_string(),
        request_id: request_id.clone(),
        seed_id: seed_id.clone(),
        node_id: args.node_id.clone(),
        role: args.role.clone(),
        machine_identity_digest: machine_identity_digest.clone(),
        machine_public_key_hex,
        nonce,
        issued_at_unix_s: now_unix_s,
        expires_at_unix_s,
        requested_capabilities: capabilities.clone(),
    };

    let grant = NodeEnrollmentGrant {
        schema: NODE_ENROLLMENT_GRANT_SCHEMA.to_string(),
        grant_id: format!("grant:{}", hex::encode(&now_unix_s.to_be_bytes())),
        authority_id: "cynic-runtime-machine-enrollment".to_string(),
        request_id,
        seed_id,
        node_id: args.node_id.clone(),
        role: args.role.clone(),
        machine_identity_digest,
        generation: 2,
        issued_at_unix_s: now_unix_s,
        expires_at_unix_s,
        granted_capabilities: capabilities,
        evidence_refs: vec![
            authority_key_id.clone(),
            format!("sha256:{}", hex::encode(Sha256::digest(args.node_id.as_bytes()))),
        ],
    };

    let mut signed = SignedNodeEnrollmentGrant {
        schema: SIGNED_NODE_ENROLLMENT_GRANT_SCHEMA.to_string(),
        authority_key_id: authority_key_id.clone(),
        signature_namespace: NODE_ENROLLMENT_SIGNATURE_NAMESPACE.to_string(),
        grant,
        signature_hex: String::new(),
    };

    let signing_bytes = match node_enrollment_grant_signing_bytes(&signed) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("blocked: serialization error: {e}");
            return ExitCode::FAILURE;
        }
    };
    let signature = authority_key.sign(&signing_bytes);
    signed.signature_hex = hex::encode(signature.to_bytes());

    // Write out artifacts
    let req_path = args.identity_dir.join("enrollment-request.json");
    let grant_path = args.identity_dir.join("enrollment-grant.json");
    let pub_path = args.config_dir.join("enrollment-authority.pub");

    let req_json = serde_json::to_string_pretty(&request).unwrap();
    let grant_json = serde_json::to_string_pretty(&signed).unwrap();

    if let Err(e) = fs::write(&req_path, &req_json) {
        eprintln!("blocked: failed to write {}: {e}", req_path.display());
        return ExitCode::FAILURE;
    }
    if let Err(e) = fs::write(&grant_path, &grant_json) {
        eprintln!("blocked: failed to write {}: {e}", grant_path.display());
        return ExitCode::FAILURE;
    }
    if let Err(e) = fs::write(&pub_path, &authority_pub_hex) {
        eprintln!("blocked: failed to write {}: {e}", pub_path.display());
        return ExitCode::FAILURE;
    }

    if args.json {
        let report = serde_json::json!({
            "status": "converged",
            "schema": "cynic.node_enrollment.renewal_report.v1",
            "node_id": args.node_id,
            "role": args.role,
            "issued_at_unix_s": now_unix_s,
            "expires_at_unix_s": expires_at_unix_s,
            "authority_key_id": authority_key_id,
            "authority_pub_path": pub_path.display().to_string(),
            "grant_path": grant_path.display().to_string(),
            "request_path": req_path.display().to_string(),
        });
        println!("{}", serde_json::to_string_pretty(&report).unwrap());
    } else {
        println!("Enrollment grant successfully renewed for {}", args.node_id);
        println!("  Expires at:    {expires_at_unix_s} (in {} days)", args.ttl_days);
        println!("  Authority ID:  {authority_key_id}");
        println!("  Grant Path:    {}", grant_path.display());
    }

    ExitCode::SUCCESS
}
