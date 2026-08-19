//! Thin CLI facade for issuing a signed `build-runner-bootstrap` capability binding.
//!
//! Argument decoding + JSON emission only. All cryptography, canonical-payload
//! construction, and binding validation live in the host/port and protocol layers.
//! Mirrors the structure of `build_runner_bootstrap::execute` (consume side): this
//! is the *issue* side — it produces the binding a future `build-runner-bootstrap`
//! invocation will consume.

use std::fs;
use std::path::Path;

use cynic_protocol::{
    MachineEnrollment,
    build::{
        BUILD_RUNNER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA,
        BuildRunnerBootstrapCapabilityBinding, build_runner_bootstrap_action_digest,
        build_runner_bootstrap_input_digest, validate_build_runner_bootstrap_capability_binding,
    },
    one_shot_capability::OneShotCapability,
    ONE_SHOT_CAPABILITY_SCHEMA,
};
use sha2::{Digest, Sha256};

use crate::one_shot_authority::{
    CapabilitySigningPort, LocalCapabilitySigningKey, OneShotCapabilityAuthorityPort,
    LocalOneShotCapabilityAuthority,
};
use crate::host_exec_cli::{args, io};

/// Issue a signed bootstrap capability binding to `--output`.
///
/// Args:
///   --enrollment <path>          MachineEnrollment JSON
///   --private-key <path>         32-byte raw seed or OpenSSH blob (signing key)
///   --authority-public-key <path>  32-byte hex pubkey (for verify round-trip, optional)
///   --source-commit <sha1>
///   --source-bundle-digest <sha256:...>
///   --registry-digest <sha256:...>
///   --executor-digest <sha256:...>
///   --allowed-action <make:build-host-exec>
///   --expires-at <unix_s>
///   --output <path>
///   [--verify]                   self-verify via LocalOneShotCapabilityAuthority
pub(crate) fn execute(args: &[String]) -> Result<std::process::ExitCode, String> {
    let enrollment_path = required(args, "--enrollment")?;
    let private_key = required(args, "--private-key")?;
    let authority_pub = args::flag_value(args, "--authority-public-key");
    let source_commit = required(args, "--source-commit")?;
    let source_bundle_digest = required(args, "--source-bundle-digest")?;
    let registry_digest = required(args, "--registry-digest")?;
    let executor_digest = required(args, "--executor-digest")?;
    let allowed_action = required(args, "--allowed-action")?;
    let expires_at = required(args, "--expires-at")?;
    let output = required(args, "--output")?;
    let now = now_unix_s()?;

    let enrollment: MachineEnrollment =
        read_json(&enrollment_path, "enrollment")?;

    let signing_key = LocalCapabilitySigningKey::from_seed_path(Path::new(&private_key))?;
    let authority_key_id = signing_key.authority_key_id();
    if enrollment.authority_key_id() != authority_key_id {
        return Err("capability_issue_authority_key_id_enrollment_mismatch".to_string());
    }

    let action_digest = build_runner_bootstrap_action_digest(&allowed_action)
        .map_err(|e| format!("action_digest_failed:{e}"))?;

    let mut capability = OneShotCapability {
        schema: ONE_SHOT_CAPABILITY_SCHEMA.to_string(),
        status: "admitted".to_string(),
        authority_class: "MachineEnrollment".to_string(),
        capability_id: format!("build_{}", enrollment.node_id),
        subject: enrollment.node_subject(),
        scope: "build-runner-bootstrap".to_string(),
        input_digest: String::new(), // computed after binding is partial-assembled
        executor_digest: executor_digest.clone(),
        issued_at_unix_s: now,
        expires_at_unix_s: expires_at.parse::<u64>()
            .map_err(|e| format!("invalid --expires-at: {e}"))?,
        allowed_action_digests: vec![action_digest.clone()],
        authority_key_id: authority_key_id.clone(),
        signature_hex: String::new(), // signed below
        evidence_refs: vec![format!("machine_enrollment:{}", enrollment.node_id)],
    };

    // Build the partial binding (capability.input_digest empty) to derive the
    // canonical input_digest, then assign so signing_payload is self-consistent.
    let partial_binding = BuildRunnerBootstrapCapabilityBinding {
        schema: BUILD_RUNNER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA.to_string(),
        node_subject: capability.subject.clone(),
        source_commit: source_commit.clone(),
        source_bundle_digest: source_bundle_digest.clone(),
        registry_digest: registry_digest.clone(),
        executor_digest: executor_digest.clone(),
        capability: capability.clone(),
    };
    let input_digest = build_runner_bootstrap_input_digest(&partial_binding)
        .map_err(|e| format!("input_digest_failed:{e}"))?;
    capability.input_digest = input_digest;
    let binding = BuildRunnerBootstrapCapabilityBinding {
        capability,
        ..partial_binding
    };

    // Validate structurally (pure protocol invariants) before signing.
    let blockers = validate_build_runner_bootstrap_capability_binding(
        &binding,
        &allowed_action,
        now,
    );
    if !blockers.is_empty() {
        return Err(format!("capability_issue_validation_failed: {}", blockers.join(",")));
    }

    // Sign via the host signing port (ed25519-dalek v2, raw 64-byte sig).
    let port = signing_key;
    let sig_hex = port
        .sign_capability(&mut (binding.capability.clone()))
        .map_err(|e| format!("signing failed: {e}"))?;
    let mut signed_capability = binding.capability.clone();
    port.sign_capability(&mut signed_capability)?;
    debug_assert_eq!(signed_capability.signature_hex, sig_hex);
    debug_assert_eq!(signed_capability.signature_hex.len(), 128);

    let signing_payload = signed_capability
        .signing_payload()
        .map_err(|e| format!("signing payload re-serialization failed: {e}"))?;
    let _payload_digest = format!("sha256:{}", hex::encode(Sha256::digest(&signing_payload)));

    let signed_binding = BuildRunnerBootstrapCapabilityBinding {
        capability: signed_capability,
        ..binding
    };

    // Optional self-verify: round-trip the issued binding through the verifier.
    if args::has_flag(args, "--verify") {
        if let Some(pub_path) = authority_pub {
            let authority =
                LocalOneShotCapabilityAuthority::new(&pub_path, "/tmp/cynic-cap-issue-verify");
            let action_digest = build_runner_bootstrap_action_digest(&allowed_action)
                .map_err(|_| "action_digest_failed".to_string())?;
            let _consumed = authority.verify_and_consume(
                &signed_binding.capability,
                &signed_binding.capability.subject,
                "build-runner-bootstrap",
                &action_digest,
                now,
            ).map_err(|e| format!("self_verify_failed: {e}"))?;
        }
    }

    let json = serde_json::to_vec_pretty(&signed_binding)
        .map_err(|e| format!("failed to serialize binding: {e}"))?;
    fs::write(&output, json)
        .map_err(|e| format!("failed to write binding: {e}"))?;
    io::emit_json(
        &serde_json::json!({
            "status": "issued",
            "binding_path": &output,
            "capability_id": signed_binding.capability.capability_id,
            "authority_key_id": signed_binding.capability.authority_key_id,
            "signature_len": signed_binding.capability.signature_hex.len(),
        }),
        true,
    )
}

fn required(args: &[String], flag: &str) -> Result<String, String> {
    args::flag_value(args, flag)
        .ok_or_else(|| format!("capability issue requires {flag}"))
}

fn read_json<T: serde::de::DeserializeOwned>(path: &str, label: &str) -> Result<T, String> {
    let bytes = fs::read(path)
        .map_err(|_| format!("capability_issue_{label}_read_failed: {path}"))?;
    serde_json::from_slice(&bytes)
        .map_err(|_| format!("capability_issue_{label}_decode_failed: {path}"))
}

fn now_unix_s() -> Result<u64, String> {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| "capability_issue_clock_unavailable".to_string())
}
