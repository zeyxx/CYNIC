use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use chrono::Utc;
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize)]
pub struct ReleaseWebhookPayload {
    pub build_id: String,
    pub commit_sha: String,
    pub status: String, // "success" | "failed"
    pub pusher: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReleaseProof {
    pub schema: &'static str,
    pub release_id: String,
    pub commit_sha: String,
    pub status: &'static str, // "pushed" | "blocked" | "skipped"
    pub reason: String,
    pub released_at_ms: i64,
}

#[derive(Clone)]
pub struct ReleaseWebhookConfig {
    pub webhook_secret: Option<String>,
    pub credential_ref: String,
    pub github_remote: String,
}

pub async fn handle_release_webhook(
    State(config): State<ReleaseWebhookConfig>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<Json<ReleaseProof>, (StatusCode, String)> {
    // 1. Verify HMAC if secret is configured (fail-closed if missing)
    let Some(secret) = &config.webhook_secret else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "webhook authentication unavailable: webhook_secret is not configured".into(),
        ));
    };
    let signature = headers
        .get("x-forgejo-signature")
        .or_else(|| headers.get("x-hub-signature-256"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !verify_signature(secret, &body, signature) {
        return Err((StatusCode::UNAUTHORIZED, "invalid webhook signature".into()));
    }

    // 2. Parse payload
    let payload: ReleaseWebhookPayload = serde_json::from_slice(&body)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid JSON: {e}")))?;

    // 3. Only process successful builds
    if payload.status != "success" {
        return Ok(Json(ReleaseProof {
            schema: "cynic.release.skip.v1",
            release_id: format!("release-{}", Uuid::new_v4()),
            commit_sha: payload.commit_sha,
            status: "skipped",
            reason: format!("build status is {}", payload.status),
            released_at_ms: Utc::now().timestamp_millis(),
        }));
    }

    // 4. Run gate
    let root = std::env::current_dir().unwrap_or_default();
    let gate = cynic_host::release_bot::run_gate(&root);

    if !gate.all_passed() {
        let failed: Vec<&str> = gate
            .checks
            .iter()
            .filter(|c| !c.passed)
            .map(|c| c.name)
            .collect();
        return Ok(Json(ReleaseProof {
            schema: "cynic.release.blocked.v1",
            release_id: format!("release-{}", Uuid::new_v4()),
            commit_sha: payload.commit_sha,
            status: "blocked",
            reason: format!("gate failed: {}", failed.join(", ")),
            released_at_ms: Utc::now().timestamp_millis(),
        }));
    }

    // 5. Push to github with credential injection
    let observation = cynic_host::source_sync::observe_all_layers(&root);
    let decision = cynic_host::source_sync::reconcile(&observation);
    let plan = cynic_host::source_sync::plan(&decision);

    if plan.steps.is_empty() {
        return Ok(Json(ReleaseProof {
            schema: "cynic.release.noop.v1",
            release_id: format!("release-{}", Uuid::new_v4()),
            commit_sha: payload.commit_sha,
            status: "skipped",
            reason: "no sync actions needed — layers converged".into(),
            released_at_ms: Utc::now().timestamp_millis(),
        }));
    }

    let proof = cynic_host::source_sync::execute_plan_with_enrollment(
        &root,
        &plan,
        Some(&config.credential_ref),
        Some(cynic_host::credential::CredentialType::GitHubPat),
    );

    Ok(Json(ReleaseProof {
        schema: "cynic.release.pushed.v1",
        release_id: format!("release-{}", Uuid::new_v4()),
        commit_sha: payload.commit_sha,
        status: if proof.success { "pushed" } else { "failed" },
        reason: if proof.success {
            format!("{} steps executed", proof.steps_completed.len())
        } else {
            "sync execution failed".into()
        },
        released_at_ms: Utc::now().timestamp_millis(),
    }))
}

fn verify_signature(secret: &str, body: &[u8], signature: &str) -> bool {
    let sig = signature.trim();
    let sig = sig.strip_prefix("sha256=").unwrap_or(sig);
    let presented = match hex::decode(sig) {
        Ok(bytes) => bytes,
        Err(_) => return false,
    };
    let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
        Ok(mac) => mac,
        Err(_) => return false,
    };
    mac.update(body);
    mac.verify_slice(&presented).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_signature_valid() {
        let secret = "test-secret";
        let body = b"{\"commit_sha\":\"abc123\"}";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = hex::encode(mac.finalize().into_bytes());
        assert!(verify_signature(secret, body, &expected));
    }

    #[test]
    fn verify_signature_invalid() {
        let secret = "test-secret";
        let body = b"{\"commit_sha\":\"abc123\"}";
        assert!(!verify_signature(secret, body, "invalid"));
    }

    #[test]
    fn verify_signature_with_sha256_prefix() {
        let secret = "test-secret";
        let body = b"{\"commit_sha\":\"abc123\"}";
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
        mac.update(body);
        let expected = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
        assert!(verify_signature(secret, body, &expected));
    }

    #[test]
    fn verify_signature_rejects_non_hex() {
        let secret = "test-secret";
        let body = b"{\"commit_sha\":\"abc123\"}";
        assert!(!verify_signature(secret, body, "not-hex"));
    }
}
