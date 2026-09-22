//! Sovereign Inference Gateway (`InferenceGateway`).
//!
//! Provides an always-on, OpenAI-compatible HTTP inference proxy server
//! with automated model hot-swapping driven by empirical telemetry proofs
//! and dynamic memory safety constraints.

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::Router;
use axum::body::Body;
use axum::extract::{Json, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use cynic_protocol::{ContextTier, ParametricConfigPoint};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::process::Child;
use tokio::sync::{Mutex, RwLock};

use crate::domain::matrix_sweep::HardwareBudget;
use crate::domain::matrix_sweep::empirical_store::{ConstraintFilter, OptimizationGoal};
use crate::infra::domain_task_queue::DomainTaskQueue;
use crate::ports::empirical_store_port::EmpiricalStorePort;

use super::matrix_sweep_executor::config_to_llama_argv;

pub const INFERENCE_GATEWAY_SCHEMA: &str = "cynic.inference.gateway.v1";

#[derive(Debug, Clone)]
pub struct InferenceGatewayConfig {
    pub listen_addr: SocketAddr,
    pub backend_port: u16,
    pub llama_server_bin: PathBuf,
    pub vault_dir: PathBuf,
    pub default_model: String,
    pub startup_timeout_secs: u64,
}

impl Default for InferenceGatewayConfig {
    fn default() -> Self {
        Self {
            listen_addr: "127.0.0.1:8088".parse().unwrap(),
            backend_port: 8089,
            llama_server_bin: PathBuf::from("/usr/local/bin/llama-server"),
            vault_dir: PathBuf::from("/var/lib/cynic/vault/models"),
            default_model: "Qwen3.5-2B-Q4_K_M".to_string(),
            startup_timeout_secs: 180,
        }
    }
}

#[derive(Clone)]
pub struct GatewaySharedState {
    pub config: InferenceGatewayConfig,
    pub store: Arc<dyn EmpiricalStorePort>,
    pub task_queue: Arc<DomainTaskQueue>,
    pub active_process: Arc<Mutex<Option<Child>>>,
    pub current_model: Arc<RwLock<Option<String>>>,
    pub http_client: Client,
    pub hardware: HardwareBudget,
}
impl std::fmt::Debug for GatewaySharedState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GatewaySharedState")
            .field("config", &self.config)
            .field("hardware", &self.hardware)
            .finish()
    }
}

impl std::fmt::Debug for InferenceGateway {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InferenceGateway")
            .field("state", &self.state)
            .finish()
    }
}

pub struct InferenceGateway {
    state: GatewaySharedState,
}

impl InferenceGateway {
    pub fn new(
        config: InferenceGatewayConfig,
        store: Arc<dyn EmpiricalStorePort>,
        task_queue: Arc<DomainTaskQueue>,
        hardware: HardwareBudget,
    ) -> Self {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .unwrap_or_default();

        let state = GatewaySharedState {
            config,
            store,
            task_queue,
            active_process: Arc::new(Mutex::new(None)),
            current_model: Arc::new(RwLock::new(None)),
            http_client,
            hardware,
        };

        Self { state }
    }

    /// Build the Axum router for the gateway.
    pub fn router(&self) -> Router {
        Router::new()
            .route("/health", get(health_handler))
            .route("/v1/models", get(models_handler))
            .route("/v1/chat/completions", post(chat_completions_handler))
            .with_state(self.state.clone())
    }

    /// Start listening and serving requests.
    pub async fn serve(&self) -> Result<(), std::io::Error> {
        let listener = tokio::net::TcpListener::bind(self.state.config.listen_addr).await?;
        tracing::info!(
            addr = %self.state.config.listen_addr,
            "Sovereign Inference Gateway listening"
        );
        let app = self.router();
        axum::serve(listener, app).await
    }
}

// ── Handler: GET /health ──────────────────────────────────────────

#[derive(Serialize)]
struct GatewayHealthResponse {
    schema: &'static str,
    status: &'static str,
    loaded_model: Option<String>,
    backend_port: u16,
    active_process_alive: bool,
}

async fn health_handler(State(state): State<GatewaySharedState>) -> impl IntoResponse {
    let current = state.current_model.read().await.clone();
    let is_alive = {
        let mut guard = state.active_process.lock().await;
        if let Some(ref mut child) = *guard {
            child.try_wait().ok().flatten().is_none()
        } else {
            false
        }
    };

    let response = GatewayHealthResponse {
        schema: INFERENCE_GATEWAY_SCHEMA,
        status: "ok",
        loaded_model: current,
        backend_port: state.config.backend_port,
        active_process_alive: is_alive,
    };

    (StatusCode::OK, Json(response))
}

// ── Handler: GET /v1/models ───────────────────────────────────────

#[derive(Serialize)]
struct ModelCard {
    id: String,
    object: &'static str,
    created: u64,
    owned_by: &'static str,
}

#[derive(Serialize)]
struct ModelListResponse {
    object: &'static str,
    data: Vec<ModelCard>,
}

async fn models_handler(State(state): State<GatewaySharedState>) -> impl IntoResponse {
    let mut models = Vec::new();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if state.config.vault_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&state.config.vault_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("gguf") {
                    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                        models.push(ModelCard {
                            id: stem.to_string(),
                            object: "model",
                            created: now,
                            owned_by: "cynic-sovereign",
                        });
                    }
                }
            }
        }
    }

    (
        StatusCode::OK,
        Json(ModelListResponse {
            object: "list",
            data: models,
        }),
    )
}

// ── Handler: POST /v1/chat/completions ─────────────────────────────

#[derive(Deserialize)]
struct ChatCompletionRequest {
    #[serde(default)]
    model: Option<String>,
    #[serde(flatten)]
    extra: serde_json::Value,
}

async fn chat_completions_handler(
    State(state): State<GatewaySharedState>,
    Json(payload): Json<ChatCompletionRequest>,
) -> Response {
    let requested_model = payload
        .model
        .clone()
        .unwrap_or_else(|| state.config.default_model.clone());

    // 1. Check if we need to hot-swap models
    let swap_needed = {
        let current = state.current_model.read().await;
        current.as_deref() != Some(&requested_model)
    };

    if swap_needed {
        if let Err(err_resp) = ensure_model_loaded(&state, &requested_model).await {
            return err_resp;
        }
    }

    // 2. Forward request to backend llama-server
    let backend_url = format!(
        "http://127.0.0.1:{}/v1/chat/completions",
        state.config.backend_port
    );

    let mut full_payload = payload.extra.clone();
    if let Some(obj) = full_payload.as_object_mut() {
        obj.insert("model".into(), serde_json::json!(requested_model));
    }

    match state
        .http_client
        .post(&backend_url)
        .json(&full_payload)
        .send()
        .await
    {
        Ok(resp) => {
            let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::OK);
            let mut headers = HeaderMap::new();
            for (key, val) in resp.headers() {
                headers.insert(key.clone(), val.clone());
            }

            let stream = resp.bytes_stream();
            let body = Body::from_stream(stream);
            (status, headers, body).into_response()
        }
        Err(e) => {
            let err_json = serde_json::json!({
                "error": {
                    "message": format!("Failed to proxy to backend llama-server: {e}"),
                    "type": "cynic_gateway_proxy_error",
                    "code": 502
                }
            });
            (StatusCode::BAD_GATEWAY, Json(err_json)).into_response()
        }
    }
}

/// Dynamic Model Swapper: ensures the requested model is running with proven optimal flags.
async fn ensure_model_loaded(
    state: &GatewaySharedState,
    model_alias: &str,
) -> Result<(), Response> {
    let model_filename = if model_alias.ends_with(".gguf") {
        model_alias.to_string()
    } else {
        format!("{model_alias}.gguf")
    };

    let model_path = state.config.vault_dir.join(&model_filename);
    if !model_path.exists() {
        let err_json = serde_json::json!({
            "error": {
                "message": format!("Model file '{model_filename}' not found in vault {:?}", state.config.vault_dir),
                "type": "model_not_found",
                "code": 404
            }
        });
        return Err((StatusCode::NOT_FOUND, Json(err_json)).into_response());
    }

    let _weight_bytes = model_path
        .metadata()
        .map(|m| m.len())
        .unwrap_or(2 * 1024 * 1024 * 1024);
    let system_reserve = state.hardware.total_ram_bytes / 10;

    // Solve for the optimal configuration from the empirical ledger
    let optimal_config = state
        .store
        .resolve_optimal(
            model_alias,
            state.hardware,
            system_reserve,
            &ConstraintFilter::default(),
            OptimizationGoal::Balanced,
        )
        .await
        .ok()
        .flatten()
        .map(|entry| entry.config)
        .unwrap_or_else(|| {
            // Fallback baseline configuration if model hasn't been swept yet
            ParametricConfigPoint {
                model_alias: model_alias.to_string(),
                model_file: model_filename.clone(),
                context_tier: ContextTier::Ctx4k,
                kv_quant: cynic_protocol::KvQuantization::Fp16,
                threads_gen: 4,
                threads_batch: 8,
                batch_size: 512,
                ubatch_size: 256,
                flash_attn: false,
                gpu_layers: 0,
                rope_scaling: cynic_protocol::RopeScaling::None,
                topology: cynic_protocol::TopologyStrategy::DedicatedSingle,
                target_harness: cynic_protocol::HarnessProtocol::OpenAiToolCall,
            }
        });

    let argv = config_to_llama_argv(
        model_path.to_str().unwrap_or(""),
        &optimal_config,
        state.config.backend_port,
    );

    tracing::info!(
        model = %model_alias,
        ctx = %optimal_config.context_tier.token_count(),
        "Hot-swapping model with optimal empirical vector"
    );

    // Acquire lock and swap process
    let mut process_guard = state.active_process.lock().await;

    // 1. Terminate old process if any
    if let Some(ref mut child) = *process_guard {
        let _ = child.kill().await;
        let _ = child.wait().await;
    }

    // 2. Spawn new process
    let child = match tokio::process::Command::new(&state.config.llama_server_bin)
        .args(&argv)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let err_json = serde_json::json!({
                "error": {
                    "message": format!("Failed to spawn llama-server: {e}"),
                    "type": "cynic_spawn_error",
                    "code": 500
                }
            });
            return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(err_json)).into_response());
        }
    };

    *process_guard = Some(child);

    // 3. Wait for /health readiness
    let health_url = format!("http://127.0.0.1:{}/health", state.config.backend_port);
    let deadline = Instant::now() + Duration::from_secs(state.config.startup_timeout_secs);
    let mut ready = false;

    while Instant::now() < deadline {
        if let Some(ref mut c) = *process_guard {
            if let Ok(Some(exit)) = c.try_wait() {
                let err_json = serde_json::json!({
                    "error": {
                        "message": format!("llama-server crashed during startup (exit: {exit:?})"),
                        "type": "cynic_server_crashed",
                        "code": 500
                    }
                });
                return Err((StatusCode::INTERNAL_SERVER_ERROR, Json(err_json)).into_response());
            }
        }

        if let Ok(resp) = state.http_client.get(&health_url).send().await {
            if resp.status().is_success() {
                ready = true;
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    if !ready {
        let err_json = serde_json::json!({
            "error": {
                "message": format!("Timeout waiting for llama-server readiness on port {}", state.config.backend_port),
                "type": "cynic_startup_timeout",
                "code": 504
            }
        });
        return Err((StatusCode::GATEWAY_TIMEOUT, Json(err_json)).into_response());
    }

    // 4. Update current model
    let mut cur = state.current_model.write().await;
    *cur = Some(model_alias.to_string());

    Ok(())
}
