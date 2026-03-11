use surrealdb::engine::any::Any;
use surrealdb::Surreal;
use tonic::{Request, Response, Status};
use std::sync::Arc;
use crate::cynic_v2::cognitive_memory_server::CognitiveMemory;
use crate::cynic_v2::{Fact, PublishAck, TrustEntry, TrustVerifyResponse};

pub struct CynicStorage {
    pub db: Surreal<Any>,
}

impl CynicStorage {
    pub async fn init() -> Result<Self, Box<dyn std::error::Error>> {
        let url = std::env::var("SURREALDB_URL")
            .unwrap_or_else(|_| "http://localhost:8000".to_string());
        Self::init_with(&url, "cynic", "v2").await
    }

    pub async fn init_with(url: &str, ns: &str, db_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let db: Surreal<Any> = Surreal::init();
        db.connect(url).await?;
        db.use_ns(ns).use_db(db_name).await?;
        println!("[Ring 1 / UAL] Linked to Sidecar Memory at {}", url);
        Ok(Self { db })
    }
}

pub struct CognitiveMemoryService {
    storage: Arc<CynicStorage>,
}

impl CognitiveMemoryService {
    pub fn new(storage: Arc<CynicStorage>) -> Self {
        Self { storage }
    }
}

#[tonic::async_trait]
impl CognitiveMemory for CognitiveMemoryService {
    async fn store_fact(
        &self,
        request: Request<Fact>,
    ) -> Result<Response<PublishAck>, Status> {
        let fact = request.into_inner();
        let meta = fact.meta.clone();
        let node_id = meta.as_ref().map(|m| m.node_id.clone()).unwrap_or_default();
        let trace_id = meta.as_ref().map(|m| m.trace_id.clone()).unwrap_or_default();

        let sql = "CREATE fact SET agent = $agent, content = $content, confidence = $conf, trace_id = $trace, timestamp = time::now()";
        self.storage.db.query(sql)
            .bind(("agent", node_id))
            .bind(("content", fact.content))
            .bind(("conf", fact.confidence))
            .bind(("trace", trace_id))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(PublishAck {
            meta,
            success: true,
        }))
    }

    async fn register_trust(
        &self,
        request: Request<TrustEntry>,
    ) -> Result<Response<PublishAck>, Status> {
        let entry = request.into_inner();
        let meta = entry.meta.clone();
        let trace_id = meta.as_ref().map(|m| m.trace_id.clone()).unwrap_or_default();

        let sql = "UPSERT trusted_model:[$name] SET sha256 = $sha, last_verified = time::now(), trace_id = $trace";
        self.storage.db.query(sql)
            .bind(("name", entry.model_name))
            .bind(("sha", entry.sha256))
            .bind(("trace", trace_id))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(PublishAck {
            meta,
            success: true,
        }))
    }

    async fn verify_trust(
        &self,
        request: Request<TrustEntry>,
    ) -> Result<Response<TrustVerifyResponse>, Status> {
        let entry = request.into_inner();
        let meta = entry.meta.clone();

        let sql = "SELECT * FROM trusted_model WHERE id = trusted_model:[$name] AND sha256 = $sha";
        let mut response = self.storage.db.query(sql)
            .bind(("name", entry.model_name))
            .bind(("sha", entry.sha256))
            .await
            .map_err(|e| Status::internal(e.to_string()))?;

        let results: Vec<serde_json::Value> = response.take(0).map_err(|e| Status::internal(e.to_string()))?;

        Ok(Response::new(TrustVerifyResponse {
            meta,
            is_trusted: !results.is_empty(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn store_and_retrieve_fact() {
        let url = std::env::var("SURREALDB_URL")
            .unwrap_or_else(|_| "http://localhost:8000".to_string());

        let storage = CynicStorage::init_with(&url, "test_cynic", "ci").await
            .expect("SurrealDB doit être accessible sur forge");

        // Stocker un fait
        let result = storage.db
            .query("CREATE fact SET content = $content, confidence = $conf")
            .bind(("content", "test fact from integration test"))
            .bind(("conf", 0.9f64))
            .await;
        assert!(result.is_ok(), "store_fact doit réussir: {:?}", result.err());

        // Relire
        let mut resp = storage.db
            .query("SELECT * FROM fact WHERE content = $c")
            .bind(("c", "test fact from integration test"))
            .await
            .expect("SELECT doit réussir");
        let rows: Vec<serde_json::Value> = resp.take(0).expect("take(0)");
        assert!(!rows.is_empty(), "Le fait doit être retrouvé");

        // Nettoyage
        storage.db
            .query("DELETE fact WHERE content = $c")
            .bind(("c", "test fact from integration test"))
            .await
            .ok();
    }
}
