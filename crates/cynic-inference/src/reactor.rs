//! Le Moteur Récursif (Recursive Reactor)
//! 
//! Implémentation du paradigme "Recursive Agents > Looping Agents" au niveau global.
//! Ne se limite pas au RAG, mais s'applique à tous les domaines (infra, code, browser, etc.).
//!
//! Au lieu d'une boucle de contexte infinie, le Cortex décompose un problème complexe en
//! sous-tâches isolées (`supervised_cortex_delegate`). Ces sous-tâches sont poussées dans 
//! le Mempool du Kernel via `cynic_dispatch_agent_task`. Chaque sous-agent a un contexte 
//! vierge, pur, dédié à son unique domaine, et renvoie un résultat structuré.

use serde::{Deserialize, Serialize};

/// Représente l'intention d'une tâche dans l'organisme (Mempool-compatible).
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskIntent {
    pub objective: String,
    pub domain: Domain, // "infra", "rag", "code", "browser", "fleet"
    pub targets: Vec<String>, // Fichiers, nœuds, ou bundle_ids concernés
    pub parent_task_id: Option<String>, // Permet de reconstruire l'arbre récursif
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Domain {
    Infra,
    Code,
    Browser,
    Rag,
    Fleet,
    Coordination,
}

/// Le résultat purifié qu'un `supervised_cortex_delegate` remonte au parent.
#[derive(Debug, Serialize, Deserialize)]
pub struct TaskYield {
    pub status: String, // "success", "failed", "blocked"
    pub synthesis: String, // Résultat condensé (la substantifique moelle)
    pub artifacts_produced: Vec<String>, // Liens vers les preuves/fichiers
}

pub struct RecursiveReactor {
    pub kernel_mempool_endpoint: String,
}

impl RecursiveReactor {
    pub fn new(kernel_mempool_endpoint: String) -> Self {
        Self { kernel_mempool_endpoint }
    }

    /// Point d'entrée pour résoudre un problème complexe.
    pub async fn resolve_tree(&self, root_intent: TaskIntent) -> Result<TaskYield, String> {
        // 1. Le LLM parent analyse `root_intent`.
        // 2. Si le problème nécessite des sous-étapes (ex: Déployer Infra + Chercher Doc),
        //    il forge des enfants.
        
        let sub_intents = self.decompose_task(&root_intent).await?;
        let mut child_yields = Vec::new();

        for child_intent in sub_intents {
            // Dispatch dans le Kernel Mempool (Coordination Bus)
            let task_id = self.dispatch_to_mempool(child_intent).await?;
            
            // Attente (asynchrone) que le supervised_cortex_delegate termine
            let child_yield = self.wait_for_task_result(&task_id).await?;
            child_yields.push(child_yield);
        }

        // Le parent synthétise les rendements purs des enfants sans polluer son propre prompt.
        self.synthesize_tree(root_intent, child_yields).await
    }

    async fn decompose_task(&self, intent: &TaskIntent) -> Result<Vec<TaskIntent>, String> {
        // Logique de routing/planification. 
        // Ex: Si le domain est Infra, spawn un delegate pour le réseau, un pour le stockage.
        Ok(vec![])
    }

    async fn dispatch_to_mempool(&self, intent: TaskIntent) -> Result<String, String> {
        // Envoi réel au Mempool du Kernel via REST API (cynic_dispatch_agent_task)
        let client = reqwest::Client::new();
        let url = format!("{}/agent-tasks", self.kernel_mempool_endpoint);
        
        let response = client.post(&url)
            .json(&intent)
            .send()
            .await
            .map_err(|e| format!("Network error dispatching to mempool: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Kernel rejected intent: HTTP {}", response.status()));
        }

        // On s'attend à recevoir { "task_id": "..." }
        #[derive(Deserialize)]
        struct DispatchResponse {
            task_id: String,
        }
        
        let data = response.json::<DispatchResponse>()
            .await
            .map_err(|e| format!("Failed to parse Kernel response: {}", e))?;

        Ok(data.task_id)
    }

    async fn wait_for_task_result(&self, task_id: &str) -> Result<TaskYield, String> {
        let client = reqwest::Client::new();
        let url = format!("{}/agent-tasks/{}", self.kernel_mempool_endpoint, task_id);
        
        // Polling (Phase 3.2: idéalement via un Webhook/WebSocket inverse pour l'Inférence)
        for _ in 0..60 { // Wait up to 60 seconds (1s intervals)
            let response = client.get(&url)
                .send()
                .await
                .map_err(|e| format!("Network error polling mempool: {}", e))?;
                
            if response.status().is_success() {
                #[derive(Deserialize)]
                struct TaskStatus {
                    status: String,
                    result: Option<TaskYield>,
                }
                
                if let Ok(task_status) = response.json::<TaskStatus>().await {
                    if task_status.status == "completed" {
                        if let Some(res) = task_status.result {
                            return Ok(res);
                        }
                    } else if task_status.status == "failed" {
                        return Err(format!("Delegate task {} failed execution.", task_id));
                    }
                }
            }
            
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
        }

        Err(format!("Timeout waiting for delegate task {}", task_id))
    }

    async fn synthesize_tree(&self, root_intent: TaskIntent, _yields: Vec<TaskYield>) -> Result<TaskYield, String> {
        Ok(TaskYield {
            status: "success".to_string(),
            synthesis: format!("Root objective '{}' resolved.", root_intent.objective),
            artifacts_produced: vec![],
        })
    }
}
