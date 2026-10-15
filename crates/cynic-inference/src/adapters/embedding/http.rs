//! Embedding adapter — calls any OpenAI-compatible /v1/embeddings endpoint.
//! Default target: local llama-server on :8081 with Qwen3-Embedding.

use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::domain::embedding::{Embedding, EmbeddingError};
use crate::domain::inference::{BackendInitError, BackendStatus};
use crate::ports::embedding_port::EmbeddingPort;
use crate::ports::inference_port::BackendPort;

const DEFAULT_REST_ADDR: &str = "127.0.0.1:3030";

/// Specification of an embedding endpoint with its physical concurrent slot capacity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointSpec {
    pub url: String,
    pub slots: usize,
}

impl EndpointSpec {
    pub fn parse(raw: &str, default_slots: usize) -> Self {
        let trimmed = raw.trim();
        if let Some((url_part, hash_part)) = trimmed.split_once('#') {
            let url = url_part.trim_end_matches('/').to_string();
            let slots = hash_part
                .strip_prefix("slots=")
                .unwrap_or(hash_part)
                .parse::<usize>()
                .ok()
                .filter(|&s| (1..=16).contains(&s))
                .unwrap_or(default_slots);
            Self { url, slots }
        } else {
            Self {
                url: trimmed.trim_end_matches('/').to_string(),
                slots: default_slots,
            }
        }
    }
}

#[derive(Debug)]
pub struct EmbeddingBackend {
    client: Client,
    endpoints: Vec<EndpointSpec>,
    api_key: Option<String>,
    model: String,
}

impl EmbeddingBackend {
    pub fn new(
        base_url: &str,
        api_key: Option<String>,
        model: &str,
    ) -> Result<Self, BackendInitError> {
        // Batched lots of long chunks can prefill well past 30 s on a
        // CPU-only server: default 120 s, override with CYNIC_EMBED_TIMEOUT_MS.
        let timeout_ms = std::env::var("CYNIC_EMBED_TIMEOUT_MS")
            .ok()
            .and_then(|raw| raw.parse::<u64>().ok())
            .filter(|ms| (1_000..=600_000).contains(ms))
            .unwrap_or(120_000);

        let default_slots = fanout_width();
        let endpoints: Vec<EndpointSpec> = base_url
            .split(',')
            .map(|s| EndpointSpec::parse(s, default_slots))
            .filter(|e| !e.url.is_empty())
            .collect();

        if endpoints.is_empty() {
            return Err(BackendInitError::from_http(
                "no valid embedding base url provided",
            ));
        }

        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_millis(timeout_ms))
                .build()
                .map_err(BackendInitError::from_http)?,
            endpoints,
            api_key,
            model: model.to_string(),
        })
    }

    pub fn base_url(&self) -> &str {
        &self.endpoints[0].url
    }

    pub fn base_urls(&self) -> Vec<String> {
        self.endpoints.iter().map(|e| e.url.clone()).collect()
    }

    pub fn endpoints(&self) -> &[EndpointSpec] {
        &self.endpoints
    }

    pub async fn discover(host: &str, api_key: Option<String>, model: &str) -> Option<Self> {
        let ports = [
            std::env::var("CYNIC_EMBED_PORT").unwrap_or_else(|_| "8081".into()),
            "8080".into(),
        ];
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(3))
            .build()
            .ok()?;

        for port in &ports {
            let base_url = format!("http://{host}:{port}/v1");
            let health_url = format!("http://{host}:{port}/health");
            let mut req = client.get(&health_url);
            if let Some(ref key) = api_key {
                req = req.header("Authorization", format!("Bearer {key}"));
            }
            match tokio::time::timeout(std::time::Duration::from_secs(3), req.send()).await {
                Ok(Ok(resp)) if resp.status().is_success() => {
                    tracing::info!(host, port, "embedding backend discovered");
                    return Self::new(&base_url, api_key, model)
                        .map_err(|error| {
                            tracing::warn!(host, port, error = %error, "embedding init failed")
                        })
                        .ok();
                }
                _ => {
                    tracing::debug!(host, port, "embedding port unreachable");
                    continue;
                }
            }
        }
        None
    }

    /// Resolves the embedding cluster from the workspace registry.
    ///
    /// Walks up from the current directory to the workspace root (the first
    /// ancestor holding `infra/registry.json`) and collects the
    /// `bench_endpoint` of every `embedding-server` service. Services
    /// without one (excluded runners) never join. Returns `None` when no
    /// registry resolves, letting the caller fall back further.
    fn endpoints_from_registry() -> Option<String> {
        let mut dir = std::env::current_dir().ok()?;
        loop {
            let candidate = dir.join("infra/registry.json");
            if candidate.is_file() {
                let data = std::fs::read_to_string(candidate).ok()?;
                let registry: serde_json::Value = serde_json::from_str(&data).ok()?;
                let endpoints: Vec<String> = registry
                    .get("nodes")?
                    .as_array()?
                    .iter()
                    .flat_map(|n| {
                        n.get("services")
                            .and_then(|s| s.as_array())
                            .map(|a| a.iter().cloned().collect::<Vec<_>>())
                            .unwrap_or_default()
                    })
                    .filter(|s| s.get("name").and_then(|n| n.as_str()) == Some("embedding-server"))
                    .filter_map(|s| s.get("bench_endpoint")?.as_str().map(str::to_string))
                    .filter(|e| !e.is_empty())
                    .collect();
                return (!endpoints.is_empty()).then(|| endpoints.join(","));
            }
            if !dir.pop() {
                return None;
            }
        }
    }

    pub fn from_env() -> Result<Self, BackendInitError> {
        // Explicit URL wins. Otherwise resolve the cluster from the
        // workspace registry (infra/registry.json, `bench_endpoint` on each
        // embedding-server service; excluded services carry none and stay
        // out by construction). Last resort is the historical single default.
        let base_url = std::env::var("CYNIC_EMBED_URL")
            .ok()
            .filter(|u| !u.is_empty())
            .or_else(|| Self::endpoints_from_registry())
            .unwrap_or_else(|| {
                let rest =
                    std::env::var("CYNIC_REST_ADDR").unwrap_or_else(|_| DEFAULT_REST_ADDR.into());
                let host = rest.split(':').next().unwrap_or("127.0.0.1");
                let port = std::env::var("CYNIC_EMBED_PORT").unwrap_or_else(|_| "8081".into());
                format!("http://{host}:{port}/v1")
            });
        let api_key = cynic_secrets::env_var("sovereign-api-key", "SOVEREIGN_API_KEY");
        let model = std::env::var("CYNIC_EMBED_MODEL").unwrap_or_else(|_| "qwen3-embed".into());
        Self::new(&base_url, api_key, &model)
    }
}

#[async_trait]
impl BackendPort for EmbeddingBackend {
    fn name(&self) -> &str {
        &self.model
    }

    async fn health(&self) -> BackendStatus {
        let start = Instant::now();
        let url = self.base_url().replace("/v1", "/health");
        match self.client.get(&url).send().await {
            Ok(resp) if resp.status().is_success() => {
                let latency = start.elapsed().as_millis() as f64;
                if latency > 2000.0 {
                    BackendStatus::Degraded {
                        latency_ms: latency,
                    }
                } else {
                    BackendStatus::Healthy
                }
            }
            Ok(resp) => {
                tracing::warn!(status = %resp.status(), "embedding backend degraded");
                BackendStatus::Degraded {
                    latency_ms: start.elapsed().as_millis() as f64,
                }
            }
            Err(error) => {
                tracing::error!(%error, "embedding backend unreachable");
                BackendStatus::Critical
            }
        }
    }
}

#[derive(Debug, Serialize)]
struct EmbeddingRequest {
    input: Vec<String>,
    model: String,
}

#[derive(Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
    usage: Option<EmbeddingUsage>,
}

#[derive(Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

#[derive(Deserialize)]
struct EmbeddingUsage {
    #[serde(default)]
    prompt_tokens: Option<u32>,
}

/// One `/v1/embeddings` round-trip per call: the server batches the whole
/// array on its continuous-batch engine, so one call of N inputs costs ~one
/// prefill instead of N round-trips.
const EMBED_BATCH_SIZE: usize = 32;
/// Upper clamp: beyond this the server-side physical batch (not ours) does
/// the splitting, so bigger lots only add latency, never throughput.
const EMBED_BATCH_MAX: usize = 128;
/// Sub-lot per in-flight request: small enough to fit one server slot, so
/// WIDTH of them keep every slot fed without queueing behind a giant lot.
const FANOUT_SUB_LOT: usize = 8;
/// Default concurrent in-flight requests; the sequential client starves the
/// server (GPU ~10% busy observed live), so fan-out is the throughput lever.
const FANOUT_DEFAULT_WIDTH: usize = 4;
const FANOUT_MAX_WIDTH: usize = 8;

/// Calibration lever (crate-visible): lot size follows `CYNIC_EMBED_LOT_SIZE`
/// (default 32) so batch/ubatch sweeps can run without recompiling.
pub(crate) fn lot_size() -> usize {
    parse_lot_size(std::env::var("CYNIC_EMBED_LOT_SIZE").ok().as_deref())
}

/// Pure parser behind `lot_size`: out-of-range and non-numeric inputs fall
/// back to the default, so calibration sweeps can never break requests.
fn parse_lot_size(raw: Option<&str>) -> usize {
    raw.and_then(|text| text.parse::<usize>().ok())
        .filter(|size| (1..=EMBED_BATCH_MAX).contains(size))
        .unwrap_or(EMBED_BATCH_SIZE)
}

/// Concurrent in-flight requests; 1 restores strict sequential behavior.
fn parse_fanout_width(raw: Option<&str>) -> usize {
    raw.and_then(|text| text.parse::<usize>().ok())
        .filter(|width| (1..=FANOUT_MAX_WIDTH).contains(width))
        .unwrap_or(FANOUT_DEFAULT_WIDTH)
}

fn fanout_width() -> usize {
    parse_fanout_width(std::env::var("CYNIC_EMBED_WIDTH").ok().as_deref())
}

#[async_trait]
impl EmbeddingPort for EmbeddingBackend {
    async fn embed(&self, text: &str) -> Result<Embedding, EmbeddingError> {
        let mut out = self.post_batch(vec![text.to_string()]).await?;
        out.pop()
            .ok_or_else(|| EmbeddingError::Protocol("empty response".into()))
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Embedding>, EmbeddingError> {
        self.embed_batch_reported(texts, &|_, _| {}).await
    }
}

impl EmbeddingBackend {
    /// Batched embedding with per-lot progress: `report(done, total)` fires
    /// after each round-trip so long corpora stay observable.
    /// Embeds a batch of texts using dynamic work-stealing pull across all endpoints.
    /// This method is total by construction: it never aborts on single-lot errors,
    /// returning `(results, errors)` so long measurement runs do not lose partial progress.
    pub async fn embed_batch_total_reported(
        &self,
        texts: &[&str],
        report: &(dyn Fn(usize, usize) + Sync),
    ) -> (Vec<Option<Embedding>>, Vec<(usize, String)>) {
        let sublots: Vec<(usize, Vec<String>)> = texts
            .chunks(FANOUT_SUB_LOT)
            .enumerate()
            .map(|(sub_no, lot)| (sub_no, lot.iter().map(|text| (*text).to_string()).collect()))
            .collect();
        let total_sublots = sublots.len();
        if total_sublots == 0 {
            return (Vec::new(), Vec::new());
        }

        // Dynamic work-stealing pull queue across all endpoints with failover requeuing
        // Items: (sub_no, inputs, attempt_count)
        let queue: std::sync::Arc<
            tokio::sync::Mutex<std::collections::VecDeque<(usize, Vec<String>, usize)>>,
        > = std::sync::Arc::new(tokio::sync::Mutex::new(
            sublots
                .into_iter()
                .map(|(sub_no, inputs)| (sub_no, inputs, 0))
                .collect(),
        ));
        let results = std::sync::Arc::new(tokio::sync::Mutex::new(vec![None; total_sublots]));
        let errors: std::sync::Arc<tokio::sync::Mutex<Vec<Option<String>>>> =
            std::sync::Arc::new(tokio::sync::Mutex::new(vec![None; total_sublots]));
        let completed = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

        let mut worker_futures = Vec::new();

        // Spawn `endpoint.slots` concurrent workers for each endpoint according to physical capacity
        for endpoint in &self.endpoints {
            for _ in 0..endpoint.slots {
                let queue = queue.clone();
                let results = results.clone();
                let errors = errors.clone();
                let completed = completed.clone();
                let endpoint_url = endpoint.url.clone();

                worker_futures.push(async move {
                    loop {
                        let maybe_lot = {
                            let mut q = queue.lock().await;
                            q.pop_front()
                        };
                        let (sub_no, inputs, attempts) = match maybe_lot {
                            Some(item) => item,
                            None => break, // Queue exhausted, worker exits cleanly
                        };

                        match self.post_batch_to(&endpoint_url, inputs.clone()).await {
                            Ok(batch) => {
                                {
                                    let mut res = results.lock().await;
                                    res[sub_no] = Some(batch);
                                }
                                let done_lots = completed
                                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
                                    + 1;
                                report((done_lots * FANOUT_SUB_LOT).min(texts.len()), texts.len());
                            }
                            Err(error) => {
                                if attempts + 1 < 3 {
                                    // S1 & S2 fix: Requeue sub-lot to shared queue so peer endpoints can steal it
                                    let mut q = queue.lock().await;
                                    q.push_back((sub_no, inputs, attempts + 1));
                                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                                } else {
                                    // Poisoned lot after 3 distributed attempts across endpoints
                                    let mut errs = errors.lock().await;
                                    errs[sub_no] = Some(format!(
                                        "lot {} failed on {} after {} attempts: {}",
                                        sub_no + 1,
                                        endpoint_url,
                                        attempts + 1,
                                        error
                                    ));
                                    completed.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                                }
                            }
                        }
                    }
                    Ok::<(), EmbeddingError>(())
                });
            }
        }

        // Run all workers concurrently until queue is empty
        let _ = futures_util::future::join_all(worker_futures).await;

        // Collect per-lot errors
        let mut recorded_errors = Vec::new();
        {
            let errs = errors.lock().await;
            for (sub_no, maybe_err) in errs.iter().enumerate() {
                if let Some(err) = maybe_err {
                    recorded_errors.push((sub_no, err.clone()));
                }
            }
        }

        // Reassemble in exact deterministic sequence
        let mut final_results = results.lock().await;
        let mut out = Vec::with_capacity(texts.len());
        for (sub_no, maybe_batch) in final_results.iter_mut().enumerate() {
            let lot_size = texts
                .chunks(FANOUT_SUB_LOT)
                .nth(sub_no)
                .map(|c| c.len())
                .unwrap_or(0);
            if let Some(batch) = maybe_batch.take() {
                for emb in batch {
                    out.push(Some(emb));
                }
            } else {
                for _ in 0..lot_size {
                    out.push(None);
                }
            }
        }
        report(out.iter().filter(|o| o.is_some()).count(), texts.len());
        (out, recorded_errors)
    }

    /// Batched embedding with per-lot progress: `report(done, total)` fires
    /// after each round-trip so long corpora stay observable.
    /// Fails closed if any lot in the batch fails after 3 distributed attempts across endpoints.
    pub async fn embed_batch_reported(
        &self,
        texts: &[&str],
        report: &(dyn Fn(usize, usize) + Sync),
    ) -> Result<Vec<Embedding>, EmbeddingError> {
        let (total_results, errors) = self.embed_batch_total_reported(texts, report).await;
        if !errors.is_empty() {
            let sample_err = errors[0].1.clone();
            let total_sublots = (texts.len() + FANOUT_SUB_LOT - 1) / FANOUT_SUB_LOT;
            return Err(EmbeddingError::Protocol(format!(
                "{}/{} sub-lots failed across cluster. Sample error: {sample_err}",
                errors.len(),
                total_sublots
            )));
        }

        let out = total_results
            .into_iter()
            .map(|opt| opt.expect("all items present when errors is empty"))
            .collect();
        Ok(out)
    }
}

impl EmbeddingBackend {
    async fn post_batch(&self, inputs: Vec<String>) -> Result<Vec<Embedding>, EmbeddingError> {
        self.post_batch_to(self.base_url(), inputs).await
    }

    async fn post_batch_to(
        &self,
        base_url: &str,
        inputs: Vec<String>,
    ) -> Result<Vec<Embedding>, EmbeddingError> {
        let url = format!("{base_url}/embeddings");
        let start = Instant::now();

        let req_body = EmbeddingRequest {
            input: inputs,
            model: self.model.clone(),
        };

        let mut req = self.client.post(&url).json(&req_body);

        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {key}"));
        }

        let resp = req.send().await.map_err(|error| {
            if error.is_timeout() {
                EmbeddingError::Timeout {
                    ms: start.elapsed().as_millis() as u64,
                }
            } else {
                EmbeddingError::Unreachable(error.to_string())
            }
        })?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(EmbeddingError::Protocol(format!("HTTP {status}: {body}")));
        }

        let parsed: EmbeddingResponse = resp
            .json()
            .await
            .map_err(|error| EmbeddingError::Protocol(format!("JSON parse error: {error}")))?;

        // `#[serde(default)]` on an Option is None, so an absent counter is None. It was
        // `u32` with the same attribute, which made a missing field indistinguishable from a
        // backend that legitimately reported zero.
        let prompt_tokens = parsed.usage.and_then(|usage| usage.prompt_tokens);
        Ok(parsed
            .data
            .into_iter()
            .map(|data| Embedding {
                dimensions: data.embedding.len(),
                vector: data.embedding,
                prompt_tokens,
            })
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_env_defaults() {
        // Explicit URL wins over the workspace registry; pin it so this
        // unit test never depends on the checkout it runs in.
        unsafe { std::env::set_var("CYNIC_EMBED_URL", "http://127.0.0.1:8081/v1") };
        let backend = EmbeddingBackend::from_env().unwrap();
        unsafe { std::env::remove_var("CYNIC_EMBED_URL") };
        assert!(backend.base_url().contains("8081"));
        assert_eq!(backend.base_urls().len(), 1);
    }

    #[test]
    fn resolves_cluster_from_registry() {
        // The workspace registry carries bench_endpoint per embedding-server
        // service; excluded runners carry none and stay out.
        let resolved = EmbeddingBackend::endpoints_from_registry().unwrap();
        assert!(
            resolved.contains("127.0.0.1:8081"),
            "cortex joins: {resolved}"
        );
        assert!(
            resolved.contains("192.168.0.25:8080"),
            "compute-01 joins: {resolved}"
        );
        assert!(
            !resolved.contains("10.10.10.106"),
            "lxc106 stays out: {resolved}"
        );
    }

    #[test]
    fn parses_multiple_comma_separated_endpoints() {
        let backend = EmbeddingBackend::new(
            "http://127.0.0.1:8081/v1, http://192.168.0.25:8080/v1, http://10.10.10.106:8080/v1/",
            None,
            "test-model",
        )
        .unwrap();
        assert_eq!(backend.base_urls().len(), 3);
        assert_eq!(backend.base_urls()[0], "http://127.0.0.1:8081/v1");
        assert_eq!(backend.base_urls()[1], "http://192.168.0.25:8080/v1");
        assert_eq!(backend.base_urls()[2], "http://10.10.10.106:8080/v1");
    }

    #[test]
    fn parses_custom_slot_capacities() {
        let backend = EmbeddingBackend::new(
            "http://127.0.0.1:8081/v1#4, http://192.168.0.25:8080/v1#slots=4, http://10.10.10.106:8080/v1#2",
            None,
            "test-model",
        )
        .unwrap();
        assert_eq!(backend.endpoints().len(), 3);
        assert_eq!(backend.endpoints()[0].slots, 4);
        assert_eq!(backend.endpoints()[1].slots, 4);
        assert_eq!(backend.endpoints()[2].slots, 2);
    }

    #[test]
    fn lot_size_follows_env_with_safe_bounds() {
        assert_eq!(parse_lot_size(None), EMBED_BATCH_SIZE);
        assert_eq!(parse_lot_size(Some("8")), 8);
        assert_eq!(parse_lot_size(Some("0")), EMBED_BATCH_SIZE);
        assert_eq!(parse_lot_size(Some("9999")), EMBED_BATCH_SIZE);
        assert_eq!(parse_lot_size(Some("not-a-number")), EMBED_BATCH_SIZE);
    }

    #[test]
    fn fanout_width_defaults_and_clamps() {
        assert_eq!(parse_fanout_width(None), FANOUT_DEFAULT_WIDTH);
        assert_eq!(parse_fanout_width(Some("1")), 1);
        assert_eq!(parse_fanout_width(Some("0")), FANOUT_DEFAULT_WIDTH);
        assert_eq!(parse_fanout_width(Some("99")), FANOUT_DEFAULT_WIDTH);
    }
}
