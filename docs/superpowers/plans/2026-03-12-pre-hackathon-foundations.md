# Pre-Hackathon Foundations Implementation Plan

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor CYNIC V2 kernel to full hexagonal purity with universal OpenAI-compat backends, InferenceDog, parallel Judge with residual detection, StoragePort, probe refactor, frontend, and infrastructure.

**Architecture:** Domain core defines traits (Dog, ChatPort, InferencePort, StoragePort). Adapters implement them (OpenAiCompatBackend, SurrealDbAdapter). Composition root (main.rs) wires everything via backends.toml. Frontend is vanilla HTML/JS served by Axum.

**Tech Stack:** Rust 1.94, Axum 0.8, reqwest 0.13, SurrealDB 3.0, Three.js (CDN), tower-http (cors + static), futures, toml.

**Build/test:** NEVER cargo build on Windows. Push via git → forge validates + deploys. Use Tailscale MCP for remote ops.

**Spec:** `docs/superpowers/specs/2026-03-12-pre-hackathon-foundations-design.md`

**Note:** All `src/`, `tests/`, `static/` paths are relative to `cynic-kernel/`. E.g., `src/dog.rs` = `cynic-kernel/src/dog.rs`.

---

## File Map

### Create
| File | Responsibility |
|------|---------------|
| `src/chat_port.rs` | ChatPort trait + ChatError (domain core) |
| `src/backend_openai.rs` | OpenAiCompatBackend — implements ChatPort + InferencePort |
| `src/inference_dog.rs` | InferenceDog — Dog that uses ChatPort for axiom evaluation |
| `src/config.rs` | BackendConfig, AuthStyle, load_backends() from TOML |
| `src/storage_port.rs` | StoragePort trait (domain core) |
| `tests/dog_contract.rs` | Dog trait contract test suite |
| `tests/chat_port_contract.rs` | ChatPort trait contract test suite |
| `static/index.html` | Frontend layout, form, dark theme |
| `static/verdict.js` | Fetch /judge, render verdict card |
| `static/radar.js` | Canvas radar chart, axioms per Dog |
| `static/hypercube.js` | Three.js 3D axiom-space visualization |
| `src/probe/mod.rs` | Probe orchestration (replaces probe.rs monolith) |
| `src/probe/gpu.rs` | GpuDetector trait (domain core) |
| `src/probe/gpu_linux.rs` | SysfsDetector adapter |
| `src/probe/system.rs` | Hardware info, env detection |
| `src/probe/models.rs` | GGUF + Ollama discovery |
| `src/probe/servers.rs` | Inference server discovery |
| `src/probe/advisor.rs` | SovereigntyAdvisor pure logic |
| `src/probe/persistence.rs` | Load/save NodeConfig TOML |

### Modify
| File | Changes |
|------|---------|
| `src/dog.rs:118-136` | Add `DogError::Timeout`, `DogScore` struct, extend `Verdict` with `dog_scores`, `anomaly_detected`, `max_disagreement`, `anomaly_axiom` |
| `src/judge.rs:18-66` | Parallel eval via `futures::future::join_all`, residual detection, per-dog score collection |
| `src/rest.rs:17-18,21-24,36-43,75-88,160-178` | Use `Arc<dyn StoragePort>`, add static serving, extend JudgeResponse with dog_scores/anomaly |
| `src/storage.rs:1-30` | Implement StoragePort trait on CynicStorage |
| `src/main.rs:57-191` | New boot sequence: load backends.toml → OpenAiCompatBackend → InferenceDog → Judge |
| `src/lib.rs:1-17` | Add new modules, remove gemini_dog + backend_llamacpp |
| `Cargo.toml:6-24` | Add `futures`, `dirs`, tower-http `fs` feature |

### Delete
| File | Reason |
|------|--------|
| `src/gemini_dog.rs` | Replaced by `inference_dog.rs` |
| `src/backend_llamacpp.rs` | Replaced by `backend_openai.rs` |
| `src/probe.rs` | Replaced by `src/probe/` module tree (P0 refactor) |

---

## Chunk 1: Domain Core Traits

### Task 1: Add DogError::Timeout and DogScore to dog.rs

**Files:**
- Modify: `src/dog.rs:118-136`

- [ ] **Step 1: Add DogError::Timeout variant**

In `src/dog.rs`, add `Timeout` to the `DogError` enum:

```rust
#[derive(Debug)]
pub enum DogError {
    ApiError(String),
    ParseError(String),
    RateLimited(String),
    Timeout,
}
```

Update `Display` impl to handle new variant:

```rust
impl std::fmt::Display for DogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiError(msg) => write!(f, "Dog API error: {}", msg),
            Self::ParseError(msg) => write!(f, "Dog parse error: {}", msg),
            Self::RateLimited(msg) => write!(f, "Dog rate limited: {}", msg),
            Self::Timeout => write!(f, "Dog evaluation timed out"),
        }
    }
}
```

- [ ] **Step 2: Add DogScore struct**

After `AxiomReasoning`, add:

```rust
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DogScore {
    pub dog_id: String,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub reasoning: AxiomReasoning,
}
```

- [ ] **Step 3: Extend Verdict struct**

Add new fields to `Verdict`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub id: String,
    pub kind: VerdictKind,
    pub q_score: QScore,
    pub reasoning: AxiomReasoning,
    pub dog_id: String,
    pub stimulus_summary: String,
    pub timestamp: String,
    #[serde(default)]
    pub dog_scores: Vec<DogScore>,
    #[serde(default)]
    pub anomaly_detected: bool,
    #[serde(default)]
    pub max_disagreement: f64,
    #[serde(default)]
    pub anomaly_axiom: Option<String>,
}
```

- [ ] **Step 4: Commit**

```bash
git add src/dog.rs
git commit -m "feat(dog): add DogError::Timeout, DogScore, extend Verdict for residual detection"
```

---

### Task 2: Create ChatPort trait

**Files:**
- Create: `src/chat_port.rs`

- [ ] **Step 1: Write ChatPort trait**

```rust
//! ChatPort — minimal text-in/text-out contract for LLM inference.
//! Dogs use this. MCTS uses InferencePort. Same backend, two interfaces (ISP).

use async_trait::async_trait;
use crate::backend::BackendStatus;

#[derive(Debug, Clone)]
pub enum ChatError {
    Unreachable(String),
    Timeout { ms: u64 },
    RateLimited(String),
    Protocol(String),
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(msg) => write!(f, "Chat unreachable: {}", msg),
            Self::Timeout { ms } => write!(f, "Chat timed out after {}ms", ms),
            Self::RateLimited(msg) => write!(f, "Chat rate limited: {}", msg),
            Self::Protocol(msg) => write!(f, "Chat protocol error: {}", msg),
        }
    }
}

#[async_trait]
pub trait ChatPort: Send + Sync {
    /// Send a system+user prompt, get back the assistant's text response.
    async fn chat(&self, system: &str, user: &str) -> Result<String, ChatError>;

    /// Health check this backend.
    async fn health(&self) -> BackendStatus;

    /// Human-readable name for this backend (e.g. "gemini", "hf-mistral", "local-phi3").
    fn name(&self) -> &str;
}

/// Mock implementation for tests.
pub struct MockChatBackend {
    pub response: String,
    pub name: String,
    pub force_error: Option<ChatError>,
}

impl MockChatBackend {
    pub fn new(name: &str, response: &str) -> Self {
        Self {
            response: response.to_string(),
            name: name.to_string(),
            force_error: None,
        }
    }
}

#[async_trait]
impl ChatPort for MockChatBackend {
    async fn chat(&self, _system: &str, _user: &str) -> Result<String, ChatError> {
        if let Some(ref err) = self.force_error {
            return Err(err.clone());
        }
        Ok(self.response.clone())
    }

    async fn health(&self) -> BackendStatus {
        if self.force_error.is_some() {
            BackendStatus::Critical
        } else {
            BackendStatus::Healthy
        }
    }

    fn name(&self) -> &str {
        &self.name
    }
}
```

- [ ] **Step 2: Register module in lib.rs**

Add `pub mod chat_port;` to `src/lib.rs`.

- [ ] **Step 3: Commit**

```bash
git add src/chat_port.rs src/lib.rs
git commit -m "feat(domain): add ChatPort trait — minimal text-in/text-out contract for Dogs"
```

---

### Task 3: Create StoragePort trait

**Files:**
- Create: `src/storage_port.rs`
- Modify: `src/storage.rs`

- [ ] **Step 1: Write StoragePort trait**

```rust
//! StoragePort v1 — domain contract for verdict persistence.
//! Domain core defines this trait. SurrealDB adapter implements it.
//! NOTE: CYNIC-ARCHITECTURE-TRUTHS.md defines a broader StoragePort (store_fact, query_facts,
//! register_trust, verify_trust). This v1 is scoped to verdict CRUD for the hackathon.
//! The full fact/trust API will extend this trait post-hackathon.

use async_trait::async_trait;
use crate::dog::Verdict;

#[derive(Debug)]
pub enum StorageError {
    ConnectionFailed(String),
    QueryFailed(String),
    NotFound(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed(m) => write!(f, "Storage connection failed: {}", m),
            Self::QueryFailed(m) => write!(f, "Storage query failed: {}", m),
            Self::NotFound(m) => write!(f, "Not found: {}", m),
        }
    }
}

#[async_trait]
pub trait StoragePort: Send + Sync {
    async fn store_verdict(&self, verdict: &Verdict) -> Result<(), StorageError>;
    async fn get_verdict(&self, id: &str) -> Result<Option<Verdict>, StorageError>;
    async fn list_verdicts(&self, limit: u32) -> Result<Vec<Verdict>, StorageError>;
}
```

- [ ] **Step 2: Implement StoragePort on CynicStorage**

In `src/storage.rs`, add after the existing `impl CynicStorage` block:

```rust
use crate::storage_port::{StoragePort, StorageError};

#[async_trait::async_trait]
impl StoragePort for CynicStorage {
    async fn store_verdict(&self, verdict: &crate::dog::Verdict) -> Result<(), StorageError> {
        self.store_verdict_internal(verdict).await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))
    }

    async fn get_verdict(&self, id: &str) -> Result<Option<crate::dog::Verdict>, StorageError> {
        self.get_verdict_internal(id).await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))
    }

    async fn list_verdicts(&self, limit: u32) -> Result<Vec<crate::dog::Verdict>, StorageError> {
        self.list_verdicts_internal(limit).await
            .map_err(|e| StorageError::QueryFailed(e.to_string()))
    }
}
```

Rename existing `store_verdict` → `store_verdict_internal`, `get_verdict` → `get_verdict_internal`, `list_verdicts` → `list_verdicts_internal` (private methods, trait methods are the public API).

- [ ] **Step 3: Register module in lib.rs**

Add `pub mod storage_port;` to `src/lib.rs`.

- [ ] **Step 4: Commit**

```bash
git add src/storage_port.rs src/storage.rs src/lib.rs
git commit -m "feat(domain): add StoragePort trait, implement on CynicStorage"
```

---

## Chunk 2: OpenAiCompatBackend + InferenceDog

### Task 4: Create config.rs with BackendConfig

**Files:**
- Create: `src/config.rs`

- [ ] **Step 1: Write config module**

```rust
//! Backend configuration — loaded from backends.toml or env vars.
//! Lives in infrastructure layer. NEVER imported by domain core.

use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
    pub auth_style: AuthStyle,
}

#[derive(Debug, Clone)]
pub enum AuthStyle {
    Bearer,
    QueryParam(String),
    None,
}

#[derive(Deserialize)]
struct BackendsFile {
    backend: std::collections::HashMap<String, BackendEntry>,
}

#[derive(Deserialize)]
struct BackendEntry {
    base_url: String,
    api_key_env: Option<String>,
    model: String,
    auth_style: Option<String>,
}

/// Load backend configs from TOML file. Resolves api_key_env to actual env var values.
pub fn load_backends(path: &Path) -> Vec<BackendConfig> {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("[config] Cannot read {}: {}", path.display(), e);
            return Vec::new();
        }
    };

    let file: BackendsFile = match toml::from_str(&content) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("[config] Invalid TOML in {}: {}", path.display(), e);
            return Vec::new();
        }
    };

    file.backend
        .into_iter()
        .filter_map(|(name, entry)| {
            let api_key = entry.api_key_env.and_then(|env_name| {
                match std::env::var(&env_name) {
                    Ok(val) if !val.is_empty() => Some(val),
                    Ok(_) => {
                        eprintln!("[config] {} env var is empty, skipping backend '{}'", env_name, name);
                        None
                    }
                    Err(_) => {
                        eprintln!("[config] {} not set, skipping backend '{}'", env_name, name);
                        None
                    }
                }
            });

            // If api_key_env was specified but not resolved, skip this backend
            if entry.api_key_env.is_some() && api_key.is_none() {
                return None;
            }

            let auth_style = match entry.auth_style.as_deref() {
                Some("bearer") | None => AuthStyle::Bearer,
                Some("none") => AuthStyle::None,
                Some(other) if other.starts_with("query:") => {
                    AuthStyle::QueryParam(other.trim_start_matches("query:").to_string())
                }
                Some(other) => {
                    eprintln!("[config] Unknown auth_style '{}' for backend '{}', defaulting to bearer", other, name);
                    AuthStyle::Bearer
                }
            };

            Some(BackendConfig {
                name,
                base_url: entry.base_url,
                api_key,
                model: entry.model,
                auth_style,
            })
        })
        .collect()
}

/// Fallback: build configs from legacy env vars (backward compat).
pub fn load_backends_from_env() -> Vec<BackendConfig> {
    let mut configs = Vec::new();

    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        let model = std::env::var("GEMINI_MODEL")
            .unwrap_or_else(|_| "gemini-2.5-flash".to_string());
        configs.push(BackendConfig {
            name: "gemini".to_string(),
            base_url: "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
            api_key: Some(api_key),
            model,
            auth_style: AuthStyle::Bearer,
        });
    }

    configs
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn parse_valid_toml() {
        let toml_content = r#"
[backend.local]
base_url = "http://localhost:8080/v1"
model = "phi-3-mini"
auth_style = "none"
"#;
        let dir = std::env::temp_dir().join("cynic_config_test");
        std::fs::create_dir_all(&dir).ok();
        let path = dir.join("backends.toml");
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(toml_content.as_bytes()).unwrap();

        let configs = load_backends(&path);
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0].name, "local");
        assert!(configs[0].api_key.is_none());
        assert!(matches!(configs[0].auth_style, AuthStyle::None));

        std::fs::remove_file(&path).ok();
    }

    #[test]
    fn missing_file_returns_empty() {
        let configs = load_backends(Path::new("/nonexistent/backends.toml"));
        assert!(configs.is_empty());
    }
}
```

- [ ] **Step 2: Register module in lib.rs**

Add `pub mod config;` to `src/lib.rs`.

- [ ] **Step 3: Commit**

```bash
git add src/config.rs src/lib.rs
git commit -m "feat(config): backends.toml loader with env var resolution and legacy fallback"
```

---

### Task 5: Create OpenAiCompatBackend

**Files:**
- Create: `src/backend_openai.rs`

- [ ] **Step 1: Write OpenAiCompatBackend**

```rust
//! OpenAiCompatBackend — universal adapter for any OpenAI-compatible inference server.
//! Implements ChatPort (for Dogs) and InferencePort (for MCTS).
//! One type, N instances (Gemini, HuggingFace, llama.cpp, vLLM, SGLang).

use crate::backend::*;
use crate::chat_port::{ChatPort, ChatError};
use crate::config::{BackendConfig, AuthStyle};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;

pub struct OpenAiCompatBackend {
    client: Client,
    config: BackendConfig,
    capability: BackendCapability,
}

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    n: Option<u32>,
}

#[derive(Serialize, Deserialize, Clone)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    model: String,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

impl OpenAiCompatBackend {
    /// Create a new backend from config. Does NOT health-check — call health() after.
    pub fn new(config: BackendConfig) -> Self {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("Failed to build HTTP client");

        let capability = BackendCapability {
            id: config.name.clone(),
            kind: if config.base_url.contains("localhost") || config.base_url.contains("127.0.0.1") {
                BackendKind::Local
            } else {
                BackendKind::Remote { url: config.base_url.clone() }
            },
            device_name: format!("openai-compat:{}", config.name),
            vram_total_gb: 0.0,
            vram_available_gb: 0.0,
            latency_ms: 0.0,
            loaded_models: vec![config.model.clone()],
        };

        Self { client, config, capability }
    }

    fn build_url(&self, path: &str) -> String {
        let base = self.config.base_url.trim_end_matches('/');
        let url = format!("{}{}", base, path);

        match &self.config.auth_style {
            AuthStyle::QueryParam(param) => {
                if let Some(ref key) = self.config.api_key {
                    format!("{}?{}={}", url, param, key)
                } else {
                    url
                }
            }
            _ => url,
        }
    }

    fn apply_auth(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.config.auth_style {
            AuthStyle::Bearer => {
                if let Some(ref key) = self.config.api_key {
                    req.header("Authorization", format!("Bearer {}", key))
                } else {
                    req
                }
            }
            // QueryParam auth is handled in build_url
            AuthStyle::QueryParam(_) | AuthStyle::None => req,
        }
    }

    async fn post_chat(&self, messages: Vec<Message>, temperature: Option<f32>, max_tokens: Option<u32>, n: Option<u32>) -> Result<ChatCompletionResponse, ChatError> {
        let url = self.build_url("/chat/completions");
        let body = ChatCompletionRequest {
            model: self.config.model.clone(),
            messages,
            temperature,
            max_tokens,
            n,
        };

        let req = self.client.post(&url).json(&body);
        let req = self.apply_auth(req);

        let resp = req.send().await.map_err(|e| {
            if e.is_timeout() {
                ChatError::Timeout { ms: 120_000 }
            } else {
                ChatError::Unreachable(format!("{}: {}", self.config.name, e))
            }
        })?;

        if resp.status().as_u16() == 429 {
            return Err(ChatError::RateLimited(format!("{}: rate limited", self.config.name)));
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(ChatError::Protocol(format!("{} HTTP {}: {}", self.config.name, status, text)));
        }

        resp.json().await
            .map_err(|e| ChatError::Protocol(format!("{}: parse error: {}", self.config.name, e)))
    }
}

// ── ChatPort implementation (for Dogs) ──────────────────────

#[async_trait]
impl ChatPort for OpenAiCompatBackend {
    async fn chat(&self, system: &str, user: &str) -> Result<String, ChatError> {
        let mut messages = Vec::new();
        if !system.is_empty() {
            messages.push(Message { role: "system".to_string(), content: system.to_string() });
        }
        messages.push(Message { role: "user".to_string(), content: user.to_string() });

        let resp = self.post_chat(messages, Some(0.3), Some(500), None).await?;

        resp.choices.into_iter().next()
            .map(|c| c.message.content)
            .ok_or_else(|| ChatError::Protocol(format!("{}: empty response", self.config.name)))
    }

    async fn health(&self) -> BackendStatus {
        let start = Instant::now();
        let url = self.build_url("/models");
        let req = self.client.get(&url);
        let req = self.apply_auth(req);

        match req.send().await {
            Ok(resp) if resp.status().is_success() => {
                let latency = start.elapsed().as_millis() as f64;
                if latency > 2000.0 {
                    BackendStatus::Degraded { latency_ms: latency }
                } else {
                    BackendStatus::Healthy
                }
            }
            Ok(_) => BackendStatus::Degraded { latency_ms: start.elapsed().as_millis() as f64 },
            Err(_) => BackendStatus::Critical,
        }
    }

    fn name(&self) -> &str {
        &self.config.name
    }
}

// ── InferencePort implementation (for MCTS) ─────────────────

#[async_trait]
impl InferencePort for OpenAiCompatBackend {
    fn capability(&self) -> &BackendCapability {
        &self.capability
    }

    async fn infer(&self, req: InferenceRequest) -> Result<InferenceResponse, BackendError> {
        let start = Instant::now();

        let mut messages = Vec::new();
        if !req.system_prompt.is_empty() {
            messages.push(Message { role: "system".to_string(), content: req.system_prompt });
        }
        messages.push(Message { role: "user".to_string(), content: req.context });

        let resp = self.post_chat(messages, Some(req.temperature), None, Some(req.num_branches.max(1)))
            .await
            .map_err(|e| match e {
                ChatError::Unreachable(m) => BackendError::Unreachable(m),
                ChatError::Timeout { ms } => BackendError::Timeout { backend_id: self.config.name.clone(), ms },
                ChatError::RateLimited(m) | ChatError::Protocol(m) => BackendError::Protocol(m),
            })?;

        let model_used = resp.model;
        let hypotheses: Vec<String> = resp.choices.into_iter().map(|c| c.message.content).collect();

        Ok(InferenceResponse {
            trace_id: req.trace_id,
            hypotheses,
            latency_ms: start.elapsed().as_millis() as f64,
            model_used,
            backend_id: self.config.name.clone(),
        })
    }

    async fn health(&self) -> BackendStatus {
        ChatPort::health(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_bearer_no_query() {
        let backend = OpenAiCompatBackend::new(BackendConfig {
            name: "test".into(),
            base_url: "https://api.example.com/v1".into(),
            api_key: Some("sk-123".into()),
            model: "gpt-4".into(),
            auth_style: AuthStyle::Bearer,
        });
        assert_eq!(backend.build_url("/chat/completions"), "https://api.example.com/v1/chat/completions");
    }

    #[test]
    fn build_url_query_param() {
        let backend = OpenAiCompatBackend::new(BackendConfig {
            name: "test".into(),
            base_url: "https://api.example.com/v1".into(),
            api_key: Some("key123".into()),
            model: "gemini".into(),
            auth_style: AuthStyle::QueryParam("key".into()),
        });
        assert_eq!(backend.build_url("/chat/completions"), "https://api.example.com/v1/chat/completions?key=key123");
    }

    #[test]
    fn build_url_no_auth() {
        let backend = OpenAiCompatBackend::new(BackendConfig {
            name: "local".into(),
            base_url: "http://localhost:8080/v1".into(),
            api_key: None,
            model: "phi-3".into(),
            auth_style: AuthStyle::None,
        });
        assert_eq!(backend.build_url("/chat/completions"), "http://localhost:8080/v1/chat/completions");
    }

    #[test]
    fn trailing_slash_handled() {
        let backend = OpenAiCompatBackend::new(BackendConfig {
            name: "test".into(),
            base_url: "https://api.example.com/v1/".into(),
            api_key: None,
            model: "m".into(),
            auth_style: AuthStyle::None,
        });
        assert_eq!(backend.build_url("/chat/completions"), "https://api.example.com/v1/chat/completions");
    }
}
```

- [ ] **Step 2: Register module in lib.rs**

Replace `pub mod backend_llamacpp;` with `pub mod backend_openai;` in `src/lib.rs`.

- [ ] **Step 3: Commit**

```bash
git add src/backend_openai.rs src/lib.rs
git commit -m "feat(adapter): OpenAiCompatBackend — universal adapter implementing ChatPort + InferencePort"
```

---

### Task 6: Create InferenceDog

**Files:**
- Create: `src/inference_dog.rs`

- [ ] **Step 1: Write InferenceDog**

```rust
//! InferenceDog — model-agnostic Dog that uses any ChatPort for axiom evaluation.
//! ONE prompt template, N backends. The Dog never knows which model it's talking to.

use crate::dog::*;
use crate::chat_port::ChatPort;
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;

pub struct InferenceDog {
    chat: Arc<dyn ChatPort>,
    dog_name: String,
}

impl InferenceDog {
    pub fn new(chat: Arc<dyn ChatPort>, name: String) -> Self {
        Self { chat, dog_name: name }
    }

    fn build_system_prompt() -> &'static str {
        "You are CYNIC, a sovereign judgment engine. You evaluate stimuli through axioms with honest uncertainty. DO NOT inflate scores. If unsure, score lower. Overconfidence is the enemy."
    }

    fn build_user_prompt(stimulus: &Stimulus) -> String {
        let context_block = stimulus.context.as_deref().unwrap_or("(no additional context)");
        let domain = stimulus.domain.as_deref().unwrap_or("general");

        format!(r#"DOMAIN: {domain}
STIMULUS: {content}
CONTEXT: {context_block}

Score each axiom from 0.0 to 1.0 with honest uncertainty.

AXIOMS:
1. FIDELITY — Is this faithful to truth/reality? Does it reflect what IS, not what we wish?
2. PHI — Is this structurally harmonious? Well-proportioned? Elegant or clumsy?
3. VERIFY — Is this verifiable or falsifiable? Can we test it? What evidence supports/refutes it?

Respond ONLY with this exact JSON (no markdown, no explanation):
{{"fidelity": 0.XX, "phi": 0.XX, "verify": 0.XX, "fidelity_reason": "...", "phi_reason": "...", "verify_reason": "..."}}"#,
            content = stimulus.content,
        )
    }
}

#[derive(Deserialize)]
struct AxiomResponse {
    fidelity: f64,
    phi: f64,
    verify: f64,
    #[serde(default)]
    fidelity_reason: String,
    #[serde(default)]
    phi_reason: String,
    #[serde(default)]
    verify_reason: String,
}

#[async_trait]
impl Dog for InferenceDog {
    fn id(&self) -> &str {
        &self.dog_name
    }

    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError> {
        let system = Self::build_system_prompt();
        let user = Self::build_user_prompt(stimulus);

        let text = self.chat.chat(system, &user).await
            .map_err(|e| match e {
                crate::chat_port::ChatError::RateLimited(m) => DogError::RateLimited(m),
                crate::chat_port::ChatError::Timeout { .. } => DogError::Timeout,
                other => DogError::ApiError(other.to_string()),
            })?;

        let json_str = extract_json(&text)
            .ok_or_else(|| DogError::ParseError(format!("No JSON found in: {}", text)))?;

        let parsed: AxiomResponse = serde_json::from_str(json_str)
            .map_err(|e| DogError::ParseError(format!("JSON parse failed: {} in: {}", e, json_str)))?;

        Ok(AxiomScores {
            fidelity: parsed.fidelity,
            phi: parsed.phi,
            verify: parsed.verify,
            reasoning: AxiomReasoning {
                fidelity: parsed.fidelity_reason,
                phi: parsed.phi_reason,
                verify: parsed.verify_reason,
            },
        })
    }
}

/// Extract JSON object from text that might contain markdown fences or extra text.
fn extract_json(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let mut depth = 0;
    let mut end = start;
    for (i, ch) in text[start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i + 1;
                    break;
                }
            }
            _ => {}
        }
    }
    if depth == 0 && end > start {
        Some(&text[start..end])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_port::MockChatBackend;

    #[test]
    fn extract_json_from_clean() {
        let input = r#"{"fidelity": 0.5, "phi": 0.4, "verify": 0.3}"#;
        assert_eq!(extract_json(input), Some(input));
    }

    #[test]
    fn extract_json_from_markdown() {
        let input = "```json\n{\"fidelity\": 0.5, \"phi\": 0.4, \"verify\": 0.3}\n```";
        let json = extract_json(input).unwrap();
        assert!(json.starts_with('{'));
        let parsed: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(parsed["fidelity"], 0.5);
    }

    #[test]
    fn prompt_contains_stimulus() {
        let stimulus = Stimulus {
            content: "e4 e5 Nf3".into(),
            context: Some("Chess opening".into()),
            domain: Some("chess".into()),
        };
        let prompt = InferenceDog::build_user_prompt(&stimulus);
        assert!(prompt.contains("e4 e5 Nf3"));
        assert!(prompt.contains("chess"));
        assert!(prompt.contains("FIDELITY"));
    }

    #[tokio::test]
    async fn mock_chat_produces_valid_scores() {
        let mock = Arc::new(MockChatBackend::new(
            "test-mock",
            r#"{"fidelity": 0.6, "phi": 0.5, "verify": 0.4, "fidelity_reason": "good", "phi_reason": "ok", "verify_reason": "decent"}"#,
        ));

        let dog = InferenceDog::new(mock, "test-dog".into());
        let stimulus = Stimulus {
            content: "The sky is blue.".into(),
            context: None,
            domain: None,
        };

        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!((scores.fidelity - 0.6).abs() < 0.01);
        assert!((scores.phi - 0.5).abs() < 0.01);
        assert!((scores.verify - 0.4).abs() < 0.01);
        assert_eq!(scores.reasoning.fidelity, "good");
    }
}
```

- [ ] **Step 2: Replace gemini_dog module in lib.rs**

Replace `pub mod gemini_dog;` with `pub mod inference_dog;` in `src/lib.rs`.

- [ ] **Step 3: Commit**

```bash
git add src/inference_dog.rs src/lib.rs
git commit -m "feat(dog): InferenceDog — model-agnostic Dog using ChatPort, replaces GeminiDog"
```

---

## Chunk 3: Judge Improvements

### Task 7: Parallel evaluation + residual detection + per-dog scores

**Files:**
- Modify: `src/judge.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Add futures dependency**

In `Cargo.toml`, add:
```toml
futures = "0.3"
```

- [ ] **Step 2: Rewrite Judge::evaluate()**

Replace the entire `evaluate` method in `src/judge.rs`:

```rust
use crate::dog::*;
use chrono::Utc;
use uuid::Uuid;

pub struct Judge {
    dogs: Vec<Box<dyn Dog>>,
}

impl Judge {
    pub fn new(dogs: Vec<Box<dyn Dog>>) -> Self {
        Self { dogs }
    }

    pub async fn evaluate(&self, stimulus: &Stimulus) -> Result<Verdict, JudgeError> {
        if self.dogs.is_empty() {
            return Err(JudgeError::NoDogs);
        }

        // Parallel evaluation — all Dogs run concurrently
        let futures: Vec<_> = self.dogs.iter()
            .map(|dog| {
                let id = dog.id().to_string();
                async move {
                    let result = dog.evaluate(stimulus).await;
                    (id, result)
                }
            })
            .collect();

        let results = futures::future::join_all(futures).await;

        let mut dog_scores: Vec<DogScore> = Vec::new();
        let mut errors: Vec<String> = Vec::new();

        for (id, result) in results {
            match result {
                Ok(scores) => {
                    dog_scores.push(DogScore {
                        dog_id: id,
                        fidelity: scores.fidelity,
                        phi: scores.phi,
                        verify: scores.verify,
                        reasoning: scores.reasoning,
                    });
                }
                Err(e) => errors.push(format!("{}: {}", id, e)),
            }
        }

        if dog_scores.is_empty() {
            return Err(JudgeError::AllDogsFailed(errors));
        }

        // Aggregate: arithmetic mean of per-axiom raw scores, then compute_qscore applies
        // geometric mean internally (Q = ³√(F×Φ×V)). This matches the spec: "geometric mean
        // (phi-bounded)" refers to compute_qscore, not to how we aggregate across Dogs.
        let n = dog_scores.len() as f64;
        let avg_fidelity = dog_scores.iter().map(|s| s.fidelity).sum::<f64>() / n;
        let avg_phi = dog_scores.iter().map(|s| s.phi).sum::<f64>() / n;
        let avg_verify = dog_scores.iter().map(|s| s.verify).sum::<f64>() / n;

        // Use median Dog's reasoning (deterministic under parallel execution)
        let mut sorted_by_q: Vec<&DogScore> = dog_scores.iter().collect();
        sorted_by_q.sort_by(|a, b| {
            let qa = compute_qscore(&AxiomScores { fidelity: a.fidelity, phi: a.phi, verify: a.verify, reasoning: AxiomReasoning::default() }).total;
            let qb = compute_qscore(&AxiomScores { fidelity: b.fidelity, phi: b.phi, verify: b.verify, reasoning: AxiomReasoning::default() }).total;
            qa.partial_cmp(&qb).unwrap_or(std::cmp::Ordering::Equal)
        });
        let median_reasoning = sorted_by_q.get(sorted_by_q.len() / 2)
            .map(|s| s.reasoning.clone())
            .unwrap_or_default();

        let aggregated = AxiomScores {
            fidelity: avg_fidelity,
            phi: avg_phi,
            verify: avg_verify,
            reasoning: median_reasoning,
        };

        let q_score = compute_qscore(&aggregated);
        let kind = verdict_kind(q_score.total);

        // Residual detection: compare per-Dog Q-Scores in verdict space
        let consensus_q = q_score.total;
        let max_disagreement = if dog_scores.len() > 1 {
            dog_scores.iter()
                .map(|s| {
                    let dog_raw = AxiomScores {
                        fidelity: s.fidelity, phi: s.phi, verify: s.verify,
                        reasoning: AxiomReasoning::default(),
                    };
                    let dog_q = compute_qscore(&dog_raw).total;
                    (dog_q - consensus_q).abs()
                })
                .fold(0.0_f64, f64::max)
        } else {
            0.0
        };
        let anomaly_detected = max_disagreement > PHI_INV2;

        // Find which axiom has the largest inter-Dog spread (for anomaly_axiom)
        let anomaly_axiom = if anomaly_detected && dog_scores.len() > 1 {
            let axioms = ["fidelity", "phi", "verify"];
            let spreads: Vec<(f64, &str)> = axioms.iter().map(|&name| {
                let values: Vec<f64> = dog_scores.iter().map(|s| match name {
                    "fidelity" => s.fidelity,
                    "phi" => s.phi,
                    "verify" => s.verify,
                    _ => 0.0,
                }).collect();
                let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
                let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
                (max - min, name)
            }).collect();
            spreads.into_iter().max_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(_, name)| name.to_string())
        } else {
            None
        };

        let dog_ids: Vec<&str> = dog_scores.iter().map(|s| s.dog_id.as_str()).collect();

        Ok(Verdict {
            id: Uuid::new_v4().to_string(),
            kind,
            q_score,
            reasoning: aggregated.reasoning,
            dog_id: dog_ids.join("+"),
            stimulus_summary: stimulus.content.chars().take(100).collect(),
            timestamp: Utc::now().to_rfc3339(),
            dog_scores,
            anomaly_detected,
            max_disagreement,
            anomaly_axiom,
        })
    }
}
```

- [ ] **Step 3: Verify existing tests still compile**

The existing `FixedDog` test helper and all tests should still work. The `Verdict` struct now has new fields that are populated.

- [ ] **Step 4: Add residual detection test**

```rust
#[tokio::test]
async fn residual_detection_flags_high_disagreement() {
    let judge = Judge::new(vec![
        Box::new(FixedDog {
            name: "optimist".into(),
            scores: AxiomScores {
                fidelity: 0.9, phi: 0.9, verify: 0.9,
                reasoning: AxiomReasoning::default(),
            },
        }),
        Box::new(FixedDog {
            name: "pessimist".into(),
            scores: AxiomScores {
                fidelity: 0.1, phi: 0.1, verify: 0.1,
                reasoning: AxiomReasoning::default(),
            },
        }),
    ]);

    let verdict = judge.evaluate(&test_stimulus()).await.unwrap();
    assert!(verdict.anomaly_detected, "Dogs disagreeing 0.9 vs 0.1 should trigger anomaly");
    assert!(verdict.max_disagreement > PHI_INV2);
    assert_eq!(verdict.dog_scores.len(), 2);
}

#[tokio::test]
async fn no_anomaly_when_dogs_agree() {
    let judge = Judge::new(vec![
        Box::new(FixedDog {
            name: "a".into(),
            scores: AxiomScores {
                fidelity: 0.5, phi: 0.5, verify: 0.5,
                reasoning: AxiomReasoning::default(),
            },
        }),
        Box::new(FixedDog {
            name: "b".into(),
            scores: AxiomScores {
                fidelity: 0.52, phi: 0.48, verify: 0.51,
                reasoning: AxiomReasoning::default(),
            },
        }),
    ]);

    let verdict = judge.evaluate(&test_stimulus()).await.unwrap();
    assert!(!verdict.anomaly_detected, "Similar scores should not trigger anomaly");
}
```

- [ ] **Step 5: Commit**

```bash
git add src/judge.rs Cargo.toml
git commit -m "feat(judge): parallel Dog evaluation + residual detection (anomaly > phi^-2)"
```

---

## Chunk 4: REST Refactor + Boot Sequence

### Task 8: Update REST to use StoragePort + extend response

**Files:**
- Modify: `src/rest.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Add tower-http fs feature to Cargo.toml**

Change tower-http line:
```toml
tower-http = { version = "0.6", features = ["cors", "fs"] }
```

- [ ] **Step 2: Rewrite rest.rs to use StoragePort and extended response**

Replace `use crate::storage::CynicStorage;` with `use crate::storage_port::StoragePort;`.

Change `AppState`:
```rust
pub struct AppState {
    pub judge: Judge,
    pub storage: Arc<dyn StoragePort>,
}
```

Add `DogScoreResponse` and extend `JudgeResponse`:
```rust
#[derive(Serialize)]
pub struct DogScoreResponse {
    pub dog_id: String,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub reasoning: ReasoningResponse,
}

#[derive(Serialize)]
pub struct JudgeResponse {
    pub verdict_id: String,
    pub verdict: String,
    pub q_score: QScoreResponse,
    pub reasoning: ReasoningResponse,
    pub dogs_used: String,
    pub phi_max: f64,
    pub dog_scores: Vec<DogScoreResponse>,
    pub anomaly_detected: bool,
    pub max_disagreement: f64,
    pub anomaly_axiom: Option<String>,
}
```

Update `verdict_to_response` to populate new fields:
```rust
fn verdict_to_response(v: &Verdict) -> JudgeResponse {
    JudgeResponse {
        verdict_id: v.id.clone(),
        verdict: format!("{:?}", v.kind),
        q_score: QScoreResponse {
            total: v.q_score.total,
            fidelity: v.q_score.fidelity,
            phi: v.q_score.phi,
            verify: v.q_score.verify,
        },
        reasoning: ReasoningResponse {
            fidelity: v.reasoning.fidelity.clone(),
            phi: v.reasoning.phi.clone(),
            verify: v.reasoning.verify.clone(),
        },
        dogs_used: v.dog_id.clone(),
        phi_max: PHI_INV,
        dog_scores: v.dog_scores.iter().map(|ds| DogScoreResponse {
            dog_id: ds.dog_id.clone(),
            fidelity: ds.fidelity,
            phi: ds.phi,
            verify: ds.verify,
            reasoning: ReasoningResponse {
                fidelity: ds.reasoning.fidelity.clone(),
                phi: ds.reasoning.phi.clone(),
                verify: ds.reasoning.verify.clone(),
            },
        }).collect(),
        anomaly_detected: v.anomaly_detected,
        max_disagreement: v.max_disagreement,
        anomaly_axiom: v.anomaly_axiom.clone(),
    }
}
```

Add static file serving to the router:
```rust
use tower_http::services::ServeDir;

pub fn router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/judge", post(judge_handler))
        .route("/verdict/{id}", get(get_verdict_handler))
        .route("/verdicts", get(list_verdicts_handler))
        .route("/health", get(health_handler))
        .nest_service("/", ServeDir::new("static"))
        .layer(cors)
        .with_state(state)
}
```

Update storage calls in handlers to use `StoragePort` error type (map `StorageError` to HTTP status).

- [ ] **Step 3: Commit**

```bash
git add src/rest.rs Cargo.toml
git commit -m "feat(rest): use StoragePort trait, extend response with dog_scores/anomaly, serve static files"
```

---

### Task 9: Rewrite main.rs boot sequence

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Rewrite boot sequence**

Replace the Dog construction section (lines 123-143) with:

```rust
// ─── RING 2: Load Backend Configs ──────────────────────────
let backends_path = dirs::config_dir()
    .unwrap_or_else(|| std::path::PathBuf::from("."))
    .join("cynic")
    .join("backends.toml");

let backend_configs = if backends_path.exists() {
    println!("[Ring 2] Loading backends from {}", backends_path.display());
    config::load_backends(&backends_path)
} else {
    println!("[Ring 2] No backends.toml found, using env var fallback");
    config::load_backends_from_env()
};

// ─── RING 2: Build Dogs (model-agnostic evaluators) ───────
let mut dogs: Vec<Box<dyn dog::Dog>> = Vec::new();

// Always add the deterministic Dog (free, fast)
dogs.push(Box::new(deterministic_dog::DeterministicDog));
println!("[Ring 2] DeterministicDog loaded");

// Create InferenceDog per configured backend
for cfg in backend_configs {
    let backend = Arc::new(backend_openai::OpenAiCompatBackend::new(cfg.clone()));
    let health = chat_port::ChatPort::health(backend.as_ref()).await;
    match health {
        backend::BackendStatus::Healthy | backend::BackendStatus::Degraded { .. } => {
            println!("[Ring 2] InferenceDog '{}' loaded (model: {}, health: {:?})", cfg.name, cfg.model, health);
            dogs.push(Box::new(inference_dog::InferenceDog::new(backend, cfg.name.clone())));
        }
        _ => {
            println!("[Ring 2] WARNING: Backend '{}' unreachable, skipping", cfg.name);
        }
    }
}

println!("[Ring 2] {} Dog(s) active", dogs.len());
```

Remove `backend_llamacpp` import. Add new imports at top:
```rust
use cynic_kernel::config;
use cynic_kernel::backend_openai;
use cynic_kernel::inference_dog;
use cynic_kernel::chat_port;
```

Update `AppState` construction to use `Arc<dyn StoragePort>`:
```rust
use cynic_kernel::storage_port::StoragePort;

let rest_state = Arc::new(rest::AppState {
    judge,
    storage: Arc::clone(&storage) as Arc<dyn StoragePort>,
});
```

Add `dirs` dependency to `Cargo.toml`:
```toml
dirs = "6"
```

- [ ] **Step 2: Delete old files**

Delete `src/gemini_dog.rs` and `src/backend_llamacpp.rs`.

- [ ] **Step 3: Update lib.rs final state**

```rust
pub mod probe;
pub mod supervisor;
pub mod hal;
pub mod pulse;
pub mod storage;
pub mod storage_port;
pub mod backend;
pub mod backend_openai;
pub mod router;
pub mod dog;
pub mod inference_dog;
pub mod deterministic_dog;
pub mod judge;
pub mod rest;
pub mod chat_port;
pub mod config;

pub mod cynic_v2 {
    tonic::include_proto!("cynic.v2");
}
```

- [ ] **Step 4: Commit**

```bash
git rm src/gemini_dog.rs src/backend_llamacpp.rs
git add src/main.rs src/lib.rs Cargo.toml
git commit -m "feat(boot): new boot sequence with backends.toml, InferenceDog per backend, remove GeminiDog/LlamaCppBackend"
```

---

## Chunk 5: Contract Tests

### Task 10: Dog contract tests

**Files:**
- Create: `tests/dog_contract.rs`

- [ ] **Step 1: Write contract test suite**

```rust
//! Dog trait contract tests — any Dog implementation must pass these.

use cynic_kernel::dog::*;
use cynic_kernel::deterministic_dog::DeterministicDog;
use cynic_kernel::inference_dog::InferenceDog;
use cynic_kernel::chat_port::MockChatBackend;
use std::sync::Arc;

async fn dog_contract(dog: &dyn Dog) {
    // 1. ID must be non-empty
    assert!(!dog.id().is_empty(), "Dog must have non-empty id");

    // 2. Evaluate a simple stimulus
    let stimulus = Stimulus {
        content: "The Earth orbits the Sun, based on centuries of astronomical observation.".into(),
        context: None,
        domain: Some("science".into()),
    };

    let result = dog.evaluate(&stimulus).await;
    match result {
        Ok(scores) => {
            // Scores must be in [0.0, 1.0]
            assert!(scores.fidelity >= 0.0 && scores.fidelity <= 1.0,
                "fidelity {} out of range", scores.fidelity);
            assert!(scores.phi >= 0.0 && scores.phi <= 1.0,
                "phi {} out of range", scores.phi);
            assert!(scores.verify >= 0.0 && scores.verify <= 1.0,
                "verify {} out of range", scores.verify);
        }
        Err(DogError::ApiError(_)) => {} // acceptable
        Err(DogError::ParseError(_)) => {} // acceptable
        Err(DogError::RateLimited(_)) => {} // acceptable
        Err(DogError::Timeout) => {} // acceptable
    }
}

#[tokio::test]
async fn deterministic_dog_passes_contract() {
    dog_contract(&DeterministicDog).await;
}

#[tokio::test]
async fn inference_dog_with_mock_passes_contract() {
    let mock = Arc::new(MockChatBackend::new(
        "mock",
        r#"{"fidelity": 0.7, "phi": 0.6, "verify": 0.5, "fidelity_reason": "r1", "phi_reason": "r2", "verify_reason": "r3"}"#,
    ));
    let dog = InferenceDog::new(mock, "mock-dog".into());
    dog_contract(&dog).await;
}
```

- [ ] **Step 2: Commit**

```bash
git add tests/dog_contract.rs
git commit -m "test(contract): Dog trait contract tests — DeterministicDog + InferenceDog(mock) verified"
```

---

### Task 11: ChatPort contract tests

**Files:**
- Create: `tests/chat_port_contract.rs`

- [ ] **Step 1: Write contract test suite**

```rust
//! ChatPort trait contract tests — any ChatPort implementation must pass these.

use cynic_kernel::chat_port::*;
use cynic_kernel::backend::BackendStatus;

async fn chat_port_contract(port: &dyn ChatPort) {
    // 1. Name must be non-empty
    assert!(!port.name().is_empty(), "ChatPort must have non-empty name");

    // 2. Health must return a valid BackendStatus
    let health = port.health().await;
    match health {
        BackendStatus::Unknown | BackendStatus::Healthy
        | BackendStatus::Degraded { .. } | BackendStatus::Critical
        | BackendStatus::Recovering => {} // all valid
    }

    // 3. Chat must return Ok or a well-formed error
    let result = port.chat("You are a test.", "Say hello.").await;
    match result {
        Ok(text) => assert!(!text.is_empty(), "Successful chat should return non-empty text"),
        Err(ChatError::Unreachable(_)) => {}
        Err(ChatError::Timeout { .. }) => {}
        Err(ChatError::RateLimited(_)) => {}
        Err(ChatError::Protocol(_)) => {}
    }
}

#[tokio::test]
async fn mock_chat_passes_contract() {
    let mock = MockChatBackend::new("test-mock", "Hello!");
    chat_port_contract(&mock).await;
}
```

- [ ] **Step 2: Commit**

```bash
git add tests/chat_port_contract.rs
git commit -m "test(contract): ChatPort trait contract tests — MockChatBackend verified"
```

---

## Chunk 6: Health Reconciliation + backends.toml Deploy

### Task 12: Reconcile 5-state health in CLAUDE.md

**Files:**
- Modify: `CLAUDE.md`

- [ ] **Step 1: Update CLAUDE.md health axiom**

Change:
```
- **3-state health:** HEALTHY / DEGRADED / CRITICAL. Never boolean. Circuit breaker per backend.
```
To:
```
- **5-state health:** UNKNOWN → HEALTHY → DEGRADED → CRITICAL → RECOVERING. Never boolean. Circuit breaker per backend. UNKNOWN at boot (epistemic honesty). RECOVERING = half-open probe.
```

- [ ] **Step 2: Commit**

```bash
git add CLAUDE.md
git commit -m "docs(constitution): reconcile health states — 5-state lifecycle matches circuit breaker reality"
```

---

### Task 12b: Probe refactor (P0) — inject GpuDetector trait

**Files:**
- Create: `src/probe/mod.rs` (orchestration)
- Create: `src/probe/gpu.rs` (GpuDetector trait — domain core)
- Create: `src/probe/gpu_linux.rs` (SysfsDetector adapter)
- Create: `src/probe/system.rs` (hardware info, env detection)
- Create: `src/probe/models.rs` (GGUF + Ollama discovery)
- Create: `src/probe/servers.rs` (inference server discovery)
- Create: `src/probe/advisor.rs` (SovereigntyAdvisor — pure logic)
- Create: `src/probe/persistence.rs` (load/save NodeConfig TOML)
- Delete: `src/probe.rs` (monolith replaced by module tree)

Per CYNIC-ARCHITECTURE-TRUTHS.md: probe.rs is 897 LOC, 12 responsibilities. Target: 6+ focused modules, zero `#[cfg]` in domain code.

- [ ] **Step 1: Create GpuDetector trait (domain core)**

```rust
// src/probe/gpu.rs
//! GpuDetector — domain trait for GPU discovery.
//! Adapters implement per-platform detection. Domain core has ZERO #[cfg] gates.

use crate::probe::ComputeBackend;

#[derive(Debug, Clone)]
pub struct DetectedGpu {
    pub name: String,
    pub vram_gb: f64,
    pub backend: ComputeBackend,
    pub is_igpu: bool,
}

pub trait GpuDetector: Send + Sync {
    fn name(&self) -> &str;
    fn detect(&self) -> Vec<DetectedGpu>;
}
```

- [ ] **Step 2: Create probe/mod.rs orchestrator**

Move `NodeConfig`, `HardwareInfo`, `ComputeInfo`, `ComputeBackend`, `LlmConfig`, `GgufModel`, `EnvInfo` structs from `probe.rs` into `probe/mod.rs`. The `run()` function takes `Vec<Box<dyn GpuDetector>>` as parameter:

```rust
// src/probe/mod.rs
pub mod gpu;
pub mod gpu_linux;
pub mod system;
pub mod models;
pub mod servers;
pub mod advisor;
pub mod persistence;

// Re-export domain types
pub use gpu::{GpuDetector, DetectedGpu};

/// Run the full probe with injected GPU detectors.
/// Composition root provides platform-appropriate detectors.
pub async fn run(gpu_detectors: Vec<Box<dyn GpuDetector>>, data_dir: &std::path::Path) -> NodeConfig {
    // 1. Check cache
    if let Some(cached) = persistence::load_cached(data_dir) {
        return cached;
    }

    // 2. System info
    let hardware = system::detect_hardware();
    let env = system::detect_env();

    // 3. GPU detection via injected trait impls
    let mut best_gpu: Option<DetectedGpu> = None;
    for detector in &gpu_detectors {
        let gpus = detector.detect();
        for gpu in gpus {
            if best_gpu.as_ref().map_or(true, |b| gpu.vram_gb > b.vram_gb) {
                best_gpu = Some(gpu);
            }
        }
    }

    let compute = match best_gpu {
        Some(gpu) => ComputeInfo {
            backend: gpu.backend,
            gpu_name: gpu.name,
            vram_gb: gpu.vram_gb,
            cpu_threads: hardware.cpu_cores,
            avx2: system::has_avx2(),
            is_igpu: gpu.is_igpu,
        },
        None => ComputeInfo {
            backend: ComputeBackend::Cpu,
            cpu_threads: hardware.cpu_cores,
            avx2: system::has_avx2(),
            ..Default::default()
        },
    };

    // 4. Model discovery
    let llm = models::discover_models(data_dir, &compute);

    // 5. Sovereignty advisor
    let suggestions = advisor::advise(&hardware, &compute, &llm);

    let config = NodeConfig {
        probed_at: chrono::Utc::now().to_rfc3339(),
        hardware,
        compute,
        llm,
        env,
        suggestions,
    };

    // 6. Cache result
    persistence::save_cached(data_dir, &config);
    config
}
```

- [ ] **Step 3: Extract remaining modules from probe.rs**

Split the existing `probe.rs` logic into:
- `probe/system.rs` — `detect_hardware()`, `detect_env()`, `has_avx2()`
- `probe/models.rs` — `discover_models()`, GGUF scanning, Ollama integration
- `probe/servers.rs` — `detect_running_servers()`, HTTP probing
- `probe/advisor.rs` — `advise()` pure function
- `probe/persistence.rs` — `load_cached()`, `save_cached()` TOML IO
- `probe/gpu_linux.rs` — `SysfsDetector` implementing `GpuDetector`

Each module takes dependencies as parameters (no global state, no `#[cfg]` in domain modules).

- [ ] **Step 4: Update lib.rs**

Change `pub mod probe;` — Rust treats `probe/mod.rs` as the module root. No lib.rs change needed if the module name stays `probe`.

- [ ] **Step 5: Update main.rs to inject GpuDetectors**

```rust
// In main.rs boot sequence, replace direct probe::run() with:
let gpu_detectors: Vec<Box<dyn probe::GpuDetector>> = vec![
    #[cfg(target_os = "linux")]
    Box::new(probe::gpu_linux::SysfsDetector),
    // Future: Box::new(probe::gpu_windows::WmiDetector),
];
let node_config = probe::run(gpu_detectors, &data_dir).await;
```

Note: `#[cfg]` gates are ONLY in `main.rs` (composition root), never in domain modules.

- [ ] **Step 6: Verify existing tests pass**

Run `cargo test -p cynic-kernel` to verify the refactor preserves behavior.

- [ ] **Step 7: Commit**

```bash
git add src/probe/ src/lib.rs src/main.rs
git rm src/probe.rs
git commit -m "refactor(probe): split monolith into modules, inject GpuDetector trait — zero #[cfg] in domain"
```

---

### Task 13: Deploy backends.toml on forge

**Files:**
- Create on forge: `~/.config/cynic/backends.toml`

- [ ] **Step 1: Create backends.toml via Tailscale MCP**

Use `ts_exec` to create the file:
```bash
cat > ~/.config/cynic/backends.toml << 'EOF'
[backend.gemini]
base_url = "https://generativelanguage.googleapis.com/v1beta/openai"
api_key_env = "GEMINI_API_KEY"
model = "gemini-2.5-flash"
auth_style = "bearer"
EOF
```

- [ ] **Step 2: Verify env file has GEMINI_API_KEY**

Use `ts_exec`: `grep GEMINI ~/.config/cynic/env`

- [ ] **Step 3: Push code, verify deployment via ts_service**

```bash
git push origin main
# Wait for CI, then:
# ts_service node=forge service=cynic-kernel user=true
# Verify: "InferenceDog 'gemini' loaded"
```

---

## Chunk 7: Infrastructure — llama.cpp on Forge

### Task 14: Install llama.cpp + model on forge

- [ ] **Step 1: Install build dependencies**

Via `ts_exec`:
```bash
sudo apt-get update && sudo apt-get install -y cmake build-essential
```

- [ ] **Step 2: Clone and build llama.cpp**

```bash
cd ~ && git clone https://github.com/ggerganov/llama.cpp && cd llama.cpp && cmake -B build -DCMAKE_BUILD_TYPE=Release && cmake --build build -j2
```

- [ ] **Step 3: Download model**

```bash
pip install huggingface-hub && huggingface-cli download microsoft/Phi-3-mini-4k-instruct-gguf Phi-3-mini-4k-instruct-q4.gguf --local-dir ~/models/
```

Note: if `huggingface-cli` is not available, download directly:
```bash
mkdir -p ~/models && cd ~/models && wget "https://huggingface.co/microsoft/Phi-3-mini-4k-instruct-gguf/resolve/main/Phi-3-mini-4k-instruct-q4.gguf"
```

- [ ] **Step 4: Create systemd service**

```bash
mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/llama-server.service << 'EOF'
[Unit]
Description=llama.cpp inference server
After=network.target

[Service]
Type=simple
ExecStart=/home/kairos/llama.cpp/build/bin/llama-server --model /home/kairos/models/Phi-3-mini-4k-instruct-q4.gguf --port 8080 --ctx-size 2048 --threads 2
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF
systemctl --user daemon-reload
systemctl --user enable llama-server
systemctl --user start llama-server
```

- [ ] **Step 5: Add local backend to backends.toml**

```bash
cat >> ~/.config/cynic/backends.toml << 'EOF'

[backend.local]
base_url = "http://localhost:8080/v1"
model = "phi-3-mini"
auth_style = "none"
EOF
```

- [ ] **Step 6: Restart cynic-kernel and verify 3 Dogs**

```bash
systemctl --user restart cynic-kernel
```
Verify logs show: DeterministicDog + gemini InferenceDog + local InferenceDog = 3 Dogs.

---

## Chunk 8: Infrastructure — Cloudflare Tunnel

### Task 15: Install and run cloudflared

- [ ] **Step 1: Install cloudflared**

```bash
curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 -o ~/bin/cloudflared && chmod +x ~/bin/cloudflared
```

- [ ] **Step 2: Create systemd service for tunnel**

```bash
cat > ~/.config/systemd/user/cloudflared.service << 'EOF'
[Unit]
Description=Cloudflare Tunnel to CYNIC REST API
After=cynic-kernel.service

[Service]
Type=simple
ExecStart=/home/kairos/bin/cloudflared tunnel --url http://localhost:3030
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
EOF
systemctl --user daemon-reload
systemctl --user enable cloudflared
systemctl --user start cloudflared
```

- [ ] **Step 3: Get public URL from logs**

```bash
journalctl --user -u cloudflared -n 20 --no-pager | grep "trycloudflare.com"
```

---

## Chunk 9: Frontend

### Task 16: Create frontend files

**Files:**
- Create: `static/index.html`
- Create: `static/verdict.js`
- Create: `static/radar.js`
- Create: `static/hypercube.js`

This task uses `frontend-design` skill. The frontend must:
- Dark theme with φ-golden proportions
- Form to submit stimulus → POST `/judge`
- Verdict card with Q-Score bar and reasoning
- Per-Dog scores breakdown with timeline animation
- Canvas radar chart (3 axes: FIDELITY, PHI, VERIFY)
- Three.js hypercube (3D axiom-space, Dogs as spheres, anomaly highlight)

- [ ] **Step 1: Create static/ directory**

```bash
mkdir -p cynic-kernel/static
```

- [ ] **Step 2: Implement index.html, verdict.js, radar.js, hypercube.js**

Use `frontend-design` skill to create production-grade vanilla HTML/JS/CSS.

- [ ] **Step 3: Commit**

```bash
git add static/
git commit -m "feat(frontend): verdict UI with radar chart + 3D axiom-space hypercube"
```

---

## Chunk 10: Final Integration + Repo Cleanup

### Task 17: Integration test — full flow

- [ ] **Step 1: Push all code to forge**

```bash
git push origin main
```

- [ ] **Step 2: Verify via Tailscale MCP**

Use `ts_poll` to wait for build completion. Then:
```bash
curl -s -X POST http://localhost:3030/judge -H 'Content-Type: application/json' \
  -d '{"content": "Testing the full flow with multiple Dogs"}' | python3 -m json.tool
```

Verify response contains:
- `dog_scores` array with entries for each Dog
- `anomaly_detected` field
- `max_disagreement` field

- [ ] **Step 3: Test static frontend**

Open `http://<forge-ip>:3030/` in browser or use `ts_exec` with `curl http://localhost:3030/` to verify HTML is served.

### Task 18: Repo cleanup

- [ ] **Step 1: Update README for hackathon**

Add architecture diagram, usage examples, tool descriptions.

- [ ] **Step 2: Push to GitHub**

```bash
git push github main
```

- [ ] **Step 3: Final verification**

Use `cynic-judge` skill to evaluate the completed work against industrial standards.

---

## Execution Order Summary

```
Phase 1 (parallel):
  Task 1  — DogError::Timeout + DogScore + Verdict extension
  Task 2  — ChatPort trait
  Task 3  — StoragePort trait
  Task 4  — config.rs

Phase 2 (sequential, depends on Phase 1):
  Task 5  — OpenAiCompatBackend
  Task 6  — InferenceDog

Phase 3 (sequential, depends on Phase 2):
  Task 7  — Judge parallel + residual detection
  Task 8  — REST refactor + static serving
  Task 9  — main.rs boot sequence rewrite

Phase 4 (parallel):
  Task 10  — Dog contract tests
  Task 11  — ChatPort contract tests
  Task 12  — Health reconciliation
  Task 12b — Probe refactor (P0) — GpuDetector trait injection
  Task 13  — Deploy backends.toml

Phase 5 (parallel, independent):
  Task 14 — llama.cpp + model on forge
  Task 15 — Cloudflare tunnel

Phase 6 (sequential, depends on all above):
  Task 16 — Frontend
  Task 17 — Integration test
  Task 18 — Repo cleanup
```
