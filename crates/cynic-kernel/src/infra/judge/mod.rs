//! Runtime judgment use-case.
//!
//! The REST layer only adapts JSON to these typed inputs. Backend-specific
//! inference remains behind the OpenAI-compatible adapter and every result is
//! converted into kernel-owned `DogScore`/`Verdict` evidence.

mod parser;
mod prompt;

use crate::domain::dog::{
    AxiomReasoning, AxiomScores, DogError, DogScore, QScore, Stimulus, Verdict, VerdictKind,
    compute_qscore, validate_scores, verdict_kind,
};
use crate::infra::config::BackendConfig;
use crate::infra::dog_performance::{DogObservation, DogPerformanceCollector};
use crate::infra::routing_calc::RoutingCalculator;
use async_trait::async_trait;
use cynic_inference::adapters::inference::openai_compat::OpenAiCompatBackend;
use cynic_inference::domain::chat::{ChatError, InferenceProfile};
use cynic_inference::ports::chat_port::ChatPort;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct JudgeInput {
    pub content: String,
    pub context: Option<String>,
    pub domain: String,
    pub dogs: Option<Vec<String>>,
    pub request_id: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum JudgeError {
    #[error("no Dogs available for domain {0}")]
    NoDogs(String),
    #[error("all Dogs failed: {0}")]
    AllDogsFailed(String),
}

#[async_trait]
trait Evaluator: Send + Sync {
    fn id(&self) -> &str;
    async fn evaluate(
        &self,
        stimulus: &Stimulus,
        request_id: Option<&str>,
    ) -> Result<AxiomScores, DogError>;
}

struct DeterministicEvaluator;

#[async_trait]
impl Evaluator for DeterministicEvaluator {
    fn id(&self) -> &str {
        "deterministic-dog"
    }

    async fn evaluate(
        &self,
        stimulus: &Stimulus,
        _request_id: Option<&str>,
    ) -> Result<AxiomScores, DogError> {
        let content = stimulus.content.to_lowercase();
        let adversarial = [
            "ignore all previous",
            "[system]",
            "vaccines cause autism",
            "earth is flat",
        ]
        .iter()
        .any(|marker| content.contains(marker));
        let base = if adversarial { 0.12 } else { 0.52 };
        Ok(AxiomScores {
            fidelity: base,
            phi: (base + 0.03).min(0.618),
            verify: if adversarial { 0.10 } else { 0.50 },
            culture: base,
            burn: (base + 0.01).min(0.618),
            sovereignty: 0.55,
            reasoning: AxiomReasoning {
                fidelity: "deterministic content and adversarial-marker observation".into(),
                phi: "bounded deterministic structural score".into(),
                verify: "deterministic verification heuristic".into(),
                culture: "no external context assumed".into(),
                burn: "minimal local evaluator".into(),
                sovereignty: "executed in kernel process".into(),
            },
            ..AxiomScores::default()
        })
    }
}

struct InferenceEvaluator {
    id: String,
    backend: OpenAiCompatBackend,
}

#[async_trait]
impl Evaluator for InferenceEvaluator {
    fn id(&self) -> &str {
        &self.id
    }

    async fn evaluate(
        &self,
        stimulus: &Stimulus,
        request_id: Option<&str>,
    ) -> Result<AxiomScores, DogError> {
        let response = self
            .backend
            .chat(
                prompt::SYSTEM,
                &prompt::user(stimulus),
                InferenceProfile::SCORING,
                request_id,
            )
            .await
            .map_err(chat_error)?;
        let mut scores = parser::parse(&response.text)?;
        scores.reasoning_trace = response.reasoning;
        scores.prompt_tokens = response.prompt_tokens;
        scores.completion_tokens = response.completion_tokens;
        scores.thinking_tokens = response.thinking_tokens;
        validate_scores(&scores)?;
        Ok(scores)
    }
}

fn chat_error(error: ChatError) -> DogError {
    match error {
        ChatError::Timeout { .. } => DogError::Timeout,
        ChatError::RateLimited(message) => DogError::RateLimited(message),
        ChatError::Unreachable(message) | ChatError::Protocol(message) => {
            DogError::ApiError(message)
        }
    }
}

fn evaluator_for(id: &str, configs: &[BackendConfig]) -> Result<Box<dyn Evaluator>, DogError> {
    if id == "deterministic-dog" {
        return Ok(Box::new(DeterministicEvaluator));
    }
    let config = configs
        .iter()
        .find(|config| config.name == id)
        .ok_or_else(|| {
            DogError::ApiError(format!("Dog {id} is not declared in backend configuration"))
        })?;
    if !matches!(
        config.backend_type,
        crate::infra::config::BackendType::OpenAi
    ) {
        return Err(DogError::ApiError(format!(
            "Dog {id} has unsupported CLI adapter in judge"
        )));
    }
    let backend = OpenAiCompatBackend::new(config.to_inference_backend_config())
        .map_err(|error| DogError::ApiError(error.to_string()))?;
    Ok(Box::new(InferenceEvaluator {
        id: id.to_string(),
        backend,
    }))
}

pub async fn evaluate(
    input: JudgeInput,
    configs: &[BackendConfig],
    domain_router: &crate::infra::domain_router::DomainRouter,
    routing: &RoutingCalculator,
    performance: &DogPerformanceCollector,
) -> Result<Verdict, JudgeError> {
    let selected = input.dogs.unwrap_or_else(|| {
        let routed = routing.dogs_for_domain(&input.domain, 200);
        if routed.is_empty() {
            domain_router.dogs_for_domain(&input.domain)
        } else {
            routed
        }
    });
    if selected.is_empty() {
        return Err(JudgeError::NoDogs(input.domain));
    }

    let stimulus = Stimulus {
        content: input.content.clone(),
        context: input.context.clone(),
        domain: Some(input.domain.clone()),
        request_id: input.request_id.clone(),
    };
    let mut scores = Vec::new();
    let mut failed_dogs = Vec::new();
    let mut failed_dog_errors = BTreeMap::new();
    let mut failed_dog_error_kinds = BTreeMap::new();
    let mut excluded_dogs = Vec::new();

    for dog_id in selected {
        let evaluator = match evaluator_for(&dog_id, configs) {
            Ok(evaluator) => evaluator,
            Err(error) => {
                excluded_dogs.push(dog_id.clone());
                failed_dog_errors.insert(dog_id.clone(), error.to_string());
                failed_dog_error_kinds.insert(dog_id.clone(), "configuration_error".into());
                continue;
            }
        };
        let started = std::time::Instant::now();
        match evaluator
            .evaluate(&stimulus, input.request_id.as_deref())
            .await
        {
            Ok(raw) => {
                let latency = started.elapsed().as_millis() as u64;
                let score_total = compute_qscore(&raw).total;
                performance.observe_result(
                    &input.domain,
                    evaluator.id(),
                    DogObservation::success(latency, Some(score_total)),
                );
                scores.push(to_dog_score(evaluator.id(), latency, raw));
            }
            Err(error) => {
                let latency = started.elapsed().as_millis() as u64;
                performance.observe_result(
                    &input.domain,
                    evaluator.id(),
                    DogObservation::failure(latency, error_kind(&error)),
                );
                failed_dogs.push(evaluator.id().to_string());
                failed_dog_errors.insert(evaluator.id().to_string(), error.to_string());
                failed_dog_error_kinds.insert(evaluator.id().to_string(), error_kind(&error));
            }
        }
    }

    if scores.is_empty() {
        return Err(JudgeError::AllDogsFailed(format!(
            "failed={failed_dogs:?}, excluded={excluded_dogs:?}"
        )));
    }
    let averaged = average_scores(&scores);
    let q_score = compute_qscore(&averaged);
    let kind = verdict_kind(q_score.total);
    let (max_disagreement, anomaly_axiom) = disagreement(&scores);
    let timestamp = chrono::Utc::now().timestamp_millis();
    let mut verdict = Verdict {
        id: crate::infra::crypto::generate_secure_id(),
        domain: input.domain,
        kind,
        q_score,
        reasoning: averaged.reasoning.clone(),
        dog_id: scores
            .iter()
            .map(|score| score.dog_id.clone())
            .collect::<Vec<_>>()
            .join(","),
        stimulus_summary: input.content.chars().take(240).collect(),
        timestamp,
        voter_count: scores.len(),
        dog_scores: scores,
        anomaly_detected: max_disagreement > crate::domain::dog::PHI_INV2,
        max_disagreement,
        anomaly_axiom,
        failed_dogs,
        failed_dog_errors,
        failed_dog_error_kinds,
        excluded_dogs,
        ..empty_verdict_fields()
    };
    let proof = serde_json::json!({ "domain": verdict.domain, "q_score": verdict.q_score, "dog_scores": verdict.dog_scores, "timestamp": timestamp });
    verdict.integrity_hash = Some(
        blake3::hash(&serde_json::to_vec(&proof).unwrap_or_default())
            .to_hex()
            .to_string(),
    );
    Ok(verdict)
}

fn disagreement(scores: &[DogScore]) -> (f64, Option<String>) {
    if scores.len() < 2 {
        return (0.0, None);
    }
    let fields: [(&str, fn(&DogScore) -> f64); 6] = [
        ("fidelity", |score| score.fidelity),
        ("phi", |score| score.phi),
        ("verify", |score| score.verify),
        ("culture", |score| score.culture),
        ("burn", |score| score.burn),
        ("sovereignty", |score| score.sovereignty),
    ];
    fields
        .into_iter()
        .map(|(name, field)| {
            let (min, max) = scores
                .iter()
                .map(field)
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(min, max), value| {
                    (min.min(value), max.max(value))
                });
            (max - min, name.to_string())
        })
        .max_by(|left, right| left.0.total_cmp(&right.0))
        .map(|(spread, name)| (spread, Some(name)))
        .unwrap_or((0.0, None))
}

fn empty_verdict_fields() -> Verdict {
    Verdict {
        id: String::new(),
        domain: String::new(),
        kind: VerdictKind::Bark,
        q_score: QScore {
            total: 0.0,
            fidelity: 0.0,
            phi: 0.0,
            verify: 0.0,
            culture: 0.0,
            burn: 0.0,
            sovereignty: 0.0,
        },
        reasoning: AxiomReasoning::default(),
        dog_id: String::new(),
        stimulus_summary: String::new(),
        timestamp: 0,
        dog_scores: Vec::new(),
        anomaly_detected: false,
        max_disagreement: 0.0,
        anomaly_axiom: None,
        voter_count: 0,
        failed_dogs: Vec::new(),
        failed_dog_errors: BTreeMap::new(),
        failed_dog_error_kinds: BTreeMap::new(),
        excluded_dogs: Vec::new(),
        target: None,
        integrity_hash: None,
        prev_hash: None,
    }
}

fn to_dog_score(id: &str, latency_ms: u64, raw: AxiomScores) -> DogScore {
    let bounded = compute_qscore(&raw);
    DogScore {
        dog_id: id.into(),
        latency_ms,
        prompt_tokens: raw.prompt_tokens,
        completion_tokens: raw.completion_tokens,
        fidelity: bounded.fidelity,
        phi: bounded.phi,
        verify: bounded.verify,
        culture: bounded.culture,
        burn: bounded.burn,
        sovereignty: bounded.sovereignty,
        raw_fidelity: raw.fidelity,
        raw_phi: raw.phi,
        raw_verify: raw.verify,
        raw_culture: raw.culture,
        raw_burn: raw.burn,
        raw_sovereignty: raw.sovereignty,
        reasoning: raw.reasoning,
        reasoning_trace: raw.reasoning_trace,
        abstentions: raw.abstentions,
    }
}

fn average_scores(scores: &[DogScore]) -> AxiomScores {
    let n = scores.len() as f64;
    let avg = |field: fn(&DogScore) -> f64| scores.iter().map(field).sum::<f64>() / n;
    AxiomScores {
        fidelity: avg(|score| score.raw_fidelity),
        phi: avg(|score| score.raw_phi),
        verify: avg(|score| score.raw_verify),
        culture: avg(|score| score.raw_culture),
        burn: avg(|score| score.raw_burn),
        sovereignty: avg(|score| score.raw_sovereignty),
        reasoning: scores
            .first()
            .map(|score| score.reasoning.clone())
            .unwrap_or_default(),
        ..AxiomScores::default()
    }
}

fn error_kind(error: &DogError) -> String {
    match error {
        DogError::Timeout => "timeout",
        DogError::RateLimited(_) => "rate_limited",
        DogError::ApiError(_) => "api_error",
        DogError::ParseError(_) => "parse_error",
        DogError::SlotUnavailable => "slot_unavailable",
        _ => "evaluation_error",
    }
    .into()
}
