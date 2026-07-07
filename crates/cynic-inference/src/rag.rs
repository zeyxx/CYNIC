//! RAG Access Layer
//! 
//! Expose l'accès direct aux bundles Markdown générés par organ-twitter
//! et organ-browser. C'est la mémoire décentralisée de l'organisme.

use std::path::PathBuf;

pub struct LocalRag {
    pub context_dir: PathBuf,
}

impl LocalRag {
    pub fn new(project_root: &PathBuf) -> Self {
        Self {
            context_dir: project_root.join("organs/twitter/state/context"),
        }
    }

    pub async fn list_bundles(&self) -> Result<Vec<String>, String> {
        // Parcourt le dossier pour lister tous les .md
        Ok(vec![])
    }

    pub async fn load_bundle(&self, bundle_id: &str) -> Result<String, String> {
        let path = self.context_dir.join(format!("{}.md", bundle_id));
        tokio::fs::read_to_string(&path).await.map_err(|e| format!("Failed to read RAG bundle {}: {}", bundle_id, e))
    }
}
