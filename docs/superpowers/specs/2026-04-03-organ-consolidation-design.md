# Organ Consolidation — Make the InferenceOrgan Alive

> CYNIC verdict: HOWL (Q=0.618, all axioms at phi-max). Validated 2026-04-03.
> Previous proposal (naive delete+fix): WAG Q=0.44, FIDELITY=0.35, CULTURE=0.35.

## Problem

The InferenceOrgan is dead tissue. It senses quality (DogStats, ParseFailureGate) but never acts on it. The degradation it detects doesn't influence Dog selection. The data it collects isn't exposed to operators. 13 of 17 public symbols have zero production callers.

Three health systems (`CircuitBreaker`, `BackendHealth`, `BackendStatus`) operate in isolation — none feeds the others. The organ is dropped after boot in `main.rs`, making all aggregate query methods unreachable at runtime.

## Diagnosis (3 expert audits)

| Finding | Severity | Source |
|---------|----------|--------|
| InferenceOrgan dropped after boot — reads unreachable | P0 | Senior Rust |
| 3 health systems never communicate | P0 | DevOps |
| No Degraded→Healthy recovery | P0 | AI/ML + DevOps |
| 13/17 organ symbols zero production callers | P1 | Dead code audit |
| BackendConfig ↔ Backend duplication (22-line manual mapping) | P1 | Senior Rust |
| mean_latency_ms, tokens_per_second never written | P1 | AI/ML |
| scoring_in_range_rate alias of json_valid_rate | P2 | Senior Rust |
| Vec::remove(0) O(n) in ParseFailureGate | P2 | Senior Rust |
| node_id hardcoded "local" for all backends | P2 | DevOps |

## Design Principle

An organ senses, processes, acts, and self-regulates. A passive data collector is not an organ. The minimum viable organ connects sensing to action: quality degradation detected → bad backend gated → recovery when quality returns → operators see what the organ sees.

## Non-goals

- **Transport**: organ does not own HTTP calls. `ChatPort` (domain) is the transport contract. Dogs call `ChatPort`. The organ gates the result, not the call. This was the original plan's Task 6 — we reject it because it duplicates `ChatPort` (K3 violation).
- **Routing**: profile→cluster→backend selection is Phase 2. The current router scaffolding has an API incompatible with the organ's `Arc<Mutex<BackendEntry>>` internals. It will be redesigned when wired.
- **Cluster/Node topology**: Phase 3 network concepts. No production caller exists.

## Architecture

```
Judge evaluates stimulus
  │
  ├─ for each Dog:
  │    ├─ check CircuitBreaker (infra failure gate)
  │    ├─ check organ BackendHealth (quality gate)  ← NEW: Phase B
  │    ├─ Dog.evaluate() → ChatPort → backend HTTP
  │    └─ update organ stats (ScoreOutcome)
  │         └─ ParseFailureGate trip? → set Degraded
  │         └─ gate clear + was Degraded? → set Healthy  ← NEW: Phase C
  │
  ├─ aggregate scores → Verdict
  │
  └─ expose via API:
       ├─ /metrics: cynic_dog_json_valid_rate{dog="..."} gauge  ← NEW: Phase D
       ├─ /metrics: cynic_dog_failure_total{dog="...",mode="..."} counter  ← NEW: Phase D
       └─ /health: organ quality contributes to system status  ← NEW: Phase D
```

## Phases

### Phase A — Delete dead tissue

Remove symbols with zero production callers and no path to a consumer in this spec.

**Delete files:**
- `organ/router.rs` — 100% dead. 7 tests validate dead code. API (`&[Backend]`) incompatible with `Arc<Mutex<BackendEntry>>` internals. Will be redesigned from scratch when Phase 2 routing is wired. Three design decisions from router tests preserved here for Phase 2: (1) Degraded backends are eligible for routing. (2) Backends below JSON threshold are excluded. (3) Non-JSON backends skip the rate check.

**Delete from `organ/registry.rs`:**
- `Node` struct — zero callers, Phase 3 concept
- `NodeId` newtype — only used to hardcode `"local"`, provides no value
- `Cluster` struct — zero callers
- `ClusterId` newtype — zero callers
- `ClusterStrategy` enum — zero callers
- `CapabilityThreshold` struct — zero callers, threshold should live where it's checked
- `RemediationConfig` struct — dead mirror of `infra::config::BackendRemediation`

**Delete from `organ/registry.rs` (`MeasuredCapabilities`):**
- `scoring_in_range_rate: f64` — alias of `json_valid_rate`, never independently read
- `mean_latency_ms: u32` — never written past `Default`. Re-added in Phase E with a writer.
- `tokens_per_second: f32` — never written past `Default`

**Delete from `organ/health.rs`:**
- `DogStats::scoring_in_range_rate()` — alias method, zero callers

**Delete from `organ/mod.rs`:**
- `InferenceOrgan::backend_ids()` — zero callers
- `pub mod router;` declaration
- References to deleted types in `update_stats_entry` (scoring_in_range_rate writes)

**Simplify `Backend` struct:**
- Remove `node_id` (was hardcoded "local")
- Remove `remediation` (dead mirror)
- Remove `endpoint`, `model`, `timeout_secs` (duplicate `BackendConfig` static fields — organ only needs `id` + `declared` + `measured` + `health`)

**Update `organ/mod.rs`:**
- Remove `pub mod router;`

**Fix `ParseFailureGate`:**
- Replace `Vec<bool>` + `remove(0)` with `VecDeque<bool>` + `pop_front()` — O(1) ring buffer

**Update `main.rs` organ construction block (lines 218-243):**
- Remove the `organ_backend` construction that sets `node_id`, `endpoint`, `model`, `timeout_secs`, `remediation` — simplify to only `id + declared + measured + health`
- Remove the `RemediationConfig` mapping block (lines 233-240)

**Update test helpers:**
- `organ/mod.rs` `make_backend()` (line 155-167): simplify to match trimmed `Backend` struct

**Expected delta:** ~-200 lines removed, ~+5 lines (VecDeque import + pop_front).

### Phase B — Connect organ to Judge (the ACT)

The organ's quality gate must influence Dog selection. When `ParseFailureGate` trips, the Dog should be skipped — same as when `CircuitBreaker` opens.

**New method on `BackendHandle`:**
```rust
/// Returns true if this backend's quality gate has tripped.
/// Acquires Mutex briefly, reads health, releases. No async, no hold across .await.
pub fn is_quality_degraded(&self) -> bool {
    self.0.lock().ok().map_or(true, |guard| {  // K14: poison = degraded
        matches!(guard.backend.health, BackendHealth::Degraded { .. } | BackendHealth::Dead { .. })
    })
}
```

Defined on `BackendHandle` in `organ/mod.rs`. Returns `true` on Mutex poison (K14: poison = assume degraded).

**Write ownership invariant:** `backend.health` inside `BackendEntry` is written ONLY by `update_stats_entry()`. It is not connected to `CircuitBreaker` state. The CB gates infrastructure; the organ gates quality. They are read at the same decision point but never write to each other.

**Changes:**
- `judge.rs`: In the pre-evaluation filter (alongside `cb.should_allow()`), call `handle.is_quality_degraded()`. If `true` → skip the Dog.
- This means organ quality degradation actually prevents bad backends from voting in the consensus.
- The circuit breaker gates on infrastructure failures (unreachable, timeout). The organ gates on quality failures (zero floods, collapses, parse garbage). Both gates must pass for a Dog to run.

**Phase B falsification (Scientific Protocol):**
- **Baseline:** Current verdict Q-Score distribution without quality gate (measure over N=50 judgments).
- **Hypothesis:** Gating degraded backends improves or maintains consensus Q (bad Dogs add noise, not signal).
- **Regression signal:** If average Q-Score drops by >0.05 after enabling the gate, the gate is too aggressive — either the threshold is wrong or the gate is incorrectly classifying good backends.
- **Measurement:** Compare Q-Score distribution before/after, same stimuli set.

**Interaction model:**
```
CircuitBreaker: "Is the backend responding?" (infra)
BackendHealth:  "Is the backend producing useful output?" (quality)
Both must pass → Dog runs
Either fails → Dog skipped
```

They remain separate systems — different signals, different thresholds, different recovery. But they coordinate at the Judge's skip decision.

### Phase C — Recovery (self-regulation)

Current bug: `BackendHealth::Degraded` is set when `ParseFailureGate` trips, but never promoted back to `Healthy`. A backend that recovers stays `Degraded` forever.

**Changes:**
- In `InferenceOrgan::update_stats_entry()`: after recording the outcome, check:
  - If `gate.is_tripped()` → set `Degraded` (existing behavior)
  - If `!gate.is_tripped()` AND `health == Degraded` → promote to `Healthy`
- This gives the organ circuit-breaker-like semantics: degrade → skip → (backend recovers, window fills with successes) → gate clears → promote → Dog re-enabled.

**Note:** A `Degraded` Dog is still skipped by Phase B's check. Once the gate clears (10 successes evict failures from the sliding window), the Dog returns to the ensemble. This is automatic — no manual intervention.

**Edge case:** If a Dog is Degraded AND the circuit breaker is open, recovery requires both: the CB must close (health probe succeeds) AND the gate must clear (quality improves). The stricter gate wins.

### Phase D — Expose (operators see what the organ sees)

The data DogStats collects must have consumers. Two real consumers:

**1. Prometheus metrics (`/metrics`):**
- `cynic_dog_json_valid_rate{dog="<id>"}` — gauge, 0.0–1.0
- `cynic_dog_capability_limit_rate{dog="<id>"}` — gauge, 0.0–1.0
- `cynic_dog_total_calls{dog="<id>"}` — counter
- `cynic_dog_quality_failures{dog="<id>",mode="zero_flood|collapse|parse_error|timeout"}` — counter (named `quality_failures` to avoid collision with existing `cynic_dog_failures` in `append_dog_metrics`)

Implementation: `Judge` exposes a `pub fn dog_quality_snapshot(&self) -> Vec<(String, DogStats)>` that reads each `BackendHandle`. The metrics renderer (already in `domain/metrics.rs`) appends organ gauges.

**2. Health endpoint (`/health`):**
- The existing `HealthGate` aggregates probe status, storage status, and dog status. Add: organ quality status.
- If any backend is `Degraded` → system health includes "organ: N backends degraded" in the detail.
- If ALL inference backends are `Degraded` → system status = `Degraded` (can't produce quality verdicts).

Implementation: `Judge::dog_quality_snapshot()` feeds both consumers. No need to Arc-wrap `InferenceOrgan` separately — the Judge already holds the handles and is already in `AppState`.

### Phase E — Latency tracking (real data)

Currently `ScoreOutcome::Success` carries no data. The Judge has `elapsed_ms` in scope at the call site but doesn't pass it.

**Changes:**
- Change `ScoreOutcome::Success` to `ScoreOutcome::Success { elapsed_ms: u64 }`
- In `judge.rs`, pass `elapsed_ms` (already computed) to `update_stats_entry`
- In `DogStats`, add `total_latency_ms: u64` field, compute incremental mean: `total_latency_ms += elapsed_ms`, expose `mean_latency_ms() -> f64 { total_latency_ms as f64 / success_count as f64 }`
- Re-add `mean_latency_ms` to Prometheus: `cynic_dog_mean_latency_ms{dog="<id>"}` gauge. This replaces the existing `cynic_dog_latency_ms` from `append_dog_metrics` (usage tracker) — the organ's version is authoritative because it measures actual evaluation time, not accumulated usage stats. R12: one value, one source.
- Update all test match sites in `organ/mod.rs` that use `ScoreOutcome::Success` as a unit variant — change to `ScoreOutcome::Success { elapsed_ms: 0 }` in tests
- This satisfies "delete dead fields, re-add with a writer" — no permanently-stuck values

## What stays from the original Phase 1 plan

| Plan task | Status | Decision |
|-----------|--------|----------|
| Task 1: skeleton + bitflags | Done | Keep |
| Task 2: registry data model | Done | **Trim** (delete dead types) |
| Task 3: health (DogStats + gate) | Done | Keep + fix VecDeque + fix recovery |
| Task 4: transport traits | Not started | **Reject** — organ doesn't transport |
| Task 5: router | Done | **Delete** — 100% dead, API wrong |
| Task 6: Prometheus metrics | Not started | **Redesign** → Phase D (via Judge, not organ) |
| Task 7: InferenceOrgan facade | Partial | **Simplify** — no boot(), just handles |
| Task 8: InferenceProfile::Calibration | Not started | **Defer** — Phase 2 calibration |
| Task 9: boot from config | Not started | **Defer** — main.rs wiring is adequate |
| Task 10: wire into main.rs | Done | Keep (already wired) |
| Task 11: update DogStats from judge | Done | **Extend** (add quality gate check, recovery, latency) |
| Task 12: organ provides Dogs + HealthGates | Not started | **Reject** — organ doesn't own Dog construction |
| Task 13: final verification | Not started | Becomes verification of this spec |

## Success criteria

1. `make check` passes (build + test + clippy + lint-rules + lint-drift)
2. Zero dead symbols in `organ/` (every public fn has a production caller)
3. ParseFailureGate trip → Dog skipped in next evaluation (integration test)
4. Recovery: 10 successes after gate trip → Dog re-enabled (integration test)
5. `curl /metrics | grep cynic_dog_json_valid_rate` returns data for each Dog
6. `curl /health` reflects organ quality status
7. `cynic_dog_mean_latency_ms` gauge moves (not stuck at sentinel value)

## Falsification

What would make us reject this approach:
- If the Judge's pre-evaluation check (`is_quality_degraded()`) adds >1ms latency per Dog (measure: `Instant::now()` around the gate check in judge.rs, log p99 over 100 evaluations)
- If `dog_quality_snapshot()` lock contention causes `/metrics` p99 > 50ms under concurrent load (measurement plan: 5 concurrent `/judge` calls + 1 `/metrics` scrape, measure `/metrics` latency. Threshold: 50ms p99. Alternative if exceeded: replace `Mutex<BackendEntry>` with atomic counters for DogStats fields, read lock-free)
- If quality gate incorrectly degrades a good Dog, reducing consensus Q-Score by >0.05 (see Phase B falsification)
- If deleting router.rs makes Phase 2 routing significantly harder (track: does Phase 2 need first-eligible logic, or a different algorithm? Design decisions preserved in Phase A deletion note)
