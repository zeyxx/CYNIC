use super::SurrealHttpStorage;
use crate::domain::storage::StorageError;
use crate::domain::verdict_queue::{QueuedVerdict, SubmissionStatus};
use crate::storage::{escape_surreal, sanitize_id};

pub(super) async fn enqueue_verdict(
    storage: &SurrealHttpStorage,
    verdict_id: &str,
    content_hash: &str,
    q_score: f64,
) -> Result<(), StorageError> {
    if q_score < 0.618 {
        return Err(StorageError::InvalidInput(format!(
            "verdict q_score {q_score} below threshold 0.618"
        )));
    }

    let verdict_id = sanitize_id(verdict_id)?;
    let content_hash = sanitize_id(content_hash)?;
    let now = chrono::Utc::now().to_rfc3339();

    let sql = format!(
        "CREATE queued_verdict SET \
            verdict_id = '{}', \
            content_hash = '{}', \
            q_score = {}, \
            status = 'pending', \
            retry_count = 0, \
            created_at = d'{}', \
            submitted_at = NULL, \
            confirmed_at = NULL, \
            tx_signature = NULL, \
            verdict_pda = NULL, \
            error_reason = NULL",
        verdict_id, content_hash, q_score, now
    );

    storage.query_one(&sql).await?;
    Ok(())
}

pub(super) async fn list_pending_verdicts(
    storage: &SurrealHttpStorage,
    limit: u32,
) -> Result<Vec<QueuedVerdict>, StorageError> {
    let sql = format!(
        "SELECT * FROM queued_verdict WHERE status = 'pending' ORDER BY created_at ASC LIMIT {}",
        limit.min(100)
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows.iter().map(row_to_queued_verdict).collect())
}

pub(super) async fn get_queued_verdict(
    storage: &SurrealHttpStorage,
    verdict_id: &str,
) -> Result<Option<QueuedVerdict>, StorageError> {
    let verdict_id = sanitize_id(verdict_id)?;
    let sql = format!(
        "SELECT * FROM queued_verdict WHERE verdict_id = '{}' LIMIT 1",
        verdict_id
    );
    let rows = storage.query_one(&sql).await?;
    Ok(rows.first().map(row_to_queued_verdict))
}

pub(super) async fn update_verdict_submitted(
    storage: &SurrealHttpStorage,
    verdict_id: &str,
    tx_signature: &str,
    verdict_pda: &str,
) -> Result<(), StorageError> {
    let verdict_id = sanitize_id(verdict_id)?;
    let tx_signature = escape_surreal(tx_signature);
    let verdict_pda = escape_surreal(verdict_pda);
    let now = chrono::Utc::now().to_rfc3339();

    let sql = format!(
        "UPDATE queued_verdict SET status = 'submitted', tx_signature = '{}', verdict_pda = '{}', submitted_at = d'{}' WHERE verdict_id = '{}'",
        tx_signature, verdict_pda, now, verdict_id
    );

    storage.query_one(&sql).await?;
    Ok(())
}

pub(super) async fn update_verdict_confirmed(
    storage: &SurrealHttpStorage,
    verdict_id: &str,
) -> Result<(), StorageError> {
    let verdict_id = sanitize_id(verdict_id)?;
    let now = chrono::Utc::now().to_rfc3339();

    let sql = format!(
        "UPDATE queued_verdict SET status = 'confirmed', confirmed_at = d'{}' WHERE verdict_id = '{}'",
        now, verdict_id
    );

    storage.query_one(&sql).await?;
    Ok(())
}

pub(super) async fn update_verdict_failed(
    storage: &SurrealHttpStorage,
    verdict_id: &str,
    error_reason: &str,
    retry_count: u32,
) -> Result<(), StorageError> {
    let verdict_id = sanitize_id(verdict_id)?;
    let error_reason = escape_surreal(error_reason);
    let status = if retry_count >= 3 {
        "failed"
    } else {
        "pending"
    };

    let sql = format!(
        "UPDATE queued_verdict SET status = '{}', error_reason = '{}', retry_count = {} WHERE verdict_id = '{}'",
        status, error_reason, retry_count, verdict_id
    );

    storage.query_one(&sql).await?;
    Ok(())
}

pub(super) async fn queue_status_counts(
    storage: &SurrealHttpStorage,
) -> Result<(u64, u64, u64, u64), StorageError> {
    let sql = r#"
        SELECT
            count(id) as cnt,
            status
        FROM queued_verdict
        GROUP BY status
    "#;
    let rows = storage.query_one(sql).await?;

    let mut pending = 0u64;
    let mut submitted = 0u64;
    let mut confirmed = 0u64;
    let mut failed = 0u64;

    for row in rows.iter() {
        let status = row["status"].as_str().unwrap_or("");
        let count = row["cnt"].as_u64().unwrap_or(0);
        match status {
            "pending" => pending = count,
            "submitted" => submitted = count,
            "confirmed" => confirmed = count,
            "failed" => failed = count,
            _ => {}
        }
    }

    Ok((pending, submitted, confirmed, failed))
}

fn row_to_queued_verdict(row: &serde_json::Value) -> QueuedVerdict {
    let status_str = row["status"].as_str().unwrap_or("pending");
    let status = match status_str {
        "submitted" => SubmissionStatus::Submitted,
        "confirmed" => SubmissionStatus::Confirmed,
        "failed" => SubmissionStatus::Failed,
        _ => SubmissionStatus::Pending,
    };

    QueuedVerdict {
        verdict_id: row["verdict_id"].as_str().unwrap_or("").to_string(),
        content_hash: row["content_hash"].as_str().unwrap_or("").to_string(),
        q_score: row["q_score"].as_f64().unwrap_or(0.0),
        status,
        tx_signature: row["tx_signature"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        verdict_pda: row["verdict_pda"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        retry_count: row["retry_count"].as_u64().unwrap_or(0) as u32,
        error_reason: row["error_reason"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        created_at: row["created_at"].as_str().unwrap_or("").to_string(),
        submitted_at: row["submitted_at"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
        confirmed_at: row["confirmed_at"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string()),
    }
}
