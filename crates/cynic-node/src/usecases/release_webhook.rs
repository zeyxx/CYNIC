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

use cynic_release::{
    RelayActionKind, RelayDecisionProof, RelayPolicyContract, ReleaseEligibilityVerdict,
    ReleaseVersion,
};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Deserialize)]
pub struct ReleaseWebhookPayload {
    pub build_id: String,
    pub commit_sha: String,
    pub status: String,      // "success" | "failed"
    pub tag: Option<String>, // release tag if available
    pub pusher: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ReleaseProof {
    pub schema: &'static str,
    pub release_id: String,
    pub commit_sha: String,
    pub status: &'static str, // "pushed" | "blocked" | "skipped" | "failed"
    pub reason: String,
    pub relay_decision: Option<RelayDecisionProofView>,
    pub readback: Option<RelayReadbackView>,
    pub released_at_ms: i64,
}

#[derive(Debug, Serialize)]
pub struct RelayDecisionProofView {
    pub schema: &'static str,
    pub action_kind: &'static str,
    pub target_remote: String,
    pub target_ref: String,
}

#[derive(Debug, Serialize)]
pub struct RelayReadbackView {
    pub matches: bool,
    pub local_tag_sha: Option<String>,
    pub remote_tag_sha: Option<String>,
}

#[derive(Clone)]
pub struct ReleaseWebhookConfig {
    pub webhook_secret: Option<String>,
    pub credential_ref: String,
    pub github_remote: String,
    pub relay_policy_path: String,
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
            relay_decision: None,
            readback: None,
            released_at_ms: Utc::now().timestamp_millis(),
        }));
    }

    // 4. Run gate via host adapter and evaluate via cynic-release domain engine
    let root = std::env::current_dir().unwrap_or_default();
    let proofs = cynic_host::adapters::release_gate_adapter::collect_host_gate_proofs(&root);
    let report = cynic_release::evaluate_gate_records(proofs);

    if !report.all_passed {
        return Ok(Json(ReleaseProof {
            schema: "cynic.release.blocked.v1",
            release_id: format!("release-{}", Uuid::new_v4()),
            commit_sha: payload.commit_sha,
            status: "blocked",
            reason: format!("gate failed: {}", report.blockers.join(", ")),
            relay_decision: None,
            readback: None,
            released_at_ms: Utc::now().timestamp_millis(),
        }));
    }

    // 5. Read relay policy and evaluate relay eligibility (pure domain logic)
    let relay_contract = match std::fs::read_to_string(&config.relay_policy_path) {
        Ok(json) => match RelayPolicyContract::parse(&json) {
            Ok(c) => c,
            Err(e) => {
                return Ok(Json(ReleaseProof {
                    schema: "cynic.release.failed.v1",
                    release_id: format!("release-{}", Uuid::new_v4()),
                    commit_sha: payload.commit_sha,
                    status: "failed",
                    reason: format!("relay policy parse error: {e}"),
                    relay_decision: None,
                    readback: None,
                    released_at_ms: Utc::now().timestamp_millis(),
                }));
            }
        },
        Err(e) => {
            return Ok(Json(ReleaseProof {
                schema: "cynic.release.failed.v1",
                release_id: format!("release-{}", Uuid::new_v4()),
                commit_sha: payload.commit_sha,
                status: "failed",
                reason: format!("relay policy read error: {e}"),
                relay_decision: None,
                readback: None,
                released_at_ms: Utc::now().timestamp_millis(),
            }));
        }
    };

    let tag_name = payload.tag.clone().unwrap_or_else(|| "v0.2.0".to_string());
    let version =
        ReleaseVersion::parse_tag(&tag_name).unwrap_or_else(|| ReleaseVersion::new(0, 2, 0, None));

    let gate_verdict = ReleaseEligibilityVerdict::Eligible {
        schema: "cynic.release_verdict.v1",
        version,
        commit_sha: payload.commit_sha.clone(),
        tag_name: tag_name.clone(),
        decided_at_unix_s: Utc::now().timestamp() as u64,
    };

    let target_ref = format!("refs/tags/{tag_name}");

    let relay_decision = cynic_release::evaluate_relay_eligibility(
        &gate_verdict,
        &relay_contract,
        &config.github_remote,
        &target_ref,
        false,
    );

    let relay_decision_view = match &relay_decision {
        RelayDecisionProof::Authorized {
            action_kind,
            target_remote,
            target_ref,
            schema,
            ..
        } => Some(RelayDecisionProofView {
            schema: *schema,
            action_kind: match *action_kind {
                RelayActionKind::TagPush => "tag_push",
                RelayActionKind::MainFastForward => "main_fast_forward",
                RelayActionKind::MainForcedPromotion => "main_forced_promotion",
            },
            target_remote: target_remote.clone(),
            target_ref: target_ref.clone(),
        }),
        RelayDecisionProof::Rejected { .. } => None,
    };

    // 6. If authorized, push via host relay adapter
    if relay_decision.is_authorized() {
        let push_success = match &relay_decision {
            RelayDecisionProof::Authorized { action_kind, .. } => match *action_kind {
                RelayActionKind::TagPush => {
                    cynic_host::adapters::release_relay_adapter::push_tag_to_remote(
                        &root,
                        &config.github_remote,
                        &tag_name,
                    )
                }
                RelayActionKind::MainForcedPromotion => {
                    cynic_host::adapters::release_relay_adapter::force_push_tag_to_remote(
                        &root,
                        &config.github_remote,
                        &tag_name,
                    )
                }
                _ => cynic_host::adapters::release_relay_adapter::push_tag_to_remote(
                    &root,
                    &config.github_remote,
                    &tag_name,
                ),
            },
            _ => false,
        };

        if !push_success {
            return Ok(Json(ReleaseProof {
                schema: "cynic.release.failed.v1",
                release_id: format!("release-{}", Uuid::new_v4()),
                commit_sha: payload.commit_sha,
                status: "failed",
                reason: "relay push failed".into(),
                relay_decision: relay_decision_view,
                readback: None,
                released_at_ms: Utc::now().timestamp_millis(),
            }));
        }

        // 7. Read-back proof: verify SHA match after push
        let readback = cynic_host::adapters::release_relay_adapter::verify_relay_readback(
            &root,
            &config.github_remote,
            &tag_name,
        );

        return Ok(Json(ReleaseProof {
            schema: "cynic.release.pushed.v1",
            release_id: format!("release-{}", Uuid::new_v4()),
            commit_sha: payload.commit_sha,
            status: if readback.matches { "pushed" } else { "failed" },
            reason: if readback.matches {
                format!(
                    "relay successful — tag {tag_name} verified on {}",
                    config.github_remote
                )
            } else {
                format!(
                    "read-back mismatch for tag {}: local={:?} remote={:?}",
                    tag_name, readback.local_tag_sha, readback.remote_tag_sha
                )
            },
            relay_decision: relay_decision_view,
            readback: Some(RelayReadbackView {
                matches: readback.matches,
                local_tag_sha: readback.local_tag_sha,
                remote_tag_sha: readback.remote_tag_sha,
            }),
            released_at_ms: Utc::now().timestamp_millis(),
        }));
    }

    // 8. Rejected by policy
    Ok(Json(ReleaseProof {
        schema: "cynic.release.blocked.v1",
        release_id: format!("release-{}", Uuid::new_v4()),
        commit_sha: payload.commit_sha,
        status: "blocked",
        reason: "relay rejected by policy".into(),
        relay_decision: relay_decision_view,
        readback: None,
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
