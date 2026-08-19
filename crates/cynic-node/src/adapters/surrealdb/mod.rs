#![allow(dead_code)]
use crate::domain::mesh::PeerIdentity;
use crate::ports::mesh_port::MeshStateProvider;
use crate::ports::ssot_port::SSoTPort;
use reqwest::Client;
use serde::Deserialize;
use std::error::Error;
use std::net::IpAddr;

#[derive(Debug, Clone)]
pub struct SurrealDbAdapter {
    endpoint: String,
    ns: String,
    db: String,
    user: String,
    pass: String,
    client: Client,
}

impl SurrealDbAdapter {
    pub fn new(endpoint: &str, ns: &str, db: &str, user: &str, pass: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            ns: ns.to_string(),
            db: db.to_string(),
            user: user.to_string(),
            pass: pass.to_string(),
            client: Client::new(),
        }
    }

    async fn execute_query(&self, query: &str) -> Result<String, Box<dyn Error>> {
        let url = format!("{}/sql", self.endpoint);
        let res = self
            .client
            .post(&url)
            .header("surreal-ns", &self.ns)
            .header("surreal-db", &self.db)
            .basic_auth(&self.user, Some(&self.pass))
            .header("Accept", "application/json")
            .body(query.to_string())
            .send()
            .await?;

        if !res.status().is_success() {
            return Err(format!("SurrealDB request failed: {}", res.status()).into());
        }

        let body = res.text().await?;
        Ok(body)
    }
}

#[derive(Deserialize)]
struct SurrealResponse<T> {
    status: String,
    result: Option<T>,
}

#[derive(Deserialize)]
struct OrganRecord {
    id: String,
}

#[derive(Deserialize)]
struct MeshPeerRecord {
    id: String,
    public_key: String,
    internal_ip: String,
    endpoint: String,
}

impl SSoTPort for SurrealDbAdapter {
    async fn fetch_canonical_organs(&self) -> Result<Vec<u32>, Box<dyn Error>> {
        let body = self
            .execute_query("SELECT * FROM organs WHERE status = 'active'")
            .await?;
        let responses: Vec<SurrealResponse<Vec<OrganRecord>>> = serde_json::from_str(&body)?;

        let mut ids = Vec::new();
        if let Some(res) = responses.into_iter().next() {
            if let Some(records) = res.result {
                for r in records {
                    if let Some(num_str) = r.id.strip_prefix("organs:") {
                        if let Ok(num) = num_str.parse::<u32>() {
                            ids.push(num);
                        }
                    }
                }
            }
        }
        Ok(ids)
    }
}

impl MeshStateProvider for SurrealDbAdapter {
    async fn fetch_known_peers(&self) -> Result<Vec<PeerIdentity>, Box<dyn Error>> {
        let body = self.execute_query("SELECT * FROM mesh_peers").await?;
        let responses: Vec<SurrealResponse<Vec<MeshPeerRecord>>> = serde_json::from_str(&body)?;

        let mut peers = Vec::new();
        if let Some(res) = responses.into_iter().next() {
            if let Some(records) = res.result {
                for r in records {
                    if let Ok(ip) = r.internal_ip.parse::<IpAddr>() {
                        peers.push(PeerIdentity {
                            public_key: r.public_key,
                            internal_ip: ip,
                            endpoint: Some(r.endpoint),
                        });
                    }
                }
            }
        }
        Ok(peers)
    }
}
