use surrealdb::engine::any::Any;
use surrealdb::opt::auth::Root;
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
            .unwrap_or_else(|_| "ws://localhost:8000".to_string());
        Self::init_with(&url, "cynic", "v2").await
    }

    pub async fn init_with(url: &str, ns: &str, db_name: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let user = std::env::var("SURREALDB_USER").unwrap_or_else(|_| "root".to_string());
        let pass = std::env::var("SURREALDB_PASS").expect("SURREALDB_PASS must be set — no default credentials");
        let db: Surreal<Any> = Surreal::init();
        db.connect(url).await?;
        db.signin(Root { username: user, password: pass }).await?;
        db.use_ns(ns).use_db(db_name).await?;
        println!("[Ring 1 / UAL] Linked to Sidecar Memory at {}", url);
        Ok(Self { db })
    }
}

// ── VERDICT STORAGE ────────────────────────────────────────
impl CynicStorage {
    pub async fn store_verdict(&self, verdict: &crate::dog::Verdict) -> Result<(), Box<dyn std::error::Error>> {
        let sql = "CREATE verdict SET \
            verdict_id = $vid, \
            kind = $kind, \
            total = $total, \
            fidelity = $fidelity, \
            phi = $phi, \
            verify = $verify, \
            reasoning_fidelity = $rf, \
            reasoning_phi = $rp, \
            reasoning_verify = $rv, \
            dog_id = $did, \
            stimulus = $stim, \
            created_at = $ts";

        self.db.query(sql)
            .bind(("vid", verdict.id.clone()))
            .bind(("kind", format!("{:?}", verdict.kind)))
            .bind(("total", verdict.q_score.total))
            .bind(("fidelity", verdict.q_score.fidelity))
            .bind(("phi", verdict.q_score.phi))
            .bind(("verify", verdict.q_score.verify))
            .bind(("rf", verdict.reasoning.fidelity.clone()))
            .bind(("rp", verdict.reasoning.phi.clone()))
            .bind(("rv", verdict.reasoning.verify.clone()))
            .bind(("did", verdict.dog_id.clone()))
            .bind(("stim", verdict.stimulus_summary.clone()))
            .bind(("ts", verdict.timestamp.clone()))
            .await?;
        Ok(())
    }

    pub async fn get_verdict(&self, verdict_id: &str) -> Result<Option<crate::dog::Verdict>, Box<dyn std::error::Error>> {
        let sql = "SELECT * FROM verdict WHERE verdict_id = $vid LIMIT 1";
        let mut resp = self.db.query(sql)
            .bind(("vid", verdict_id.to_string()))
            .await?;

        let rows: Vec<serde_json::Value> = resp.take(0)?;
        match rows.into_iter().next() {
            None => Ok(None),
            Some(row) => {
                let kind_str = row["kind"].as_str().unwrap_or("Bark");
                let kind = match kind_str {
                    "Howl" => crate::dog::VerdictKind::Howl,
                    "Wag" => crate::dog::VerdictKind::Wag,
                    "Growl" => crate::dog::VerdictKind::Growl,
                    _ => crate::dog::VerdictKind::Bark,
                };

                Ok(Some(crate::dog::Verdict {
                    id: row["verdict_id"].as_str().unwrap_or("").to_string(),
                    kind,
                    q_score: crate::dog::QScore {
                        total: row["total"].as_f64().unwrap_or(0.0),
                        fidelity: row["fidelity"].as_f64().unwrap_or(0.0),
                        phi: row["phi"].as_f64().unwrap_or(0.0),
                        verify: row["verify"].as_f64().unwrap_or(0.0),
                    },
                    reasoning: crate::dog::AxiomReasoning {
                        fidelity: row["reasoning_fidelity"].as_str().unwrap_or("").to_string(),
                        phi: row["reasoning_phi"].as_str().unwrap_or("").to_string(),
                        verify: row["reasoning_verify"].as_str().unwrap_or("").to_string(),
                    },
                    dog_id: row["dog_id"].as_str().unwrap_or("").to_string(),
                    stimulus_summary: row["stimulus"].as_str().unwrap_or("").to_string(),
                    timestamp: row["created_at"].as_str().unwrap_or("").to_string(),
                }))
            }
        }
    }

    pub async fn list_verdicts(&self, limit: u32) -> Result<Vec<crate::dog::Verdict>, Box<dyn std::error::Error>> {
        let sql = "SELECT * FROM verdict ORDER BY created_at DESC LIMIT $lim";
        let mut resp = self.db.query(sql)
            .bind(("lim", limit))
            .await?;

        let rows: Vec<serde_json::Value> = resp.take(0)?;
        let mut verdicts = Vec::new();

        for row in rows {
            let kind_str = row["kind"].as_str().unwrap_or("Bark");
            let kind = match kind_str {
                "Howl" => crate::dog::VerdictKind::Howl,
                "Wag" => crate::dog::VerdictKind::Wag,
                "Growl" => crate::dog::VerdictKind::Growl,
                _ => crate::dog::VerdictKind::Bark,
            };

            verdicts.push(crate::dog::Verdict {
                id: row["verdict_id"].as_str().unwrap_or("").to_string(),
                kind,
                q_score: crate::dog::QScore {
                    total: row["total"].as_f64().unwrap_or(0.0),
                    fidelity: row["fidelity"].as_f64().unwrap_or(0.0),
                    phi: row["phi"].as_f64().unwrap_or(0.0),
                    verify: row["verify"].as_f64().unwrap_or(0.0),
                },
                reasoning: crate::dog::AxiomReasoning {
                    fidelity: row["reasoning_fidelity"].as_str().unwrap_or("").to_string(),
                    phi: row["reasoning_phi"].as_str().unwrap_or("").to_string(),
                    verify: row["reasoning_verify"].as_str().unwrap_or("").to_string(),
                },
                dog_id: row["dog_id"].as_str().unwrap_or("").to_string(),
                stimulus_summary: row["stimulus"].as_str().unwrap_or("").to_string(),
                timestamp: row["created_at"].as_str().unwrap_or("").to_string(),
            });
        }

        Ok(verdicts)
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
            .unwrap_or_else(|_| "ws://localhost:8000".to_string());

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
