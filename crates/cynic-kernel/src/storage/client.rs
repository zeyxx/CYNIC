//! HTTP storage client for SurrealDB 3.x.
//!
//! The client uses direct HTTP calls to `/sql` and keeps storage contracts
//! adapter-local: auth lifecycle, schema bootstrap, request retry, and low-level
//! query execution.

use crate::domain::storage::StorageError;
use crate::infra::config::StorageConfig;
use crate::storage::validation::parse_sql_table_op;

use base64::Engine;
use reqwest::Client;
use serde::Deserialize;

#[derive(Debug)]
pub struct SurrealHttpStorage {
    pub(crate) client: Client,
    pub(crate) url: String,
    pub(crate) ns: String,
    pub(crate) db: String,
    pub(crate) auth: std::sync::RwLock<String>,
    basic_auth: String,
    pub(crate) metrics: crate::storage::StorageMetrics,
}

#[derive(Deserialize, Debug)]
pub(crate) struct SurrealResponse {
    pub(crate) result: Option<serde_json::Value>,
    pub(crate) status: Option<String>,
}

impl SurrealHttpStorage {
    pub fn db_name(&self) -> &str {
        &self.db
    }

    pub fn ns_name(&self) -> &str {
        &self.ns
    }

    pub async fn init(config: &StorageConfig) -> Result<Self, StorageError> {
        Self::init_with(&config.url, &config.namespace, &config.database).await
    }

    pub async fn init_with(url: &str, ns: &str, db: &str) -> Result<Self, StorageError> {
        let user = std::env::var("SURREALDB_USER").unwrap_or_else(|_| "root".to_string());
        let pass = std::env::var("SURREALDB_PASS").map_err(|e| {
            StorageError::ConnectionFailed(format!("SURREALDB_PASS must be set: {e}"))
        })?;

        let credentials = format!("{user}:{pass}");
        let basic_auth = format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(&credentials)
        );

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| StorageError::ConnectionFailed(format!("HTTP client: {e}")))?;
        let url = url.trim_end_matches('/').to_string();

        let bootstrap_sql = format!(
            "DEFINE NAMESPACE IF NOT EXISTS `{ns}`; USE NS `{ns}`; DEFINE DATABASE IF NOT EXISTS `{db}`;"
        );
        let resp = client
            .post(format!("{url}/sql"))
            .header("Accept", "application/json")
            .header("Authorization", &basic_auth)
            .body(bootstrap_sql)
            .send()
            .await
            .map_err(|e| {
                StorageError::ConnectionFailed(format!("SurrealDB unreachable at {url}: {e}"))
            })?;

        if !resp.status().is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(StorageError::ConnectionFailed(format!(
                "Bootstrap failed: {body}"
            )));
        }

        let bearer_auth = Self::signin(&client, &url, &user, &pass)
            .await
            .map_err(|e| {
                tracing::warn!(error = %e, "JWT signin failed, falling back to Basic auth");
                e
            })
            .unwrap_or_else(|_| basic_auth.clone());

        let storage = Self {
            client,
            url: url.clone(),
            ns: ns.to_string(),
            db: db.to_string(),
            auth: std::sync::RwLock::new(bearer_auth),
            basic_auth,
            metrics: crate::storage::StorageMetrics::new(),
        };

        storage.query("RETURN true").await.map_err(|e| {
            StorageError::ConnectionFailed(format!("SurrealDB unreachable at {url}: {e}"))
        })?;

        crate::storage::schema::bootstrap_schema(&storage).await;

        klog!(
            "[Ring 1 / UAL] Linked to SurrealDB (HTTP) at {}",
            storage.url
        );

        Ok(storage)
    }

    async fn signin(
        client: &Client,
        url: &str,
        user: &str,
        pass: &str,
    ) -> Result<String, StorageError> {
        let body = serde_json::json!({"user": user, "pass": pass});
        let resp = client
            .post(format!("{url}/signin"))
            .header("Content-Type", "application/json")
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| StorageError::ConnectionFailed(format!("/signin failed: {e}")))?;

        let status = resp.status();
        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| StorageError::ConnectionFailed(format!("/signin parse: {e}")))?;

        if !status.is_success() {
            return Err(StorageError::ConnectionFailed(format!(
                "/signin returned {status}: {}",
                json.get("details")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
            )));
        }

        let token = json.get("token").and_then(|v| v.as_str()).ok_or_else(|| {
            StorageError::ConnectionFailed("/signin: no token in response".into())
        })?;

        Ok(format!("Bearer {token}"))
    }

    async fn refresh_token(&self) -> Result<(), StorageError> {
        let encoded = self
            .basic_auth
            .strip_prefix("Basic ")
            .ok_or_else(|| StorageError::ConnectionFailed("corrupt basic_auth".into()))?;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| StorageError::ConnectionFailed(format!("base64 decode: {e}")))?;
        let cred_str = String::from_utf8(decoded)
            .map_err(|e| StorageError::ConnectionFailed(format!("utf8: {e}")))?;
        let (user, pass) = cred_str
            .split_once(':')
            .ok_or_else(|| StorageError::ConnectionFailed("bad credentials format".into()))?;

        let new_auth = Self::signin(&self.client, &self.url, user, pass).await?;
        if let Ok(mut guard) = self.auth.write() {
            *guard = new_auth;
        }
        tracing::info!("SurrealDB JWT refreshed");
        Ok(())
    }

    pub async fn query(&self, sql: &str) -> Result<Vec<Vec<serde_json::Value>>, StorageError> {
        let start = std::time::Instant::now();
        self.metrics
            .queries
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        let result = self.query_inner(sql).await;

        let elapsed_us = start.elapsed().as_micros() as u64;
        self.metrics
            .total_latency_us
            .fetch_add(elapsed_us, std::sync::atomic::Ordering::Relaxed);

        let elapsed_ms = elapsed_us / 1000;
        if elapsed_ms > crate::storage::metrics::SLOW_QUERY_MS {
            self.metrics
                .slow_queries
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let preview: String = sql.chars().take(80).collect();
            tracing::warn!(latency_ms = elapsed_ms, query = %preview, "slow query");
        }

        let is_err = result.is_err();
        if is_err {
            self.metrics
                .errors
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }

        if let Some((table, op)) = parse_sql_table_op(sql) {
            self.metrics
                .record_table_op(&table, &op, elapsed_us, is_err);
        }

        result
    }

    async fn query_inner(&self, sql: &str) -> Result<Vec<Vec<serde_json::Value>>, StorageError> {
        let mut last_err = String::new();

        for attempt in 0..3u64 {
            let current_auth = self
                .auth
                .read()
                .map(|g| g.clone())
                .unwrap_or_else(|_| self.basic_auth.clone());

            let resp = self
                .client
                .post(format!("{}/sql", self.url))
                .header("Accept", "application/json")
                .header("surreal-ns", &self.ns)
                .header("surreal-db", &self.db)
                .header("Authorization", &current_auth)
                .body(sql.to_string())
                .send()
                .await
                .map_err(|e| StorageError::ConnectionFailed(e.to_string()))?;

            if resp.status().as_u16() == 401 && attempt < 2 {
                last_err = resp.text().await.unwrap_or_default();
                if attempt == 0 {
                    if let Err(e) = self.refresh_token().await {
                        tracing::warn!(error = %e, "JWT refresh failed, retrying with current auth");
                    }
                } else {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }

                continue;
            }

            if !resp.status().is_success() {
                let status = resp.status();
                let body = resp.text().await.unwrap_or_default();
                return Err(StorageError::QueryFailed(format!("HTTP {status}: {body}")));
            }

            let results: Vec<SurrealResponse> = resp
                .json()
                .await
                .map_err(|e| StorageError::QueryFailed(format!("JSON parse error: {e}")))?;

            for r in &results {
                if r.status.as_deref() == Some("ERR") {
                    let msg = r
                        .result
                        .as_ref()
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown SurrealDB error");
                    return Err(StorageError::QueryFailed(msg.to_string()));
                }
            }

            return Ok(results
                .into_iter()
                .map(|r| match r.result {
                    Some(serde_json::Value::Array(arr)) => arr,
                    Some(val) => vec![val],
                    None => Vec::new(),
                })
                .collect());
        }

        Err(StorageError::QueryFailed(format!(
            "401 after 3 retries: {last_err}"
        )))
    }

    pub async fn query_one(&self, sql: &str) -> Result<Vec<serde_json::Value>, StorageError> {
        let mut results = self.query(sql).await?;
        Ok(results.pop().unwrap_or_default())
    }
}
