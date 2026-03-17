# Hackathon MVP — CYNIC Judgment Engine

> **For agentic workers:** REQUIRED: Use superpowers:subagent-driven-development (if subagents available) or superpowers:executing-plans to implement this plan. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a working CYNIC judgment engine that takes any stimulus, evaluates it through 3 axioms via Gemini, returns a phi-bounded verdict — and stores the result. Expose via REST API for S.'s chess React app.

**Architecture:** REST (axum) + existing gRPC kernel. A `Dog` trait defines the model-agnostic evaluator interface. `GeminiDog` calls Google AI Studio free tier (Gemini 2.5 Flash). Axiom scores get phi-bounded server-side. Verdicts stored in SurrealDB. One closed loop: stimulus → evaluate → verdict → store → retrievable.

**Tech Stack:** Rust, axum (REST), reqwest (Gemini API), serde_json, existing SurrealDB + tonic gRPC. Google AI Studio free tier (Gemini 2.5 Flash: 10 RPM, 250 req/day, zero cost).

**Hackathon:** Gemini 3 Paris — March 14, 2026. ~36 hours from plan creation.

---

## File Structure

```
cynic-kernel/src/
├── main.rs              (MODIFY — add axum REST server alongside gRPC)
├── dog.rs               (CREATE — Dog trait + axiom types + phi-bounding)
├── gemini_dog.rs         (CREATE — GeminiDog: calls Gemini API, parses axiom scores)
├── deterministic_dog.rs  (CREATE — rule-based Dog, proves mixed intelligence)
├── judge.rs             (CREATE — orchestrates Dogs, computes Q-Score, emits Verdict)
├── rest.rs              (CREATE — axum REST handlers: POST /judge, GET /verdict/:id)
├── backend.rs           (existing — InferencePort, unchanged)
├── storage.rs           (MODIFY — add verdict storage/retrieval)
├── probe.rs             (existing — unchanged)
├── hal.rs               (existing — unchanged)
├── pulse.rs             (existing — unchanged)
├── supervisor.rs        (existing — unchanged)
Cargo.toml               (MODIFY — add axum, uuid deps)
```

---

## Chunk 1: Dog Trait + Phi-Bounding + Axiom Types

### Task 1: Core Domain Types — `dog.rs`

**Files:**
- Create: `cynic-kernel/src/dog.rs`

This is the heart of CYNIC. The Dog trait is the model-agnostic interface. Any intelligence source (Gemini, Llama, GPT, deterministic code) implements this trait.

- [ ] **Step 1: Create `dog.rs` with domain types and Dog trait**

```rust
//! Dog — the model-agnostic evaluator interface.
//! Any intelligence source implements this trait.
//! The Dog receives a Stimulus, returns AxiomScores.
//! The kernel phi-bounds and aggregates — the Dog never self-caps.

use serde::{Deserialize, Serialize};

// ── PHI CONSTANTS ──────────────────────────────────────────
pub const PHI: f64 = 1.618_033_988_749_895;
pub const PHI_INV: f64 = 0.618_033_988_749_895; // max confidence
pub const PHI_INV2: f64 = 0.381_966_011_250_105; // anomaly threshold

// ── STIMULUS ───────────────────────────────────────────────
/// What the organism perceives. Domain-agnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stimulus {
    /// What to evaluate (chess move, code, statement, anything)
    pub content: String,
    /// Optional context (board state, file contents, conversation)
    pub context: Option<String>,
    /// Domain hint for weight adjustment (e.g. "chess", "code", "geopolitics")
    pub domain: Option<String>,
}

// ── AXIOM SCORES ───────────────────────────────────────────
/// Raw scores from a Dog. NOT phi-bounded — the kernel does that.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AxiomScores {
    /// Truth loyalty: is this faithful to reality?
    pub fidelity: f64,
    /// Structural harmony: is this well-proportioned?
    pub phi: f64,
    /// Evidence + falsifiability: is this verifiable/falsifiable?
    pub verify: f64,
    /// Optional reasoning per axiom
    pub reasoning: AxiomReasoning,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AxiomReasoning {
    pub fidelity: String,
    pub phi: String,
    pub verify: String,
}

// ── PHI-BOUNDED Q-SCORE ────────────────────────────────────
/// The kernel's final score. Every value capped at phi^-1 (0.618).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QScore {
    pub total: f64,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
}

// ── VERDICT ────────────────────────────────────────────────
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerdictKind {
    Howl,   // >= 0.82 raw (phi-bounded: high conviction)
    Wag,    // >= 0.618 raw (positive)
    Growl,  // >= 0.382 raw (cautious)
    Bark,   // < 0.382 (rejection / insufficient confidence)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Verdict {
    pub id: String,
    pub kind: VerdictKind,
    pub q_score: QScore,
    pub reasoning: AxiomReasoning,
    pub dog_id: String,
    pub stimulus_summary: String,
    pub timestamp: String,
}

// ── PHI-BOUNDING ───────────────────────────────────────────
/// Clamp a raw score to [0.0, phi^-1]. This is non-negotiable.
pub fn phi_bound(raw: f64) -> f64 {
    raw.clamp(0.0, PHI_INV)
}

/// Compute phi-bounded Q-Score from raw axiom scores.
/// Uses geometric mean — one weak axiom drags everything down.
pub fn compute_qscore(raw: &AxiomScores) -> QScore {
    let f = phi_bound(raw.fidelity);
    let p = phi_bound(raw.phi);
    let v = phi_bound(raw.verify);

    // Geometric mean of 3 axioms, then phi-bound the result
    let geo = (f * p * v).powf(1.0 / 3.0);
    let total = phi_bound(geo);

    QScore { total, fidelity: f, phi: p, verify: v }
}

/// Determine verdict from Q-Score total
pub fn verdict_kind(total: f64) -> VerdictKind {
    // These thresholds are on the phi-bounded scale (0..0.618)
    // Map: HOWL > 0.528 (φ⁻²+φ⁻⁴ golden subdivision), WAG > 0.382, GROWL > 0.236, BARK below
    if total > PHI_INV2 + PHI_INV4 { VerdictKind::Howl }
    else if total > PHI_INV2 { VerdictKind::Wag }
    else if total > PHI_INV2 * PHI_INV { VerdictKind::Growl }
    else { VerdictKind::Bark }
}

// ── DOG TRAIT ──────────────────────────────────────────────
/// The contract every evaluator must fulfill.
/// Model-agnostic: Gemini, Llama, GPT, deterministic code — all identical to the caller.
#[async_trait::async_trait]
pub trait Dog: Send + Sync {
    /// Unique identifier for this Dog
    fn id(&self) -> &str;

    /// Evaluate a stimulus, return raw axiom scores (NOT phi-bounded)
    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError>;
}

#[derive(Debug)]
pub enum DogError {
    /// Model API returned an error
    ApiError(String),
    /// Response couldn't be parsed into axiom scores
    ParseError(String),
    /// Rate limited or timeout
    RateLimited(String),
}

impl std::fmt::Display for DogError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ApiError(msg) => write!(f, "Dog API error: {}", msg),
            Self::ParseError(msg) => write!(f, "Dog parse error: {}", msg),
            Self::RateLimited(msg) => write!(f, "Dog rate limited: {}", msg),
        }
    }
}
```

- [ ] **Step 2: Register module in `main.rs`**

Add `pub mod dog;` to the module declarations in `main.rs`.

- [ ] **Step 3: Add `async-trait` dependency to `Cargo.toml`**

Add under `[dependencies]`:
```toml
async-trait = "0.1"
```

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p cynic-kernel`
Expected: compiles with no errors

- [ ] **Step 5: Write unit tests for phi-bounding**

Add to bottom of `dog.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn phi_bound_clamps_high_values() {
        assert!((phi_bound(0.95) - PHI_INV).abs() < 1e-10);
        assert!((phi_bound(1.0) - PHI_INV).abs() < 1e-10);
    }

    #[test]
    fn phi_bound_preserves_low_values() {
        assert!((phi_bound(0.3) - 0.3).abs() < 1e-10);
        assert!((phi_bound(0.0) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn phi_bound_clamps_negative() {
        assert!((phi_bound(-0.5) - 0.0).abs() < 1e-10);
    }

    #[test]
    fn qscore_geometric_mean_correct() {
        let raw = AxiomScores {
            fidelity: 0.6,
            phi: 0.5,
            verify: 0.4,
            reasoning: AxiomReasoning::default(),
        };
        let q = compute_qscore(&raw);
        // All values should be <= PHI_INV
        assert!(q.total <= PHI_INV + 1e-10);
        assert!(q.fidelity <= PHI_INV + 1e-10);
        // Geometric mean of (0.6, 0.5, 0.4) ≈ 0.4932
        assert!((q.total - (0.6_f64 * 0.5 * 0.4).powf(1.0/3.0)).abs() < 0.01);
    }

    #[test]
    fn one_weak_axiom_drags_score_down() {
        let strong = AxiomScores {
            fidelity: 0.6, phi: 0.6, verify: 0.6,
            reasoning: AxiomReasoning::default(),
        };
        let weak = AxiomScores {
            fidelity: 0.6, phi: 0.6, verify: 0.1,
            reasoning: AxiomReasoning::default(),
        };
        let q_strong = compute_qscore(&strong);
        let q_weak = compute_qscore(&weak);
        // One weak axiom must significantly reduce total
        assert!(q_weak.total < q_strong.total * 0.7);
    }

    #[test]
    fn verdict_thresholds() {
        assert_eq!(verdict_kind(PHI_INV), VerdictKind::Howl);
        assert_eq!(verdict_kind(0.45), VerdictKind::Wag);
        assert_eq!(verdict_kind(0.25), VerdictKind::Growl);
        assert_eq!(verdict_kind(0.1), VerdictKind::Bark);
    }
}
```

- [ ] **Step 6: Run tests**

Run: `cargo test -p cynic-kernel -- dog::tests`
Expected: all 5 tests pass

- [ ] **Step 7: Commit**

```bash
git add cynic-kernel/src/dog.rs cynic-kernel/src/main.rs cynic-kernel/Cargo.toml
git commit -m "feat(dog): Dog trait, axiom types, phi-bounded Q-Score

The irreducible contract: any intelligence evaluates a Stimulus
through 3 axioms (FIDELITY, PHI, VERIFY/FALSIFY).
Geometric mean + phi-bound (max 0.618) enforced by kernel.
Verdicts: HOWL/WAG/GROWL/BARK."
```

---

## Chunk 2: GeminiDog + DeterministicDog

### Task 2: GeminiDog — Gemini API Integration

**Files:**
- Create: `cynic-kernel/src/gemini_dog.rs`

Calls Google AI Studio free tier. Sends structured prompt asking for axiom scores. Parses JSON response.

- [ ] **Step 1: Create `gemini_dog.rs`**

```rust
//! GeminiDog — calls Google AI Studio (Gemini 2.5 Flash) for axiom evaluation.
//! Free tier: 10 RPM, 250 req/day, zero cost.
//! The Dog returns RAW scores. Phi-bounding happens in the kernel.

use crate::dog::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub struct GeminiDog {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl GeminiDog {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: "gemini-2.5-flash".to_string(),
        }
    }

    pub fn with_model(api_key: String, model: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model,
        }
    }

    fn build_prompt(stimulus: &Stimulus) -> String {
        let context_block = stimulus.context.as_deref().unwrap_or("(no additional context)");
        let domain = stimulus.domain.as_deref().unwrap_or("general");

        format!(r#"You are CYNIC, a sovereign judgment engine. Evaluate this stimulus through 3 axioms.

DOMAIN: {domain}
STIMULUS: {content}
CONTEXT: {context_block}

Score each axiom from 0.0 to 1.0 with honest uncertainty. DO NOT inflate scores.
If you're unsure, score lower. Overconfidence is the enemy.

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
struct GeminiAxiomResponse {
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

// Google AI Studio generateContent request/response structures
#[derive(Serialize)]
struct GenerateContentRequest {
    contents: Vec<Content>,
}

#[derive(Serialize)]
struct Content {
    parts: Vec<Part>,
}

#[derive(Serialize)]
struct Part {
    text: String,
}

#[derive(Deserialize)]
struct GenerateContentResponse {
    candidates: Option<Vec<Candidate>>,
}

#[derive(Deserialize)]
struct Candidate {
    content: Option<CandidateContent>,
}

#[derive(Deserialize)]
struct CandidateContent {
    parts: Option<Vec<CandidatePart>>,
}

#[derive(Deserialize)]
struct CandidatePart {
    text: Option<String>,
}

#[async_trait]
impl Dog for GeminiDog {
    fn id(&self) -> &str {
        "gemini-dog"
    }

    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError> {
        let prompt = Self::build_prompt(stimulus);

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.model, self.api_key
        );

        let body = GenerateContentRequest {
            contents: vec![Content {
                parts: vec![Part { text: prompt }],
            }],
        };

        let resp = self.client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| DogError::ApiError(e.to_string()))?;

        if resp.status() == 429 {
            return Err(DogError::RateLimited("Gemini free tier rate limit hit".into()));
        }

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            return Err(DogError::ApiError(format!("{}: {}", status, text)));
        }

        let gen_resp: GenerateContentResponse = resp.json().await
            .map_err(|e| DogError::ParseError(format!("Failed to parse Gemini response: {}", e)))?;

        let text = gen_resp.candidates
            .and_then(|c| c.into_iter().next())
            .and_then(|c| c.content)
            .and_then(|c| c.parts)
            .and_then(|p| p.into_iter().next())
            .and_then(|p| p.text)
            .ok_or_else(|| DogError::ParseError("Empty Gemini response".into()))?;

        // Extract JSON from response (Gemini sometimes wraps in markdown)
        let json_str = extract_json(&text)
            .ok_or_else(|| DogError::ParseError(format!("No JSON found in: {}", text)))?;

        let parsed: GeminiAxiomResponse = serde_json::from_str(json_str)
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

/// Extract JSON object from text that might contain markdown fences or extra text
fn extract_json(text: &str) -> Option<&str> {
    // Try to find { ... } boundaries
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
    fn extract_json_from_nested() {
        let input = r#"Here is the result: {"fidelity": 0.5, "phi": 0.4, "verify": 0.3, "nested": {"a": 1}} done"#;
        let json = extract_json(input).unwrap();
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
        let prompt = GeminiDog::build_prompt(&stimulus);
        assert!(prompt.contains("e4 e5 Nf3"));
        assert!(prompt.contains("chess"));
        assert!(prompt.contains("FIDELITY"));
    }
}
```

- [ ] **Step 2: Register module in `main.rs`**

Add `pub mod gemini_dog;` to module declarations.

- [ ] **Step 3: Run tests**

Run: `cargo test -p cynic-kernel -- gemini_dog::tests`
Expected: all 4 tests pass

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/gemini_dog.rs cynic-kernel/src/main.rs
git commit -m "feat(dog): GeminiDog — Gemini 2.5 Flash axiom evaluator

Calls Google AI Studio free tier. Structured prompt for 3 axioms.
JSON extraction handles markdown-wrapped responses.
Returns raw scores — kernel phi-bounds."
```

### Task 3: DeterministicDog — Rule-Based Evaluator

**Files:**
- Create: `cynic-kernel/src/deterministic_dog.rs`

Proves mixed intelligence: not every Dog needs an LLM. This Dog uses rules.

- [ ] **Step 1: Create `deterministic_dog.rs`**

```rust
//! DeterministicDog — rule-based axiom evaluator.
//! Proves mixed intelligence: not every Dog needs an LLM.
//! Uses heuristics to score stimuli. Fast, free, deterministic.

use crate::dog::*;
use async_trait::async_trait;

pub struct DeterministicDog;

#[async_trait]
impl Dog for DeterministicDog {
    fn id(&self) -> &str {
        "deterministic-dog"
    }

    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError> {
        let content = &stimulus.content;
        let len = content.len();

        // FIDELITY: penalize vague/short claims, reward specificity
        let fidelity = if len < 10 {
            0.2
        } else if content.contains("always") || content.contains("never") || content.contains("100%") {
            0.15 // Absolute claims are suspect
        } else if content.contains("probably") || content.contains("likely") || content.contains("approximately") {
            0.55 // Epistemic humility rewarded
        } else {
            0.35 // Neutral
        };

        // PHI: structural coherence — length, punctuation, structure
        let phi = if len > 500 {
            0.3 // Overly verbose
        } else if len > 50 && content.contains('.') {
            0.5 // Has structure
        } else if len < 20 {
            0.25 // Too terse for meaningful structure
        } else {
            0.4
        };

        // VERIFY: does it reference evidence or make falsifiable claims?
        let verify = if content.contains("because") || content.contains("evidence")
            || content.contains("data") || content.contains("according to")
        {
            0.5 // References evidence
        } else if content.contains("?") {
            0.45 // Questions are verifiable by nature
        } else if content.contains("I think") || content.contains("I believe") {
            0.3 // Opinions without evidence
        } else {
            0.35
        };

        Ok(AxiomScores {
            fidelity,
            phi,
            verify,
            reasoning: AxiomReasoning {
                fidelity: format!("Heuristic: len={}, absolutes={}", len,
                    content.contains("always") || content.contains("never")),
                phi: format!("Heuristic: len={}, structured={}", len, content.contains('.')),
                verify: format!("Heuristic: evidence_words={}",
                    content.contains("because") || content.contains("evidence")),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn penalizes_absolute_claims() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "This will always work 100% of the time".into(),
            context: None,
            domain: None,
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(scores.fidelity < 0.2);
    }

    #[tokio::test]
    async fn rewards_epistemic_humility() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "This will probably work in most cases, approximately 60% of the time".into(),
            context: None,
            domain: None,
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(scores.fidelity > 0.5);
    }

    #[tokio::test]
    async fn rewards_evidence_references() {
        let dog = DeterministicDog;
        let stimulus = Stimulus {
            content: "According to the data, this approach works because of X".into(),
            context: None,
            domain: None,
        };
        let scores = dog.evaluate(&stimulus).await.unwrap();
        assert!(scores.verify >= 0.5);
    }
}
```

- [ ] **Step 2: Register module in `main.rs`**

Add `pub mod deterministic_dog;` to module declarations.

- [ ] **Step 3: Run tests**

Run: `cargo test -p cynic-kernel -- deterministic_dog::tests`
Expected: all 3 tests pass

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/deterministic_dog.rs cynic-kernel/src/main.rs
git commit -m "feat(dog): DeterministicDog — rule-based evaluator

Proves mixed intelligence: LLM Dogs + deterministic Dogs coexist.
Heuristic scoring: penalizes absolutes, rewards epistemic humility."
```

---

## Chunk 3: Judge + Verdict Storage

### Task 4: Judge — Orchestrates Dogs, Produces Verdicts

**Files:**
- Create: `cynic-kernel/src/judge.rs`

The Judge receives a Stimulus, sends it to all Dogs, aggregates scores, phi-bounds, produces a Verdict. This is the closed loop minus storage (next task).

- [ ] **Step 1: Create `judge.rs`**

```rust
//! Judge — orchestrates Dogs, computes consensus, emits Verdicts.
//! Currently runs Dogs sequentially (MVP). Future: parallel + BFT consensus.

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

    /// Evaluate a stimulus through all Dogs, aggregate, produce Verdict.
    pub async fn evaluate(&self, stimulus: &Stimulus) -> Result<Verdict, JudgeError> {
        if self.dogs.is_empty() {
            return Err(JudgeError::NoDogs);
        }

        let mut all_scores: Vec<(String, AxiomScores)> = Vec::new();
        let mut errors: Vec<String> = Vec::new();

        // MVP: sequential evaluation. Future: tokio::join! for parallel.
        for dog in &self.dogs {
            match dog.evaluate(stimulus).await {
                Ok(scores) => all_scores.push((dog.id().to_string(), scores)),
                Err(e) => errors.push(format!("{}: {}", dog.id(), e)),
            }
        }

        if all_scores.is_empty() {
            return Err(JudgeError::AllDogsFailed(errors));
        }

        // Aggregate: average raw scores across Dogs, then phi-bound
        let n = all_scores.len() as f64;
        let avg_fidelity = all_scores.iter().map(|(_, s)| s.fidelity).sum::<f64>() / n;
        let avg_phi = all_scores.iter().map(|(_, s)| s.phi).sum::<f64>() / n;
        let avg_verify = all_scores.iter().map(|(_, s)| s.verify).sum::<f64>() / n;

        let aggregated = AxiomScores {
            fidelity: avg_fidelity,
            phi: avg_phi,
            verify: avg_verify,
            reasoning: all_scores.last().map(|(_, s)| s.reasoning.clone())
                .unwrap_or_default(),
        };

        let q_score = compute_qscore(&aggregated);
        let kind = verdict_kind(q_score.total);

        let dog_ids: Vec<&str> = all_scores.iter().map(|(id, _)| id.as_str()).collect();

        Ok(Verdict {
            id: Uuid::new_v4().to_string(),
            kind,
            q_score,
            reasoning: aggregated.reasoning,
            dog_id: dog_ids.join("+"),
            stimulus_summary: stimulus.content.chars().take(100).collect(),
            timestamp: Utc::now().to_rfc3339(),
        })
    }
}

#[derive(Debug)]
pub enum JudgeError {
    NoDogs,
    AllDogsFailed(Vec<String>),
}

impl std::fmt::Display for JudgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoDogs => write!(f, "No Dogs configured"),
            Self::AllDogsFailed(errs) => write!(f, "All Dogs failed: {}", errs.join("; ")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // A test Dog that returns fixed scores
    struct FixedDog {
        name: String,
        scores: AxiomScores,
    }

    #[async_trait::async_trait]
    impl Dog for FixedDog {
        fn id(&self) -> &str { &self.name }
        async fn evaluate(&self, _: &Stimulus) -> Result<AxiomScores, DogError> {
            Ok(self.scores.clone())
        }
    }

    struct FailDog;

    #[async_trait::async_trait]
    impl Dog for FailDog {
        fn id(&self) -> &str { "fail-dog" }
        async fn evaluate(&self, _: &Stimulus) -> Result<AxiomScores, DogError> {
            Err(DogError::ApiError("test failure".into()))
        }
    }

    fn test_stimulus() -> Stimulus {
        Stimulus {
            content: "e4 e5 Nf3".into(),
            context: None,
            domain: Some("chess".into()),
        }
    }

    #[tokio::test]
    async fn single_dog_produces_verdict() {
        let judge = Judge::new(vec![
            Box::new(FixedDog {
                name: "test".into(),
                scores: AxiomScores {
                    fidelity: 0.5, phi: 0.5, verify: 0.5,
                    reasoning: AxiomReasoning::default(),
                },
            }),
        ]);

        let verdict = judge.evaluate(&test_stimulus()).await.unwrap();
        assert!(verdict.q_score.total <= PHI_INV + 1e-10);
        assert!(verdict.q_score.total > 0.0);
        assert_eq!(verdict.dog_id, "test");
    }

    #[tokio::test]
    async fn multiple_dogs_averaged() {
        let judge = Judge::new(vec![
            Box::new(FixedDog {
                name: "high".into(),
                scores: AxiomScores {
                    fidelity: 0.8, phi: 0.8, verify: 0.8,
                    reasoning: AxiomReasoning::default(),
                },
            }),
            Box::new(FixedDog {
                name: "low".into(),
                scores: AxiomScores {
                    fidelity: 0.2, phi: 0.2, verify: 0.2,
                    reasoning: AxiomReasoning::default(),
                },
            }),
        ]);

        let verdict = judge.evaluate(&test_stimulus()).await.unwrap();
        assert!(verdict.dog_id.contains("high"));
        assert!(verdict.dog_id.contains("low"));
        // Average should be around 0.5, phi-bounded
        assert!(verdict.q_score.total > 0.3);
        assert!(verdict.q_score.total < 0.55);
    }

    #[tokio::test]
    async fn surviving_dog_still_produces_verdict() {
        let judge = Judge::new(vec![
            Box::new(FailDog),
            Box::new(FixedDog {
                name: "survivor".into(),
                scores: AxiomScores {
                    fidelity: 0.5, phi: 0.5, verify: 0.5,
                    reasoning: AxiomReasoning::default(),
                },
            }),
        ]);

        let verdict = judge.evaluate(&test_stimulus()).await.unwrap();
        assert_eq!(verdict.dog_id, "survivor");
    }

    #[tokio::test]
    async fn all_dogs_fail_returns_error() {
        let judge = Judge::new(vec![Box::new(FailDog)]);
        let result = judge.evaluate(&test_stimulus()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn no_dogs_returns_error() {
        let judge = Judge::new(vec![]);
        let result = judge.evaluate(&test_stimulus()).await;
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Add uuid dependency to `Cargo.toml`**

```toml
uuid = { version = "1", features = ["v4"] }
```

- [ ] **Step 3: Register module in `main.rs`**

Add `pub mod judge;` to module declarations.

- [ ] **Step 4: Run tests**

Run: `cargo test -p cynic-kernel -- judge::tests`
Expected: all 5 tests pass

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/judge.rs cynic-kernel/src/main.rs cynic-kernel/Cargo.toml
git commit -m "feat(judge): Judge orchestrates Dogs, produces phi-bounded Verdicts

Sequential Dog evaluation (MVP). Averages raw scores, phi-bounds.
Geometric mean enforces: one weak axiom drags everything down.
Surviving Dogs produce verdicts even if others fail."
```

### Task 5: Verdict Storage in SurrealDB

**Files:**
- Modify: `cynic-kernel/src/storage.rs`

- [ ] **Step 1: Add verdict storage methods to `CynicStorage`**

Add these methods to the `impl CynicStorage` block in `storage.rs`:

```rust
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
            .bind(("vid", &verdict.id))
            .bind(("kind", format!("{:?}", verdict.kind)))
            .bind(("total", verdict.q_score.total))
            .bind(("fidelity", verdict.q_score.fidelity))
            .bind(("phi", verdict.q_score.phi))
            .bind(("verify", verdict.q_score.verify))
            .bind(("rf", &verdict.reasoning.fidelity))
            .bind(("rp", &verdict.reasoning.phi))
            .bind(("rv", &verdict.reasoning.verify))
            .bind(("did", &verdict.dog_id))
            .bind(("stim", &verdict.stimulus_summary))
            .bind(("ts", &verdict.timestamp))
            .await?;
        Ok(())
    }

    pub async fn get_verdict(&self, verdict_id: &str) -> Result<Option<crate::dog::Verdict>, Box<dyn std::error::Error>> {
        let sql = "SELECT * FROM verdict WHERE verdict_id = $vid LIMIT 1";
        let mut resp = self.db.query(sql)
            .bind(("vid", verdict_id))
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
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p cynic-kernel`
Expected: compiles

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/src/storage.rs
git commit -m "feat(storage): verdict storage + retrieval in SurrealDB

Store/get/list verdicts. Closes the loop: evaluate → store → retrieve."
```

---

## Chunk 4: REST API + Boot Wiring

### Task 6: REST API — axum Handlers

**Files:**
- Create: `cynic-kernel/src/rest.rs`
- Modify: `cynic-kernel/Cargo.toml`

S.'s React app needs a JSON API. Two endpoints:
- `POST /judge` — submit stimulus, get verdict
- `GET /verdict/:id` — retrieve a stored verdict
- `GET /verdicts` — list recent verdicts
- `GET /health` — health check

- [ ] **Step 1: Add axum + tower dependencies to `Cargo.toml`**

```toml
axum = "0.8"
tower-http = { version = "0.6", features = ["cors"] }
uuid = { version = "1", features = ["v4"] }
```

- [ ] **Step 2: Create `rest.rs`**

```rust
//! REST API — JSON interface for external clients (React, curl, etc.)
//! Runs alongside gRPC on a separate port.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::dog::{Stimulus, Verdict, QScore, VerdictKind, PHI_INV};
use crate::judge::Judge;
use crate::storage::CynicStorage;

// ── SHARED STATE ───────────────────────────────────────────

pub struct AppState {
    pub judge: Judge,
    pub storage: Arc<CynicStorage>,
}

// ── REQUEST / RESPONSE TYPES ───────────────────────────────

#[derive(Deserialize)]
pub struct JudgeRequest {
    pub content: String,
    pub context: Option<String>,
    pub domain: Option<String>,
}

#[derive(Serialize)]
pub struct JudgeResponse {
    pub verdict_id: String,
    pub verdict: String,
    pub q_score: QScoreResponse,
    pub reasoning: ReasoningResponse,
    pub dogs_used: String,
    pub phi_max: f64,
}

#[derive(Serialize)]
pub struct QScoreResponse {
    pub total: f64,
    pub fidelity: f64,
    pub phi: f64,
    pub verify: f64,
}

#[derive(Serialize)]
pub struct ReasoningResponse {
    pub fidelity: String,
    pub phi: String,
    pub verify: String,
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
}

// ── ROUTER ─────────────────────────────────────────────────

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
        .layer(cors)
        .with_state(state)
}

// ── HANDLERS ───────────────────────────────────────────────

async fn judge_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<JudgeRequest>,
) -> Result<Json<JudgeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let stimulus = Stimulus {
        content: req.content,
        context: req.context,
        domain: req.domain,
    };

    let verdict = state.judge.evaluate(&stimulus).await
        .map_err(|e| (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse { error: e.to_string() }),
        ))?;

    // Store verdict (best effort — don't fail the request if storage is down)
    if let Err(e) = state.storage.store_verdict(&verdict).await {
        eprintln!("[REST] Warning: failed to store verdict: {}", e);
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

async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "sovereign".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        phi_max: PHI_INV,
        axioms: vec![
            "FIDELITY".into(),
            "PHI".into(),
            "VERIFY/FALSIFY".into(),
        ],
    })
}

// ── HELPERS ────────────────────────────────────────────────

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
    }
}
```

- [ ] **Step 3: Register module in `main.rs`**

Add `pub mod rest;` to module declarations.

- [ ] **Step 4: Verify it compiles**

Run: `cargo check -p cynic-kernel`
Expected: compiles

- [ ] **Step 5: Commit**

```bash
git add cynic-kernel/src/rest.rs cynic-kernel/src/main.rs cynic-kernel/Cargo.toml
git commit -m "feat(rest): axum REST API — POST /judge, GET /verdict/:id

JSON interface for React/external clients alongside gRPC.
CORS enabled for S.'s chess app integration.
Stimulus in, phi-bounded verdict out. Domain-agnostic."
```

### Task 7: Wire Everything in `main.rs`

**Files:**
- Modify: `cynic-kernel/src/main.rs`

Boot sequence: probe → storage → build Dogs → build Judge → start REST + gRPC.

- [ ] **Step 1: Rewrite `main.rs` to wire Dogs + Judge + REST**

Replace the entire `main()` function (keep module declarations and VascularService):

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("╔══════════════════════════════════════╗");
    println!("║       CYNIC OS V2 — SOVEREIGN BOOT    ║");
    println!("╚══════════════════════════════════════╝");

    // ─── RING 0: Omniscience & Probing ────────────────────────
    let force_reprobe = std::env::args().any(|a| a == "--reset");
    let node_config = probe::run(force_reprobe).await;

    println!("[Ring 0] Omniscience Active. Reality Mapped.");
    println!("[Ring 0] Host: {} | Compute: {:?} | VRAM: {}GB",
        std::env::consts::OS,
        node_config.compute.backend,
        node_config.compute.vram_gb
    );

    // ─── RING 1: Native Storage Client (UAL) ──────────────────
    let storage = Arc::new(storage::CynicStorage::init().await?);

    // ─── RING 2: Build Dogs (model-agnostic evaluators) ───────
    let mut dogs: Vec<Box<dyn dog::Dog>> = Vec::new();

    // Always add the deterministic Dog (free, fast)
    dogs.push(Box::new(deterministic_dog::DeterministicDog));
    println!("[Ring 2] DeterministicDog loaded");

    // Add GeminiDog if API key is available
    if let Ok(api_key) = std::env::var("GEMINI_API_KEY") {
        let model = std::env::var("GEMINI_MODEL")
            .unwrap_or_else(|_| "gemini-2.5-flash".to_string());
        dogs.push(Box::new(gemini_dog::GeminiDog::with_model(api_key, model)));
        println!("[Ring 2] GeminiDog loaded (model: {})",
            std::env::var("GEMINI_MODEL").unwrap_or_else(|_| "gemini-2.5-flash".into()));
    } else {
        println!("[Ring 2] GEMINI_API_KEY not set — running deterministic-only mode");
    }

    println!("[Ring 2] {} Dog(s) active", dogs.len());

    // ─── RING 2: Build Judge ──────────────────────────────────
    let judge = judge::Judge::new(dogs);

    // ─── RING 3: REST API (for React/external clients) ────────
    let rest_state = Arc::new(rest::AppState {
        judge,
        storage: Arc::clone(&storage),
    });
    let rest_app = rest::router(rest_state);
    let rest_addr = "0.0.0.0:3000";

    println!("[Ring 3] REST API on http://{}", rest_addr);

    let rest_listener = tokio::net::TcpListener::bind(rest_addr).await?;
    let rest_server = tokio::spawn(async move {
        axum::serve(rest_listener, rest_app).await.unwrap();
    });

    // ─── RING 1: Vascular System (gRPC IPC) ──────────────────
    let grpc_addr = "[::1]:50051".parse()?;
    println!("[Ring 1] Vascular Law enforced on {}", grpc_addr);

    let pulse_service = pulse::PulseService::default();
    let muscle_service = hal::MuscleService::new(Arc::clone(&storage));
    let cognitive_service = storage::CognitiveMemoryService::new(Arc::clone(&storage));

    let grpc_server = Server::builder()
        .add_service(VascularSystemServer::new(VascularService::default()))
        .add_service(KPulseServer::new(pulse_service))
        .add_service(MuscleHalServer::new(muscle_service))
        .add_service(CognitiveMemoryServer::new(cognitive_service))
        .serve(grpc_addr);

    println!("╔══════════════════════════════════════╗");
    println!("║   CYNIC SOVEREIGN — ALL SYSTEMS GO   ║");
    println!("║   REST: http://{}          ║", rest_addr);
    println!("║   gRPC: {}                ║", grpc_addr);
    println!("║   Max confidence: φ⁻¹ = 0.618        ║");
    println!("╚══════════════════════════════════════╝");

    // Run both servers concurrently
    tokio::select! {
        _ = rest_server => eprintln!("[FATAL] REST server stopped"),
        r = grpc_server => {
            if let Err(e) = r {
                eprintln!("[FATAL] gRPC server error: {}", e);
            }
        }
    }

    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check -p cynic-kernel`
Expected: compiles

- [ ] **Step 3: Test the full boot locally (no SurrealDB needed for compile check)**

Run: `cargo build -p cynic-kernel`
Expected: builds successfully

- [ ] **Step 4: Commit**

```bash
git add cynic-kernel/src/main.rs
git commit -m "feat(boot): wire Dogs + Judge + REST into boot sequence

Ring 0: Probe → Ring 1: Storage + gRPC → Ring 2: Dogs + Judge → Ring 3: REST
GeminiDog loads if GEMINI_API_KEY set, falls back to deterministic-only.
REST on 0.0.0.0:3000, gRPC on [::1]:50051."
```

---

## Chunk 5: Integration Test + Demo Script

### Task 8: End-to-End Test

**Files:**
- Create: `cynic-kernel/tests/integration_judge.rs`

- [ ] **Step 1: Create integration test**

```rust
//! Integration test: Stimulus → Judge → Verdict → Storage round-trip
//! Uses DeterministicDog only (no API key needed for CI).

use cynic_kernel::dog::*;
use cynic_kernel::deterministic_dog::DeterministicDog;
use cynic_kernel::judge::Judge;

#[tokio::test]
async fn deterministic_dog_produces_valid_verdict() {
    let judge = Judge::new(vec![Box::new(DeterministicDog)]);

    let stimulus = Stimulus {
        content: "According to the data, this approach probably works because of evidence from X".into(),
        context: Some("Testing epistemic humility".into()),
        domain: Some("general".into()),
    };

    let verdict = judge.evaluate(&stimulus).await.unwrap();

    // Q-Score must be phi-bounded
    assert!(verdict.q_score.total <= PHI_INV + 1e-10);
    assert!(verdict.q_score.fidelity <= PHI_INV + 1e-10);
    assert!(verdict.q_score.phi <= PHI_INV + 1e-10);
    assert!(verdict.q_score.verify <= PHI_INV + 1e-10);

    // Must have a valid verdict kind
    assert!(matches!(verdict.kind, VerdictKind::Howl | VerdictKind::Wag | VerdictKind::Growl | VerdictKind::Bark));

    // Must have reasoning
    assert!(!verdict.reasoning.fidelity.is_empty());

    // ID must be a valid UUID
    assert_eq!(verdict.id.len(), 36);

    println!("Verdict: {:?} | Q-Score: {:.3} | F:{:.3} Φ:{:.3} V:{:.3}",
        verdict.kind, verdict.q_score.total,
        verdict.q_score.fidelity, verdict.q_score.phi, verdict.q_score.verify);
}

#[tokio::test]
async fn absolute_claim_scores_lower() {
    let judge = Judge::new(vec![Box::new(DeterministicDog)]);

    let humble = Stimulus {
        content: "This probably works in most cases according to the data".into(),
        context: None, domain: None,
    };
    let absolute = Stimulus {
        content: "This always works 100% guaranteed never fails".into(),
        context: None, domain: None,
    };

    let v_humble = judge.evaluate(&humble).await.unwrap();
    let v_absolute = judge.evaluate(&absolute).await.unwrap();

    assert!(v_humble.q_score.total > v_absolute.q_score.total,
        "Humble ({:.3}) should score higher than absolute ({:.3})",
        v_humble.q_score.total, v_absolute.q_score.total);
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test -p cynic-kernel --test integration_judge`
Expected: both tests pass

- [ ] **Step 3: Commit**

```bash
git add cynic-kernel/tests/integration_judge.rs
git commit -m "test: integration test — stimulus → judge → verdict round-trip

Validates phi-bounding, geometric mean, verdict classification.
Epistemic humility scores higher than absolute claims."
```

### Task 9: Demo Script

**Files:**
- Create: `scripts/demo.sh`

Quick demo for the hackathon: boot the kernel, hit the REST API with curl.

- [ ] **Step 1: Create `scripts/demo.sh`**

```bash
#!/bin/bash
set -euo pipefail

echo "╔══════════════════════════════════════╗"
echo "║       CYNIC DEMO — Hackathon MVP     ║"
echo "╚══════════════════════════════════════╝"

API="http://localhost:3000"

echo ""
echo "=== Health Check ==="
curl -s "$API/health" | python3 -m json.tool

echo ""
echo "=== Judge: Chess Move ==="
curl -s -X POST "$API/judge" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "e4 e5 Nf3 Nc6 Bb5",
    "context": "Ruy Lopez opening - a classical, well-studied opening",
    "domain": "chess"
  }' | python3 -m json.tool

echo ""
echo "=== Judge: Overconfident Claim ==="
curl -s -X POST "$API/judge" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "This trading strategy will always make money 100% guaranteed",
    "domain": "trading"
  }' | python3 -m json.tool

echo ""
echo "=== Judge: Honest Analysis ==="
curl -s -X POST "$API/judge" \
  -H "Content-Type: application/json" \
  -d '{
    "content": "Based on the evidence, this approach probably works in approximately 60% of cases because of X and Y factors",
    "context": "Analyzing market patterns with epistemic humility",
    "domain": "geopolitics"
  }' | python3 -m json.tool

echo ""
echo "=== Recent Verdicts ==="
curl -s "$API/verdicts" | python3 -m json.tool

echo ""
echo "✅ Demo complete. Max confidence: φ⁻¹ = 0.618"
```

- [ ] **Step 2: Make executable**

Run: `chmod +x scripts/demo.sh`

- [ ] **Step 3: Commit**

```bash
git add scripts/demo.sh
git commit -m "feat(demo): hackathon demo script — curl against REST API

Shows health, chess move evaluation, overconfident vs humble claims.
Ready for Gemini 3 Paris hackathon presentation."
```

---

## Execution Checklist

| # | Task | Time Est. | Dependencies |
|---|------|-----------|-------------|
| 1 | Dog trait + phi-bounding | 5 min | None |
| 2 | GeminiDog | 5 min | Task 1 |
| 3 | DeterministicDog | 3 min | Task 1 |
| 4 | Judge | 5 min | Task 1 |
| 5 | Verdict storage | 3 min | Task 1 |
| 6 | REST API | 5 min | Task 4, 5 |
| 7 | Wire main.rs | 5 min | Task 2, 3, 4, 5, 6 |
| 8 | Integration test | 3 min | Task 3, 4 |
| 9 | Demo script | 2 min | Task 7 |

**Total: ~36 min of coding.** Then boot, test with demo script, verify Gemini API works.

**After MVP:** Coordinate with S. — his React chess app calls `POST /judge` with chess moves.

---

## Environment Variables

```bash
# Required for Gemini Dog (optional — falls back to deterministic-only)
export GEMINI_API_KEY="your-google-ai-studio-key"

# Optional
export GEMINI_MODEL="gemini-2.5-flash"  # default
export SURREALDB_URL="ws://localhost:8000"
export SURREALDB_USER="root"
export SURREALDB_PASS="root"
```
