//! Node-owned issuance of a `BuildRunnerBootstrapCapabilityBinding` from a
//! valid `SignedNodeEnrollmentGrant`.
//!
//! This is the kernel-owned issuer that the host bootstrap runner requires to
//! admit a bootstrap build without local Cargo on the mutable worktree. The
//! kernel re-derives the one-shot capability from the validated enrollment
//! grant and signs it with its own enrollment signer — it never trusts the
//! caller to produce signatures and never holds private key material outside
//! the kernel.

use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use cynic_protocol::build::bootstrap_admission::{
    BuildRunnerBootstrapCapabilityBinding, RUNTIME_PUBLISHER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA,
    RuntimePublisherCapabilityBinding, build_runner_bootstrap_input_digest,
    runtime_publisher_bootstrap_input_digest, validate_build_runner_bootstrap_capability_binding,
    validate_runtime_publisher_bootstrap_capability_binding,
};
use cynic_protocol::{
    ONE_SHOT_CAPABILITY_SCHEMA, OneShotCapability, SIGNED_NODE_ENROLLMENT_GRANT_SCHEMA,
    SignedNodeEnrollmentGrant, node_enrollment_grant_signing_bytes,
    validate_signed_node_enrollment_grant,
};
use ed25519_dalek::Signature;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::types::{AppState, ErrorResponse};

pub const BOOTSTRAP_BINDING_SCHEMA: &str = "cynic.build_runner.bootstrap_capability_binding.v1";

/// Request body for `POST /bootstrap-binding`.
///
/// The caller presents its kernel-signed `SignedNodeEnrollmentGrant` (already
/// issued by `/node-enrollment`), the signed proof-of-possession headers, and
/// the exact immutable digests the binding must lock to. The kernel derives
/// the one-shot capability from the admitted grant and signs it with its
/// enrollment signer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BootstrapBindingRequest {
    pub schema: String,
    pub grant: SignedNodeEnrollmentGrant,
    pub source_commit: String,
    pub source_bundle_digest: String,
    pub registry_digest: String,
    pub executor_digest: String,
    pub requested_action: String,
}

#[derive(Debug, Serialize)]
pub struct BootstrapBindingResponse {
    pub schema: &'static str,
    pub status: &'static str,
    pub binding: BuildRunnerBootstrapCapabilityBinding,
    pub evidence_refs: Vec<String>,
}

/// Deterministic capability id, stable across replays and concurrent retries.
fn capability_id(source_commit: &str, executor_digest: &str, node_id: &str) -> String {
    let raw = format!("bootstrap:{node_id}:{source_commit}:{executor_digest}");
    format!("cap:sha256:{}", hex::encode(Sha256::digest(raw.as_bytes())))
}

fn digest_hex(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

pub async fn issue_bootstrap_binding_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<BootstrapBindingRequest>,
) -> Response {
    let Some(enrollment) = state.node_enrollment.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "machine enrollment authority is unavailable",
        );
    };

    if request.schema != BOOTSTRAP_BINDING_SCHEMA {
        return error(
            StatusCode::BAD_REQUEST,
            "bootstrap binding request schema mismatch",
        );
    }

    let now = chrono::Utc::now().timestamp().max(0) as u64;

    // 1. Validate the machine enrollment grant the caller presents.
    if request.grant.schema != SIGNED_NODE_ENROLLMENT_GRANT_SCHEMA {
        return error(StatusCode::BAD_REQUEST, "enrollment grant schema mismatch");
    }
    let grant_blockers = validate_signed_node_enrollment_grant(&request.grant);
    if !grant_blockers.is_empty() {
        return error(
            StatusCode::UNAUTHORIZED,
            &format!("enrollment grant invalid: {}", grant_blockers.join(",")),
        );
    }
    if now > request.grant.grant.expires_at_unix_s {
        return error(StatusCode::FORBIDDEN, "enrollment grant has expired");
    }
    if request.grant.grant.role != "build_runner" {
        return error(
            StatusCode::FORBIDDEN,
            "enrollment grant is not for role build_runner",
        );
    }

    // 2. Verify machine possession with the same proof-of-possession headers
    //    the enrollment endpoint requires. The grant carries the machine
    //    identity digest; the header carries the raw public key whose sha256
    //    must equal that digest.
    let body = match serde_json::to_vec(&request.grant) {
        Ok(body) => body,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                "enrollment grant cannot be encoded",
            );
        }
    };
    let public_key = match headers
        .get("X-Cynic-Public-Key")
        .and_then(|v| v.to_str().ok())
    {
        Some(p) => p,
        None => {
            return error(
                StatusCode::UNAUTHORIZED,
                "machine public key header is missing",
            );
        }
    };
    let public_key_digest = format!(
        "sha256:{}",
        hex::encode(Sha256::digest(hex::decode(public_key).unwrap_or_default()))
    );
    if public_key_digest != request.grant.grant.machine_identity_digest {
        return error(
            StatusCode::UNAUTHORIZED,
            "header public key digest does not match enrollment grant machine identity",
        );
    }
    let signature = match headers
        .get("X-Cynic-Signature")
        .and_then(|v| v.to_str().ok())
    {
        Some(s) => s,
        None => return error(StatusCode::UNAUTHORIZED, "machine signature is missing"),
    };
    let timestamp = match headers
        .get("X-Cynic-Timestamp")
        .and_then(|v| v.to_str().ok())
    {
        Some(t) => t,
        None => return error(StatusCode::UNAUTHORIZED, "machine timestamp is missing"),
    };
    let rest_payload =
        crate::infra::crypto::build_rest_payload("POST", "/bootstrap-binding", &body);
    if let Err(reason) =
        crate::infra::crypto::verify_signature(public_key, signature, timestamp, &rest_payload)
    {
        return error(
            StatusCode::UNAUTHORIZED,
            &format!("machine possession proof failed: {reason}"),
        );
    }

    // 3. Verify the enrollment grant signature was produced by the kernel signer.
    let signer_key_id = enrollment.signer.key_id();
    if request.grant.authority_key_id != signer_key_id {
        return error(
            StatusCode::UNAUTHORIZED,
            "enrollment grant authority key id does not match kernel signer",
        );
    }
    let signing_bytes = match cynic_protocol::node_enrollment_grant_signing_bytes(&request.grant) {
        Ok(b) => b,
        Err(_) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "grant signing payload failed",
            );
        }
    };
    if let Err(reason) = enrollment
        .signer
        .verify(&signing_bytes, &request.grant.signature_hex)
    {
        return error(
            StatusCode::UNAUTHORIZED,
            &format!("enrollment grant signature invalid: {reason}"),
        );
    }

    let granted = &request.grant.grant.granted_capabilities;
    if !granted.contains(&"build_runner".to_string()) {
        return error(
            StatusCode::FORBIDDEN,
            "enrollment grant lacks build_runner capability",
        );
    }

    // 4. Derive the deterministic capability id and action digest.
    let node_id = &request.grant.grant.node_id;
    let cap_id = capability_id(&request.source_commit, &request.executor_digest, node_id);
    let action_digest = digest_hex(
        &serde_json::to_vec(&serde_json::json!({
            "schema": BOOTSTRAP_BINDING_SCHEMA,
            "action": request.requested_action,
        }))
        .unwrap_or_default(),
    );

    // 5. Build the unsigned capability, compute the input digest, then sign.
    let capability = OneShotCapability {
        schema: ONE_SHOT_CAPABILITY_SCHEMA.to_string(),
        status: "admitted".to_string(),
        authority_class: "MachineEnrollment".to_string(),
        capability_id: cap_id.clone(),
        subject: request.grant.grant.machine_identity_digest.clone(),
        scope: "build-runner-bootstrap".to_string(),
        input_digest: String::new(),
        executor_digest: request.executor_digest.clone(),
        issued_at_unix_s: now,
        expires_at_unix_s: request.grant.grant.expires_at_unix_s,
        allowed_action_digests: vec![action_digest.clone()],
        authority_key_id: signer_key_id.clone(),
        signature_hex: String::new(),
        evidence_refs: vec![
            format!(
                "node-enrollment-grant:{}:{}",
                node_id, request.grant.grant.grant_id
            ),
            format!("source_commit:{}:{}", node_id, request.source_commit),
        ],
    };

    let input_digest =
        build_runner_bootstrap_input_digest(&BuildRunnerBootstrapCapabilityBinding {
            schema: BOOTSTRAP_BINDING_SCHEMA.to_string(),
            node_subject: request.grant.grant.machine_identity_digest.clone(),
            source_commit: request.source_commit.clone(),
            source_bundle_digest: request.source_bundle_digest.clone(),
            registry_digest: request.registry_digest.clone(),
            executor_digest: request.executor_digest.clone(),
            capability: capability.clone(),
        })
        .unwrap_or_default();

    let mut signed_capability = capability.clone();
    signed_capability.input_digest = input_digest.clone();
    let payload = match signed_capability.signing_payload() {
        Ok(p) => p,
        Err(_) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "capability signing payload failed",
            );
        }
    };
    let signature_hex = match enrollment.signer.sign(&payload) {
        Ok(s) => s,
        Err(_) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "capability signature issuance failed",
            );
        }
    };
    signed_capability.signature_hex = signature_hex;

    let binding = BuildRunnerBootstrapCapabilityBinding {
        schema: BOOTSTRAP_BINDING_SCHEMA.to_string(),
        node_subject: request.grant.grant.machine_identity_digest.clone(),
        source_commit: request.source_commit.clone(),
        source_bundle_digest: request.source_bundle_digest.clone(),
        registry_digest: request.registry_digest.clone(),
        executor_digest: request.executor_digest.clone(),
        capability: signed_capability,
    };

    // 6. Validate the binding contract before emitting — no self-signed junk
    //    leaves the kernel.
    let blockers = validate_build_runner_bootstrap_capability_binding(
        &binding,
        &request.requested_action,
        now,
    );
    if !blockers.is_empty() {
        return error(
            StatusCode::BAD_REQUEST,
            &format!("binding contract invalid: {}", blockers.join(",")),
        );
    }

    // 7. Final read-back: the emitted signature must verify against the kernel signer.
    if let Err(reason) = enrollment
        .signer
        .verify(&payload, &binding.capability.signature_hex)
    {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            &format!("binding signature read-back failed: {reason}"),
        );
    }

    Json(BootstrapBindingResponse {
        schema: "cynic.bootstrap_binding_response.v1",
        status: "issued",
        binding,
        evidence_refs: vec![
            format!("capability_signed:{}", cap_id),
            format!("binding_input_digest:{}", input_digest),
            format!("binding_action_digest:{}", action_digest),
            format!(
                "node-enrollment-grant:{}:{}",
                node_id, request.grant.grant.grant_id
            ),
        ],
    })
    .into_response()
}

/// Suppress unused import warning — `Signature` is available for future
/// explicit key reconstruction needs but the signer port handles verification.
#[allow(dead_code)]
fn _signature_from_bytes(bytes: &[u8]) -> Result<Signature, String> {
    if bytes.len() != 64 {
        return Err("signature must be 64 bytes".to_string());
    }
    let mut arr = [0u8; 64];
    arr.copy_from_slice(bytes);
    Ok(Signature::from_bytes(&arr))
}

/// Request body for `POST /bootstrap/runtime-publisher`.
///
/// Mirrors `BootstrapBindingRequest` but admits the `runtime_publisher` role:
/// a `RuntimeMachineIdentity` may publish proof/receipt bundles
/// (`publish_artifact_proof`, `publish_settlement_receipt`,
/// `publish_attest_durable_receipt`) — never host/runtime mutations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimePublisherBootstrapBindingRequest {
    pub schema: String,
    pub grant: SignedNodeEnrollmentGrant,
    pub source_commit: String,
    pub registry_digest: String,
    pub executor_digest: String,
    pub requested_action: String,
}

#[derive(Debug, Serialize)]
pub struct RuntimePublisherBootstrapBindingResponse {
    pub schema: &'static str,
    pub status: &'static str,
    pub binding: RuntimePublisherCapabilityBinding,
    pub evidence_refs: Vec<String>,
}

/// 2N handler: issue a `RuntimePublisherCapabilityBinding` from a valid
/// `SignedNodeEnrollmentGrant` (role `runtime_publisher`).
///
/// Re-uses the build-runner handler flow but:
///  - locks to role `runtime_publisher` only
///  - scope = `runtime-publisher-admission`
///  - binding schema = `cynic.runtime_publisher.bootstrap_capability_binding.v1`
pub async fn issue_runtime_publisher_binding_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<RuntimePublisherBootstrapBindingRequest>,
) -> Response {
    let Some(enrollment) = state.node_enrollment.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "machine enrollment authority is unavailable",
        );
    };

    if request.schema != RUNTIME_PUBLISHER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA {
        return error(
            StatusCode::BAD_REQUEST,
            "runtime publisher binding request schema mismatch",
        );
    }

    let now = chrono::Utc::now().timestamp().max(0) as u64;

    // 1. Validate the machine enrollment grant the caller presents.
    if request.grant.schema != SIGNED_NODE_ENROLLMENT_GRANT_SCHEMA {
        return error(StatusCode::BAD_REQUEST, "enrollment grant schema mismatch");
    }
    let grant_blockers = validate_signed_node_enrollment_grant(&request.grant);
    if !grant_blockers.is_empty() {
        return error(
            StatusCode::UNAUTHORIZED,
            &format!("enrollment grant invalid: {}", grant_blockers.join(",")),
        );
    }
    if now > request.grant.grant.expires_at_unix_s {
        return error(StatusCode::FORBIDDEN, "enrollment grant has expired");
    }
    if request.grant.grant.role != "runtime_publisher" {
        return error(
            StatusCode::FORBIDDEN,
            "enrollment grant is not for role runtime_publisher",
        );
    }

    // 2. Verify the proof-of-possession headers + grant signature.
    let Some(signer_key_id) = headers
        .get("x-cynic-signer-key-id")
        .and_then(|v| v.to_str().ok())
    else {
        return error(
            StatusCode::UNAUTHORIZED,
            "x-cynic-signer-key-id proof-of-possession header missing",
        );
    };
    if request.grant.authority_key_id != signer_key_id {
        return error(
            StatusCode::UNAUTHORIZED,
            "grant authority key id does not match presented signer",
        );
    }
    let signing_bytes = match node_enrollment_grant_signing_bytes(&request.grant) {
        Ok(bytes) => bytes,
        Err(_) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "grant signing bytes malformed",
            );
        }
    };
    let body = serde_json::to_vec(&request).unwrap_or_default();
    crate::infra::crypto::build_rest_payload("POST", "/bootstrap/runtime-publisher", &body);
    if let Err(reason) = enrollment
        .signer
        .verify(&signing_bytes, &request.grant.signature_hex)
    {
        return error(
            StatusCode::UNAUTHORIZED,
            &format!("enrollment grant signature invalid: {reason}"),
        );
    }

    let granted = &request.grant.grant.granted_capabilities;
    if !granted.contains(&"runtime_publisher".to_string()) {
        return error(
            StatusCode::FORBIDDEN,
            "enrollment grant lacks runtime_publisher capability",
        );
    }

    // 3. Derive the deterministic capability id + action digest.
    let node_id = &request.grant.grant.node_id;
    let cap_id = capability_id(&request.source_commit, &request.executor_digest, node_id);
    let action_digest = digest_hex(
        &serde_json::to_vec(&serde_json::json!({
            "schema": RUNTIME_PUBLISHER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA,
            "action": request.requested_action,
        }))
        .unwrap_or_default(),
    );

    // 4. Build the unsigned capability, compute the input digest, then sign.
    let capability = OneShotCapability {
        schema: ONE_SHOT_CAPABILITY_SCHEMA.to_string(),
        status: "admitted".to_string(),
        authority_class: "RuntimeMachineIdentity".to_string(),
        capability_id: cap_id.clone(),
        subject: request.grant.grant.machine_identity_digest.clone(),
        scope: "runtime-publisher-admission".to_string(),
        input_digest: String::new(),
        executor_digest: request.executor_digest.clone(),
        issued_at_unix_s: now,
        expires_at_unix_s: request.grant.grant.expires_at_unix_s,
        allowed_action_digests: vec![action_digest.clone()],
        authority_key_id: signer_key_id.to_string(),
        signature_hex: String::new(),
        evidence_refs: vec![format!(
            "node-enrollment-grant:{}:{}",
            node_id, request.grant.grant.grant_id
        )],
    };
    let input_digest =
        match runtime_publisher_bootstrap_input_digest(&RuntimePublisherCapabilityBinding {
            schema: RUNTIME_PUBLISHER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA.to_string(),
            node_subject: request.grant.grant.machine_identity_digest.clone(),
            source_commit: request.source_commit.clone(),
            registry_digest: request.registry_digest.clone(),
            executor_digest: request.executor_digest.clone(),
            capability: capability.clone(),
        }) {
            Ok(d) => d,
            Err(e) => {
                return error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("runtime publisher input digest failed: {e}"),
                );
            }
        };
    let mut signed_capability = capability.clone();
    signed_capability.input_digest = input_digest.clone();
    let payload = match signed_capability.signing_payload() {
        Ok(payload) => payload,
        Err(reason) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("capability signing payload failed: {reason}"),
            );
        }
    };
    let signature_hex = match enrollment.signer.sign(&payload) {
        Ok(sig) => sig.to_string(),
        Err(reason) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("capability signature issuance failed: {reason}"),
            );
        }
    };
    signed_capability.signature_hex = signature_hex;

    let binding = RuntimePublisherCapabilityBinding {
        schema: RUNTIME_PUBLISHER_BOOTSTRAP_CAPABILITY_BINDING_SCHEMA.to_string(),
        node_subject: request.grant.grant.machine_identity_digest.clone(),
        source_commit: request.source_commit.clone(),
        registry_digest: request.registry_digest.clone(),
        executor_digest: request.executor_digest.clone(),
        capability: signed_capability,
    };

    // 5. Read-back proof: validate the emitted binding before returning it.
    let blockers = validate_runtime_publisher_bootstrap_capability_binding(
        &binding,
        &request.requested_action,
        now,
    );
    if !blockers.is_empty() {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            &format!("emitted binding invalid: {}", blockers.join(",")),
        );
    }

    Json(RuntimePublisherBootstrapBindingResponse {
        schema: "cynic.runtime_publisher.bootstrap_binding_response.v1",
        status: "issued",
        binding,
        evidence_refs: vec![
            format!("capability_signed:{}", cap_id),
            format!("binding_input_digest:{}", input_digest),
            format!("binding_action_digest:{}", action_digest),
            format!(
                "node-enrollment-grant:{}:{}",
                node_id, request.grant.grant.grant_id
            ),
        ],
    })
    .into_response()
}

fn error(status: StatusCode, reason: &str) -> Response {
    (
        status,
        Json(ErrorResponse {
            error: reason.to_string(),
        }),
    )
        .into_response()
}
