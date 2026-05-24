---
description: Rust kernel rules — loaded when editing cynic-kernel/
globs: ["cynic-kernel/**"]
---

## Kernel Rules

### Enforced (make lint-rules / compiler)

K1. **Domain purity.** Zero `#[cfg]` in domain code (except `#[cfg(test)]`). — `make lint-rules`
K2. **Every adapter through a port trait.** No raw `reqwest::Client` outside `backends/` and `storage/`. — `make lint-rules`
K3. **No logic duplication across API surfaces.** Pipeline lives in `pipeline/mod.rs`. REST and MCP call it. Extract at 2nd occurrence, not 3rd — LLMs replicate patterns before humans notice. — `make lint-rules` (protected function list: `format_crystal_context`, `compute_qscore`, `trimmed_mean`; add new entries when duplication is found)
K4. **No trait name collisions.** Each port trait name unique across `domain/`. — `make lint-rules`
K5. **No cross-layer type leakage.** App services never import from `api/`. Domain never exposes infra types. — `make lint-rules` (checks `serde_json::Value`, `reqwest::`, `surrealdb::`, `axum::` in domain/)

### Design Principles

K6. **Timeout every background `.await`.** Any `.await` inside `tokio::spawn` must use `tokio::time::timeout()`.
K8. **Every SQL query has an integration test.** Round-trip: INSERT + SELECT + assert on shape. — `make check-storage` (when SurrealDB available)

### LLM Development Principles

The codebase is the prompt. Every pattern in code will be replicated by future LLM sessions. Bad patterns spread exponentially — each copy reinforces the signal.

K12. **`#[allow]` is an instruction.** Every lint suppression tells the next LLM "do this." Require adjacent `// WHY:` comment explaining the suppression. Suppress without justification = amplified debt. — `make lint-rules`
K14. **Poison/missing = assume degraded.** When reading shared state (`RwLock`, `Option`), the fallback on error must be the SAFE default (degraded/unavailable), never the OPTIMISTIC default (ok/sovereign). `unwrap_or(true)` for degradation checks, never `unwrap_or(false)`.

## Build

- `make check` = build + test + clippy + lint-rules + lint-drift + audit (mandatory after every change)
- `make test-gates` = R21 gate falsification (inject violation → verify gate catches → restore)
- `#![deny(dead_code, unused_imports, clippy::unwrap_used, clippy::expect_used)]` via workspace lints
- `#![cfg_attr(test, allow(dead_code, clippy::unwrap_used, clippy::expect_used))]` in `lib.rs` — test code is exempt. Do NOT add per-function `#[allow(clippy::expect_used)]` in `#[cfg(test)]` modules; the crate-level exemption already covers them.
- Workspace lints in root Cargo.toml (Axum/Embark baseline)
- Integration tests: `tests/integration_storage.rs` with `#[ignore]`

K15. **Every producer has a consumer that ACTS.** Storing, displaying, or logging is not consuming. A consumer must change system behavior: gate a request, trigger a state transition, emit an alert that is routed to a human. `store_*` without an acting reader = Rule 3 violation. `emit_event` without an acting handler = dead nervous system. — `make lint-drift` (producer-consumer audit)

K17. **Every port trait method forwarded in proxies.** `ReconnectableStorage` must forward ALL `StoragePort` methods. Default trait impls return `Ok(())` or empty — a new method that isn't forwarded silently succeeds with no persistence. This has caused 3 separate incidents (state_log, agent_tasks, last_observation). After adding a `StoragePort` method: verify `ReconnectableStorage` forwards it. — `make lint-drift` (method count comparison)

K16. **Context is metabolic.** Every line the LLM reads costs cognition. Comments that paraphrase code, imports unused in the current concern, and mixed responsibilities in a single file are active noise — empirically worse than absent context (arXiv 2406.11927). Split by responsibility, not by line count. A file should be describable in 3 words; if not, it does too much.

K18. **Subprocesses die with their parent.** Every `tokio::process::Command` that uses `timeout()` MUST set `kill_on_drop(true)` before `spawn()`. Use `spawn()` + `wait_with_output()`, never raw `cmd.output()` inside a timeout. Without this, timed-out children become zombies in the systemd cgroup, surviving SIGTERM and polluting subsequent boots. Incident: 70 gemini-cli zombies in 25h (PR#94). — `make lint-rules` (grep `tokio::process::Command` without `kill_on_drop`)

K20. **Stimulus format = API contract with Dogs.** Any change to `build_token_stimulus` (or any `build_*_stimulus`) format MUST update the corresponding deterministic dog parser AND run its tests in the same commit. `unwrap_or(0)` on parsed stimulus fields silently swallows format mismatches — the parser reports 0 instead of failing. Incident: `holders: 20+` broke `v.parse::<u64>()` → dog scored holders=0 in production for all tokens (PR#116). — Falsify: change a stimulus line format without updating the parser; tests should fail.

K19. **Deterministic serialization.** Any struct serialized to DB or included in a hash MUST use `BTreeMap`, never `HashMap`. HashMap iteration order is non-deterministic across runs (Rust hash randomization). A HashMap serialized to JSON produces different byte sequences for identical data → integrity chain breaks, test flakiness, irreproducible DB content. Incident: `failed_dog_errors: HashMap` caused non-deterministic `dog_errors_json` column (PR#94).

K21. **Pipeline output fed back as input = tag and filter.** When a pipeline's output (e.g. verdict) is stored as an observation for the compound loop, it MUST carry a distinguishing tag (e.g. `"compound-loop"`). Any consumer that re-processes observations (nightshift Phase 2, K15 consumers) MUST filter these tags to prevent amplification. Without filtering, each verdict generates a re-judgment which generates another verdict — bounded by domain gates but wasting compute proportional to verdict volume. Incident: nightshift re-judged 53% of observations as compound-loop feedback, saturating the sovereign Dog's single inference slot (2026-05-08). — Falsify: remove the tag filter from nightshift Phase 2; compound-loop observations should be re-judged and slot utilization should spike.

K23. **New domain = 4 wiring items.** Adding a judgment domain requires: (1) `semantic_slug` case in `intake.rs` (prevents crystal collapse), (2) domain prompt in `domains/X.md` + registered in `embedded_domains.rs` (Dogs get correct axiom criteria), (3) curation sync path from live source to `cynic-python/curation/` (data reaches kernel), (4) verify `is_crystallizable_domain` includes the domain (crystals form). Omitting any one causes silent data loss — D2 had 4388 curated signals but 1 crystal for 9 days because all 4 were missing. Incident: D2 domain added to capture (2026-05-12) but never wired to consumption — 97% data loss until 2026-05-21 (PR#243). — Falsify: add a test domain without one of the 4 items; the corresponding pipeline stage should produce zero output for that domain.

K25. **Sovereign inference dispatch is rate-limited.** Any consumer that calls `judge.evaluate()` in a loop (nightshift, convergence, background jobs) MUST include `tokio::time::sleep()` between iterations. Without rate-limiting, a batch of N judgments saturates all sovereign Dog slots for the entire batch, starving interactive `/judge` requests. The SlotSemaphore reserves the last slot for User/Hermes, but only if callers yield between iterations. Incident: Hermes kanban 5 workers on 2 slots = 100% timeout (2026-04); nightshift 300+ observations tight loop = dogs=1 for all interactive requests (2026-05-24). — Falsify: remove `sleep` from nightshift observation loop; `journalctl | grep "slots saturated"` should spike within 2 cycles.

K22. **Restored state must not poison fresh gates.** When persisting component statistics across restarts (DogStats, circuit breaker counters), quality-gate counters (total_calls, success_count, failure counts) MUST reset to zero at boot. Only calibration data (token budgets, max observed tokens) should be restored. Without this, stale 0% success rates from a broken endpoint immediately trip the json_valid_rate gate and lock out recovering Dogs — a deadlock where recovery requires participation but participation requires recovery. Incident: 87 calls at 0% success (from dead HF endpoint) restored at boot locked all inference Dogs out of /judge indefinitely (2026-05-08). — Falsify: restore full DogStats at boot with a failing dog; the dog should be immediately excluded from /judge despite fresh boot.
