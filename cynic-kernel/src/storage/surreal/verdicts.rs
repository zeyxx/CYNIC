use super::SurrealHttpStorage;
use crate::domain::dog::{AxiomReasoning, DogScore, QScore, Verdict, VerdictKind};
use crate::domain::storage::StorageError;
use crate::storage::{escape_surreal, safe_limit, sanitize_id};

pub(super) async fn store_verdict(
    storage: &SurrealHttpStorage,
    verdict: &Verdict,
) -> Result<(), StorageError> {
    let sql = verdict_to_sql(verdict);
    storage.query_one(&sql).await?;
    Ok(())
}

pub(super) async fn get_verdict(
    storage: &SurrealHttpStorage,
    id: &str,
) -> Result<Option<Verdict>, StorageError> {
    let id = sanitize_id(id)?;
    let sql = format!("SELECT * FROM verdict WHERE verdict_id = '{id}' LIMIT 1");
    let rows = storage.query_one(&sql).await?;
    Ok(rows.first().map(row_to_verdict))
}

pub(super) async fn list_verdicts(
    storage: &SurrealHttpStorage,
    limit: u32,
) -> Result<Vec<Verdict>, StorageError> {
    let sql = format!(
        "SELECT * FROM verdict ORDER BY created_at DESC LIMIT {}",
        safe_limit(limit)
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows.iter().map(row_to_verdict).collect())
}

fn verdict_to_sql(v: &Verdict) -> String {
    let escape = |s: &str| escape_surreal(s);

    let integrity = v.integrity_hash.as_deref().unwrap_or("");
    let prev = v.prev_hash.as_deref().unwrap_or("");

    format!(
        "CREATE verdict SET \
            verdict_id = '{}', \
            domain = '{}', \
            kind = '{:?}', \
            total = {}, \
            fidelity = {}, \
            phi = {}, \
            verify = {}, \
            culture = {}, \
            burn = {}, \
            sovereignty = {}, \
            reasoning_fidelity = '{}', \
            reasoning_phi = '{}', \
            reasoning_verify = '{}', \
            reasoning_culture = '{}', \
            reasoning_burn = '{}', \
            reasoning_sovereignty = '{}', \
            dog_id = '{}', \
            stimulus = '{}', \
            integrity_hash = '{}', \
            prev_hash = '{}', \
            anomaly_detected = {}, \
            max_disagreement = {}, \
            anomaly_axiom = '{}', \
            voter_count = {}, \
            dog_scores_json = '{}', \
            failed_dogs = {}, \
            failed_dog_errors = '{}', \
            created_at = d'{}'",
        escape(&v.id),
        escape(&v.domain),
        v.kind,
        v.q_score.total,
        v.q_score.fidelity,
        v.q_score.phi,
        v.q_score.verify,
        v.q_score.culture,
        v.q_score.burn,
        v.q_score.sovereignty,
        escape(&v.reasoning.fidelity),
        escape(&v.reasoning.phi),
        escape(&v.reasoning.verify),
        escape(&v.reasoning.culture),
        escape(&v.reasoning.burn),
        escape(&v.reasoning.sovereignty),
        escape(&v.dog_id),
        escape(&v.stimulus_summary),
        escape(integrity),
        escape(prev),
        v.anomaly_detected,
        v.max_disagreement,
        escape(v.anomaly_axiom.as_deref().unwrap_or("")),
        v.voter_count,
        escape(&serde_json::to_string(&v.dog_scores).unwrap_or_else(|_| "[]".to_string())),
        serde_json::to_string(&v.failed_dogs).unwrap_or_else(|_| "[]".to_string()),
        escape(&serde_json::to_string(&v.failed_dog_errors).unwrap_or_else(|_| "{}".to_string())),
        escape(&v.timestamp),
    )
}

/// Extract a datetime from a SurrealDB JSON value.
/// SurrealDB HTTP API returns datetimes as ISO 8601 strings (observed on /observations).
/// Fallback: if the value is a non-null non-string (object/number), serialize it so we
/// capture something rather than silently returning empty.
fn extract_surreal_datetime(val: &serde_json::Value) -> String {
    if let Some(s) = val.as_str() {
        return s.to_string();
    }
    if val.is_null() {
        return String::new();
    }
    // Unexpected format — log and serialize raw
    tracing::warn!(raw = %val, "SurrealDB datetime not a string — serializing raw value");
    val.to_string().trim_matches('"').to_string()
}

fn row_to_verdict(row: &serde_json::Value) -> Verdict {
    let kind_str = row["kind"].as_str().unwrap_or("Bark");
    let kind = match kind_str {
        "Howl" => VerdictKind::Howl,
        "Wag" => VerdictKind::Wag,
        "Growl" => VerdictKind::Growl,
        _ => VerdictKind::Bark,
    };

    Verdict {
        id: row["verdict_id"].as_str().unwrap_or("").to_string(),
        domain: row["domain"].as_str().unwrap_or("general").to_string(),
        kind,
        q_score: QScore {
            total: row["total"].as_f64().unwrap_or(0.0),
            fidelity: row["fidelity"].as_f64().unwrap_or(0.0),
            phi: row["phi"].as_f64().unwrap_or(0.0),
            verify: row["verify"].as_f64().unwrap_or(0.0),
            culture: row["culture"].as_f64().unwrap_or(0.0),
            burn: row["burn"].as_f64().unwrap_or(0.0),
            sovereignty: row["sovereignty"].as_f64().unwrap_or(0.0),
        },
        reasoning: AxiomReasoning {
            fidelity: row["reasoning_fidelity"].as_str().unwrap_or("").to_string(),
            phi: row["reasoning_phi"].as_str().unwrap_or("").to_string(),
            verify: row["reasoning_verify"].as_str().unwrap_or("").to_string(),
            culture: row["reasoning_culture"].as_str().unwrap_or("").to_string(),
            burn: row["reasoning_burn"].as_str().unwrap_or("").to_string(),
            sovereignty: row["reasoning_sovereignty"]
                .as_str()
                .unwrap_or("")
                .to_string(),
        },
        dog_id: row["dog_id"].as_str().unwrap_or("").to_string(),
        stimulus_summary: row["stimulus"].as_str().unwrap_or("").to_string(),
        timestamp: extract_surreal_datetime(&row["created_at"]),
        dog_scores: {
            let verdict_id_for_log = row["verdict_id"].as_str().unwrap_or("?");
            let voter_count_for_log = row["voter_count"].as_u64().unwrap_or(0);
            let scores: Vec<DogScore> = row["dog_scores_json"]
                .as_str()
                .filter(|s| !s.is_empty())
                .and_then(|s| {
                    serde_json::from_str(s)
                        .map_err(|e| {
                            tracing::error!(
                                verdict_id = %verdict_id_for_log,
                                error = %e,
                                "dog_scores_json parse failed — verdict provenance corrupted"
                            );
                            e
                        })
                        .ok()
                })
                .unwrap_or_default();
            if scores.is_empty() && voter_count_for_log > 0 {
                tracing::warn!(
                    verdict_id = %verdict_id_for_log,
                    voter_count = voter_count_for_log,
                    "dog_scores empty but voter_count > 0 — per-dog provenance lost"
                );
            }
            scores
        },
        anomaly_detected: row["anomaly_detected"].as_bool().unwrap_or(true),
        max_disagreement: row["max_disagreement"].as_f64().unwrap_or(0.0),
        anomaly_axiom: row["anomaly_axiom"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        voter_count: row["voter_count"].as_u64().unwrap_or(0) as usize,
        failed_dogs: row["failed_dogs"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        failed_dog_errors: row["failed_dog_errors"]
            .as_str()
            .filter(|s| !s.is_empty())
            .and_then(|s| {
                serde_json::from_str(s)
                    .inspect_err(|e| tracing::warn!("failed_dog_errors deserialize: {e}"))
                    .ok()
            })
            .unwrap_or_default(),
        integrity_hash: row["integrity_hash"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        prev_hash: row["prev_hash"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
    }
}

#[cfg(test)]
#[allow(clippy::print_stderr)] // WHY: integration tests use eprintln! for diagnostic output visible in `cargo test -- --nocapture`
mod tests {
    use super::*;
    use crate::domain::storage::StoragePort;

    fn test_verdict() -> Verdict {
        Verdict {
            id: "test-001".into(),
            domain: "test".into(),
            kind: VerdictKind::Wag,
            q_score: QScore {
                total: 0.5,
                fidelity: 0.5,
                phi: 0.5,
                verify: 0.5,
                culture: 0.5,
                burn: 0.5,
                sovereignty: 0.5,
            },
            reasoning: AxiomReasoning {
                fidelity: "solid".into(),
                phi: "humble".into(),
                verify: "checked".into(),
                culture: "neutral".into(),
                burn: "concise".into(),
                sovereignty: "independent".into(),
            },
            dog_id: "deterministic".into(),
            stimulus_summary: "test stimulus".into(),
            timestamp: "2026-03-13T12:00:00Z".into(),
            dog_scores: Vec::new(),
            anomaly_detected: false,
            max_disagreement: 0.0,
            anomaly_axiom: None,
            voter_count: 0,
            failed_dogs: Vec::new(),
            failed_dog_errors: Default::default(),
            integrity_hash: Some("deadbeef".into()),
            prev_hash: None,
        }
    }

    #[test]
    fn verdict_sql_is_valid() {
        let v = test_verdict();
        let sql = verdict_to_sql(&v);
        assert!(sql.contains("CREATE verdict SET"));
        assert!(sql.contains("verdict_id = 'test-001'"));
        assert!(sql.contains("kind = 'Wag'"));
        assert!(sql.contains("fidelity = 0.5"));
        assert!(sql.contains("created_at = d'2026-03-13T12:00:00Z'"));
        assert!(sql.contains("integrity_hash = 'deadbeef'"));
        assert!(sql.contains("prev_hash = ''"));
        assert!(sql.contains("voter_count = 0"));
    }

    #[test]
    fn row_to_verdict_parses_correctly() {
        let row = serde_json::json!({
            "verdict_id": "v-123",
            "kind": "Howl",
            "total": 0.82,
            "fidelity": 0.9,
            "phi": 0.85,
            "verify": 0.88,
            "culture": 0.80,
            "burn": 0.75,
            "sovereignty": 0.70,
            "reasoning_fidelity": "strong evidence",
            "reasoning_phi": "appropriately humble",
            "reasoning_verify": "falsifiable",
            "reasoning_culture": "culturally aware",
            "reasoning_burn": "concise",
            "reasoning_sovereignty": "no vendor lock",
            "dog_id": "inference-gemini",
            "stimulus": "test claim",
            "created_at": "2026-03-13T10:00:00Z"
        });

        let v = row_to_verdict(&row);
        assert_eq!(v.id, "v-123");
        assert_eq!(v.kind, VerdictKind::Howl);
        assert_eq!(v.q_score.total, 0.82);
        assert_eq!(v.q_score.sovereignty, 0.70);
        assert_eq!(v.reasoning.burn, "concise");
        assert_eq!(v.timestamp, "2026-03-13T10:00:00Z");
        assert_eq!(v.voter_count, 0);
    }

    #[test]
    fn extract_surreal_datetime_handles_variants() {
        // Normal string (SurrealDB HTTP default)
        let s = serde_json::json!("2026-04-17T12:00:00Z");
        assert_eq!(extract_surreal_datetime(&s), "2026-04-17T12:00:00Z");

        // Null → empty
        let n = serde_json::json!(null);
        assert_eq!(extract_surreal_datetime(&n), "");

        // Object fallback (defensive — shouldn't happen with SurrealDB HTTP)
        let o = serde_json::json!({"secs": 1234567890});
        let result = extract_surreal_datetime(&o);
        assert!(
            !result.is_empty(),
            "object should produce non-empty fallback"
        );
    }

    #[test]
    fn row_to_verdict_reads_voter_count() {
        let row = serde_json::json!({
            "verdict_id": "v-456",
            "kind": "Wag",
            "total": 0.5,
            "fidelity": 0.5, "phi": 0.5, "verify": 0.5,
            "culture": 0.5, "burn": 0.5, "sovereignty": 0.5,
            "reasoning_fidelity": "", "reasoning_phi": "", "reasoning_verify": "",
            "reasoning_culture": "", "reasoning_burn": "", "reasoning_sovereignty": "",
            "dog_id": "det+gemini+sovereign",
            "stimulus": "test",
            "voter_count": 3,
            "created_at": "2026-03-27T00:00:00Z"
        });

        let v = row_to_verdict(&row);
        assert_eq!(v.voter_count, 3, "voter_count must round-trip through DB");
    }

    #[test]
    fn sql_escapes_quotes() {
        let mut v = test_verdict();
        v.stimulus_summary = "it's a \"test\"".into();
        let sql = verdict_to_sql(&v);
        assert!(sql.contains("it\\'s a "));
    }

    #[tokio::test]
    async fn store_and_retrieve_verdict() {
        let storage = match SurrealHttpStorage::init_with(
            "http://localhost:8000",
            "test_cynic",
            "ci",
        )
        .await
        {
            Ok(s) => s,
            Err(e) => {
                eprintln!("[SKIP] SurrealDB unavailable (localhost:8000): {e} — skipping test");
                return;
            }
        };

        let verdict_id = format!("test-{}", uuid::Uuid::new_v4());
        let mut v = test_verdict();
        v.id = verdict_id.clone();
        storage.store_verdict(&v).await.expect("store must succeed");

        let retrieved = storage
            .get_verdict(&verdict_id)
            .await
            .expect("get must succeed");
        assert!(retrieved.is_some());
        let r = retrieved.unwrap();
        assert_eq!(r.id, verdict_id);
        assert_eq!(r.kind, VerdictKind::Wag);

        let _ = storage
            .query_one(&format!("DELETE verdict WHERE verdict_id = '{}'", v.id))
            .await;
    }
}
