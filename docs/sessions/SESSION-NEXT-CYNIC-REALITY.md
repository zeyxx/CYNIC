# Session: CYNIC Reality Audit — What to Keep, Build, or Burn

## Context

Deep crystallize-truth analysis (2026-03-24) audited CYNIC's entire codebase against its claims. This document is the complete, unfiltered reality for the next session to act on.

---

## CYNIC — What It Actually IS (code-verified)

### 1. Epistemic Scoring API (REAL, WORKING)

**Pipeline** (`pipeline.rs::run()`):
1. Embed stimulus → vector (for cache + crystal search)
2. Cache check (cosine similarity on embeddings) → return cached if hit
3. Crystal retrieval (KNN HNSW on SurrealDB, top 10, Crystallized/Canonical only)
4. Session summaries (last 5, 400 char budget)
5. Context assembly (user context + crystals 1100 chars + session summaries)
6. Fan-out to ALL active Dogs in parallel (`join_all`, per-Dog timeout, circuit breaker skip)
7. Verdict assembly: phi-bound each score [0.05, 0.618], geometric mean → QScore, anomaly detection (disagreement > φ⁻²)
8. Side effects (best-effort, never abort): store verdict, track usage, observe crystal, cache, emit SSE events

**Input limits**: content ≤ 4000, context ≤ 2000, domain ≤ 64. Rate: 30 req/min global, 10 /judge/min.

**Integrity**: BLAKE3 hash chain links each verdict to the previous one.

### 2. Crystal Feedback Loop (REAL, VALIDATED Δ=+0.02-0.04)

**Crystal lifecycle** (`domain/ccm.rs`):
- Identity: FNV-1a hash of `domain:stimulus_summary`, with semantic merge (cosine ≥ 0.75 in same domain reuses existing crystal)
- Observation: atomic SurrealDB transaction. Running mean: `(prev_conf × prev_obs + new_score) / new_obs`
- States: **Forming** (default) → **Crystallized** (obs ≥ 21 AND conf ≥ 0.618) → **Canonical** (obs ≥ 233 AND conf ≥ 0.618). Also: **Decaying** (obs ≥ 21 AND conf < 0.382)
- Thresholds: 21 = Fibonacci F(8), 233 = Fibonacci F(13)
- Injection: only Crystallized/Canonical, sorted by `confidence × e^(-age_days/90)`, budget 1100 chars
- Confidence normalization: `q_score.total / 0.618` (so realistic 0.55 maps to 0.89)

### 3. Five Dogs (REAL, ALL TESTED)

| Dog | Type | Where | Cost |
|-----|------|-------|------|
| deterministic-dog | In-kernel heuristics | Always loaded | Free |
| gemini | Gemini Flash | Google API | Per-token |
| huggingface | Mistral 7B | HF Inference API | Free tier |
| sovereign | Gemma 3 12B | S. GPU machine, RTX 4060 Ti (Ollama) | Free |
| sovereign-ubuntu | Gemma 3 4B | Ubuntu CPU (llama-server) | Free |

**DeterministicDog**: Abstains (NEUTRAL=0.309) on FIDELITY, VERIFY, CULTURE. Actively scores PHI (structure), BURN (density), SOVEREIGNTY (coercion words).

**InferenceDog**: Generic wrapper for any OpenAI-compatible API. Config via `backends.toml`. System prompt is identical for all Dogs. Domain prompts substitute the axiom section.

**Circuit breakers**: Per-Dog. ApiError/ParseError trip. RateLimited/Timeout don't. Health loop probes sovereign backends every 30s. Remediation config can SSH-restart failed backends.

### 4. Multi-Agent Coordination (REAL, DEPLOYED)

- `coord/register(agent_id, agent_type, intent)` — session in SurrealDB
- `coord/claim(agent_id, target, claim_type)` — exclusive lock, 409 on conflict
- `coord/claim-batch` — up to 20 targets atomically
- `coord/release(agent_id, target?)` — release one or all
- Background expiry: every 60s, sessions >5min without heartbeat → deactivated, orphan claims removed
- Full audit trail in SurrealDB `audit` table

### 5. SSE Event Bus (REAL, BUT DISCONNECTED)

Channel capacity 256. Events emitted:
- `VerdictIssued { verdict_id, domain, verdict, q_score }`
- `CrystalObserved { crystal_id, domain }`
- `DogFailed { dog_id, error }`
- `SessionRegistered { agent_id }`
- `BackfillComplete { count }`
- `Anomaly { kind, message, severity }`

**Critical gap**: ZERO internal consumers. Events are emitted into the void unless an external SSE client is listening. No organ reacts to events.

### 6. MCP Server (REAL, 10 TOOLS)

`cynic_health`, `cynic_judge`, `cynic_infer`, `cynic_verdicts`, `cynic_crystals`, `cynic_coord_register`, `cynic_coord_claim`, `cynic_coord_claim_batch`, `cynic_coord_release`, `cynic_coord_who`, `cynic_audit_query`.

### 7. Storage (SurrealDB 3.x over HTTP)

Tables: `verdict`, `crystal`, `observation`, `agent_session`, `claim`, `audit`, `usage`, `session_summary`.
KNN: `WHERE embedding <|K,40|> $q` (HNSW index on crystals).
Graceful degradation: `NullStorage` + `NullCoord` if SurrealDB unreachable at boot.

### 8. Background Tasks

- Health loop (30s): probes sovereign backends, updates circuit breakers
- CCM aggregator (5min): tool frequency + co-occurrence patterns from observations → **NOT fed back anywhere**
- Session summarizer (periodic): 3+ observation sessions → sovereign LLM → `session_summary` table → injected in Dog prompts
- Coord expiry (60s): deactivate stale agent sessions
- Crystal backfill (boot): embed crystals missing vectors, up to 200

### 9. Domain Prompts (2 domains)

- `domains/chess.md` (2.8K) — per-axiom HIGH/MEDIUM/LOW criteria for chess evaluation
- `domains/trading.md` (3.8K) — per-axiom criteria for market hypotheses and quant analysis
- Loaded at boot, injected as axiom section replacement in Dog prompts

### 10. Observability

- `/health` — 200 (sovereign) or 503 (degraded/critical). Auth'd version: Dog states, storage, usage, cost, alerts, embedding status
- `/metrics` — Prometheus format (counters + per-Dog gauges)
- `/usage` — per-Dog token usage, costs, latencies
- Introspection loop (5min, MAPE-K Analyze): anomaly signals → `introspection_alerts` + SSE

### 11. Build Quality

- 253 test functions, `#![deny(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]`
- Hexagonal architecture: 43 files, 9 modules
- v0.7.2 tagged, 247+ tests at time of tag

---

## CYNIC — What It Claims But Does NOT Have (zero code)

| Claimed | Evidence of absence |
|---------|-------------------|
| **Q-Learning** | `grep 'q_table\|q_learning\|QTable' src/` = 0 matches |
| **Thompson Sampling** | `grep 'thompson\|Thompson' src/` = 0 matches |
| **EWC (Elastic Weight Consolidation)** | `grep 'ewc\|elastic.*weight' src/` = 0 matches |
| **SONA (11 parallel learning loops)** | `grep 'sona\|SONA\|learning_loop' src/` = 0 matches |
| **Consciousness levels (REFLEX/MICRO/MACRO/META)** | `grep 'ConsciousState\|LODController\|consciousness' src/` = 0 matches |
| **7-step cognitive cycle traits** | `grep 'Perceive\|Decide\|Act\|Learn\|Account\|Emerge' src/` = 0 (as traits) |
| **Ring 0 hardware detection** | `Ring 0/1/2/3` labels exist but are boot sequence comments, not capability probes |
| **gRPC (4 services)** | Feature-gated `--features grpc`, no client exists, `tonic`/`prost` in Cargo.toml but zero production usage |
| **3.2x MCTS convergence speedup** | Benchmark was on synthetic data, code removed |

---

## CYNIC — What Exists But Is FAKE (dead architecture, Rule #21)

### Temporal Perspectives — `dog_scores[i % 7]` relabeled

**What the code does** (`api/rest/response.rs:68-110`):
```rust
// This is the ENTIRE "temporal evaluation":
let perspectives = TemporalPerspective::ALL;  // [Past, Present, Future, Cycle, Trend, Emergence, Transcendence]
dog_scores.iter().enumerate().filter_map(|(i, ds)| {
    let perspective = perspectives.get(i % perspectives.len())?;  // Dog 0 = "Past", Dog 1 = "Present", etc.
    // ... copies the Dog's existing axiom scores as-is
})
```

**What it claims**: "7 temporal perspectives evaluating stimuli through different time lenses"
**What it does**: Takes Dog scores, assigns perspective LABELS by index modulo 7. No temporal evaluation. No different prompts. No different behavior. Pure relabeling.

**Impact**: Every `/judge` response includes a `temporal` field that LIES. External consumers (dashboard, agents) may believe temporal evaluation is real.

**Where it lives**:
- `domain/temporal.rs` — 212 lines: enum, descriptions, aggregate function, 7 tests
- `api/rest/response.rs:66-110` — `compute_temporal_from_dogs()` — the relabeling function
- `api/rest/types.rs:150,164` — `TemporalResponse` in `JudgeResponse`
- `api/rest/health.rs:13-30` — `/temporal` endpoint returning perspective definitions
- `api/rest/mod.rs:73` — route registration

**Note**: `domain/ccm.rs` has a REAL `temporal_relevance()` function (crystal decay `confidence × e^(-age_days/90)`). This is genuine and must NOT be burned. Naming collision is confusing.

### CCM Aggregator — computes, feeds nothing

**What it does**: Every 5 minutes, queries `observation` table for tool frequency and file co-occurrence patterns. Logs them. That's it.
**What it should do**: Feed patterns somewhere (crystal system? introspection? dashboard?).
**Current value**: Zero. Pure computation waste.

---

## 7 Weaknesses — Prioritized (updated 2026-03-24)

| # | Weakness | Status | Resolution |
|---|----------|--------|------------|
| 1 | **Dead architecture (temporal fake)** | **DONE** ✓ | Burned `compute_temporal_from_dogs()`, `/temporal` endpoint, types. Renamed `decay_relevance`. `domain/temporal.rs` dormant on disk. (5a63bf3) |
| 2 | **Dead claims (Q-Learning, SONA, etc.)** | **DONE** ✓ | CYNIC-CRYSTALLIZED-TRUTH.md marked historical. gRPC proto burned. Probe renamed. CLAUDE.md fixed. (04a5873) |
| 3 | **Event bus with 0 consumers** | OPEN | Deferred — low urgency. Wire Anomaly → WARN + metric counter as minimum. |
| 4 | **Crystal poison (no external validation)** | **DONE** ✓ | Epistemic soft gate: 3 tiers (Agreed/Disputed/Contested). Abstention ≠ disagreement fix. (fe1d192, 355ce96) |
| 5 | **No cross-domain calibration** | OPEN | Deferred — generic approach works for now. |
| 6 | **CCM aggregator computes for nothing** | OPEN | Intentionally dead per code comments (RAG contamination risk). Wire to /health or burn. |
| 7 | **Domains = text files only** | OPEN | Lower priority. |

### NEW weakness discovered (2026-03-24):

| # | Weakness | Type | Risk if ignored | Suggested action |
|---|----------|------|-----------------|------------------|
| 8 | **Crystal positive-only learning bias** | Feedback loop gap | Crystals only learn GOOD patterns. Bad patterns (Bark/Growl) get low confidence → Decaying → never injected. The system has no way to express "we're confident this is BAD." | Three options: (a) inject Decaying crystals with negative label, (b) separate confidence from valence, (c) invert confidence for consistently-bad patterns. See analysis below. |
| 9 | **DeterministicDog structural disagreement** | Calibration gap | Det-dog NEUTRAL=0.309 creates permanent ~0.22 disagreement on abstained axioms. Partially fixed by abstention exclusion (355ce96), but det-dog's active scores (PHI, BURN, SOVEREIGNTY) may also diverge structurally from inference Dogs. Monitor over time. |

### Weakness #8 — Crystal Positive-Only Bias (detailed)

**The problem**: `crystal_confidence = q_score.total / φ⁻¹`. A Bark (Q=0.20) → conf=0.32 → after 21 obs → **Decaying** (conf < φ⁻²). Decaying crystals are filtered out of injection (line 166 of ccm.rs). The crystal system can identify bad patterns but never USES that knowledge.

**Additionally**: bad verdicts (Bark/Growl) often have high Dog disagreement → contested by soft gate → quarantined → crystal never even observed. Double blockage.

**Three design options**:
1. **Inject Decaying as warnings** — smallest change. Add `[DECAYING]` crystals to `format_crystal_context` with warning label. Dogs see "this pattern scored poorly."
2. **Separate confidence from valence** — `confidence` (certainty) + `valence` (good/bad). A pattern can have high confidence AND negative valence. Clean but requires schema change.
3. **Invert for consistent negatives** — "20 observations averaging Bark" = high certainty of badness. Mechanically invert: `crystal_conf = 1 - (q/φ⁻¹)` for consistently-low patterns.

**Research needed**: how do ML systems handle negative exemplars in feedback loops? (active learning with negative examples, contrastive learning for crystals)

---

## What Is Genuinely Novel and Valuable (PROTECT)

1. **φ-bounded multi-validator consensus** — 5 independent Dogs, geometric mean, anomaly detection on disagreement > φ⁻². No other system does this.
2. **Crystal feedback loop** — verdict → observe → crystallize → inject → better verdict. Validated Δ=+0.02-0.04 on chess. The only feature that makes CYNIC more than a stateless LLM wrapper.
3. **Semantic crystal merging** — cosine ≥ 0.75 in same domain → accumulate on existing crystal. Prevents fragmentation.
4. **φ-derived thresholds** — Fibonacci observation counts (21, 233), golden ratio confidence bounds. Mathematically grounded, not arbitrary.
5. **BLAKE3 hash chain** — tamper-evident verdict history. Subtle but important for trust.
6. **Multi-agent coordination** — simple, effective, battle-tested across Claude+Gemini+Hermes parallel sessions.

---

## Architecture Reality (honest summary)

CYNIC is a **cognitive accumulator** — it judges content via independent validators, crystallizes repeated patterns into reusable wisdom, and coordinates multi-agent workflows. It is NOT an OS, NOT a learning system (beyond the crystal loop), and NOT temporally aware.

The organism metaphor (Dogs=senses, CCM=memory, Judge=cognition) is MORE accurate than the OS metaphor (Ring 0/1/2/3). CYNIC doesn't manage processes, schedule tasks, or allocate resources. It accumulates judgment.

**One-line truth**: CYNIC is a φ-bounded judgment accumulator with a crystal feedback loop and multi-agent coordination.

---

## Decision Framework for Next Session

For each component, decide:

```
KEEP & PROTECT  → genuinely novel, validated, working
BUILD FOR REAL  → the concept is right but implementation is fake/missing
BURN            → dead code, false claims, zero value
DEFER           → good idea but not priority now
```

### Components — decisions made (2026-03-24):

- [x] `domain/temporal.rs` + `compute_temporal_from_dogs()` → **BURNED wiring, kept math dormant** (5a63bf3)
- [x] gRPC feature gate → **BURNED** (proto deleted, comments removed, CLAUDE.md fixed) (04a5873)
- [x] CYNIC-CRYSTALLIZED-TRUTH.md → **MARKED HISTORICAL** (04a5873)
- [x] Crystal poison detection → **BUILT: epistemic soft gate** (fe1d192) + **abstention fix** (355ce96)
- [x] `probe/mod.rs` Ring 0 → **RENAMED to "Probe"** (04a5873)

### Components — still requiring work:

- [ ] CCM aggregator — wire to /health introspection or burn
- [ ] Event bus internal consumers — minimum: Anomaly → WARN + metric
- [ ] Cross-domain calibration — deferred, generic works
- [ ] **Crystal positive-only bias (NEW #8)** — crystals can't learn bad patterns. Design decision needed.
- [ ] **Temporal rebuild** — research done (Dipper NeurIPS 2024, arXiv 2507.21168). Path: short directives → A/B test → wire if validated.
- [ ] CYNIC-ARCHITECTURE-TRUTHS.md — verify no stale claims remain

---

## Codebase Stats (updated 2026-03-24)

- **241 test functions** (234 active + 7 epistemic gate tests; 9 temporal tests dormant with module)
- **43 source files, 9 modules** (hexagonal)
- **v0.7.2** tagged (4 commits ahead: 5a63bf3, 04a5873, fe1d192, 355ce96)
- **Stable toolchain**: 1.94+, edition 2024
- **Known bug**: LLVM stack overflow on deep monomorphization (workaround: `jobs=1`, `RUST_MIN_STACK=16MB`)
- **Compiler denies**: `dead_code`, `unused_imports`, `clippy::unwrap_used`, `clippy::expect_used`
- **Net change this session**: -393 lines burned, +141 lines built = -252 net
