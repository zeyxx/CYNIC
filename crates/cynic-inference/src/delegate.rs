//! Délégué isolé (Small Chunk Executor)
//!
//! Implémente le concept de `supervised_cortex_delegate`.
//! C'est un agent éphémère ("Small Chunk") qui naît pour accomplir une `TaskIntent`,
//! s'isole du reste du monde avec un prompt vierge, exécute la requête via le LLM,
//! renvoie un `TaskYield` et meurt.

use crate::reactor::{Domain, TaskIntent, TaskYield};
use crate::rag::LocalRag;
use std::path::PathBuf;

pub struct CortexDelegate {
    pub endpoint_url: String, // ex: http://127.0.0.1:8080/v1/chat/completions (via le proxy Soma L4)
    pub project_root: PathBuf,
}

impl CortexDelegate {
    pub fn new(endpoint_url: String, project_root: PathBuf) -> Self {
        Self {
            endpoint_url,
            project_root,
        }
    }

    /// Exécute une intention de manière totalement isolée (Quantum Integrity)
    pub async fn execute(&self, intent: TaskIntent) -> Result<TaskYield, String> {
        // 1. Charger STRICTEMENT le contexte requis par le domaine
        let mut isolated_context = String::new();

        match intent.domain {
            Domain::Rag => {
                let rag = LocalRag::new(&self.project_root);
                for target_id in &intent.targets {
                    if let Ok(content) = rag.load_bundle(target_id).await {
                        isolated_context.push_str(&format!("--- RAG BUNDLE: {} ---\n{}\n", target_id, content));
                    }
                }
            }
            Domain::Code => {
                // Charger uniquement les fichiers de code ciblés
                for target_file in &intent.targets {
                    let path = self.project_root.join(target_file);
                    if let Ok(content) = tokio::fs::read_to_string(&path).await {
                        isolated_context.push_str(&format!("--- FILE: {} ---\n{}\n", target_file, content));
                    }
                }
            }
            _ => {
                isolated_context.push_str("Contexte injecté par défaut ou absent.");
            }
        }

        // 2. Forger un System Prompt vierge et impératif
        let system_prompt = format!(
            "You are a CYNIC supervised_cortex_delegate. \
            Your domain is {:?}. \
            You have no prior memory and no broader context. \
            Your STRICT objective is: {}. \
            Analyze the provided context and return the synthesis. \
            Do not output conversational filler. Output only facts and solutions.",
            intent.domain, intent.objective
        );

        // 3. Appel au LLM Souverain (llama-server proxy)
        let synthesis_result = self.call_llm(&system_prompt, &isolated_context).await?;

        // 4. Renvoi du "Small Chunk" au parent
        Ok(TaskYield {
            status: "success".to_string(),
            synthesis: synthesis_result,
            artifacts_produced: vec![],
        })
    }

    /// Appel HTTP direct au proxy L4 (qui route vers le bon noeud llama)
    async fn call_llm(&self, system_prompt: &str, user_context: &str) -> Result<String, String> {
        // Implémentation réelle : reqwest::Client::new().post(&self.endpoint_url) ...
        // Pour l'instant, on simule l'inférence.
        
        let mut sim = format!("SYNTHESIS OF DELEGATE:\n");
        sim.push_str(&format!("Analyzed {} bytes of pure context.\n", user_context.len()));
        sim.push_str("Objective resolved efficiently without context bloat.");
        
        Ok(sim)
    }
}
