# Calibration Corpus — Dev Domain

Reference stimuli for measuring Dog discrimination on dev-domain content.
Run each through `/judge` with `domain="dev"` and compare Q-scores.
A good Dog should score GOOD stimuli > 0.382 (WAG+) and BAD stimuli < 0.382 (GROWL-).

## GOOD (expected WAG or HOWL)

### G1: Clean error handling
feat(api): replace unwrap() with proper error propagation using ? operator across all request handlers. Each error now carries context via anyhow::Context. Tests verify error messages reach the caller.

### G2: DRY extraction
refactor(pipeline): extract compute_qscore into shared module — was duplicated in rest.rs and mcp.rs. One function, two callers, three tests. Zero behavior change.

### G3: Port trait discipline
feat(storage): add CachePort trait with get/set/invalidate. Redis adapter implements it. Domain code imports only the trait. Integration test round-trips a value.

### G4: TDD cycle
feat(auth): add JWT validation middleware. Wrote failing test first (expired token returns 401), then minimal implementation, then edge cases (malformed, missing, valid). 4 tests, 30 lines of implementation.

### G5: Minimal sovereign fix
fix(health): circuit breaker was stuck open after recovery — reset_at wasn't cleared on successful probe. One-line fix, regression test added, before/after metrics documented.

### G6: Clean separation of concerns
refactor(judge): split 400-line judge.rs into judge.rs (orchestration), scoring.rs (phi-bounded math), prompt.rs (template construction). Each file < 150 lines, one responsibility, existing tests still pass.

### G7: Proper resource cleanup
feat(tasks): background loops now respect CancellationToken for graceful shutdown. Each loop uses tokio::select! on shutdown + interval. Tests verify sub-second shutdown on cancel.

### G8: Empirical validation
perf(pipeline): reduced /judge p95 latency from 2.3s to 0.8s by parallelizing Dog evaluation with FuturesUnordered instead of sequential awaits. Before/after flamegraph attached.

### G9: Configuration discipline
refactor(config): move all magic numbers to constants.rs with semantic names. CRYSTAL_CHALLENGE_INTERVAL, DOG_TTL_CHECK, etc. Single source of truth, grep-friendly.

### G10: Defensive coding
fix(crystal): observe_crystal now validates confidence is in [0.0, 1.0] before storing. Previously accepted any f64 — negative scores corrupted state machine transitions. Regression test added.

## BAD (expected GROWL or BARK)

### B1: Silent error swallowing
fix(api): added .ok() to all database calls to "prevent crashes". No logging, no retry, no fallback. Errors are now invisible. Ship it.

### B2: God function
feat(pipeline): added 600-line process_everything() that parses HTTP, validates auth, queries DB, scores with Dogs, stores verdict, sends Slack notification, and updates metrics. No tests.

### B3: Hardcoded secrets
feat(auth): added API key validation. Key is hardcoded as const API_KEY: &str = "sk-prod-abc123" in the source file. Works great in production.

### B4: Copy-paste duplication
feat(api): added /v2/judge endpoint. Copied entire judge_handler from v1, changed one field name. Both versions now need to be maintained independently. 200 lines duplicated.

### B5: Untested critical path
feat(billing): added payment processing. No tests because "it's too hard to mock Stripe". Manual testing confirmed it works on staging. Deployed to production.

### B6: Premature abstraction
refactor(utils): created AbstractStrategyFactoryBuilder<T, U, V> with 4 trait bounds for a function that has exactly one implementation and is called once. 150 lines of generics, 3 lines of logic.

### B7: Vendor lock-in
feat(storage): replaced SQLite with Firebase. All queries use Firebase-specific syntax directly in domain code. No abstraction layer. "We'll always use Firebase."

### B8: Hope-driven engineering
fix(timeout): increased timeout from 30s to 300s because "sometimes it's slow." No investigation into why it's slow. No metrics. No root cause analysis.

### B9: Dead architecture
feat(plugins): added plugin system with registry, lifecycle hooks, dependency injection, and hot-reload. Zero plugins exist. Zero planned. "We might need it someday."

### B10: Breaking change without migration
refactor(db): renamed all database columns from camelCase to snake_case. No migration script. No backward compatibility. Existing data is now orphaned. YOLO.
