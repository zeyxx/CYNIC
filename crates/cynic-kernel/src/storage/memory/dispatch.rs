#[async_trait]
impl DispatchStorage for InMemoryStorage {
    async fn store_agent_dispatch(
        &self,
        _dispatch: &AgentDispatch,
    ) -> Result<String, StorageError> {
        Ok("".into())
    }
    async fn get_active_dispatch_for_scope(
        &self,
        _scope: &str,
    ) -> Result<Option<AgentDispatch>, StorageError> {
        Ok(None)
    }
    async fn get_dispatch(
        &self,
        _dispatch_id: &str,
    ) -> Result<Option<AgentDispatch>, StorageError> {
        Ok(None)
    }
    async fn get_active_dispatches_for_agent(
        &self,
        _agent_id: &str,
    ) -> Result<Vec<AgentDispatch>, StorageError> {
        Ok(vec![])
    }
    async fn update_dispatch_status(
        &self,
        _dispatch_id: &str,
        _new_status: &str,
    ) -> Result<(), StorageError> {
        Ok(())
    }
    async fn update_dispatch_pr(
        &self,
        _dispatch_id: &str,
        _pr_number: u32,
    ) -> Result<(), StorageError> {
        Ok(())
    }
}
