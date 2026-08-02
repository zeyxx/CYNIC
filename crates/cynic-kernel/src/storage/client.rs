//! HTTP storage client for SurrealDB 3.x.
//!
//! The client uses direct HTTP calls to `/sql` and keeps storage contracts
//! adapter-local: auth lifecycle, schema bootstrap, request retry, and low-level
//! query execution.

use crate::domain::storage::StorageError;
use crate::infra::config::StorageConfig;
use crate::infra::runtime_secret::{
    LocalRuntimeSecretSource, SecretMaterial, resolve_runtime_secret,
};
use crate::storage::auth::{SensitiveString, StorageCredentials, signin};
use crate::storage::validation::parse_sql_table_op;

use reqwest::Client;
use serde::Deserialize;

pub struct SurrealHttpStorage {
    pub(crate) client: Client,
    pub(crate) url: String,
    pub(crate) ns: String,
    pub(crate) db: String,
    auth: std::sync::RwLock<SensitiveString>,
    credentials: StorageCredentials,
    pub(crate) metrics: crate::storage::StorageMetrics,
}

impl std::fmt::Debug for SurrealHttpStorage {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SurrealHttpStorage")
            .field("url", &self.url)
            .field("namespace", &self.ns)
            .field("database", &self.db)
            .field("username", &self.credentials.username())
            .field("credential_present", &true)
            .finish_non_exhaustive()
    }
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
        validate_runtime_storage_endpoint(&config.url, config.allow_loopback)?;
        let resolved =
            resolve_runtime_secret(&config.identity.secret_sources, &LocalRuntimeSecretSource)
                .map_err(StorageError::ConnectionFailed)?;
        tracing::info!(
            identity = %config.identity.username,
            secret_source = %resolved.source.id,
            provider = ?resolved.source.provider,
            "storage runtime machine identity resolved"
        );
        Self::init_with_identity(
            &config.url,
            &config.namespace,
            &config.database,
            &config.identity.username,
            resolved.material,
        )
        .await
    }

    pub async fn init_with(url: &str, ns: &str, db: &str) -> Result<Self, StorageError> {
        let mut config = StorageConfig::default();
        config.url = url.to_string();
        config.namespace = ns.to_string();
        config.database = db.to_string();
        Self::init(&config).await
    }

    async fn init_with_identity(
        url: &str,
        ns: &str,
        db: &str,
        user: &str,
        pass: SecretMaterial,
    ) -> Result<Self, StorageError> {
        let credentials = StorageCredentials::new(user, pass)?;
        let basic_auth = credentials.basic_auth();

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| StorageError::ConnectionFailed(format!("HTTP client: {e}")))?;
        let url = url.trim_end_matches('/').to_string();

        let bearer_auth = signin(&client, &url, ns, db, &credentials)
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
            credentials,
            metrics: crate::storage::StorageMetrics::new(),
        };

        storage.query("RETURN true").await.map_err(|e| {
            StorageError::ConnectionFailed(format!("SurrealDB unreachable at {url}: {e}"))
        })?;

        crate::storage::schema::observe_schema(&storage).await?;

        klog!(
            "[Ring 1 / UAL] Linked to SurrealDB (HTTP) at {}",
            storage.url
        );

        Ok(storage)
    }

    async fn refresh_token(&self) -> Result<(), StorageError> {
        let new_auth = signin(
            &self.client,
            &self.url,
            &self.ns,
            &self.db,
            &self.credentials,
        )
        .await?;
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
                .map(|guard| guard.clone())
                .unwrap_or_else(|_| self.credentials.basic_auth());

            let resp = self
                .client
                .post(format!("{}/sql", self.url))
                .header("Accept", "application/json")
                .header("surreal-ns", &self.ns)
                .header("surreal-db", &self.db)
                .header("Authorization", current_auth.as_str())
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

fn validate_runtime_storage_endpoint(
    url: &str,
    declared_local_authority: bool,
) -> Result<(), StorageError> {
    let endpoint = reqwest::Url::parse(url).map_err(|_| {
        StorageError::ConnectionFailed(
            "storage endpoint is absent or invalid; declare CYNIC_STORAGE_CONFIG or SURREALDB_URL"
                .to_string(),
        )
    })?;
    if !matches!(endpoint.scheme(), "http" | "https") || endpoint.host_str().is_none() {
        return Err(StorageError::ConnectionFailed(
            "storage endpoint must be an absolute http(s) URL".to_string(),
        ));
    }
    let loopback = endpoint
        .host_str()
        .is_some_and(|host| matches!(host, "localhost" | "127.0.0.1" | "::1"));
    if loopback && !loopback_storage_authorized(declared_local_authority) {
        return Err(StorageError::ConnectionFailed(
            "loopback storage is not authorized for this node role or harness".to_string(),
        ));
    }
    Ok(())
}

fn loopback_storage_authorized(declared_local_authority: bool) -> bool {
    declared_local_authority
        || std::env::var("CYNIC_RUNTIME_HARNESS_KIND").ok().as_deref() == Some("lab")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn storage_debug_never_projects_password_or_session_token() {
        let storage = SurrealHttpStorage {
            client: Client::new(),
            url: "http://localhost:8000".to_string(),
            ns: "cynic".to_string(),
            db: "main".to_string(),
            auth: std::sync::RwLock::new(SensitiveString::new(
                "Bearer session-token-value".to_string(),
            )),
            credentials: StorageCredentials::new(
                "cynic-runtime",
                SecretMaterial::new("runtime-password-value".to_string()),
            )
            .unwrap(),
            metrics: crate::storage::StorageMetrics::new(),
        };

        let debug = format!("{storage:?}");
        assert!(!debug.contains("runtime-password-value"));
        assert!(!debug.contains("session-token-value"));
        assert!(debug.contains("credential_present"));
    }

    #[test]
    fn runtime_endpoint_policy_rejects_missing_and_undeclared_loopback_storage() {
        assert!(validate_runtime_storage_endpoint("", false).is_err());
        assert!(validate_runtime_storage_endpoint("http://127.0.0.1:8000", false).is_err());
        assert!(validate_runtime_storage_endpoint("http://127.0.0.1:8000", true).is_ok());
        assert!(validate_runtime_storage_endpoint("https://storage.internal:8000", false).is_ok());
    }
}
