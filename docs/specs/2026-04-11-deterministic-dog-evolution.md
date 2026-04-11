# CYNIC Backend Evolution — Spec

**Date**: 2026-04-11
**Author**: Zey + Claude (session crystallization)
**Status**: Draft — pending review

---

## Problem Statement

The deterministic-dog scores flat 0.30 (NEUTRAL) on 3/6 axioms (FIDELITY, VERIFY, CULTURE) and uses shallow word-matching on the other 3. LLM Dogs discriminate well (ΔQ=0.261 on corpus) but have no structural output guarantee — parse failures waste inference budget. No real evaluation dataset exists. No agent consumes the system. R22 violation: 20+ days of infrastructure without a user experiment.

## Evidence Base (this session)

| Finding | Data |
|---------|------|
| Discrimination corpus (10 stimuli) | 5/5 good→Howl, 5/5 bad→Growl, ΔQ=0.261 (measured live 2026-04-10, kernel v0.7.7-64) |
| Chess test (3 cases) | Howl/Wag/Bark — correct ordering, ΔQ=0.416 |
| qwen35-9b thinking leak | Fixed: disable_thinking=true → 100% JSON valid (3/3) |
| json_schema structured output | Tested on llama.cpp: 100% valid, deterministic (temp=0) |
| Machine coupling audit | 3 HARD (fixed), 7 SOFT (documented) |
| Storage reconnect | Implemented + deployed. K15 violation closed. |
| Runtime research | llama.cpp optimal for 1-user. TabbyAPI +20-40% if re-quant. vLLM not suited. |
| Non-LLM techniques research | 5 BUILD (inline Rust), 1 new Dog (embedding), APIs deferred |

## Architecture Invariants

- deterministic-dog: pure Rust, zero dependency, zero network, < 1μs
- Embedding Dog: new Dog, separate concern, ONNX runtime acceptable
- Hermes agent: external process, consumes kernel API, does not modify kernel
- Every change measured on discrimination corpus before/after (Scientific Protocol)

---

## Phase A0 — Structured Output

**Goal**: Eliminate 100% of LLM Dog parse failures mechanically.
**Scope**: Backend adapter only. No Dog logic change.

Two levels exist in llama.cpp:
- `json_object` — forces valid JSON, no schema constraint. Already wired: `json_mode: bool` in `BackendConfig`, `ResponseFormat` in `backends/openai.rs`. Currently disabled in `backends.toml`.
- `json_schema` — forces exact field names and types. Requires extending `ResponseFormat` with an optional schema payload.

| Task | File | What |
|------|------|------|
| A0.1 | `~/.config/cynic/backends.toml` (runtime, not repo) | Set `json_mode = true` for qwen35-9b-gpu and gemma-4b-core |
| A0.2 | Measure | Run discrimination corpus — if 100% JSON valid, stop here |
| A0.3 | `cynic-kernel/src/infra/config.rs` | Only if A0.2 insufficient: add `json_schema: Option<serde_json::Value>` to `BackendConfig` |
| A0.4 | `cynic-kernel/src/backends/openai.rs` | Only if A0.3: extend `ResponseFormat` struct with optional `json_schema` field, send when configured |
| A0.5 | Measure | Run discrimination corpus, expect 100% JSON valid + ΔQ ≥ baseline |

**Falsification**: If json_schema degrades score quality (scores less discriminating), revert.

---

## Phase A — Deterministic Dog Heuristics

**Goal**: Replace flat 0.30 with real signals on abstaining axioms.
**Invariant**: Pure Rust, zero dep, < 1μs, offline.
**Protocol per commit**: baseline corpus → code → corpus → ΔQ ≥ 0 else revert.

### A1 — Hyland Hedge/Booster Expanded (FIDELITY, VERIFY)

Current: 6 "absolutes" (red-flag words) + 7 hedges. These are NOT the same as Hyland's booster/hedge categories.

Hyland's boosters ("clearly", "demonstrate", "establish") are positive epistemic markers — confident but legitimate. The current `absolutes` list ("always", "never", "guaranteed") flags overclaiming. These are distinct roles and must be scored differently.

Expansion plan:
- **Absolutes** (red flags): expand from 6 → ~20 overclaiming terms
- **Hedges** (epistemic caution): expand from 7 → ~40 terms (Hyland taxonomy)
- **Boosters** (NEW category): ~30 confidence markers — these are NOT red flags, they signal assertive (but not absolute) claims. High booster:hedge ratio without absolutes = strong claim, not overclaim.

| Signal | Axiom | Logic |
|--------|-------|-------|
| ≥3 absolutes + zero hedges | FIDELITY | Red flag → score below NEUTRAL (existing logic) |
| Hedges present | VERIFY | Epistemic uncertainty → testable → boost |
| Booster:hedge ratio > 3:1 (no absolutes) | FIDELITY | Assertive but not absolute → slight FIDELITY boost |
| Absolutes + boosters + zero hedges | FIDELITY | Overclaiming compounded → stronger penalty |

New file: `dogs/lexicon.rs` — static wordlists with explicit category separation, shared by deterministic + future Dogs.
Scoring logic change: `evaluate()` must distinguish absolute-density from booster-density.

### A2 — Coleman-Liau Readability Index (PHI)

Formula: `CLI = 0.0588 * L - 0.296 * S - 15.8` where L = avg letters per 100 words, S = avg sentences per 100 words.
No syllable counting needed — character-based, Rust-native.

| Signal | Axiom | Logic |
|--------|-------|-------|
| CLI 8-12 (accessible prose) | PHI | Golden zone → boost |
| CLI > 16 (academic/dense) | PHI | Overly complex → slight penalty |
| CLI < 4 (simplistic) | PHI | Under-structured → penalty |

~20 lines of Rust. Inline in deterministic.rs.

### A3 — Shannon Entropy (BURN)

Formula: `-Σ p(bigram) * log2(p(bigram))` over character bigrams.
High entropy = dense information = efficient. Low entropy = repetitive/formulaic.

| Signal | Axiom | Logic |
|--------|-------|-------|
| Entropy > 4.0 bits | BURN | Information-dense → boost |
| Entropy < 2.5 bits | BURN | Repetitive/formulaic → penalty |

~15 lines of Rust. Pure arithmetic.

### A4 — Lexical Density (BURN, PHI)

Ratio: content words / total words. Content = not in function-word blocklist (~150 words).

| Signal | Axiom | Logic |
|--------|-------|-------|
| Density > 0.55 | BURN | High information ratio → boost |
| Density < 0.40 | PHI | Mostly filler → structural weakness |

Blocklist as `static &[&str]`. ~30 lines.

### A5 — PTC Propaganda Signals (SOVEREIGNTY)

Six rule-based detectors from SemEval-2020 Task 11 taxonomy:

| Pattern | Detection | Technique |
|---------|-----------|-----------|
| `according to experts/scientists/studies` | Regex | Appeal to Anonymous Authority |
| Superlative + universal (`best always`, `worst never`) | Word pair | Causal Oversimplification |
| Fear vocabulary (`threat`, `danger`, `catastrophe`, `destroy`) | Wordlist | Appeal to Fear |
| Short imperative + first-person plural (`we must act`) | Pattern | Slogans / Bandwagon |
| Exclamation density > 2% of words | Count | Loaded Language |
| Same noun/phrase repeated 3+ times | Frequency | Repetition technique |

These patterns are qualitatively different from the current `coercion_count` density model.
New scoring approach: compute a `propaganda_score` from weighted pattern matches (not density-threshold).
Each detector returns 0.0 or a penalty weight. Sum is clamped and subtracted from SOVEREIGNTY_BASE.
This replaces the single-formula density model with a multi-signal scorer for this axiom.

---

## Phase B — Embedding Dog (New Dog)

**Goal**: Semantic non-LLM scoring via prototype cosine similarity.
**Dependency**: Phase A complete and measured.
**Decision gate**: If Phase A closes the gap sufficiently (ΔQ > 0.3), Phase B is deferred.

### B1 — Spike: fastembed-rs + potion-base-8M

- Test `fastembed-rs` ONNX in Rust: latency on CPU, binary size impact
- Model: potion-base-8M (8MB, sub-millisecond, Apache-2.0)
- Acceptance: < 10ms per sentence, < 50MB binary growth

### B2 — Exemplar Store Design

- 10 exemplar sentences per axiom (5 positive, 5 negative) per domain
- Precomputed embeddings loaded at boot
- Score = cosine(stimulus, positive_prototype) - cosine(stimulus, negative_prototype)
- Normalized to [0, φ⁻¹]

### B3 — EmbeddingDog Implementation

- New file: `dogs/embedding.rs`
- Implements `Dog` trait
- Reuse existing `EmbeddingPort` (domain/embedding.rs) — adapter already exists
- Circuit breaker for ONNX runtime failures

### B4 — Measurement

- Discrimination corpus before/after
- Per-axiom ΔQ with and without EmbeddingDog in consensus
- Latency impact on /judge p95

---

## Phase C — Chess Evaluation Dataset

**Goal**: Real benchmark, not 3 hardcoded cases.
**Approach**: Build a labeled dataset of chess claims (not positions).

### C1 — Dataset Design

Structure (JSON lines, Colab-friendly):
```json
{"id": "chess-001", "content": "...", "context": "...", "domain": "chess", "label": "correct|incorrect|nuanced", "source": "lichess|manual|stockfish", "difficulty": "easy|medium|hard"}
```

Target: 50 stimuli minimum (20 correct, 20 incorrect, 10 nuanced).

Sources:
- Manual: opening theory claims (verified against opening databases)
- Stockfish: position evaluations → natural language claims
- Lichess: forum assertions about openings/tactics (hand-labeled)

### C2 — Benchmark Script

- Read corpus JSON → POST /judge for each → collect results
- Compute: accuracy (correct label → Howl/Wag, incorrect → Growl/Bark)
- Compute: discrimination ΔQ per label category
- Compute: per-Dog agreement rate
- Output: markdown report

### C3 — Extend to General Domain

Same structure for domain="general". Reuse discrimination_corpus.json as seed, expand to 50.

---

## Phase D — Hermes Feeder Agent

**Goal**: First autonomous consumer of the CYNIC pipeline. R22 validation.
**Architecture**: External process. Talks to kernel via REST API. Uses coord protocol.

### D1 — Design

- Binary: `cynic-hermes` (separate crate in workspace, or Python script)
- Flow: source → extract claims → POST /judge → log results → POST /observe
- Registers via POST /coord/register at startup
- Claims work targets via POST /coord/claim
- **Must send heartbeats** (POST /coord/heartbeat) every 60s — the `expire_stale` loop expires sessions after 5 min without heartbeat, releasing all claims silently. A batch run > 5 min without heartbeat loses its claims.

### D2 — Chess Feeder (first instance)

- Source: Lichess API (public, no auth needed for game data)
- Extract: opening names, position evaluations, player annotations
- Transform: natural language claims ("This Sicilian variation is sharp and double-edged")
- Submit: POST /judge with domain="chess"
- Volume: 10-50 judgments per run (bounded, not infinite)

### D3 — General Feeder (second instance)

- Source: RSS feeds (Wikipedia Recent Changes, HackerNews, arXiv abstracts)
- Extract: first sentence/abstract as claim
- Submit: POST /judge with domain="general"

### D4 — Measurement

- Corpus growth rate (judgments/day)
- Crystal formation rate (how many stimuli crystallize)
- False positive rate (manual review of Howl verdicts on known-bad content)

---

## Phase E — Runtime Upgrade (Optional)

**Gate**: Only if inference latency becomes a bottleneck (not the case today).

| Option | When | What |
|--------|------|------|
| TabbyAPI + ExLlamaV3 | If tok/s matters | Re-quant to EXL3, +20-40% on Windows GPU |
| Ollama | If ops simplicity matters | Drop-in replace llama-server |
| llama-cpp-rs binding | If zero-HTTP matters | Load GGUF in-process (max sovereignty) |

---

## Measurement Framework

Every phase has a before/after gate using the discrimination corpus.

| Metric | Baseline (this session) | Target |
|--------|------------------------|--------|
| JSON valid rate (qwen35-9b) | 40% (historical) → 100% (post-fix) | 100% (json_schema) |
| JSON valid rate (gemma-4b) | 75% | 100% (json_schema) |
| Discrimination ΔQ (good vs bad) | 0.261 | ≥ 0.30 |
| Chess discrimination ΔQ | 0.416 (3 cases — small sample) | Maintain ≥ 0.35 at 50+ cases (regression floor, not improvement target) |
| Deterministic-dog variance | 0.00 (flat 0.30) | > 0.05 |
| Corpus size | 10 stimuli | 100+ |
| Active consumers | 0 | 1 (Hermes) |

---

## Priority Order

```
A0 (json_schema)        ← eliminates parse failures, 1 session
A1-A5 (heuristics)      ← deterministic-dog gains signal, 1-2 sessions
C1-C2 (chess dataset)   ← real benchmark, 1 session (Colab?)
D1-D2 (Hermes feeder)   ← first consumer, R22 validated, 1-2 sessions
B1-B4 (embedding dog)   ← if gap remains after A, 2-3 sessions
C3 (general dataset)    ← expand benchmark
D3 (general feeder)     ← expand consumption
E (runtime)             ← only if latency bottleneck
```

Each phase is independently shippable. No phase blocks another except B depends on A's measurement.
