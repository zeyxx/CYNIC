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
        let db_url = std::env::var("SURREALDB_URL").unwrap_or_else(|_| "http://localhost:8000".to_string());
        
        let db = Surreal::init();
        db.connect::<Any>(&db_url).await?;
        db.use_ns("cynic").use_db("v2").await?;

        println!("[Ring 1 / UAL] Linked to Sidecar Memory at {}", db_url);

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

        let sql = "CREATE fact SET agent = $agent, content = $content, confidence = $conf, trace_id = $trace, timestamp = time::now()";
        self.storage.db.query(sql)
            .bind(("agent", meta.as_ref().map(|m| m.node_id.as_str()).unwrap_or("unknown")))
            .bind(("content", fact.content))
            .bind(("conf", fact.confidence))
            .bind(("trace", meta.as_ref().map(|m| m.trace_id.as_str()).unwrap_or("none")))
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

        let sql = "UPSERT trusted_model:[$name] SET sha256 = $sha, last_verified = time::now(), trace_id = $trace";
        self.storage.db.query(sql)
            .bind(("name", entry.model_name))
            .bind(("sha", entry.sha256))
            .bind(("trace", meta.as_ref().map(|m| m.trace_id.as_str()).unwrap_or("none")))
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
