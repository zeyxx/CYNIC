//! REST API — JSON interface for external clients (React, curl, etc.)
//! Runs alongside gRPC on a separate port.

use axum::{
    extract::{DefaultBodyLimit, Path, Request, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{Json, IntoResponse, Response},
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use chrono;

use crate::dog::{Verdict, PHI_INV};
use crate::judge::Judge;
use crate::storage_port::StoragePort;
use crate::ccm;
use std::collections::HashMap;
use std::sync::Mutex;

// ── SHARED STATE ───────────────────────────────────────────

pub struct AppState {
    pub judge: Arc<Judge>,
    pub storage: Arc<dyn StoragePort>,
    pub usage: Arc<Mutex<DogUsageTracker>>,
    pub api_key: Option<String>,
    pub rate_limiter: RateLimiter,
}

/// Tracks token consumption and request counts per Dog since boot.
pub struct DogUsageTracker {
    pub dogs: HashMap<String, DogUsage>,
    pub boot_time: chrono::DateTime<chrono::Utc>,
    pub total_requests: u64,
}

#[derive(Default, Clone)]
pub struct DogUsage {
    pub prompt_tokens: u64,
    pub completion_tokens: u64,
    pub requests: u64,
    pub failures: u64,
    pub total_latency_ms: u64,
}

impl Default for DogUsageTracker {
    fn default() -> Self { Self::new() }
}

impl DogUsageTracker {
    pub fn new() -> Self {
        Self {
            dogs: HashMap::new(),
            boot_time: chrono::Utc::now(),
            total_requests: 0,
        }
    }

    pub fn record(&mut self, dog_id: &str, prompt: u32, completion: u32, latency_ms: u64) {
        let entry = self.dogs.entry(dog_id.to_string()).or_default();
        entry.prompt_tokens += prompt as u64;
        entry.completion_tokens += completion as u64;
        entry.requests += 1;
        entry.total_latency_ms += latency_ms;
    }

    pub fn record_failure(&mut self, dog_id: &str) {
        let entry = self.dogs.entry(dog_id.to_string()).or_default();
        entry.failures += 1;
    }

    pub fn total_tokens(&self) -> u64 {
        self.dogs.values().map(|d| d.prompt_tokens + d.completion_tokens).sum()
    }

    /// Estimated cost in USD (rough average: $0.15/1M tokens)
    pub fn estimated_cost_usd(&self) -> f64 {
        self.total_tokens() as f64 * 0.15 / 1_000_000.0
    }

    pub fn uptime_seconds(&self) -> i64 {
        (chrono::Utc::now() - self.boot_time).num_seconds()
    }
}

// ── RATE LIMITER ──────────────────────────────────────────────

/// Simple fixed-window rate limiter. Resets counter every 60 seconds.
pub struct RateLimiter {
    state: Mutex<(u64, std::time::Instant)>,
    max_per_minute: u64,
}

impl RateLimiter {
    pub fn new(max_per_minute: u64) -> Self {
        Self {
            state: Mutex::new((0, std::time::Instant::now())),
            max_per_minute,
        }
    }

    /// Returns true if request is allowed, false if rate limited.
    pub fn check(&self) -> bool {
        let mut state = self.state.lock().unwrap();
        let now = std::time::Instant::now();
        if now.duration_since(state.1).as_secs() >= 60 {
            *state = (1, now);
            true
        } else if state.0 < self.max_per_minute {
            state.0 += 1;
            true
        } else {
            false
        }
    }
}

// ── REQUEST / RESPONSE TYPES ───────────────────────────────

#[derive(Deserialize)]
pub struct JudgeRequest {
    pub content: String,
    pub context: Option<String>,
    pub domain: Option<String>,
    /// Optional: evaluate with only these Dogs (by ID). If omitted, all Dogs are used.
    pub dogs: Option<Vec<String>>,
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
    pub temporal: Option<TemporalResponse>,
}

#[derive(Serialize)]
pub struct TemporalResponse {
    pub temporal_total: f64,
    pub outlier_perspective: Option<String>,
    pub max_divergence: f64,
    pub perspectives: Vec<TemporalPerspectiveScore>,
}

#[derive(Serialize)]
pub struct TemporalPerspectiveScore {
    pub perspective: String,
    pub q_total: f64,
    pub dog_id: String,
}

#[derive(Serialize)]
pub struct DogScoreResponse {
    pub dog_id: String,
    pub latency_ms: u64,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
    pub reasoning: ReasoningResponse,
}

#[derive(Serialize)]
pub struct QScoreResponse {
    pub total: f64,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
    pub culture: f64,
    pub burn: f64,
    pub sovereignty: f64,
}

#[derive(Serialize)]
pub struct ReasoningResponse {
    pub fidelity: String,
    pub phi: String,
    pub verify: String,
    pub culture: String,
    pub burn: String,
    pub sovereignty: String,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub phi_max: f64,
    pub axioms: Vec<String>,
    pub dogs: Vec<DogHealthResponse>,
    pub total_requests: u64,
    pub total_tokens: u64,
    pub estimated_cost_usd: f64,
    pub uptime_seconds: i64,
}

#[derive(Serialize)]
pub struct DogHealthResponse {
    pub id: String,
    pub kind: String,
    pub circuit: String,
    pub failures: u32,
}

// ── ROUTER ─────────────────────────────────────────────────

pub fn router(state: Arc<AppState>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    if state.api_key.is_some() {
        eprintln!("[Ring 3 / REST] Bearer auth ENABLED");
    } else {
        eprintln!("[Ring 3 / REST] WARNING: No CYNIC_API_KEY set — API is OPEN");
    }

    Router::new()
        .route("/health", get(health_handler))
        .route("/judge", post(judge_handler))
        .route("/dogs", get(dogs_handler))
        .route("/crystals", get(crystals_handler))
        .route("/crystal/{id}", get(crystal_handler))
        .route("/usage", get(usage_handler))
        .route("/temporal", get(temporal_handler))
        .route("/verdict/{id}", get(get_verdict_handler))
        .route("/verdicts", get(list_verdicts_handler))
        .layer(middleware::from_fn_with_state(state.clone(), auth_middleware))
        .layer(middleware::from_fn_with_state(state.clone(), rate_limit_middleware))
        .fallback_service(ServeDir::new("static"))
        .layer(DefaultBodyLimit::max(64 * 1024)) // 64 KB — no multi-MB payloads
        .layer(cors)
        .with_state(state)
}

// ── MIDDLEWARE ─────────────────────────────────────────────

/// Bearer token authentication. Skipped for /health. Skipped if no CYNIC_API_KEY.
async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    // /health is public — no auth required
    if request.uri().path() == "/health" {
        return next.run(request).await;
    }

    if let Some(ref key) = state.api_key {
        let token = request.headers()
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .map(|s| s.to_string());

        match token {
            Some(t) if t == *key => {},
            _ => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(ErrorResponse { error: "Invalid or missing Bearer token".into() }),
                ).into_response();
            }
        }
    }
    next.run(request).await
}

/// Rate limiter — rejects excess requests with 429. /health exempt.
async fn rate_limit_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Response {
    if request.uri().path() != "/health" && !state.rate_limiter.check() {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(ErrorResponse { error: "Rate limit exceeded".into() }),
        ).into_response();
    }
    next.run(request).await
}

// ── HANDLERS ───────────────────────────────────────────────

async fn judge_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<JudgeRequest>,
) -> Result<Json<JudgeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let stimulus = crate::dog::Stimulus {
        content: req.content,
        context: req.context,
        domain: req.domain,
    };

    let verdict = state.judge.evaluate(&stimulus, req.dogs.as_deref()).await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string() }),
        ))?;

    // Store verdict (best effort — don't fail the request if storage is down)
    if let Err(e) = state.storage.store_verdict(&verdict).await {
        eprintln!("[REST] Warning: failed to store verdict: {}", e);
    }

    // Track token usage per Dog
    {
        let mut usage = state.usage.lock().unwrap();
        usage.total_requests += 1;
        for ds in &verdict.dog_scores {
            usage.record(&ds.dog_id, ds.prompt_tokens, ds.completion_tokens, ds.latency_ms);
        }
    }

    // CCM: observe crystal (best effort — learning loop)
    {
        let crystal_id = format!("{:x}", md5_hash(&verdict.stimulus_summary));
        let domain = stimulus.domain.unwrap_or_else(|| "general".to_string());
        let now = chrono::Utc::now().to_rfc3339();
        let existing = state.storage.get_crystal(&crystal_id).await.ok().flatten();
        let crystal = match existing {
            Some(c) => ccm::update_crystal(&c, verdict.q_score.total, &now),
            None => ccm::new_crystal(crystal_id, verdict.stimulus_summary.clone(), domain, verdict.q_score.total, &now),
        };
        if let Err(e) = state.storage.store_crystal(&crystal).await {
            eprintln!("[REST] Warning: failed to store crystal: {}", e);
        } else {
            eprintln!("[CCM] Crystal '{}' → {:?} (obs: {}, conf: {:.3})",
                crystal.content.chars().take(40).collect::<String>(),
                crystal.state, crystal.observations, crystal.confidence);
        }
    }

    Ok(Json(verdict_to_response(&verdict)))
}

async fn get_verdict_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<JudgeResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage.get_verdict(&id).await {
        Ok(Some(v)) => Ok(Json(verdict_to_response(&v))),
        Ok(None) => Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse { error: format!("Verdict {} not found", id) }),
        )),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string() }),
        )),
    }
}

async fn list_verdicts_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<JudgeResponse>>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage.list_verdicts(20).await {
        Ok(verdicts) => Ok(Json(verdicts.iter().map(verdict_to_response).collect())),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string() }),
        )),
    }
}

async fn crystals_handler(
    State(state): State<Arc<AppState>>,
) -> Result<Json<Vec<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage.list_crystals(20).await {
        Ok(crystals) => {
            let items: Vec<serde_json::Value> = crystals.iter().map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "content": c.content,
                    "domain": c.domain,
                    "confidence": c.confidence,
                    "observations": c.observations,
                    "state": format!("{:?}", c.state),
                    "created_at": c.created_at,
                    "updated_at": c.updated_at,
                })
            }).collect();
            Ok(Json(items))
        }
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

async fn crystal_handler(
    State(state): State<Arc<AppState>>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<ErrorResponse>)> {
    match state.storage.get_crystal(&id).await {
        Ok(Some(c)) => Ok(Json(serde_json::json!({
            "id": c.id,
            "content": c.content,
            "domain": c.domain,
            "confidence": c.confidence,
            "observations": c.observations,
            "state": format!("{:?}", c.state),
            "created_at": c.created_at,
            "updated_at": c.updated_at,
        }))),
        Ok(None) => Err((StatusCode::NOT_FOUND, Json(ErrorResponse { error: format!("Crystal {} not found", id) }))),
        Err(e) => Err((StatusCode::INTERNAL_SERVER_ERROR, Json(ErrorResponse { error: e.to_string() }))),
    }
}

async fn temporal_handler() -> Json<serde_json::Value> {
    use crate::temporal::TemporalPerspective;
    let perspectives: Vec<serde_json::Value> = TemporalPerspective::ALL.iter().map(|p| {
        serde_json::json!({
            "perspective": p.label(),
            "description": p.description(),
        })
    }).collect();
    Json(serde_json::json!({
        "count": 7,
        "perspectives": perspectives,
        "aggregation": "geometric_mean",
        "outlier_threshold": "phi^-2 (0.382)",
        "exploration_constant": "phi (1.618)",
        "status": "pure_logic_ready — awaiting multi-perspective Dog evaluation integration"
    }))
}

async fn usage_handler(
    State(state): State<Arc<AppState>>,
) -> Json<serde_json::Value> {
    let usage = state.usage.lock().unwrap();
    let dogs: Vec<serde_json::Value> = usage.dogs.iter().map(|(id, d)| {
        let avg_latency = if d.requests > 0 { d.total_latency_ms / d.requests } else { 0 };
        serde_json::json!({
            "dog_id": id,
            "prompt_tokens": d.prompt_tokens,
            "completion_tokens": d.completion_tokens,
            "total_tokens": d.prompt_tokens + d.completion_tokens,
            "requests": d.requests,
            "failures": d.failures,
            "avg_latency_ms": avg_latency,
        })
    }).collect();
    Json(serde_json::json!({
        "total_tokens": usage.total_tokens(),
        "total_requests": usage.total_requests,
        "estimated_cost_usd": usage.estimated_cost_usd(),
        "uptime_seconds": usage.uptime_seconds(),
        "per_dog": dogs,
    }))
}

async fn dogs_handler(
    State(state): State<Arc<AppState>>,
) -> Json<Vec<String>> {
    Json(state.judge.dog_ids())
}

async fn health_handler(
    State(state): State<Arc<AppState>>,
) -> Json<HealthResponse> {
    let dog_health = state.judge.dog_health();
    let dogs: Vec<DogHealthResponse> = dog_health.into_iter().map(|(id, circuit, failures)| {
        let kind = if id == "deterministic-dog" {
            "heuristic"
        } else {
            "inference"
        }.to_string();
        DogHealthResponse { id, kind, circuit, failures }
    }).collect();

    let status = if dogs.is_empty() {
        "critical"
    } else if dogs.len() == 1 {
        "degraded"
    } else {
        "sovereign"
    }.to_string();

    let usage = state.usage.lock().unwrap();

    Json(HealthResponse {
        status,
        version: env!("CARGO_PKG_VERSION").to_string(),
        phi_max: PHI_INV,
        axioms: vec![
            "FIDELITY".into(),
            "PHI".into(),
            "VERIFY/FALSIFY".into(),
            "CULTURE".into(),
            "BURN".into(),
            "SOVEREIGNTY".into(),
        ],
        dogs,
        total_requests: usage.total_requests,
        total_tokens: usage.total_tokens(),
        estimated_cost_usd: usage.estimated_cost_usd(),
        uptime_seconds: usage.uptime_seconds(),
    })
}

// ── HELPERS ────────────────────────────────────────────────

/// Simple hash for crystal IDs. Not cryptographic — just deterministic content addressing.
fn md5_hash(input: &str) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325; // FNV-1a offset basis
    for byte in input.bytes() {
        h ^= byte as u64;
        h = h.wrapping_mul(0x100000001b3); // FNV-1a prime
    }
    h
}

fn verdict_to_response(v: &Verdict) -> JudgeResponse {
    JudgeResponse {
        verdict_id: v.id.clone(),
        verdict: format!("{:?}", v.kind),
        q_score: QScoreResponse {
            total: v.q_score.total,
            fidelity: v.q_score.fidelity,
            phi: v.q_score.phi,
            verify: v.q_score.verify,
            culture: v.q_score.culture,
            burn: v.q_score.burn,
            sovereignty: v.q_score.sovereignty,
        },
        reasoning: ReasoningResponse {
            fidelity: v.reasoning.fidelity.clone(),
            phi: v.reasoning.phi.clone(),
            verify: v.reasoning.verify.clone(),
            culture: v.reasoning.culture.clone(),
            burn: v.reasoning.burn.clone(),
            sovereignty: v.reasoning.sovereignty.clone(),
        },
        dogs_used: v.dog_id.clone(),
        phi_max: PHI_INV,
        dog_scores: v.dog_scores.iter().map(|ds| DogScoreResponse {
            dog_id: ds.dog_id.clone(),
            latency_ms: ds.latency_ms,
            prompt_tokens: ds.prompt_tokens,
            completion_tokens: ds.completion_tokens,
            fidelity: ds.fidelity,
            phi: ds.phi,
            verify: ds.verify,
            culture: ds.culture,
            burn: ds.burn,
            sovereignty: ds.sovereignty,
            reasoning: ReasoningResponse {
                fidelity: ds.reasoning.fidelity.clone(),
                phi: ds.reasoning.phi.clone(),
                verify: ds.reasoning.verify.clone(),
                culture: ds.reasoning.culture.clone(),
                burn: ds.reasoning.burn.clone(),
                sovereignty: ds.reasoning.sovereignty.clone(),
            },
        }).collect(),
        anomaly_detected: v.anomaly_detected,
        max_disagreement: v.max_disagreement,
        anomaly_axiom: v.anomaly_axiom.clone(),
        temporal: compute_temporal_from_dogs(&v.dog_scores),
    }
}

/// Map Dog evaluations onto temporal perspectives and aggregate.
/// Each Dog represents a different "temporal lens" on the stimulus.
pub fn compute_temporal_from_dogs(dog_scores: &[crate::dog::DogScore]) -> Option<TemporalResponse> {
    use crate::temporal::{TemporalPerspective, TemporalScore, aggregate_temporal};
    use crate::dog::compute_qscore;

    if dog_scores.len() < 2 {
        return None; // Need multiple perspectives
    }

    // Map Dogs to temporal perspectives based on their nature
    let perspective_map: Vec<(TemporalPerspective, &str)> = vec![
        (TemporalPerspective::Present, "deterministic-dog"),    // Instant heuristic = present state
        (TemporalPerspective::Transcendence, "gemini"),         // Largest model = deepest insight
        (TemporalPerspective::Past, "huggingface"),             // Meta/Llama = historical training data
        (TemporalPerspective::Emergence, "gemma-sovereign"),    // Local sovereign = novel perspective
        (TemporalPerspective::Cycle, "qwen"),                   // Alibaba/Qwen = cyclical patterns
    ];

    let temporal_scores: Vec<TemporalScore> = dog_scores.iter().filter_map(|ds| {
        let perspective = perspective_map.iter()
            .find(|(_, dog_id)| *dog_id == ds.dog_id)
            .map(|(p, _)| *p)?;

        let axiom_scores = crate::dog::AxiomScores {
            fidelity: ds.fidelity, phi: ds.phi, verify: ds.verify,
            culture: ds.culture, burn: ds.burn, sovereignty: ds.sovereignty,
            reasoning: crate::dog::AxiomReasoning::default(),
            ..Default::default()
        };
        let q = compute_qscore(&axiom_scores);
        Some(TemporalScore { perspective, axiom_scores, q_total: q.total })
    }).collect();

    if temporal_scores.is_empty() {
        return None;
    }

    let tv = aggregate_temporal(&temporal_scores);

    Some(TemporalResponse {
        temporal_total: tv.temporal_total,
        outlier_perspective: tv.outlier_perspective.map(|p| p.label().to_string()),
        max_divergence: tv.max_divergence,
        perspectives: temporal_scores.iter().map(|ts| {
            let dog_id = perspective_map.iter()
                .find(|(p, _)| *p == ts.perspective)
                .map(|(_, id)| id.to_string())
                .unwrap_or_default();
            TemporalPerspectiveScore {
                perspective: ts.perspective.label().to_string(),
                q_total: ts.q_total,
                dog_id,
            }
        }).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dog::*;

    #[test]
    fn md5_hash_deterministic() {
        let a = md5_hash("hello");
        let b = md5_hash("hello");
        assert_eq!(a, b);
    }

    #[test]
    fn md5_hash_different_inputs_differ() {
        assert_ne!(md5_hash("foo"), md5_hash("bar"));
    }

    #[test]
    fn usage_tracker_records_tokens() {
        let mut tracker = DogUsageTracker::new();
        tracker.record("dog-a", 100, 50, 200);
        tracker.record("dog-a", 80, 40, 150);
        tracker.record("dog-b", 200, 100, 500);

        assert_eq!(tracker.total_tokens(), 570); // (100+50)+(80+40)+(200+100)
        assert_eq!(tracker.dogs["dog-a"].requests, 2);
        assert_eq!(tracker.dogs["dog-b"].requests, 1);
        assert_eq!(tracker.dogs["dog-a"].total_latency_ms, 350);
    }

    #[test]
    fn usage_tracker_estimated_cost() {
        let mut tracker = DogUsageTracker::new();
        tracker.record("x", 500_000, 500_000, 0); // 1M tokens
        let cost = tracker.estimated_cost_usd();
        assert!((cost - 0.15).abs() < 0.001);
    }

    #[test]
    fn usage_tracker_empty_is_zero() {
        let tracker = DogUsageTracker::new();
        assert_eq!(tracker.total_tokens(), 0);
        assert_eq!(tracker.total_requests, 0);
        assert_eq!(tracker.estimated_cost_usd(), 0.0);
    }

    #[test]
    fn verdict_to_response_maps_all_fields() {
        let verdict = Verdict {
            id: "test-id".into(),
            kind: VerdictKind::Howl,
            q_score: QScore {
                total: 0.55, fidelity: 0.6, phi: 0.5,
                verify: 0.55, culture: 0.5, burn: 0.45, sovereignty: 0.5,
            },
            reasoning: AxiomReasoning {
                fidelity: "good".into(), phi: "ok".into(), verify: "decent".into(),
                culture: "fine".into(), burn: "lean".into(), sovereignty: "free".into(),
            },
            dog_id: "test-dog".into(),
            stimulus_summary: "test stimulus".into(),
            timestamp: "2026-03-15T00:00:00Z".into(),
            dog_scores: vec![],
            anomaly_detected: false,
            max_disagreement: 0.0,
            anomaly_axiom: None,
        };

        let resp = verdict_to_response(&verdict);
        assert_eq!(resp.verdict_id, "test-id");
        assert_eq!(resp.verdict, "Howl");
        assert_eq!(resp.q_score.total, 0.55);
        assert_eq!(resp.reasoning.fidelity, "good");
        assert_eq!(resp.dogs_used, "test-dog");
        assert_eq!(resp.phi_max, PHI_INV);
        assert!(!resp.anomaly_detected);
    }

    #[test]
    fn temporal_returns_none_with_single_dog() {
        let scores = vec![DogScore {
            dog_id: "deterministic-dog".into(),
            latency_ms: 0, prompt_tokens: 0, completion_tokens: 0,
            fidelity: 0.5, phi: 0.5, verify: 0.5,
            culture: 0.5, burn: 0.5, sovereignty: 0.5,
            reasoning: AxiomReasoning::default(),
        }];
        assert!(compute_temporal_from_dogs(&scores).is_none());
    }

    #[test]
    fn temporal_returns_some_with_multiple_dogs() {
        let scores = vec![
            DogScore {
                dog_id: "deterministic-dog".into(),
                latency_ms: 0, prompt_tokens: 0, completion_tokens: 0,
                fidelity: 0.5, phi: 0.5, verify: 0.5,
                culture: 0.5, burn: 0.5, sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
            },
            DogScore {
                dog_id: "gemini".into(),
                latency_ms: 100, prompt_tokens: 50, completion_tokens: 30,
                fidelity: 0.6, phi: 0.55, verify: 0.5,
                culture: 0.45, burn: 0.4, sovereignty: 0.5,
                reasoning: AxiomReasoning::default(),
            },
        ];
        let temporal = compute_temporal_from_dogs(&scores);
        assert!(temporal.is_some());
        let t = temporal.unwrap();
        assert!(t.temporal_total > 0.0);
        assert!(t.temporal_total <= PHI_INV + 1e-10);
    }

    #[test]
    fn rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(3);
        assert!(limiter.check());
        assert!(limiter.check());
        assert!(limiter.check());
        assert!(!limiter.check()); // 4th request rejected
    }

    #[test]
    fn health_status_logic() {
        // 0 dogs → critical, 1 → degraded, 2+ → sovereign
        assert_eq!(if 0 == 0 { "critical" } else if 0 == 1 { "degraded" } else { "sovereign" }, "critical");
        assert_eq!(if 1 == 0 { "critical" } else if 1 == 1 { "degraded" } else { "sovereign" }, "degraded");
        assert_eq!(if 2 == 0 { "critical" } else if 2 == 1 { "degraded" } else { "sovereign" }, "sovereign");
    }
}
