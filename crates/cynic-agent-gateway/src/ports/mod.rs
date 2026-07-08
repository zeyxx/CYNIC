use async_trait::async_trait;
use crate::domain::AgentExecutionEnvelope;

/// Port sortant (Driving/Driven) : Interaction avec la Mempool (CYNIC Kernel)
#[async_trait]
pub trait MempoolPort: Send + Sync {
    async fn fetch_pending_task(&self) -> Result<Option<AgentExecutionEnvelope>, Box<dyn std::error::Error>>;
    async fn mark_task_completed(&self, task_id: &str) -> Result<(), Box<dyn std::error::Error>>;
}

/// Port sortant : Envoi de la télémétrie asynchrone (Bloodstream / Kanban)
#[async_trait]
pub trait ObservabilityPort: Send + Sync {
    async fn stream_kanban_event(&self, task_id: &str, event_json: serde_json::Value) -> Result<(), Box<dyn std::error::Error>>;
    async fn stream_log(&self, task_id: &str, log_line: &str) -> Result<(), Box<dyn std::error::Error>>;
}

/// Port d'exécution : L'interface que chaque framework (Hermes, OpenCode) doit respecter.
#[async_trait]
pub trait AgentProviderPort: Send + Sync {
    /// Retourne le nom du framework géré par cet adaptateur (ex: "hermes-agent")
    fn provider_name(&self) -> &str;
    
    /// Exécute la tâche de manière asynchrone et stream les logs vers le port d'observabilité
    async fn execute(&self, envelope: AgentExecutionEnvelope, observability: std::sync::Arc<dyn ObservabilityPort>) -> Result<(), Box<dyn std::error::Error>>;
}
