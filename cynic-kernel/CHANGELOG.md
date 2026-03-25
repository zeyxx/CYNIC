# Changelog — cynic-kernel

All notable changes. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.7.4] — 2026-03-25

### Build

- **workflow:** industrial enforcement stack L0-L3 — 8-layer architecture

## [0.7.3] — 2026-03-25

### Added

- **kernel:** domain prompts + crystals A/B flag + chess benchmark
- **kernel:** φ-derived crystal budget + chess crystal injection script
- **kernel:** add trading domain prompt for KAIROS integration
- **kernel:** epistemic soft gate for crystal feedback loop

### Fixed

- **kernel:** distinguish abstention from disagreement in anomaly detection
- **rc2:** honest health — count healthy dogs, not total dogs
- **rc8:** restrict CORS methods + add cargo audit to pipeline
- **rc2:** add /live and /ready probes, fix healthcheck service
- **rc5:** eliminate 9 silent failure paths — honest errors everywhere
- **rc3:** boot config validation — model verification + runtime paths
- **rc1:** MCP security — rate limiting, input validation, error sanitization
- **rc4:** SQL safety — escape backticks, collision-free record IDs, flush_usage escaped
- **rc7:** request tracing — unique request_id in pipeline span
- **review:** 3 bugs found by adversarial review of RC1+RC4

### Release

- **v0.7.3:** industrial hardening — 29 findings fixed, 260 tests

### Removed

- **kernel:** remove dead MCTS types + rewrite API.md from ground truth
- **kernel:** remove temporal fake wiring + rename decay_relevance
- **docs:** remove dead gRPC claims + mark CRYSTALLIZED-TRUTH historical

### Testing

- **kernel:** add domain prompt injection test — both paths covered

## [0.7.2] — 2026-03-23

### Changed

- **kernel:** move Metrics from infra/ to domain/ — Rule #32

### Fixed

- **kernel:** complete Rule #32 — Judge accepts breakers, rate eviction to main

### Hardened

- **kernel:** Rule #8 — log all fallible I/O before fallback
- **kernel:** complete verdict schema + version 0.7.2
- **kernel:** HealthGate trait + Rule #10 timeouts — v0.7.2 D+E

## [0.7.1] — 2026-03-23

### Fixed

- **kernel:** persist dog_scores to DB — v0.7.1
- **kernel:** log dog_scores_json parse failure — Rule #8 compliance

## [0.7.0] — 2026-03-23

### Added

- **kernel:** circuit breaker getters for health loop
- **kernel:** background health probe loop for Dog backends
- **kernel:** wire health loop + remediation — CYNIC self-heals
- **mcp:** semantic verdict cache + cynic_infer validation
- **kernel:** per-backend inference config — timeout, max_tokens, temperature, disable_thinking
- **kernel:** P1 backfill + P2 dark tables + P3 introspection loop
- **kernel:** Event Bus + SSE — real-time kernel visibility
- **kernel:** VERSION.md + pipeline refactor + Rule #32

### Changed

- **kernel:** flush_usage → StoragePort + Rule #8 sweep
- **config:** single source of truth — merge remediation into backends.toml
- **kernel:** move KernelEvent to domain/ — v0.7 Gate 1+2
- **kernel:** extract spawns to infra/tasks.rs — v0.7 Gate 5
- **kernel:** spawn functions return JoinHandle<()>
- **kernel:** typed domain structs + thiserror — v0.7 Gate 3 COMPLETE

### Documentation

- **kernel:** CHANGELOG.md + /status skill rewrite

### Fixed

- **kernel:** rate limiter eviction off hot path → background timer
- **kernel:** /judge now respects global rate limit
- **kernel:** StorageError + CoordError implement std::error::Error
- **kernel:** robustness hardening + per-dog cost tracking
- **kernel:** sovereign thinking model support + timeout alignment
- **mcp:** background tasks + error handling + coordination audit trail
- **kernel:** health endpoint 30s→3s timeout + load all backends at boot
- **kernel:** review fixes from parallel session audit
- **security:** production hardening — backup, monitoring, input validation
- **kernel:** coord claim TOCTOU deadlock + VerdictCache O(1) eviction
- **kernel:** compiler as impact checker + API contract tests + health O(1)
- **health:** return HTTP 503 on degraded/critical — industry standard
- **crystal:** normalize Q-Score for crystal confidence — unblocks feedback loop
- **pipeline:** semantic crystal merging — end fragmentation via HNSW similarity
- **pipeline:** log all embedding failures and crystal fallback paths
- **pipeline:** cache hits now feed the crystal loop — closes the compound gap
- **kernel:** code review fixes — C1 H1 H2 H3 M1 M4 M5

### Hardened

- **kernel:** production robustness — 10 structural fixes + 4 new rules
- **kernel:** storage config in TOML, domain-scoped crystal fallback, boot legibility
- **kernel:** lifecycle orchestration, crystal CRUD, transaction safety
- **kernel:** observability + visibility + crystal defragmentation
- **kernel:** honest task_health + burn redundant timer
- **kernel:** deny(clippy::expect_used) — v0.7 Gate 4

### Other

- **kernel:** last let _ = escape comment (middleware audit)
- Merge branch 'docs/kairos-on-cynic-design'
- **kernel:** bump version to 0.7.0 — v0.7 "Le Socle Architectural"

### Testing

- **storage:** 13 integration tests against real SurrealDB + crystal ID fix
- **contract:** /judge API contract — 12 fields verified with consumer map

## [0.6.0] — 2026-03-19

### Added

- **kernel:** P2 semantic verdict cache + storage observability
- **kernel:** Dog::health() — unified backend observability

### Changed

- **kernel:** domain purity — remove concrete type leak + external deps

### Fixed

- **kernel+hooks:** data integrity + hook robustness — 5 fixes
- **build:** watch refs/heads for git describe rebuild on new commits
- **build:** stop watching .git for version — rebuild at deploy, not commit
- **kernel:** compound G — health storage check, error responses, failure tracking
- **kernel:** shutdown flush + dead code burn + MCP client reuse
- **kernel:** EmbeddingBackend 5-state health + probe $HOME → dirs
- **kernel:** replace audit DELETE subquery with date-based cleanup
- **kernel:** retry on SurrealDB 3.x intermittent 401
- **kernel:** 3 Google-review blockers + systemd hardening

### Other

- **kernel:** version 0.6.0 — align Cargo.toml with reality

### Removed

- **kernel:** gRPC stubs + robustness fixes — -761 lines

## [0.5.0] — 2026-03-18

### Added

- sovereign foundation — single-crate workspace, InferencePort P0
- **backend:** make InferencePort object-safe via async-trait + add model_hint
- **muscle:** add LlamaCppBackend — InferencePort for llama.cpp HTTP API
- **muscle:** add BackendRouter — route + fan_out with round-robin and health filtering
- **hal:** rewrite — route through BackendRouter instead of hardcoded stub
- **hal:** make SurrealDB optional — kernel boots without memory service if auth fails
- **kernel:** 5-state health + circuit breaker + proto fixes
- **mvp:** hackathon judgment engine — Dog trait + Judge + REST API
- **kernel:** hexagonal refactor — ChatPort, OpenAiCompatBackend, InferenceDog, parallel Judge
- **kernel:** extend axiom system from 3 to 6 — CULTURE, BURN, SOVEREIGNTY
- **kernel:** add CCM — Cognitive Crystallization Mechanism
- **kernel:** add MCTS Temporal — 7 temporal perspectives with phi exploration
- **kernel:** HTTP storage adapter, burn dead code, boot gracieux
- **kernel:** Dog selection API — frontend chooses which Dogs to use
- **kernel:** enriched /health with Dog status, smarter deterministic-dog
- **kernel:** CCM feedback loop — CYNIC learns from verdicts
- **kernel:** per-Dog latency tracking + persistent SurrealDB
- **kernel:** add token count fields to DogScore (placeholder for counting)
- **kernel:** real token counting per Dog via ChatResponse
- **kernel:** circuit breaker per Dog — Closed → Open → HalfOpen
- **kernel:** GET /usage — token consumption tracking per Dog
- **kernel:** temporal perspectives in verdict + GET /temporal
- **kernel:** GET /crystal/{id} endpoint + circuit breaker verified live
- **kernel:** 5th Dog — Qwen 2.5 72B via HF, 4 model families
- **kernel:** self-aware cost tracking — CYNIC knows what it costs
- **kernel:** MCP server over stdio + klog macro for dual-mode logging
- **kernel:** adaptive context routing — Dogs filtered by ctx capacity
- **kernel:** /health verifies DB connectivity — no more silent failures
- **kernel:** multi-agent coordination via MCP — register, claim, release, who
- **kernel:** deterministic-dog as FORM judge, SCORE_FLOOR, FNV-1a hash
- **workflow:** Phase B — Session Lifecycle + coord fix + Claude MCP config
- **kernel:** CCM feedback loop — crystallized wisdom injected into Dog prompts
- **kernel:** CCM workflow observation pipeline — capture, aggregate, crystallize
- **kernel:** CCM co-occurrence patterns — detect files edited together
- **usage:** persist DogUsageTracker to SurrealDB — survives restarts
- **kernel+hooks:** REST coord endpoints + hook enforcement
- **kernel:** EmbeddingPort + sovereign embedding backend
- **kernel:** coord_claim_batch — claim N targets in one call
- **kernel:** git describe in /health — traceable builds without manual versioning

### Changed

- **domain:** extract DogUsageTracker from rest adapter — fix V1 cross-adapter coupling
- **domain:** InferenceRouter trait — decouple hal from concrete BackendRouter (fix V4)
- **domain:** add CoordPort trait + SurrealHttpStorage implementation
- **domain:** CoordPort wiring — eliminate raw SurrealHttpStorage bypass (fix V2+V3)
- **kernel:** hexagonal module structure — 23 flat files → 9 modules
- **probe:** split mod.rs (896 LOC) into 5 focused files
- **rest:** split mod.rs (779 LOC) into 7 focused files
- **kernel:** stabilize build, gate gRPC, remove dead code

### Fixed

- prost name normalization + surrealdb type ambiguity + lifetime
- **clippy:** resolve all -D warnings — unused imports, doc comments, collapsible ifs
- **storage:** add signin to SurrealDB connection — 400 on unauthenticated RPC
- **storage:** use ws:// protocol — /rpc requires WebSocket on SurrealDB 3.x
- **storage:** surrealdb 3.x Root fields are String not &str
- **probe:** scan HOME+/home instead of / on Linux
- **storage:** no default password — SURREALDB_PASS must be set explicitly
- **router:** collapse nested ifs to satisfy clippy collapsible_if lint
- **kernel:** implement missing StreamInference + GetHealth RPCs
- **router:** honor explicit Critical from health probe immediately
- **kernel:** add lib.rs for integration tests + fix SurrealDB binds
- **main:** import InferencePort trait for capability() method
- **rest:** use port 3030 (3000 is Forgejo), configurable via CYNIC_REST_ADDR
- **deps:** pin surrealdb to 2.2 — version 3.0.3 has compilation bug
- **kernel:** prompt judges substance not form, upgrade to Gemini 3
- **kernel:** lenient JSON parsing for small models, 3 Dogs active
- **kernel:** reduce backend timeout to 30s, fix HF inference provider URL
- **kernel:** human-readable reasoning, rename sovereign-ubuntu → gemma-sovereign
- **kernel:** anomaly detection on per-axiom spread, not Q-Score total
- **kernel:** DB schema + indexes, unified usage tracker, remove dead gRPC code
- **kernel:** SQL injection, input bounds, unified klog macro
- **kernel:** circuit breaker race, unified thresholds, centralized DB config
- **kernel:** remaining security hardening
- **rest:** SELECT VALUE in /agents + remove dead HealthResponse struct
- **inference:** clarify FIDELITY axiom — judge substance, not description accuracy
- **storage:** use agent_id instead of session_id for co-occurrence queries
- **ccm:** co-occurrence query groups by agent_id instead of empty session_id
- **coord:** resolve 8 L2 coordination debts + add expire_stale
- **ccm:** observe_crystal compute state in Rust, not nested IF...END SQL
- **ccm:** deduplicate crystals — hash summary not full content
- **kernel:** replace Mutex .unwrap() with poison recovery in all production paths
- **kernel:** usage flush correctness + gate BackendRouter behind grpc
- **kernel:** eliminate IO debt — industrial-grade async hygiene
- **kernel:** HOWL threshold — replace arbitrary 0.82 with φ-derived golden subdivision
- **kernel+hooks:** SurrealDB 3.x audit fix, REST heartbeat, coord deny
- **kernel:** compound audit — 14 interconnected fixes across score pipeline and data integrity

### Hardened

- **hooks+kernel:** audit-driven fixes from 4 parallel code reviews
- **kernel:** security surface + graceful shutdown — 6 fixes

### Other

- remove unused imports in main.rs
- **deps:** bump surrealdb 2.1 → 3.0.3
- Revert "feat(hal): make SurrealDB optional — kernel boots without memory service if auth fails"
- keeping Replit frontend changes
- **kernel:** auth middleware, rate limiting, injection hardening
- **kernel:** audit log, /judge rate limit, /health info reduction
- **kernel:** 60s evaluation timeout, backtick IDs, Dogs filter hardening
- **kernel:** token bucket rate limiter + atomic crystal updates
- **kernel:** circuit breaker only trips on infrastructure failures
- **kernel:** audit purge, dynamic temporal, collision-resistant crystal IDs
- **kernel:** 7 hardening fixes — input validation, per-IP rate limit, constant-time auth, CORS restrict, bind localhost, per-dog timeout, error masking

### Performance

- **kernel:** shorter reasoning prompt — 18s → 13s (-28% latency)

### Testing

- **storage:** real SurrealDB integration test — no mocks
- **kernel:** 86 tests — judge, router, rest fully covered
- **kernel:** add 39 unit tests — middleware, usage, judge, observe


