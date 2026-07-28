use cynic_node::ports::agent_workspace_port::{
    AgentWorkspaceClaimStorePort, AgentWorkspacePortError,
};
use cynic_protocol::{
    AgentWorkspaceClaimAdmission, AgentWorkspaceClaimIntent, AgentWorkspaceClaimObservation,
    AgentWorkspaceClaimOutcome, AgentWorkspaceZone, LeaseGuard, agent_workspace_target,
    workspace_claim_admission_matches,
};

use crate::storage::{SurrealHttpStorage, escape_surreal, sanitize_id};

impl AgentWorkspaceClaimStorePort for SurrealHttpStorage {
    async fn reserve(
        &self,
        intent: &AgentWorkspaceClaimIntent,
        admission: &AgentWorkspaceClaimAdmission,
        now_ms: u64,
    ) -> Result<AgentWorkspaceClaimOutcome, AgentWorkspacePortError> {
        if !workspace_claim_admission_matches(intent, admission) {
            return Ok(AgentWorkspaceClaimOutcome::Rejected {
                violations: vec![cynic_protocol::AgentWorkspaceViolation {
                    code: "workspace_claim_admission_rejected".to_string(),
                    message: "claim admission is absent, denied, or bound to another intent"
                        .to_string(),
                }],
            });
        }
        validate_claim_ids(intent)?;
        let expires_at_ms = now_ms.checked_add(intent.requested_ttl_ms).ok_or_else(|| {
            port_error("workspace_claim_expiry_overflow", "claim expiry overflow")
        })?;
        let hard_expires_at_ms =
            now_ms
                .checked_add(admission.max_lifetime_ms)
                .ok_or_else(|| {
                    port_error(
                        "workspace_claim_hard_expiry_overflow",
                        "claim hard expiry overflow",
                    )
                })?;
        let lease_id = next_workspace_lease_id();
        sanitize_id(&lease_id).map_err(storage_validation_error)?;
        let zones = serde_json::to_string(&intent.zones).map_err(|error| {
            port_error(
                "workspace_claim_zones_not_serializable",
                format!("zones could not be serialized: {error}"),
            )
        })?;
        let target = agent_workspace_target(&intent.repository_id, &intent.workspace_id);
        let sql = format!(
            "BEGIN TRANSACTION; \
             LET $lock = UPSERT ONLY agent_workspace_claim_lock:`{repository_key}` SET \
                repository_id = '{repository}', fencing_token += 1 RETURN AFTER; \
             LET $requested = {zones}; \
             LET $owned = SELECT * FROM agent_workspace_claim WHERE \
                active = true AND expires_at_ms > {now_ms} \
                AND repository_id = '{repository}' \
                AND workspace_id = '{workspace}' \
                AND workload_id = '{workload}' \
                AND agent_id = '{agent}' \
                AND intent_digest = '{intent_digest}'; \
             IF array::len($owned) > 0 {{ \
                RETURN {{ kind: 'granted', claim: array::first($owned) }} \
             }}; \
             LET $conflicts = SELECT * FROM agent_workspace_claim WHERE \
                active = true AND expires_at_ms > {now_ms} \
                AND repository_id = '{repository}' \
                AND zones.any(|$existing| $requested.any(|$candidate| \
                    $existing.path = $candidate.path \
                    OR ($existing.kind = 'tree' AND string::starts_with($candidate.path, string::concat($existing.path, '/'))) \
                    OR ($candidate.kind = 'tree' AND string::starts_with($existing.path, string::concat($candidate.path, '/'))) \
                )); \
             IF array::len($conflicts) > 0 {{ \
                RETURN {{ kind: 'conflict', conflicts: $conflicts }} \
             }}; \
             LET $claim = CREATE ONLY agent_workspace_claim:`{lease_key}` CONTENT {{ \
                repository_id: '{repository}', workspace_id: '{workspace}', \
                workload_id: '{workload}', node_id: '{node}', agent_id: '{agent}', \
                target: '{target}', lease_id: '{lease}', intent_digest: '{intent_digest}', \
                fencing_token: $lock.fencing_token, zones: $requested, active: true, \
                claimed_at_ms: {now_ms}, expires_at_ms: {expires_at_ms}, \
                hard_expires_at_ms: {hard_expires_at_ms}, released_at_ms: NONE \
             }}; \
             RETURN {{ kind: 'granted', claim: $claim }}; \
             COMMIT TRANSACTION;",
            repository_key =
                sanitize_id(&intent.repository_id).map_err(storage_validation_error)?,
            lease_key = lease_id,
            repository = escape_surreal(&intent.repository_id),
            workspace = escape_surreal(&intent.workspace_id),
            workload = escape_surreal(&intent.workload_id),
            node = escape_surreal(&intent.node_id),
            agent = escape_surreal(&intent.agent_id),
            target = escape_surreal(&target),
            lease = escape_surreal(&lease_id),
            intent_digest = escape_surreal(&admission.intent_digest),
        );
        let values = self.query(&sql).await.map_err(|error| {
            port_error(
                "workspace_claim_transaction_failed",
                format!("atomic claim transaction failed: {error}"),
            )
        })?;
        parse_reservation_outcome(values)
    }

    async fn repository_claims(
        &self,
        repository_id: &str,
        now_ms: u64,
    ) -> Result<Vec<AgentWorkspaceClaimObservation>, AgentWorkspacePortError> {
        sanitize_id(repository_id).map_err(storage_validation_error)?;
        let sql = format!(
            "SELECT * FROM agent_workspace_claim WHERE repository_id = '{}' \
             AND active = true AND expires_at_ms > {now_ms};",
            escape_surreal(repository_id)
        );
        self.query_one(&sql)
            .await
            .map_err(|error| {
                port_error(
                    "workspace_claim_observation_failed",
                    format!("cannot observe repository claims: {error}"),
                )
            })?
            .iter()
            .map(parse_claim)
            .collect()
    }

    async fn renew(
        &self,
        guard: &LeaseGuard,
        requested_ttl_ms: u64,
        now_ms: u64,
    ) -> Result<AgentWorkspaceClaimObservation, AgentWorkspacePortError> {
        if requested_ttl_ms == 0 {
            return Err(port_error(
                "workspace_claim_renewal_ttl_missing",
                "renewal TTL must be positive",
            ));
        }
        let expires_at_ms = now_ms.checked_add(requested_ttl_ms).ok_or_else(|| {
            port_error(
                "workspace_claim_renewal_expiry_overflow",
                "renewal expiry overflow",
            )
        })?;
        let sql = format!(
            "UPDATE agent_workspace_claim SET expires_at_ms = {expires_at_ms} WHERE \
             active = true AND expires_at_ms > {now_ms} \
             AND hard_expires_at_ms >= {expires_at_ms} \
             AND agent_id = '{agent}' AND target = '{target}' \
             AND lease_id = '{lease}' AND fencing_token = {fence} RETURN AFTER;",
            agent = escape_surreal(&guard.agent_id),
            target = escape_surreal(&guard.target),
            lease = escape_surreal(&guard.lease_id),
            fence = guard.fencing_token,
        );
        let rows = self.query_one(&sql).await.map_err(|error| {
            port_error(
                "workspace_claim_renewal_failed",
                format!("claim renewal failed: {error}"),
            )
        })?;
        rows.first().map(parse_claim).transpose()?.ok_or_else(|| {
            port_error(
                "workspace_claim_guard_rejected",
                "claim renewal rejected stale, expired, or overlong guard",
            )
        })
    }

    async fn release(
        &self,
        guard: &LeaseGuard,
        now_ms: u64,
    ) -> Result<(), AgentWorkspacePortError> {
        let sql = format!(
            "UPDATE agent_workspace_claim SET active = false, released_at_ms = {now_ms} WHERE \
             active = true AND expires_at_ms > {now_ms} \
             AND agent_id = '{agent}' AND target = '{target}' \
             AND lease_id = '{lease}' AND fencing_token = {fence} RETURN AFTER;",
            agent = escape_surreal(&guard.agent_id),
            target = escape_surreal(&guard.target),
            lease = escape_surreal(&guard.lease_id),
            fence = guard.fencing_token,
        );
        let rows = self.query_one(&sql).await.map_err(|error| {
            port_error(
                "workspace_claim_release_failed",
                format!("claim release failed: {error}"),
            )
        })?;
        if rows.is_empty() {
            return Err(port_error(
                "workspace_claim_guard_rejected",
                "claim release rejected stale or expired guard",
            ));
        }
        Ok(())
    }

    async fn finalize(
        &self,
        guard: &LeaseGuard,
        proof_ref: &str,
        now_ms: u64,
    ) -> Result<(), AgentWorkspacePortError> {
        if !valid_sha256_digest(proof_ref) {
            return Err(port_error(
                "workspace_claim_proof_ref_invalid",
                "claim finalization requires a sha256 content-addressed proof",
            ));
        }
        let sql = format!(
            "UPDATE agent_workspace_claim SET active = false, finalized_at_ms = {now_ms}, \
             proof_ref = '{proof}' WHERE active = true AND expires_at_ms > {now_ms} \
             AND agent_id = '{agent}' AND target = '{target}' \
             AND lease_id = '{lease}' AND fencing_token = {fence} RETURN AFTER;",
            proof = escape_surreal(proof_ref),
            agent = escape_surreal(&guard.agent_id),
            target = escape_surreal(&guard.target),
            lease = escape_surreal(&guard.lease_id),
            fence = guard.fencing_token,
        );
        let rows = self.query_one(&sql).await.map_err(|failure| {
            port_error(
                "workspace_claim_finalization_failed",
                format!("claim finalization failed: {failure}"),
            )
        })?;
        if rows.is_empty() {
            return Err(port_error(
                "workspace_claim_guard_rejected",
                "claim finalization rejected stale or expired guard",
            ));
        }
        Ok(())
    }
}

fn parse_reservation_outcome(
    values: Vec<Vec<serde_json::Value>>,
) -> Result<AgentWorkspaceClaimOutcome, AgentWorkspacePortError> {
    let outcome = values
        .into_iter()
        .flatten()
        .find(|value| value.get("kind").is_some())
        .ok_or_else(|| {
            port_error(
                "workspace_claim_outcome_missing",
                "claim transaction returned no structured outcome",
            )
        })?;
    match outcome.get("kind").and_then(serde_json::Value::as_str) {
        Some("granted") => outcome
            .get("claim")
            .ok_or_else(|| {
                port_error(
                    "workspace_claim_grant_missing",
                    "claim transaction returned no grant",
                )
            })
            .and_then(parse_claim)
            .map(|claim| AgentWorkspaceClaimOutcome::Granted { claim }),
        Some("conflict") => {
            let conflicts = outcome
                .get("conflicts")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| {
                    port_error(
                        "workspace_claim_conflicts_missing",
                        "claim conflict outcome returned no conflicts",
                    )
                })?
                .iter()
                .map(parse_claim)
                .collect::<Result<Vec<_>, _>>()?;
            Ok(AgentWorkspaceClaimOutcome::Conflict { conflicts })
        }
        _ => Err(port_error(
            "workspace_claim_outcome_invalid",
            "claim transaction returned an unknown outcome",
        )),
    }
}

fn parse_claim(
    value: &serde_json::Value,
) -> Result<AgentWorkspaceClaimObservation, AgentWorkspacePortError> {
    let string = |field: &str| {
        value
            .get(field)
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
            .ok_or_else(|| {
                port_error(
                    "workspace_claim_record_invalid",
                    format!("claim record has no string field '{field}'"),
                )
            })
    };
    let number = |field: &str| {
        value
            .get(field)
            .and_then(serde_json::Value::as_u64)
            .ok_or_else(|| {
                port_error(
                    "workspace_claim_record_invalid",
                    format!("claim record has no integer field '{field}'"),
                )
            })
    };
    let repository_id = string("repository_id")?;
    let workspace_id = string("workspace_id")?;
    let agent_id = string("agent_id")?;
    let zones = serde_json::from_value::<Vec<AgentWorkspaceZone>>(
        value.get("zones").cloned().unwrap_or_default(),
    )
    .map_err(|error| {
        port_error(
            "workspace_claim_zones_invalid",
            format!("claim zones are invalid: {error}"),
        )
    })?;
    Ok(AgentWorkspaceClaimObservation {
        workload_id: string("workload_id")?,
        workspace_id: workspace_id.clone(),
        node_id: string("node_id")?,
        agent_id: agent_id.clone(),
        zones,
        guard: LeaseGuard {
            agent_id,
            target: agent_workspace_target(&repository_id, &workspace_id),
            lease_id: string("lease_id")?,
            fencing_token: number("fencing_token")?,
        },
        expires_at_ms: number("expires_at_ms")?,
        hard_expires_at_ms: number("hard_expires_at_ms")?,
    })
}

fn validate_claim_ids(intent: &AgentWorkspaceClaimIntent) -> Result<(), AgentWorkspacePortError> {
    for (field, value) in [
        ("repository_id", intent.repository_id.as_str()),
        ("workspace_id", intent.workspace_id.as_str()),
        ("workload_id", intent.workload_id.as_str()),
        ("node_id", intent.node_id.as_str()),
        ("agent_id", intent.agent_id.as_str()),
    ] {
        sanitize_id(value).map_err(|error| {
            port_error(
                "workspace_claim_identifier_rejected",
                format!("{field} is not storage-safe: {error}"),
            )
        })?;
    }
    Ok(())
}

fn next_workspace_lease_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{SystemTime, UNIX_EPOCH};

    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let sequence = SEQUENCE.fetch_add(1, Ordering::Relaxed);
    format!("workspace-{millis}-{}-{sequence}", std::process::id())
}

fn storage_validation_error(
    error: crate::domain::storage::StorageError,
) -> AgentWorkspacePortError {
    port_error(
        "workspace_claim_identifier_rejected",
        format!("storage identifier rejected: {error}"),
    )
}

fn port_error(code: impl Into<String>, message: impl Into<String>) -> AgentWorkspacePortError {
    AgentWorkspacePortError::new(code, message)
}

fn valid_sha256_digest(value: &str) -> bool {
    value
        .strip_prefix("sha256:")
        .is_some_and(|hex| hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

#[cfg(test)]
#[path = "workspace_claims_tests.rs"]
mod tests;
