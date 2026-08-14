#[async_trait]
impl ProofOfHistoryStorage for InMemoryStorage {
    async fn store_proof_of_history(&self, _poh: &ProofOfHistory) -> Result<String, StorageError> {
        Err(StorageError::ConnectionFailed(
            "InMemoryStorage does not persist proof of history".to_string(),
        ))
    }

    async fn list_proof_of_history(
        &self,
        _limit: u32,
    ) -> Result<Vec<ProofOfHistory>, StorageError> {
        Ok(vec![])
    }
}
