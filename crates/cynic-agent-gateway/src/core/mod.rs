use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::{sleep, Duration};
use tracing::{error, info, warn};

use crate::domain::AgentExecutionEnvelope;
use crate::ports::{AgentProviderPort, MempoolPort, ObservabilityPort};

/// Le cœur de l'application (Use Case).
/// Il orchestre la Mempool et délègue l'exécution au bon adaptateur (Hexagonal Architecture).
pub struct GatewayOrchestrator {
    mempool: Arc<dyn MempoolPort>,
    observability: Arc<dyn ObservabilityPort>,
    providers: HashMap<String, Arc<dyn AgentProviderPort>>,
}

impl GatewayOrchestrator {
    pub fn new(
        mempool: Arc<dyn MempoolPort>,
        observability: Arc<dyn ObservabilityPort>,
    ) -> Self {
        Self {
            mempool,
            observability,
            providers: HashMap::new(),
        }
    }

    /// Enregistre un framework externe (ex: "hermes-agent", "opencode")
    pub fn register_provider(&mut self, provider: Arc<dyn AgentProviderPort>) {
        let name = provider.provider_name().to_string();
        info!("Registering Agent Provider: {}", name);
        self.providers.insert(name, provider);
    }

    /// La boucle principale du Démon. Ne s'arrête jamais.
    pub async fn run_daemon(&self) {
        info!("Orchestrator daemon started. Polling Mempool...");

        loop {
            match self.mempool.fetch_pending_task().await {
                Ok(Some(envelope)) => {
                    self.dispatch_task(envelope).await;
                }
                Ok(None) => {
                    // Aucune tâche, on évite le polling agressif (Anti-Polling Transition)
                    sleep(Duration::from_secs(5)).await;
                }
                Err(e) => {
                    error!("Error fetching from mempool: {}", e);
                    sleep(Duration::from_secs(10)).await;
                }
            }
        }
    }

    /// Logique de routage métier vers le bon adaptateur
    async fn dispatch_task(&self, envelope: AgentExecutionEnvelope) {
        let task_id = envelope.task_id.clone();
        let provider_name = envelope.provider.clone();

        info!("Task {} assigned to provider: {}", task_id, provider_name);

        match self.providers.get(&provider_name) {
            Some(provider) => {
                // Délégation asynchrone à l'adaptateur
                if let Err(e) = provider.execute(envelope, self.observability.clone()).await {
                    error!("Provider {} failed on task {}: {}", provider_name, task_id, e);
                    // TODO: Envoyer l'erreur via l'observability port (Bloodstream)
                } else {
                    info!("Task {} successfully executed by {}", task_id, provider_name);
                    // On clôture la tâche dans la Mempool CYNIC
                    if let Err(e) = self.mempool.mark_task_completed(&task_id).await {
                        error!("Failed to mark task {} as completed: {}", task_id, e);
                    }
                }
            }
            None => {
                warn!("No registered provider found for: {}. Task {} aborted.", provider_name, task_id);
            }
        }
    }
}
