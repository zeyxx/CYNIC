<div align="center">

# CYNIC

**Epistemic consensus engine — independent AI validators reaching agreement under mathematical doubt**

`Rust` `Axum` `Tokio` `SurrealDB` `React` `TypeScript`

[Philosophy](#philosophy) · [How It Works](#how-it-works) · [Architecture](#architecture) · [Run It](#quickstart) · [API](#api)

</div>

---

## What CYNIC Does

CYNIC takes any content — a chess move, a political claim, a code review — and runs it through **multiple independent AI validators** ("Dogs") that score it across 6 philosophical axioms. Their scores are merged via trimmed-mean consensus, and **no score can exceed 61.8%** (the golden ratio inverse, φ⁻¹).

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
  "q_score": { "total": 0.507, "fidelity": 0.618, "phi": 0.400, "verify": 0.618, "culture": 0.618, "burn": 0.618, "sovereignty": 0.618 },
  "dogs_used": "deterministic-dog+gemini",
  "anomaly_detected": false
}
```

A Sicilian Defense gets a **Howl** (highest verdict). A Fool's Mate gets a **Bark** (rejected). CYNIC judges the strategy, not the description.

---

## Philosophy

```
φ distrusts φ — no claim deserves absolute confidence
```

Every score is structurally capped at **φ⁻¹ = 0.618033...**  — the golden ratio inverse. This isn't a bug. It encodes the principle that certainty is always partial.

Verdicts map to φ-derived thresholds:

| Verdict | Threshold | Meaning |
|---------|-----------|---------|
| **Howl** | ≥ 0.521 | Exceptional quality |
| **Wag** | ≥ 0.382 (φ⁻²) | Good |
| **Growl** | ≥ 0.236 (φ⁻³) | Questionable |
| **Bark** | < 0.236 | Rejected |

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

| Dog | Type | How |
|-----|------|-----|
| **deterministic-dog** | Heuristic | Rule-based form analysis (<1ms). Abstains on substance (returns NEUTRAL) |
| **gemini** | LLM | Gemini Flash via OpenAI-compatible API |
| **sovereign** | LLM | Local Qwen 3.5 9B via llama.cpp (GPU) |
| **sovereign-ubuntu** | LLM | Local Gemma 3 4B via llama.cpp (CPU) |

When Dogs disagree beyond φ⁻² (0.382) on any axiom, CYNIC flags it as an **anomaly** — a signal that the content is epistemically interesting.

### Consensus

```
1. All Dogs evaluate in parallel (60s wall-clock timeout)
2. Circuit breaker skips Dogs with 3+ consecutive failures
3. Trimmed-mean aggregation (drops highest + lowest when ≥4 Dogs)
4. Per-axiom anomaly detection via φ² residual check
5. Geometric mean → phi-bound → verdict classification
```

### Cognitive Crystallization (CCM)

Patterns that survive repeated evaluation crystallize into persistent knowledge:

```
≥ 21 observations + confidence ≥ φ⁻¹  →  CRYSTALLIZED
≥ 233 observations                     →  CANONICAL
confidence drops below φ⁻²            →  DECAYING → DISSOLVED
```

Thresholds are Fibonacci numbers. Crystals are content-addressed (FNV-1a hash).

---

## Architecture

Hexagonal architecture — domain logic has zero dependencies on frameworks, databases, or HTTP.

```
┌─────────────────────────────────────────────────────┐
│                    REST API (Axum)                   │
│              POST /judge  GET /verdicts              │
├─────────────────────────────────────────────────────┤
│                                                     │
│   ┌─────────┐    ┌──────────┐    ┌──────────────┐  │
│   │  Judge   │───▶│   Dogs   │───▶│   Backends   │  │
│   │consensus │    │evaluate  │    │ OpenAI-compat│  │
│   └────┬─────┘    └──────────┘    └──────────────┘  │
│        │                                            │
│   ┌────▼─────┐    ┌──────────┐    ┌──────────────┐  │
│   │ Verdict  │    │   CCM    │    │Circuit Breaker│  │
│   │ QScore   │    │crystals  │    │  per-Dog     │  │
│   └──────────┘    └──────────┘    └──────────────┘  │
│                                                     │
│          DOMAIN (pure Rust, no #[cfg], no IO)       │
├─────────────────────────────────────────────────────┤
│   Storage (SurrealDB)  │  MCP Server  │  Probe     │
│        INFRASTRUCTURE                               │
└─────────────────────────────────────────────────────┘
```

### Key Abstractions

Any LLM backend becomes a Dog through one trait:

```rust
#[async_trait]
pub trait ChatPort: Send + Sync {
    async fn chat(&self, system: &str, user: &str) -> Result<ChatResponse, ChatError>;
    async fn health(&self) -> BackendStatus;
    fn name(&self) -> &str;
}
```

Validators implement a single method:

```rust
#[async_trait]
pub trait Dog: Send + Sync {
    fn id(&self) -> &str;
    fn max_context(&self) -> u32 { 0 }
    async fn evaluate(&self, stimulus: &Stimulus) -> Result<AxiomScores, DogError>;
}
```

Adding a new Dog = implement `Dog` trait + register in `main.rs`. Adding a new LLM backend = implement `ChatPort`.

---

## Quickstart

### Requirements

- Rust 1.75+
- SurrealDB 3.x (optional — kernel runs without it, verdicts won't persist)
- At least one LLM backend (Gemini API key, or local llama.cpp)

### Build & Run

```bash
git clone https://github.com/zeyxx/CYNIC.git
cd CYNIC

# Configure
cp ~/.config/cynic/env.example ~/.config/cynic/env
# Edit env: set CYNIC_API_KEY, SURREALDB_URL, backend API keys

# Build
cargo build -p cynic-kernel --release

# Test (80+ tests)
cargo test -p cynic-kernel --release

# Run
cargo run -p cynic-kernel --release
# → Listening on 0.0.0.0:3030
```

### Verify

```bash
# Health check (no auth required)
curl http://localhost:3030/health

# Submit a judgment (auth required)
curl -X POST http://localhost:3030/judge \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $CYNIC_API_KEY" \
  -d '{"content": "1. e4 e5 2. Qh5 Nc6 3. Bc4 Nf6 4. Qxf7# — Scholars Mate", "domain": "chess"}'
```

---

## API

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/health` | GET | No | System status, version, axiom list |
| `/judge` | POST | Bearer | Submit content for epistemic evaluation |
| `/verdicts` | GET | Bearer | List recent verdicts |
| `/verdict/{id}` | GET | Bearer | Get specific verdict by UUID |
| `/crystals` | GET | Bearer | List crystallized patterns |
| `/usage` | GET | Bearer | Token consumption per Dog |

Rate limit: 30 req/min global, 10 req/min on `/judge`.

Full API reference with TypeScript interfaces: [`API.md`](API.md)

---

## Project Structure

```
cynic-kernel/src/
├── domain/          # Pure business logic — zero IO, zero frameworks
│   ├── dog.rs       # Dog trait, AxiomScores, QScore, phi-bounding
│   ├── ccm.rs       # Cognitive Crystallization Mechanism
│   ├── chat.rs      # ChatPort trait (LLM abstraction)
│   ├── coord.rs     # Multi-agent coordination port
│   └── temporal.rs  # Temporal perspectives, UCB1 exploration
├── dogs/            # Validator implementations
│   ├── deterministic.rs  # Heuristic form evaluator
│   └── inference.rs      # LLM-backed Dog (any ChatPort backend)
├── backends/        # LLM backend adapters
│   ├── openai.rs    # OpenAI-compatible (Gemini, HF, vLLM, SGLang)
│   ├── llamacpp.rs  # llama.cpp with cold-start polling
│   └── router.rs    # Backend registration + health probing
├── api/
│   ├── rest/        # Axum REST handlers
│   └── mcp/         # MCP server (rmcp) for AI agent integration
├── infra/           # Circuit breaker, config, rate limiting
├── storage/         # SurrealDB HTTP adapter
├── judge.rs         # Consensus orchestration
└── main.rs          # Wiring
```

---

## Numbers

```
~5000 LOC Rust  ·  80+ tests  ·  134 commits  ·  7 days
6 axioms  ·  4 Dogs  ·  circuit breakers  ·  CCM crystallization
REST + MCP  ·  multi-model inference  ·  hexagonal architecture
```

---

## Status

CYNIC is a working prototype (v0.1.0). The core judgment pipeline — Dogs, consensus, phi-bounding, CCM, REST API — works end-to-end.

**Working:** Multi-validator consensus, phi-bounded scoring, deterministic + LLM Dogs, circuit breakers, REST API with auth + rate limiting, SurrealDB persistence, CCM crystallization, React chess dashboard.

**In progress:** MCP server handlers, backend router fan-out, temporal integration, learning feedback loops.

---

## Frontend

Interactive chess judgment dashboard — React + TypeScript + Recharts.

See [`cynic-ui/`](cynic-ui/) for the frontend. Evaluates chess positions in real-time against the kernel.

---

<div align="center">

*"The dog who speaks truth, loyal to verification, not comfort"*

© 2026 — All rights reserved. Source available for review purposes.

</div>
