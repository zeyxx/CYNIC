#[async_trait]
impl SubmissionStorage for InMemoryStorage {
    async fn enqueue_verdict(
        &self,
        _verdict_id: &str,
        _content_hash: &str,
        _q_score: f64,
        _score_fidelity: f64,
        _score_phi: f64,
        _score_verify: f64,
        _score_culture: f64,
        _score_burn: f64,
        _score_sovereignty: f64,
        _dog_count: u32,
        _verdict_type: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn list_pending_verdicts(&self, _limit: u32) -> Result<Vec<QueuedVerdict>, StorageError> {
        Ok(vec![])
    }
    async fn list_submitted_verdicts(
        &self,
        _limit: u32,
    ) -> Result<Vec<QueuedVerdict>, StorageError> {
        Ok(vec![])
    }
    async fn get_queued_verdict(
        &self,
        _verdict_id: &str,
    ) -> Result<Option<QueuedVerdict>, StorageError> {
        Ok(None)
    }
    async fn update_verdict_submitted(
        &self,
        _verdict_id: &str,
        _tx_signature: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn update_verdict_confirmed(&self, _verdict_id: &str) -> Result<(), StorageError> {
        Ok(())
    }
    async fn update_verdict_failed(
        &self,
        _verdict_id: &str,
        _error_reason: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn queue_status_counts(&self) -> Result<(u32, u32, u32, u32), StorageError> {
        Ok((0, 0, 0, 0))
    }
}
