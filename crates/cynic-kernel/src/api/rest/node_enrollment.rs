use std::sync::Arc;

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use cynic_protocol::{
    NODE_ENROLLMENT_CERTIFICATE_SCHEMA, NODE_ENROLLMENT_GRANT_SCHEMA,
    NODE_ENROLLMENT_SIGNATURE_NAMESPACE, NodeCsr, NodeEnrollmentCertificate, NodeEnrollmentGrant,
    NodeEnrollmentRenewalRequest, NodeEnrollmentRequest, SIGNED_NODE_ENROLLMENT_GRANT_SCHEMA,
    SignedNodeEnrollmentGrant, admit_node_enrollment, admit_node_enrollment_renewal,
    node_enrollment_grant_signing_bytes,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::types::{AppState, ErrorResponse};

#[derive(Debug, Serialize)]
struct EnrollmentResponse {
    schema: &'static str,
    status: &'static str,
    grant: SignedNodeEnrollmentGrant,
}

pub async fn enroll_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<NodeEnrollmentRequest>,
) -> Response {
    let Some(runtime) = state.node_enrollment.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "machine enrollment authority is unavailable",
        );
    };
    let body = match serde_json::to_vec(&request) {
        Ok(body) => body,
        Err(_) => {
            return error(
                StatusCode::BAD_REQUEST,
                "enrollment request cannot be encoded",
            );
        }
    };
    if let Err(reason) = verify_machine_possession(&headers, &request, "/node-enrollment", &body) {
        return error(StatusCode::UNAUTHORIZED, &reason);
    }
    let Some(token) = header(&headers, "X-Cynic-Enrollment-Token") else {
        return error(
            StatusCode::UNAUTHORIZED,
            "machine enrollment token is missing",
        );
    };
    let token_digest = format!("sha256:{}", hex::encode(Sha256::digest(token.as_bytes())));
    let now = chrono::Utc::now().timestamp().max(0) as u64;
    let admission = admit_node_enrollment(&request, &runtime.authority, &token_digest, now);
    if admission.status != "admitted" {
        return error(StatusCode::FORBIDDEN, &admission.blockers.join(","));
    }
    let Some(expires_at_unix_s) = admission.grant_expires_at_unix_s else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "grant expiry was not produced",
        );
    };
    let grant = NodeEnrollmentGrant {
        schema: NODE_ENROLLMENT_GRANT_SCHEMA.to_string(),
        grant_id: format!("grant:{}", crate::infra::crypto::generate_secure_id()),
        authority_id: runtime.authority.authority_id.clone(),
        request_id: request.request_id.clone(),
        seed_id: request.seed_id.clone(),
        node_id: request.node_id.clone(),
        role: request.role.clone(),
        machine_identity_digest: request.machine_identity_digest.clone(),
        generation: 1,
        issued_at_unix_s: now,
        expires_at_unix_s,
        granted_capabilities: admission.granted_capabilities,
        evidence_refs: vec![format!("enrollment-request:{}", request.request_id)],
    };
    let mut signed = SignedNodeEnrollmentGrant {
        schema: SIGNED_NODE_ENROLLMENT_GRANT_SCHEMA.to_string(),
        authority_key_id: runtime.signer.key_id(),
        signature_namespace: NODE_ENROLLMENT_SIGNATURE_NAMESPACE.to_string(),
        grant,
        signature_hex: "0".repeat(128),
    };
    let payload = match node_enrollment_grant_signing_bytes(&signed) {
        Ok(payload) => payload,
        Err(_) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "grant signing payload failed",
            );
        }
    };
    signed.signature_hex = match runtime.signer.sign(&payload) {
        Ok(signature) => signature,
        Err(_) => return error(StatusCode::SERVICE_UNAVAILABLE, "grant signing failed"),
    };
    match runtime
        .store
        .consume_token_and_store_grant(&token_digest, signed.clone())
        .await
    {
        Ok(()) => {}
        Err(crate::domain::node_enrollment::NodeEnrollmentStoreError::Conflict(reason)) => {
            return error(StatusCode::CONFLICT, &reason);
        }
        Err(crate::domain::node_enrollment::NodeEnrollmentStoreError::Storage(_)) => {
            return error(StatusCode::SERVICE_UNAVAILABLE, "grant persistence failed");
        }
    }
    match runtime.store.grant(&request.node_id).await {
        Ok(Some(read_back)) if read_back == signed => Json(EnrollmentResponse {
            schema: "cynic.node_enrollment_response.v1",
            status: "issued_and_read_back",
            grant: read_back,
        })
        .into_response(),
        _ => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "grant read-back did not match",
        ),
    }
}

pub async fn renew_handler(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(renewal): Json<NodeEnrollmentRenewalRequest>,
) -> Response {
    let Some(runtime) = state.node_enrollment.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "machine enrollment authority is unavailable",
        );
    };
    let body = match serde_json::to_vec(&renewal) {
        Ok(body) => body,
        Err(_) => return error(StatusCode::BAD_REQUEST, "renewal request cannot be encoded"),
    };
    if let Err(reason) =
        verify_machine_possession(&headers, &renewal.request, "/node-enrollment/renew", &body)
    {
        return error(StatusCode::UNAUTHORIZED, &reason);
    }
    let current = match runtime.store.grant(&renewal.request.node_id).await {
        Ok(Some(current)) => current,
        Ok(None) => return error(StatusCode::NOT_FOUND, "current enrollment grant is absent"),
        Err(_) => return error(StatusCode::SERVICE_UNAVAILABLE, "grant read-back failed"),
    };
    let now = chrono::Utc::now().timestamp().max(0) as u64;
    let admission =
        admit_node_enrollment_renewal(&renewal, &current.grant, &runtime.authority, now);
    if admission.status != "admitted" {
        return error(StatusCode::FORBIDDEN, &admission.blockers.join(","));
    }
    let Some(expires_at_unix_s) = admission.grant_expires_at_unix_s else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "renewal expiry was not produced",
        );
    };
    let Some(generation) = current.grant.generation.checked_add(1) else {
        return error(StatusCode::CONFLICT, "grant generation overflow");
    };
    let grant = NodeEnrollmentGrant {
        schema: NODE_ENROLLMENT_GRANT_SCHEMA.to_string(),
        grant_id: format!("grant:{}", crate::infra::crypto::generate_secure_id()),
        authority_id: runtime.authority.authority_id.clone(),
        request_id: renewal.request.request_id.clone(),
        seed_id: renewal.request.seed_id.clone(),
        node_id: renewal.request.node_id.clone(),
        role: renewal.request.role.clone(),
        machine_identity_digest: renewal.request.machine_identity_digest.clone(),
        generation,
        issued_at_unix_s: now,
        expires_at_unix_s,
        granted_capabilities: admission.granted_capabilities,
        evidence_refs: vec![format!("renewed-from:{}", current.grant.grant_id)],
    };
    let signed = match sign_grant(runtime, grant) {
        Ok(signed) => signed,
        Err(reason) => return error(StatusCode::SERVICE_UNAVAILABLE, &reason),
    };
    match runtime
        .store
        .replace_grant(
            &renewal.current_grant_id,
            renewal.expected_generation,
            signed.clone(),
        )
        .await
    {
        Ok(()) => {}
        Err(crate::domain::node_enrollment::NodeEnrollmentStoreError::Conflict(reason)) => {
            return error(StatusCode::CONFLICT, &reason);
        }
        Err(_) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                "renewed grant persistence failed",
            );
        }
    }
    match runtime.store.grant(&renewal.request.node_id).await {
        Ok(Some(read_back)) if read_back == signed => Json(EnrollmentResponse {
            schema: "cynic.node_enrollment_response.v1",
            status: "issued_and_read_back",
            grant: read_back,
        })
        .into_response(),
        _ => error(
            StatusCode::SERVICE_UNAVAILABLE,
            "renewed grant read-back did not match",
        ),
    }
}

pub async fn csr_enroll_handler(
    State(state): State<Arc<AppState>>,
    Json(csr): Json<NodeCsr>,
) -> Response {
    let Some(runtime) = state.node_enrollment.as_ref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "machine enrollment authority is unavailable",
        );
    };
    let blockers = csr.validate();
    if !blockers.is_empty() {
        return error(
            StatusCode::BAD_REQUEST,
            &format!("invalid CSR: {}", blockers.join(",")),
        );
    }

    let signing_bytes = csr.signing_bytes();
    if let Err(e) = crate::infra::crypto::verify_detached_signature(
        &csr.public_key_hex,
        &csr.csr_signature_hex,
        &signing_bytes,
    ) {
        return error(
            StatusCode::UNAUTHORIZED,
            &format!("CSR signature verification failed: {e}"),
        );
    }

    let now = chrono::Utc::now().timestamp().max(0) as u64;
    let ttl_s = runtime.authority.max_grant_ttl_s.max(86400);
    let expires_at_unix_s = now.saturating_add(ttl_s);

    let mut cert = NodeEnrollmentCertificate {
        schema: NODE_ENROLLMENT_CERTIFICATE_SCHEMA.to_string(),
        certificate_id: String::new(),
        node_id: csr.node_id.clone(),
        role: csr.requested_role.clone(),
        subject_public_key_hex: csr.public_key_hex.clone(),
        kernel_authority_id: runtime.signer.key_id(),
        issued_at_unix_s: now,
        expires_at_unix_s,
        kernel_signature_hex: String::new(),
    };

    let cert_signing_bytes = cert.signing_bytes();
    let cert_id_digest = Sha256::digest(&cert_signing_bytes);
    cert.certificate_id = format!("sha256:{}", hex::encode(cert_id_digest));

    let signature_hex = match runtime.signer.sign(&cert_signing_bytes) {
        Ok(sig) => sig,
        Err(e) => {
            return error(
                StatusCode::SERVICE_UNAVAILABLE,
                &format!("certificate signing failed: {e}"),
            );
        }
    };
    cert.kernel_signature_hex = signature_hex;

    Json(cert).into_response()
}

fn sign_grant(
    runtime: &crate::domain::node_enrollment::NodeEnrollmentRuntime,
    grant: NodeEnrollmentGrant,
) -> Result<SignedNodeEnrollmentGrant, String> {
    let mut signed = SignedNodeEnrollmentGrant {
        schema: SIGNED_NODE_ENROLLMENT_GRANT_SCHEMA.to_string(),
        authority_key_id: runtime.signer.key_id(),
        signature_namespace: NODE_ENROLLMENT_SIGNATURE_NAMESPACE.to_string(),
        grant,
        signature_hex: "0".repeat(128),
    };
    let payload = node_enrollment_grant_signing_bytes(&signed)
        .map_err(|_| "grant signing payload failed".to_string())?;
    signed.signature_hex = runtime.signer.sign(&payload)?;
    Ok(signed)
}

fn verify_machine_possession(
    headers: &HeaderMap,
    request: &NodeEnrollmentRequest,
    path: &str,
    body: &[u8],
) -> Result<(), String> {
    let signature = header(headers, "X-Cynic-Signature").ok_or("machine signature is missing")?;
    let timestamp = header(headers, "X-Cynic-Timestamp").ok_or("machine timestamp is missing")?;
    let public_key =
        header(headers, "X-Cynic-Public-Key").ok_or("machine public key is missing")?;
    if public_key != request.machine_public_key_hex {
        return Err("header public key does not match enrollment request".to_string());
    }
    let payload = crate::infra::crypto::build_rest_payload("POST", path, body);
    crate::infra::crypto::verify_signature(public_key, signature, timestamp, &payload)
        .map_err(|error| format!("machine possession proof failed: {error}"))
}

fn header<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
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
