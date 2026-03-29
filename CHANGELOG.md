# Changelog — cynic-kernel

All notable changes. Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.7.4] — 2026-03-29

### Fixed

- **api:** crystal state lowercase + ghost dogs filtered in /usage
- **make:** add lint-security gate — detects OPEN CRIT/HIGH findings
- **domain:** add domain field to Verdict — was lost at Stimulus→Verdict boundary
- **hygiene:** organic analysis — unblock make check, purge DB fossils, close CH2
- **repo:** README reflects reality, sovereign CI gate, env.example
- **observability:** structured logging for 3 silent organs

### Other

- **docs:** consolidate audit docs + fix VERSION.md stale ref
- distill session — Rule 21 (verify gates catch violations)
- **docs:** purge 35 stale docs, restructure into identity/reference/audit/research
- distill session — R22 (USE before architecture)

## [0.7.4] — 2026-03-29

### Added

- **kernel:** v0.8 crystal lifecycle integrity — 6 boundary enforcements
- **kernel:** InMemory StoragePort adapter — v0.8 G2 foundation
- **workflow:** make lint-drift — deterministic drift detection gate
- **kernel:** event bus internal consumer — DogFailed/Anomaly now logged + tracked
- **kernel:** session compliance scoring — process loop Phase 2
- **kernel:** add --version flag — binary self-identification (L1)
- **domain:** add SystemMetricsPort trait — proprioception contract (L3)
- **infra:** add SysinfoMetrics adapter — real system sensing (L4a)
- **kernel:** broadened introspection — T8 fix + system metrics (L2+L4b)
- **mcp:** add cynic_observe tool — agent write path (L5b)

### Build

- **workflow:** industrial enforcement stack L0-L3 — 8-layer architecture
- **workflow:** git-cliff + cargo-release — automated versioning pipeline

### Changed

- **workflow:** L6-L7 context layer — CLAUDE.md 196→52 lines, .claude/rules/, docs/ reorg
- **kernel:** single source of truth for crystal thresholds — Rule 14
- **kernel:** std::sync::Mutex to tokio::sync::Mutex in InMemory adapter
- **kernel:** Display+FromStr for CrystalState — eliminate 3 duplicate mappings
- **devops:** migrate hooks to settings.json — git-tracked, reproducible
- **infra:** fleet rename — TAILSCALE_UBUNTU→CORE, TAILSCALE_STANISLAZ→GPU
- **rules:** three-tier enforcement model — deduplicate, renumber, audit
- **domain:** extract infer_domain to domain/ccm — K5 fix + infra mappings (L5a)

### Documentation

- **audit:** deep audit — 5 attack chains, 6 emergent patterns, 3 deep root causes
- **session:** next session prompt — industrial engineering methodology
- **research:** L6-L7 findings — 5 agents, academic + industrial + Hermes + SDK
- **architecture:** perennial epistemology identity + v0.8 architectural truths
- **rules:** Rule 34 — falsify before adopting architectural decisions
- **sessions:** next session prompt — v0.8 falsification protocol
- **sessions:** next session prompt — CI/CD + straightforward fixes + trait split
- **roadmap:** v0.8→v1.0 design spec — 3 gates/version, organic, reviewed
- **version:** align VERSION.md with roadmap spec — 3 gates/version
- **sessions:** next session prompt — InMemory contract tests + self-observability
- **spec:** agent workflow design — 4-layer deterministic-first architecture
- **infra:** CYNIC Infrastructure Spec v0.1 — 16 gaps identified, 8 covered
- **infra:** add G17-G19 — proprioception, sovereignty violation, SPOF detection
- **infra:** v0.1.1 — platform matrix, corrected roadmap, agent as lifecycle manager
- **spec:** proprioception + MCP write path design — v0.7.4
- **plan:** proprioception implementation plan — 8 tasks, reviewed

### Fixed

- **kernel:** v0.8w2 — 8 findings + 2 live DB bugs, 287 tests
- **kernel:** DeterministicDog F9/F10/F11 — 3 heuristic bugs, benchmarked
- **ref:** remove fleet-gen.py reference — script was deleted as over-engineering
- **lint-drift:** skill check reads workflow.md, hook check reads settings.json
- **hooks:** force C locale in session-stop printf — French locale fix
- **pipeline:** skip KNN embedding for cynic-internal domain — anti-contamination (L2)

### Other

- clean workspace — commit orphan docs + Cargo.lock version bump
- **workflow:** /build now delegates to make check — single entry point
- clean working tree — commit architecture docs, gitignore agent-memory
- bump version to v0.7.4 — proprioception + MCP write path

### Testing

- **kernel:** v0.8w3 — 5 contract tests + backtick bugfix, 292 tests

## [0.7.3] — 2026-03-25

### Added

- **kernel:** domain prompts + crystals A/B flag + chess benchmark
- **kernel:** φ-derived crystal budget + chess crystal injection script
- **kernel:** add trading domain prompt for KAIROS integration
- **kernel:** epistemic soft gate for crystal feedback loop

### Documentation

- **rules:** upgrade Rule #32 gate — catch path-qualified refs
- **session:** update reality audit with results + new weakness #8
- **audit:** deep industrial audit — 67 findings, 8 root causes, CYNIC=BARK
- **rules:** add Rule #33 — every runtime producer needs a consumer

### Fixed

- **kernel:** distinguish abstention from disagreement in anomaly detection
- **rc8:** security hygiene — status-code health checks, safe default bind
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
- **make:** comprehensive E2E + Rule #32 lint
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
- **constitution:** add development rules 8-12 from kernel audit
- **meta:** 5 crystallized truths from metathinking analysis
- **constitution:** rules 13-16 from fix-and-shift eradication
- marathon session diagnostic, ingestion design, KPIs, and overnight research
- add distill trigger to CLAUDE.md + CCM compound protocol
- rewrite production hardening with research-backed priority tiers
- KAIROS-on-CYNIC design doc — two-tier state model, 4-phase migration

### Fixed

- **kernel:** rate limiter eviction off hot path → background timer
- **kernel:** /judge now respects global rate limit
- **kernel:** StorageError + CoordError implement std::error::Error
- **infra:** rollback capability + backup verification + rtk exclusion
- **infra:** Makefile production-grade — hotfix, restore, clean, retry loop
- **kernel:** robustness hardening + per-dog cost tracking
- **kernel:** sovereign thinking model support + timeout alignment
- **mcp:** background tasks + error handling + coordination audit trail
- **kernel:** health endpoint 30s→3s timeout + load all backends at boot
- **kernel:** review fixes from parallel session audit
- **security:** production hardening — backup, monitoring, input validation
- **infra:** protect-files pre-edit blocking + RUNBOOK.md
- **kernel:** coord claim TOCTOU deadlock + VerdictCache O(1) eviction
- **kernel:** compiler as impact checker + API contract tests + health O(1)
- **health:** return HTTP 503 on degraded/critical — industry standard
- **deploy:** integration tests + healthcheck self-test in pipeline
- **workflow:** session-init injects CCM crystals + fix dog count auth
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

- clean worktree — gitignore experiments + Cargo.lock
- **kernel:** last let _ = escape comment (middleware audit)
- gitignore history.txt (SurrealDB scratch queries)
- Merge branch 'docs/kairos-on-cynic-design'
- **kernel:** bump version to 0.7.0 — v0.7 "Le Socle Architectural"

### Removed

- **infra:** -7 scripts +6 systemd units in repo

### Rules

- **claude:** #29 deploy-from-main, #30 commit-before-complete, #8 strengthened
- **claude:** #31 measure before AND after every kernel change

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
- **ui:** chess judge — structured content/context for meaningful LLM evaluation
- **kernel:** shutdown flush + dead code burn + MCP client reuse
- **kernel:** EmbeddingBackend 5-state health + probe $HOME → dirs
- **kernel:** replace audit DELETE subquery with date-based cleanup
- **kernel:** retry on SurrealDB 3.x intermittent 401
- **kernel:** 3 Google-review blockers + systemd hardening

### Other

- Cargo.lock after gRPC dep removal + gitignore update
- **kernel:** version 0.6.0 — align Cargo.toml with reality

### Removed

- **kernel:** gRPC stubs + robustness fixes — -761 lines

## [0.5.0] — 2026-03-18

### Added

- sovereign foundation — single-crate workspace, InferencePort P0
- **proto:** timestamp uint64 unix nanos + span_id + parent_span_id
- **forge:** add deploy.sh — release build + systemd auto-deploy on push
- **backend:** make InferencePort object-safe via async-trait + add model_hint
- **muscle:** add LlamaCppBackend — InferencePort for llama.cpp HTTP API
- **muscle:** add BackendRouter — route + fan_out with round-robin and health filtering
- **hal:** rewrite — route through BackendRouter instead of hardcoded stub
- **hal:** make SurrealDB optional — kernel boots without memory service if auth fails
- **foundation:** 10 skills + CLAUDE.md constitution + crystallized truth
- **kernel:** 5-state health + circuit breaker + proto fixes
- **mvp:** hackathon judgment engine — Dog trait + Judge + REST API
- **kernel:** hexagonal refactor — ChatPort, OpenAiCompatBackend, InferenceDog, parallel Judge
- **setup:** zero-friction Ubuntu setup + backends.toml example
- **kernel:** extend axiom system from 3 to 6 — CULTURE, BURN, SOVEREIGNTY
- **kernel:** add CCM — Cognitive Crystallization Mechanism
- **kernel:** add MCTS Temporal — 7 temporal perspectives with phi exploration
- **sovereignty:** enrich backends.toml with multi-family diversity guidance
- **kernel:** HTTP storage adapter, burn dead code, boot gracieux
- **hackathon:** multi-agent coordination, shared commands, public tunnel
- **demo:** terminal fallback demo script with formatted output
- **ui:** scaffold CYNIC chess+text judge dashboard
- **kernel:** Dog selection API — frontend chooses which Dogs to use
- **kernel:** enriched /health with Dog status, smarter deterministic-dog
- **kernel:** CCM feedback loop — CYNIC learns from verdicts
- **kernel:** per-Dog latency tracking + persistent SurrealDB
- **ui:** full backend alignment (Dog Selection, Latency, CCM)
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
- SessionStart hook + zero hardcoded paths rule
- universal Makefile pipeline — structural enforcement replaces discipline
- **kernel:** multi-agent coordination via MCP — register, claim, release, who
- **kernel:** deterministic-dog as FORM judge, SCORE_FLOOR, FNV-1a hash
- **workflow:** cynic-empirical + cynic-workflow skills + fix plan awk
- **workflow:** Phase A — add scope/done/agents Makefile targets
- **workflow:** Phase A — session-init outputs agent_id + coord protocol
- **workflow:** Phase B — Session Lifecycle + coord fix + Claude MCP config
- **kernel:** CCM feedback loop — crystallized wisdom injected into Dog prompts
- **ui:** auth headers + env var API base + API key in Settings
- **kernel:** CCM workflow observation pipeline — capture, aggregate, crystallize
- **kernel:** CCM co-occurrence patterns — detect files edited together
- **hooks:** session lifecycle + session_id migration + dog drift detection
- **usage:** persist DogUsageTracker to SurrealDB — survives restarts
- **kernel+hooks:** REST coord endpoints + hook enforcement
- **kernel:** EmbeddingPort + sovereign embedding backend
- **kernel:** coord_claim_batch — claim N targets in one call
- **kernel:** git describe in /health — traceable builds without manual versioning

### Build

- test pipeline end-to-end
- **forge:** add scripts/validate.sh — clippy + test + audit
- **forge:** fix hook REPO_NAME from pwd
- **forge:** fix forge-lib bypass detection
- **forge:** silence cargo audit if not installed

### Changed

- **ui:** fix lints and unite utilities for stable HMR
- **domain:** extract DogUsageTracker from rest adapter — fix V1 cross-adapter coupling
- **domain:** InferenceRouter trait — decouple hal from concrete BackendRouter (fix V4)
- **domain:** add CoordPort trait + SurrealHttpStorage implementation
- **domain:** CoordPort wiring — eliminate raw SurrealHttpStorage bypass (fix V2+V3)
- **kernel:** hexagonal module structure — 23 flat files → 9 modules
- **probe:** split mod.rs (896 LOC) into 5 focused files
- **rest:** split mod.rs (779 LOC) into 7 focused files
- **kernel:** stabilize build, gate gRPC, remove dead code

### Documentation

- **plan:** add Chunk 5 — architectural maturity axes from crystallize-truth
- **spec:** tailscale-mcp design — Go MCP server for Tailscale network
- **spec:** cynic-muscle design — LLM orchestration connecting kernel abstractions
- **spec:** address 12 review findings — object-safety, thread-safety, cold-start, config shape
- **plan:** cynic-muscle pipeline — 6 tasks from object-safety to end-to-end smoke test
- **arch:** mark Forgejo pipeline active — test end-to-end flow
- **arch:** test Forgejo webhook pipeline end-to-end
- **arch:** verify end-to-end Forgejo pipeline — local to deploy
- **arch:** pipeline test with env HOME fix
- **arch:** test systemd-run pipeline trigger
- **hackathon:** add onboarding docs, infra scripts, sovereign-ubuntu backend
- add INFERENCE-MATRIX.md — complete model selection & hardware allocation
- complete LLM landscape research — 7 categories, 569 models analyzed
- CYNIC Sovereignty Vision — from judgment engine to sovereign agent
- restructure INFERENCE-MATRIX around use cases, not just judgment
- 3 crystallized design docs — routing, compression, GPU optimization
- security rules for LLM agents — public repo protection
- **workflow:** crystallize universal workflow design spec
- **workflow:** universal workflow implementation plan — 3 chunks, 8 tasks
- **workflow:** Phase 0 — burn trigger table to 12 canonical items
- **workflow:** Phase A — GEMINI.md make check + Session Protocol
- add README — project overview, architecture, API, quickstart
- add infrastructure roadmap — inference topology, protocols, agent orchestration, compound phases
- **roadmap:** update Phase 1 done, thinking mode architecture, fleet status
- add hexagonal refactor plan + spec from 2026-03-16
- align workflow, skills, and scripts with gRPC gate + build reality
- **skill:** align cynic-kernel.md with code reality
- **skills:** crystallize skill design + φ-convergence, fix workflow debt
- align all thresholds on φ-pure values + reconcile 6 axioms everywhere
- **skills:** consolidate skill ecosystem — eliminate drift, enforce SoT
- **skills:** merge cynic-worktrees into cynic-workflow, delete duplicate
- compound research — stars analysis, model benchmarks, kernel audit

### Fixed

- prost name normalization + surrealdb type ambiguity + lifetime
- **clippy:** resolve all -D warnings — unused imports, doc comments, collapsible ifs
- **storage:** add signin to SurrealDB connection — 400 on unauthenticated RPC
- **storage:** use ws:// protocol — /rpc requires WebSocket on SurrealDB 3.x
- **storage:** surrealdb 3.x Root fields are String not &str
- **probe:** scan HOME+/home instead of / on Linux
- **storage:** no default password — SURREALDB_PASS must be set explicitly
- **router:** collapse nested ifs to satisfy clippy collapsible_if lint
- **deploy:** atomic mv + stop-before-replace + env file + systemd hardening
- **kernel:** implement missing StreamInference + GetHealth RPCs
- **router:** honor explicit Critical from health probe immediately
- **kernel:** add lib.rs for integration tests + fix SurrealDB binds
- **main:** import InferencePort trait for capability() method
- **rest:** use port 3030 (3000 is Forgejo), configurable via CYNIC_REST_ADDR
- **setup:** clone skills from private repo, remove forge refs
- **deps:** pin surrealdb to 2.2 — version 3.0.3 has compilation bug
- **kernel:** prompt judges substance not form, upgrade to Gemini 3
- **kernel:** lenient JSON parsing for small models, 3 Dogs active
- **kernel:** reduce backend timeout to 30s, fix HF inference provider URL
- **ui:** stable merge of T. rewrite & TS fixes for demo
- **kernel:** human-readable reasoning, rename sovereign-ubuntu → gemma-sovereign
- **kernel:** anomaly detection on per-axiom spread, not Q-Score total
- **kernel:** DB schema + indexes, unified usage tracker, remove dead gRPC code
- **kernel:** SQL injection, input bounds, unified klog macro
- **kernel:** circuit breaker race, unified thresholds, centralized DB config
- **kernel:** remaining security hardening
- skills + CLAUDE.md — dynamic addresses, auth, DB backup, tool ecosystem map
- **workflow:** mask real IP in session hook, improve agents error message
- **deploy:** copy cynic-mcp alongside cynic-kernel on deploy
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
- **hooks:** auto-claim kernel files — no more manual cynic_coord_claim
- **hooks:** use relative path for coord claims, not basename

### Hardened

- **hooks+kernel:** audit-driven fixes from 4 parallel code reviews
- **kernel:** security surface + graceful shutdown — 6 fixes

### Other

- enforce LF line endings across all platforms
- remove unused imports in main.rs
- add .worktrees to gitignore
- **deps:** bump surrealdb 2.1 → 3.0.3
- Revert "feat(hal): make SurrealDB optional — kernel boots without memory service if auth fails"
- **constitution:** remove 3 phantom skills from skill table
- update Cargo.lock
- Update project to use `import type` for TypeScript type imports
- Add full chess game functionality and configurable kernel URL
- Fix issues with chess game functionality and button interactions
- keeping Replit frontend changes
- **kernel:** auth middleware, rate limiting, injection hardening
- PreToolUse hook — blocks edits to secrets, SSH keys, git hooks
- **kernel:** audit log, /judge rate limit, /health info reduction
- **kernel:** 60s evaluation timeout, backtick IDs, Dogs filter hardening
- **kernel:** token bucket rate limiter + atomic crystal updates
- **kernel:** circuit breaker only trips on infrastructure failures
- **kernel:** audit purge, dynamic temporal, collision-resistant crystal IDs
- **kernel:** 7 hardening fixes — input validation, per-IP rate limit, constant-time auth, CORS restrict, bind localhost, per-dog timeout, error masking
- **hooks:** harden protect-files — cover MultiEdit, all git hooks, settings.local.json
- **skills:** deduplicate frontmatter — auto-invocation comment replaces inline metadata

### Performance

- **ci:** clippy --all-targets to share build with cargo test
- **ci:** skip rust validation+deploy when no .rs/.toml changed
- **kernel:** shorter reasoning prompt — 18s → 13s (-28% latency)

### Testing

- **storage:** real SurrealDB integration test — no mocks
- **kernel:** 86 tests — judge, router, rest fully covered
- gitleaks toml should pass clean
- **kernel:** add 39 unit tests — middleware, usage, judge, observe


