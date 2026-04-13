<div align="center">

# CYNIC

> **"Sovereign infrastructure making the cost of lying visible through the geometry of calibrated doubt."**

**Decentralized epistemic consensus — independent AI validators judging content under mathematical doubt**

[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-1.94%2B-orange.svg)](https://www.rust-lang.org/)
[![Tests](https://img.shields.io/badge/tests-434_passing-brightgreen.svg)](#development)

`Rust` `Axum` `Tokio` `SurrealDB` `React` `TypeScript`

[Why CYNIC](#why) · [How It Works](#how-it-works) · [Architecture](#architecture) · [Quickstart](#quickstart) · [API](#api)

</div>

---

## Why

AI systems increasingly make high-stakes decisions — approving trades, reviewing code, scoring content. Most rely on a **single model's opinion** with unbounded confidence.

CYNIC is a **consensus protocol for AI judgment**. Multiple independent validators ("Dogs") evaluate content across 6 axioms, their scores are merged via trimmed-mean consensus, and every score is structurally capped at **φ⁻¹ = 61.8%** — the golden ratio inverse. No validator can be certain. No single failure corrupts the output.

Patterns that survive repeated evaluation **crystallize** into persistent knowledge, creating a compound learning loop. Measured improvement: **Δ = +0.02–0.04** on chess domain.

```bash
curl -X POST http://localhost:3030/judge \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -d '{"content": "1. e4 c5 — Sicilian Defense", "domain": "chess"}'
```

```json
{
  "verdict": "Howl",
  "q_score": {
    "total": 0.577,
    "fidelity": 0.618, "phi": 0.450, "verify": 0.618,
    "culture": 0.618, "burn": 0.618, "sovereignty": 0.618
  },
  "dogs_used": "deterministic-dog+qwen35-9b-gpu+qwen-7b-hf+gemma-4b-ubuntu",
  "voter_count": 4,
  "domain": "chess",
  "anomaly_detected": false
}
```

---

## How It Works

### The Six Axioms

Every piece of content is scored across 6 independent dimensions:

| Axiom | Question |
|-------|----------|
| **Fidelity** | Is this faithful to truth? |
| **Phi** | Is it structurally harmonious? |
| **Verify** | Can it be tested or refuted? |
| **Culture** | Does it respect established patterns? |
| **Burn** | Is it efficient? Minimal waste? |
| **Sovereignty** | Does it preserve agency and freedom? |

The final **Q-Score** is the geometric mean of all axiom scores, φ-bounded. Geometric mean ensures a single weak axiom drags the overall score down — no hiding behind averages.

### Validators (Dogs)

Dogs evaluate content in parallel, independently, with no knowledge of each other's scores:

| Dog | Type | Backend | Latency |
|-----|------|---------|---------|
| **deterministic-dog** | Heuristic | In-kernel | < 1 ms |
| **qwen35-9b-gpu** | LLM (9B) | Local GPU | ~ 4 s |
| **qwen-7b-hf** | LLM (7B) | HF Inference API | ~ 1.5 s |
| **gemma-4b-ubuntu** | LLM (4B) | Local CPU | ~ 36 s |
| **gemini-flash** | LLM | Google API | ~ 0.7 s |

When Dogs disagree beyond **φ⁻² (0.382)** on any axiom, CYNIC flags an **anomaly** — a discovery signal, not an error.

### Consensus Protocol

```
1. All Dogs evaluate in parallel (dynamic timeout: slowest Dog + 5s)
2. Circuit breaker skips Dogs with 3+ consecutive failures
3. Trimmed-mean aggregation (drop highest + lowest when ≥ 4 Dogs)
4. Per-axiom anomaly detection via φ² residual check
5. Geometric mean → φ-bound → verdict classification
6. Quorum gate: single-Dog verdicts don't crystallize (min 2)
```

### Verdicts

Verdicts map to φ-derived thresholds:

| Verdict | Threshold | Meaning |
|---------|-----------|---------|
| **Howl** | > 0.528 (φ⁻² + φ⁻⁴) | Exceptional |
| **Wag** | > 0.382 (φ⁻²) | Good |
| **Growl** | > 0.236 (φ⁻³) | Questionable |
| **Bark** | ≤ 0.236 | Rejected |

### Crystal Loop (Compound Learning)

Patterns that survive repeated evaluation crystallize into persistent knowledge:

```
Stimulus → Dogs evaluate → Verdict
                              ↓
               Crystal observation recorded
                              ↓
              ≥ 21 observations + confidence ≥ φ⁻¹  →  CRYSTALLIZED
              ≥ 233 observations                     →  CANONICAL
              confidence drops below φ⁻²             →  DECAYING → DISSOLVED
                              ↓
              Injected into future Dog prompts
                              ↓
              Better judgments → more crystals → compound loop
```

Thresholds are Fibonacci numbers. Crystals are content-addressed (FNV-1a). Semantic deduplication via KNN (HNSW index). Epistemic soft gate quarantines contested judgments.

---

## Architecture

Hexagonal architecture — domain logic has zero dependencies on frameworks, databases, or HTTP.

```
cynic-kernel/src/
├── domain/           Pure business logic — zero IO, zero frameworks
│   ├── dog.rs        Dog trait, AxiomScores, QScore, φ-bounding
│   ├── ccm.rs        Crystal lifecycle, context formatting, aggregation
│   ├── storage.rs    StoragePort trait (34 methods)
│   ├── sanitize.rs   Input sanitization (injection defense)
│   ├── compliance.rs Session compliance scoring
│   └── ...           10 more domain modules
├── dogs/
│   ├── deterministic.rs  Heuristic evaluator (PHI, BURN, SOVEREIGNTY)
│   └── inference.rs      LLM-backed Dog (any OpenAI-compatible backend)
├── backends/         Adapters to LLM inference endpoints
├── storage/          SurrealDB + InMemory adapters
├── api/
│   ├── rest/         Axum REST — 24 routes, auth, rate limiting
│   └── mcp/          MCP server — 12 tools for AI agent integration
├── infra/            Background tasks, circuit breakers, config
├── probe/            Boot-time self-verification (6 probes)
├── pipeline.rs       Shared evaluation path (REST + MCP both call this)
├── judge.rs          Consensus orchestration, BLAKE3 integrity
└── main.rs           Composition root
```

### Port Traits

```rust
#[async_trait]
pub trait Dog: Send + Sync {
    fn id(&self) -> &str;
    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError>;
}

#[async_trait]
pub trait ChatPort: Send + Sync {
    async fn chat(&self, system: &str, user: &str) -> Result<ChatResponse, ChatError>;
    async fn health(&self) -> BackendStatus;
    fn name(&self) -> &str;
}
```

Adding a new validator = implement `Dog`. Adding a new LLM backend = implement `ChatPort`. 10 port traits total.

---

## Quickstart

### Requirements

- Rust 1.94+ (stable, edition 2024)
- SurrealDB 3.x (optional — kernel runs with in-memory storage)
- At least one LLM backend (local [llama.cpp](https://github.com/ggml-org/llama.cpp) server, Gemini API, or HF Inference)

### Setup

```bash
git clone https://github.com/zeyxx/CYNIC.git
cd CYNIC

# Create env file
cat > ~/.cynic-env << 'EOF'
export CYNIC_API_KEY="your-api-key-here"
export CYNIC_REST_ADDR="127.0.0.1:3030"
export SURREALDB_PASS="your-db-password"
EOF

# Configure Dogs — see backends.toml.example
cp backends.toml.example ~/.config/cynic/backends.toml

# Build + test + lint
make check

# Run
source ~/.cynic-env
cargo run -p cynic-kernel --release
```

### Verify

```bash
# Health (no auth required)
curl http://localhost:3030/health

# Submit content for judgment (auth required)
source ~/.cynic-env
curl -X POST "http://${CYNIC_REST_ADDR}/judge" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" \
  -d '{"content": "1. e4 c5 — Sicilian Defense", "domain": "chess"}'
```

---

## API

All endpoints except health probes require `Authorization: Bearer $CYNIC_API_KEY`.

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | System status — Dogs, probes, alerts |
| `/live`, `/ready` | GET | Kubernetes-compatible liveness/readiness |
| `/judge` | POST | Submit content for multi-validator evaluation |
| `/verdicts` | GET | List recent verdicts |
| `/verdict/{id}` | GET | Get verdict by ID |
| `/crystals` | GET | List crystallized patterns |
| `/crystal/{id}` | GET/DELETE | Get or dissolve a crystal |
| `/observe` | POST | Record observation (agent workflow) |
| `/dogs` | GET | Active validators and status |
| `/usage` | GET | Token consumption per Dog |
| `/agents` | GET | Registered agent sessions |
| `/events` | GET | SSE event stream |
| `/coord/*` | POST | Multi-agent coordination (claim/release) |
| `/metrics` | GET | Prometheus-compatible metrics |

Rate limit: 30 req/min global, 10 req/min on `/judge`.

Full API contract: [`API.md`](API.md)

---

## Development

```bash
make check          # Build + test + clippy + lint-rules + lint-drift + audit
make lint-rules     # Architectural rules enforcement (K1-K5)
make lint-drift     # Config/code/docs drift detection
make lint-security  # Security findings tracker gate
```

434 tests. 23K LOC (Rust + TypeScript). 5 integration test suites.

---

## License

Apache-2.0 — see [LICENSE](LICENSE).

---

<div align="center">

*φ distrusts φ — no claim deserves absolute confidence*

</div>
