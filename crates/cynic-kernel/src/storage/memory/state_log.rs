#[async_trait]
impl StateLogStorage for InMemoryStorage {
    async fn store_state_block(
        &self,
        _block: &crate::domain::state_log::StateBlock,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn last_state_block(
        &self,
    ) -> Result<Option<crate::domain::state_log::StateBlock>, StorageError> {
        Ok(None)
    }
    async fn list_state_blocks(
        &self,
        _since: &str,
        _limit: u32,
    ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError> {
        Ok(vec![])
    }
    async fn latest_state_blocks(
        &self,
        _n: u32,
    ) -> Result<Vec<crate::domain::state_log::StateBlock>, StorageError> {
        Ok(vec![])
    }
}
