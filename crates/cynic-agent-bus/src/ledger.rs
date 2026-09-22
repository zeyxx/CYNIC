use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::types::*;

/// Task event for the ledger log.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum TaskEvent {
    Created(Task),
    Updated { id: String, status: TaskStatus, result: Option<String> },
    Claimed(Claim),
    ClaimReleased { claim_id: String },
}

/// Ledger store interface.
#[async_trait::async_trait]
pub trait Ledger: Send + Sync {
    async fn create_task(&self, task: Task) -> Result<(), String>;
    async fn get_task(&self, id: &str) -> Option<Task>;
    async fn list_tasks(&self) -> Vec<Task>;
    async fn update_task(&self, id: String, status: TaskStatus, result: Option<String>) -> Result<(), String>;
    async fn post_message(&self, task_id: &str, agent: &str, body: &str) -> Result<(), String>;
    async fn list_messages(&self, task_id: &str) -> Vec<String>;
}

/// In-memory ledger for local development and testing.
pub struct InMemoryLedger {
    tasks: Arc<Mutex<HashMap<String, Task>>>,
    messages: Arc<Mutex<HashMap<String, Vec<String>>>>,
    events: Arc<Mutex<Vec<TaskEvent>>>,
}

impl InMemoryLedger {
    pub fn new() -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            messages: Arc::new(Mutex::new(HashMap::new())),
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn events(&self) -> Vec<TaskEvent> {
        self.events.lock().await.clone()
    }
}

#[async_trait::async_trait]
impl Ledger for InMemoryLedger {
    async fn create_task(&self, task: Task) -> Result<(), String> {
        let mut tasks = self.tasks.lock().await;
        if tasks.contains_key(&task.id) {
            return Err(format!("task {} already exists", task.id));
        }
        tasks.insert(task.id.clone(), task.clone());
        self.events.lock().await.push(TaskEvent::Created(task));
        Ok(())
    }

    async fn get_task(&self, id: &str) -> Option<Task> {
        self.tasks.lock().await.get(id).cloned()
    }

    async fn list_tasks(&self) -> Vec<Task> {
        self.tasks.lock().await.values().cloned().collect()
    }

    async fn update_task(&self, id: String, status: TaskStatus, result: Option<String>) -> Result<(), String> {
        let mut tasks = self.tasks.lock().await;
        let task = tasks.get_mut(&id).ok_or_else(|| format!("task {} not found", id))?;
        task.status = status.clone();
        task.result = result.clone();
        task.updated_at_unix_s = now_unix_s();
        self.events.lock().await.push(TaskEvent::Updated { id, status, result });
        Ok(())
    }

    async fn post_message(&self, task_id: &str, agent: &str, body: &str) -> Result<(), String> {
        let mut messages = self.messages.lock().await;
        let entry = messages.entry(task_id.to_string()).or_default();
        entry.push(format!("[{}] {}", agent, body));
        Ok(())
    }

    async fn list_messages(&self, task_id: &str) -> Vec<String> {
        self.messages.lock().await.get(task_id).cloned().unwrap_or_default()
    }
}

fn now_unix_s() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn ledger_create_and_get_task() {
        let ledger = InMemoryLedger::new();
        let task = Task {
            id: "t1".to_string(),
            parent_task_id: None,
            created_by: AgentId([0u8; 16]),
            assigned_to: AgentId([1u8; 16]),
            kind: TaskKind::SoloFile,
            subject: "test task".to_string(),
            body: "do something".to_string(),
            status: TaskStatus::Queued,
            result: None,
            error: None,
            created_at_unix_s: 0,
            updated_at_unix_s: 0,
            expires_at_unix_s: 0,
            budget: Budget { actions: 10, wall_seconds: 60 },
        };
        ledger.create_task(task.clone()).await.unwrap();
        let retrieved = ledger.get_task("t1").await;
        assert_eq!(retrieved.unwrap().id, "t1");
    }
}
