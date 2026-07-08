#[async_trait]
impl TaskStorage for InMemoryStorage {
    async fn store_agent_task(&self, task: &AgentTask) -> Result<String, StorageError> {
        let mut s = self.state.lock().await;
        let id = task.id.clone();
        s.tasks.insert(id.clone(), task.clone());
        Ok(id)
    }

    async fn list_pending_agent_tasks(
        &self,
        kind: &str,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<AgentTask>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s
            .tasks
            .values()
            .filter(|t| {
                t.kind == kind
                    && domain.is_none_or(|domain| t.domain == domain)
                    && t.status == "pending"
            })
            .cloned()
            .map(AgentTask::with_computed_metadata)
            .collect();
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn list_completed_agent_tasks(
        &self,
        kind: &str,
        domain: Option<&str>,
        limit: u32,
    ) -> Result<Vec<AgentTask>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s
            .tasks
            .values()
            .filter(|t| {
                t.kind == kind
                    && domain.is_none_or(|domain| t.domain == domain)
                    && (t.status == "completed" || t.status == "failed")
            })
            .cloned()
            .map(AgentTask::with_computed_metadata)
            .collect();
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn list_overdue_agent_tasks(&self, limit: u32) -> Result<Vec<AgentTask>, StorageError> {
        let s = self.state.lock().await;
        let mut v: Vec<_> = s
            .tasks
            .values()
            .filter(|t| t.status == "pending")
            .cloned()
            .map(AgentTask::with_computed_metadata)
            .filter(|t| t.overdue)
            .collect();
        v.truncate(limit as usize);
        Ok(v)
    }

    async fn claim_agent_task(&self, task_id: &str, agent_id: &str) -> Result<bool, StorageError> {
        let mut s = self.state.lock().await;
        if let Some(t) = s.tasks.get_mut(task_id)
            && t.status == "pending"
        {
            t.status = "processing".into();
            t.claimed_by = Some(agent_id.into());
            return Ok(true);
        }
        Ok(false)
    }

    async fn update_agent_task_result(
        &self,
        task_id: &str,
        result: Option<String>,
        error: Option<String>,
    ) -> Result<(), StorageError> {
        let mut s = self.state.lock().await;
        if let Some(t) = s.tasks.get_mut(task_id) {
            t.status = if error.is_some() {
                "failed".into()
            } else {
                "completed".into()
            };
            t.result = result;
            t.error = error;
        }
        Ok(())
    }
}

#[cfg(test)]
mod agent_task_memory_tests {
    use super::*;

    fn task(id: &str, status: &str) -> AgentTask {
        AgentTask {
            id: id.to_string(),
            kind: "hermes".to_string(),
            domain: "hermes-agent-executor".to_string(),
            content: "{}".to_string(),
            status: status.to_string(),
            result: None,
            created_at: 1_772_480_000_000,
            completed_at: None,
            agent_id: Some("test-agent".to_string()),
            claimed_by: None,
            error: None,
            due_at: None,
            priority: None,
            consumer: None,
            escalation_policy: None,
            overdue: false,
            age_seconds: None,
        }
    }

    #[tokio::test]
    async fn completed_listing_includes_failed_tasks_as_resolved() {
        let storage = InMemoryStorage::new();
        storage
            .store_agent_task(&task("agent_tasks:ok", "completed"))
            .await
            .unwrap();
        storage
            .store_agent_task(&task("agent_tasks:failed", "failed"))
            .await
            .unwrap();
        storage
            .store_agent_task(&task("agent_tasks:pending", "pending"))
            .await
            .unwrap();

        let tasks = storage
            .list_completed_agent_tasks("hermes", Some("hermes-agent-executor"), 10)
            .await
            .unwrap();
        let statuses: std::collections::BTreeSet<_> =
            tasks.iter().map(|task| task.status.as_str()).collect();

        assert_eq!(tasks.len(), 2);
        assert!(statuses.contains("completed"));
        assert!(statuses.contains("failed"));
        assert!(!statuses.contains("pending"));
    }
}
