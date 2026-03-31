<div align="center">

# CYNIC

**Epistemic immune system — independent AI validators reaching consensus under mathematical doubt**

`Rust` `Axum` `Tokio` `SurrealDB` `React` `TypeScript`

v0.7.6 · 399 tests · 21K LOC · 5 Dogs · φ-bounded

[Philosophy](#philosophy) · [How It Works](#how-it-works) · [Architecture](#architecture) · [Run It](#quickstart) · [API](#api)

</div>

---

## What CYNIC Does

CYNIC takes any content — a chess move, a trading signal, a code review — and runs it through **multiple independent AI validators** ("Dogs") that score it across 6 philosophical axioms. Their scores are merged via trimmed-mean consensus, and **no score can exceed 61.8%** (the golden ratio inverse, φ⁻¹).

Patterns that survive repeated evaluation **crystallize** into persistent knowledge that improves future judgments. Measured improvement: Δ=+0.02-0.04 on chess domain.

Disagreement between validators is surfaced as a **discovery signal**, not hidden.

```bash
curl -X POST http://localhost:3030/judge \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -d '{"content": "1. e4 c5 — Sicilian Defense", "domain": "chess"}'
```

```json
{
  "verdict": "Howl",
  "q_score": { "total": 0.577, "fidelity": 0.618, "phi": 0.450, "verify": 0.618, "culture": 0.618, "burn": 0.618, "sovereignty": 0.618 },
  "dogs_used": "deterministic-dog+qwen35-9b-gpu+qwen-7b-hf+gemma-4b-ubuntu",
  "voter_count": 4,
  "domain": "chess",
  "anomaly_detected": false
}
```

---

## Philosophy

```
φ distrusts φ — no claim deserves absolute confidence
```

Every score is structurally capped at **φ⁻¹ = 0.618033...** — the golden ratio inverse. This isn't a bug. It encodes the principle that certainty is always partial.

Verdicts map to φ-derived thresholds:

| Verdict | Threshold | Meaning |
|---------|-----------|---------|
| **Howl** | > 0.528 (φ⁻²+φ⁻⁴) | Exceptional quality |
| **Wag** | > 0.382 (φ⁻²) | Good |
| **Growl** | > 0.236 (φ⁻³) | Questionable |
| **Bark** | ≤ 0.236 | Rejected |

---

## How It Works

### The Six Axioms

Every piece of content is evaluated across 6 independent dimensions:

| Axiom | Question |
|-------|----------|
| **FIDELITY** | Is this faithful to truth? |
| **PHI** | Is it structurally harmonious? |
| **VERIFY** | Can it be tested or refuted? |
| **CULTURE** | Does it respect established patterns? |
| **BURN** | Is it efficient? Minimal waste? |
| **SOVEREIGNTY** | Does it preserve agency and freedom? |

The final Q-Score is the **geometric mean** of all axiom scores, phi-bounded. Geometric mean ensures a single bad axiom drags the score down — no hiding behind averages.

### Dogs (Independent Validators)

Dogs evaluate content in parallel, independently, with no knowledge of each other's scores:

| Dog | Type | Where | Latency |
|-----|------|-------|---------|
| **deterministic-dog** | Heuristic | In-kernel | <1ms |
| **qwen35-9b-gpu** | LLM | Local GPU (RTX 4060 Ti) | ~4s |
| **qwen-7b-hf** | LLM | HF Inference API | ~1.5s |
| **gemma-4b-ubuntu** | LLM | Local CPU | ~36s |
| **gemini-flash** | LLM | Google API | ~0.7s |

When Dogs disagree beyond φ⁻² (0.382) on any axiom, CYNIC flags it as an **anomaly** — a signal that the content is epistemically interesting.

### Consensus

```
1. All Dogs evaluate in parallel (dynamic wall-clock: slowest Dog timeout + 5s)
2. Circuit breaker skips Dogs with 3+ consecutive failures
3. Trimmed-mean aggregation (drops highest + lowest when ≥4 Dogs)
4. Per-axiom anomaly detection via φ² residual check
5. Geometric mean → phi-bound → verdict classification
6. Quorum gate: single-Dog verdicts don't crystallize (min 2)
```

### Crystal Loop (Compound Learning)

Patterns that survive repeated evaluation crystallize into persistent knowledge:

```
Stimulus → Dogs evaluate → Verdict → Crystal observation
                                          ↓
                            ≥ 21 obs + conf ≥ φ⁻¹ → CRYSTALLIZED
                            ≥ 233 obs              → CANONICAL
                            conf drops below φ⁻²   → DECAYING → DISSOLVED
                                          ↓
                            Injected into future Dog prompts
                                          ↓
                            Better judgments → more crystals → compound
```

Thresholds are Fibonacci numbers. Crystals are content-addressed (FNV-1a hash). Semantic merge via KNN (HNSW index) prevents duplicates. Epistemic soft gate quarantines contested judgments.

---

## Architecture

Hexagonal architecture — domain logic has zero dependencies on frameworks, databases, or HTTP.

```
cynic-kernel/src/
├── domain/           Pure business logic — zero IO, zero frameworks
│   ├── dog.rs        Dog trait, AxiomScores, QScore, phi-bounding
│   ├── ccm.rs        Crystal lifecycle, context formatting, aggregation
│   ├── storage.rs    StoragePort trait (34 methods)
│   ├── sanitize.rs   Content + observation target sanitization (CH2 defense)
│   ├── compliance.rs Session compliance scoring
│   └── ...           10 more domain modules (events, metrics, usage, etc.)
├── dogs/
│   ├── deterministic.rs  Heuristic form evaluator (PHI, BURN, SOVEREIGNTY)
│   └── inference.rs      LLM-backed Dog (any OpenAI-compatible backend)
├── backends/         Driven port adapters (HTTP to LLM endpoints)
├── storage/          SurrealDB HTTP + InMemory adapters
├── api/
│   ├── rest/         Axum REST — 24 routes, auth, rate limiting
│   └── mcp/          MCP server — 12 tools for AI agent integration
├── infra/            Background tasks, circuit breakers, config
├── probe/            Boot-time hardware + LLM discovery
├── pipeline.rs       THE shared evaluation path (REST + MCP both call this)
├── judge.rs          Consensus orchestration, BLAKE3 integrity chain
└── main.rs           Composition root
```

### Key Port Traits

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

Adding a new Dog = implement `Dog` trait. Adding a new LLM backend = implement `ChatPort`. 10 port traits total.

---

## Quickstart

### Requirements

- Rust 1.94+ (stable, edition 2024)
- SurrealDB 3.x (optional — kernel runs without it)
- At least one LLM backend (local llama-server, Gemini API, or HF Inference)

### Setup

```bash
git clone https://github.com/zeyxx/CYNIC.git
cd CYNIC

# Create env file with your secrets
cat > ~/.cynic-env << 'EOF'
export CYNIC_API_KEY="your-api-key-here"
export CYNIC_REST_ADDR="127.0.0.1:3030"
export SURREALDB_PASS="your-db-password"
EOF

# Configure Dogs in ~/.config/cynic/backends.toml
# See backends.toml.example for template

# Build + test + lint (the full gate)
make check

# Run
source ~/.cynic-env
cargo run -p cynic-kernel --release
```

### Verify

```bash
# Health (no auth)
curl http://localhost:3030/health

# Judge (auth required)
source ~/.cynic-env
curl -X POST "http://${CYNIC_REST_ADDR}/judge" \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer ${CYNIC_API_KEY}" \
  -d '{"content": "1. e4 c5 — Sicilian Defense", "domain": "chess"}'
```

---

## API

All endpoints except `/health`, `/live`, `/ready`, `/metrics`, `/events` require `Authorization: Bearer $CYNIC_API_KEY`.

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | System status, Dogs, alerts, background tasks |
| `/live` | GET | Liveness probe |
| `/ready` | GET | Readiness probe (storage check) |
| `/judge` | POST | Submit content for epistemic evaluation |
| `/verdicts` | GET | List recent verdicts |
| `/verdict/{id}` | GET | Get specific verdict |
| `/crystals` | GET | List crystallized patterns |
| `/crystal/{id}` | GET | Get specific crystal |
| `/observe` | POST | Record workflow observation |
| `/usage` | GET | Token consumption per Dog |
| `/dogs` | GET | List active Dog IDs |
| `/agents` | GET | List registered agent sessions |
| `/events` | GET | SSE event stream |
| `/coord/*` | POST | Multi-agent coordination (register, claim, release) |

Rate limit: 30 req/min global, 10 req/min on `/judge`.

Full contract with TypeScript interfaces: [`API.md`](API.md)

---

## Development

```bash
# Full validation (sovereign CI — also runs as pre-push hook)
make check

# Individual targets
make lint-rules      # K1-K5, R1-R2 grep-enforceable rules
make lint-drift      # Config/code/docs drift detection
make lint-security   # 0 OPEN CRIT/HIGH in findings tracker
```

Rules: `.claude/rules/` (universal.md, kernel.md, workflow.md, reference.md)

---

## Status

v0.7.6 — working kernel in production. v0.8 (Fondation Prouvée) in progress.

**Working:** Multi-validator consensus, φ-bounded scoring, 5 Dogs (1 heuristic + 4 LLM), circuit breakers, crystal compound loop (Δ=+0.02-0.04 chess), REST API (24 routes) + MCP server (12 tools), multi-agent coordination, SurrealDB persistence with KNN crystal search, session compliance scoring, MAPE-K introspection, proprioceptive probe system (6 probes), React chess dashboard.

**v0.8 gates:** Security closure, StoragePort agnosticism (InMemory contract tests), workflow alignment.

---

<div align="center">

*"The dog who speaks truth, loyal to verification, not comfort"*

</div>
