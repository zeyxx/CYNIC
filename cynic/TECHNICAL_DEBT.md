# Technical Debt — CYNIC Kernel

> Documented during Foundation Cleanup (2026-02-23)
> Confidence: 58% (φ-bounded, audit findings)
> Last Updated: 2026-02-23

---

## Overview

This document captures code-level technical debt discovered during the Foundation Cleanup audit (Sessions 1-14, 2026-02-16 to 2026-02-23). The debt is organized by severity tier with effort estimates and actionable remediation steps.

**Total Debt Estimate**: 60-80 hours to resolve all BLOCKER and HIGH items.

---

## Debt Tiers

### BLOCKER (Must fix before Phase 2)

#### DEBT-T1-001: Exception Handling Swallows Errors

- **Title**: Bare `except Exception:` handlers eliminate debugging visibility
- **Severity**: BLOCKER
- **Impact**: Silent failures in critical paths (chat session writes, event bus handlers, worker supervision). Makes production debugging nearly impossible.
- **Effort**: 8 hours (systematic elimination ongoing since Session 9)
- **Root Cause**: Early codebase used defensive broad catches. Session 9 fixed 374 instances, but ~50-100 remain in peripheral modules.
- **Evidence**:
  - Session 9 fixed: docker_manager.py (4 instances), chat.py (3 instances), others
  - Session 10+ focused on isolated modules
  - Full audit needed to find remaining instances
- **Action**:
  1. Run `grep -r "except Exception:" cynic/ --include="*.py"` to find all remaining
  2. For each, classify the exception type (asyncpg.Error, json.JSONDecodeError, etc.)
  3. Replace with specific exception handler or re-raise
  4. Add unit test to verify exception propagates
  5. Target: Zero bare `except Exception:` outside venv

**Status**: ~50% complete. Continue from Session 10 baseline.

---

#### DEBT-T1-002: Chat Session Race Conditions (File-Based Persistence)

- **Title**: Concurrent writes to session JSON files cause data corruption
- **Severity**: BLOCKER
- **Impact**: Multiple simultaneous API requests corrupting session state. User conversations lost. Data integrity compromised.
- **Effort**: 4 hours
- **Root Cause**: ChatSessionManager uses file-based persistence (`~/.cynic/chats/{session_id}.json`) without file locking. Python's `json.dump()` can interleave writes if multiple processes access same file.
- **Evidence**:
  - Session 7 (2026-02-22) replaced chat stub with real persistence
  - But no locking mechanism implemented
  - Concurrent request tests not written
- **Action**:
  1. Add `filelock` (Python package) dependency to handle exclusive access
  2. Wrap session writes with `FileLock(f"~/.cynic/chats/{session_id}.lock")`
  3. Add concurrent write test: 10 simultaneous requests to same session → verify JSON integrity
  4. OR migrate to PostgreSQL (chat_sessions table exists but not used for persistence)
  5. Target: All writes protected by either file lock or DB transaction

**Status**: Not started. Recommend migration to PostgreSQL (better long-term).

---

#### DEBT-T1-003: SONA Orchestrator Not Implemented

- **Title**: 11-loop learning system coordinator missing
- **Severity**: BLOCKER
- **Impact**: Cannot start Phase 2 learning work. 11 dogs (Q-Learning, Thompson, EWC, etc.) exist but are never coordinated. Kernel metrics (confidence, φ-bound) work but learning loops are dormant.
- **Effort**: 16 hours
- **Root Cause**: Architecture designed (SONA = CYNIC Organism Neural Amplifier), but implementation deferred to Phase 2. Currently: event buses exist, dogs exist, but orchestration is missing.
- **Evidence**:
  - `cynic/kernel/sona.py` marked "Phase 1 stub"
  - 11 learning loops defined in reference docs but not wired
  - Tests for individual dogs work, but no integration test
- **Action**:
  1. Implement SONA.run() main loop: repeatedly sample from 11 dogs, aggregate Q-scores
  2. Wire SONA to event bus: listen for feedback events, route to appropriate dog learners
  3. Add resync mechanism: Dogs update shared Q-table in PostgreSQL every N iterations
  4. Add metrics: track convergence, confidence evolution, loop health
  5. Add integration test: 100 iterations × 11 dogs → verify Q-table updates
  6. Target: SONA active and learning by Phase 2 start

**Status**: Architecture ready, implementation not started.

---

#### DEBT-T1-004: Event Bus Bridge Verification Under Load

- **Title**: 3-bus architecture untested at scale
- **Severity**: BLOCKER (High priority variant of HIGH)
- **Impact**: Unknown behavior under concurrent load. May drop events, deadlock, or cascade failures. Phase 2 depends on reliable event routing.
- **Effort**: 6 hours
- **Root Cause**: Session 3 (2026-02-21) designed 3 buses (Core, Automation, Agent) with bridges. Session 8 verified end-to-end connectivity (Task 3 passed), but stress testing never done.
- **Evidence**:
  - Integration tests pass (nominal case)
  - No load tests beyond Task 3 example
  - Unknown throughput limits, latency under contention
- **Action**:
  1. Create `tests/kernel/test_event_bus_load.py` with 3 scenarios:
     - 100 events/sec on single bus → measure latency percentiles (p50, p95, p99)
     - 10 concurrent subscribers per bus → verify no cross-talk
     - Injected handler exceptions → verify isolation (no cascade)
  2. Establish baseline thresholds: p99 latency < 100ms, no dropped events
  3. Add to CI/CD: run load test on every commit
  4. Target: Documented performance envelope, confidence > 60%

**Status**: Not started. Critical for Phase 2 stability.

---

### HIGH (Should fix in Phase 2)

#### DEBT-H1-001: Plaintext Conversation Storage

- **Title**: User conversations stored unencrypted at rest
- **Severity**: HIGH
- **Impact**: Security risk. Any file system access leaks all user conversations. Privacy violation.
- **Effort**: 4 hours
- **Root Cause**: ChatSessionManager writes `~/.cynic/chats/{session_id}.json` with no encryption. "Security hardening" deferred to Phase 2.
- **Evidence**:
  - Session 7 (2026-02-22) added chat persistence
  - No encryption layer implemented
  - .gitignore excludes chats/ but file system still exposed
- **Action**:
  1. Add `cryptography` (Fernet symmetric) dependency
  2. Generate per-user encryption key from hashed password (or master key from .env)
  3. Encrypt payload before `json.dump()`, decrypt after `json.load()`
  4. Add test: write encrypted, verify file contains no plaintext, decrypt and match
  5. Target: All chat files encrypted with AES-256 (Fernet standard)

**Status**: Not started.

---

#### DEBT-H1-002: ResidualDetector Phase 2 Persistence Not Implemented

- **Title**: Detector state lost on restart
- **Severity**: HIGH
- **Impact**: Learning loop resets to zero. Confidence in bug detection oscillates. Can't build longitudinal patterns.
- **Effort**: 4 hours
- **Root Cause**: ResidualDetector exists in-memory only. PostgreSQL table `residual_detections` created (Session 10) but not wired to detector.
- **Evidence**:
  - Session 10 created residual_detections table schema
  - Session 11 identified framework needs persistence
  - But write operations to DB never implemented
- **Action**:
  1. Add `ResidualDetector.save_state()` method: serialize detector.cache to JSON, write to DB
  2. Add `ResidualDetector.load_state()` on init: load cache from DB if exists
  3. Schedule periodic save: every N detections or every 5 minutes
  4. Add test: create detector, add 10 residuals, restart, verify cache intact
  5. Target: Detector state survives restarts

**Status**: Schema ready, persistence not implemented.

---

#### DEBT-H1-003: Event Bus Bridge Wiring Partially Tested

- **Title**: 3-bus bridge configuration only tested in nominal case
- **Severity**: HIGH
- **Impact**: Unknown failure modes. No verification of backpressure handling, error propagation, or reordering.
- **Effort**: 6 hours
- **Root Cause**: Session 3 designed bridges, Session 8 verified one happy path. Didn't test edge cases.
- **Evidence**:
  - Task 3 (2026-02-21) passed with nominal scenario
  - No tests for: slow subscribers, handler exceptions, reordered messages
- **Action**:
  1. Add test: slow subscriber on Core bus → verify other buses unblocked
  2. Add test: handler raises exception → verify event isolated, other handlers run
  3. Add test: bridge A→B→C message ordering → verify deterministic
  4. Add test: simultaneous pub on A and B → no cross-talk
  5. Target: 15+ edge case tests, all passing

**Status**: Nominal case done, edge cases not tested.

---

#### DEBT-H1-004: ContextCompressor Not Implemented

- **Title**: Long conversations lose history abruptly when limit reached
- **Severity**: HIGH
- **Impact**: User experience degrades for long sessions. No graceful degradation of context. Token efficiency poor.
- **Effort**: 8 hours
- **Root Cause**: Current chat session just truncates old messages. True compression (token-aware, semantic summarization) deferred.
- **Evidence**:
  - ChatSession._truncate_history() deletes messages > 50 (arbitrary limit)
  - No token counting, no compression
  - Comment: "Future: ContextCompressor"
- **Action**:
  1. Implement ContextCompressor class:
     - Count tokens in message (use tiktoken for Claude tokenization)
     - When total > 4000 tokens, compress oldest 25% of messages
     - Compression: extract key facts (entities, decisions) into summary
     - Replace 10 messages with 1 summary message
  2. Add test: 100 messages → compress → verify token count < 4000, summary present
  3. Integrate into ChatSession.add_message()
  4. Target: Long conversations maintain usable context indefinitely

**Status**: Not started. Significant effort but high value for UX.

---

#### DEBT-H1-005: MCP Learning Feedback Integration Unclear

- **Title**: Feedback from MCP tools not wired to learning system
- **Severity**: HIGH
- **Impact**: Learning loops can't improve from MCP tool results. No signal for Q-Learning to update estimates.
- **Effort**: 4 hours (investigation + wiring)
- **Root Cause**: Session 14 (2026-02-23) integrated real chat endpoint, but feedback loop design left unclear. MCPBridge produces results, but how does that feed back to Q-table?
- **Evidence**:
  - Chat response flows through MCP (via EventBus)
  - But no ActionProposal → Q-Learning feedback path documented
  - High #4 from TIER 1 blockers still open
- **Action**:
  1. Map feedback path: MCPResult → ActionProposal feedback → EventBus → Q-Learning
  2. Define feedback schema: success (0-1), latency, cost, confidence
  3. Add handler in SONA: listen for Q-feedback events, update dog states
  4. Add test: mock MCP result → Q-table updates in response
  5. Target: Q-Learning receives and processes feedback events

**Status**: Architecture unclear, integration not started.

---

#### DEBT-H1-006: No Automated Type Checking

- **Title**: Codebase has type hints but no mypy/pyright enforcement
- **Severity**: HIGH
- **Impact**: Type errors slip through. Hard to maintain type safety across large refactors.
- **Effort**: 2 hours (setup) + 8 hours (fixing violations)
- **Root Cause**: pyproject.toml updated (Task 2) but mypy not enabled in CI. Developers don't run it locally.
- **Evidence**:
  - Task 2 updated pyproject.toml with tool.mypy config
  - But no pre-commit hook, no CI enforcement
  - Type hints exist but not validated
- **Action**:
  1. Run `mypy cynic/ --strict` locally to get baseline violations count
  2. Add mypy to pre-commit hooks (.pre-commit-config.yaml)
  3. Add mypy check to GitHub Actions CI
  4. Fix high-priority type violations (> 50 occurrences)
  5. Target: `mypy cynic/ --strict` returns 0 errors

**Status**: Configuration ready, enforcement not active.

---

### MEDIUM (Nice to fix)

#### DEBT-M1-001: Event Bus Observability Limited

- **Title**: No metrics on event bus health (throughput, latency, drop rate)
- **Severity**: MEDIUM
- **Impact**: Can't detect degradation. Hard to optimize. Production visibility poor.
- **Effort**: 6 hours
- **Root Cause**: Event bus designed for reliability but not instrumented. No Prometheus metrics exposed.
- **Evidence**:
  - Core bus exists and works
  - No metrics collection points
  - Task 6 (MEDIUM item from TIER 1) mentioned metrics dashboard but not core bus metrics
- **Action**:
  1. Add counters: events_published, events_processed, events_dropped per bus
  2. Add histograms: handler_latency_ms, subscription_lag_ms
  3. Expose as Prometheus endpoint: /metrics
  4. Add dashboard: Grafana showing 3 buses side-by-side
  5. Target: Operational visibility into bus health

**Status**: Not started. Non-blocking for Phase 2.

---

#### DEBT-M1-002: No Database Automated Backups

- **Title**: PostgreSQL running without backup strategy
- **Severity**: MEDIUM
- **Impact**: Data loss if container dies. No recovery procedure.
- **Effort**: 4 hours
- **Root Cause**: Docker Compose runs PostgreSQL, but no backup configuration. Task 6 (MEDIUM) mentioned but deferred.
- **Evidence**:
  - docker-compose.yml spins up Postgres but no volumes persist
  - No backup service configured
- **Action**:
  1. Add named volume for Postgres data persistence
  2. Create backup script: `pg_dump` daily to S3
  3. Add restore test: simulate restore from backup
  4. Document backup procedure in RUNBOOK.md
  5. Target: Daily automated backups, tested recovery

**Status**: Not started.

---

#### DEBT-M1-003: Load Testing Baseline Not Established

- **Title**: No performance baseline; unknown scaling characteristics
- **Severity**: MEDIUM
- **Impact**: Can't validate performance assumptions. No SLA targets. Unknown bottlenecks.
- **Effort**: 6 hours
- **Root Cause**: API works but never tested at scale. Task 6 (MEDIUM) from TIER 1 mentioned profiling but deferred.
- **Evidence**:
  - API endpoints functional
  - No k6/wrk load tests
  - No latency targets defined
- **Action**:
  1. Create k6 load test: /api/chat endpoint, 10 RPS sustained, 100 RPS burst
  2. Measure: p50, p95, p99 latencies; error rate; throughput ceiling
  3. Establish baselines: target p99 < 500ms, error_rate < 0.1%
  4. Repeat monthly: track regression
  5. Target: Documented SLA, baseline performance trending

**Status**: Not started.

---

#### DEBT-M1-004: asyncio.get_event_loop() Deprecation Warnings

- **Title**: Some modules use deprecated asyncio pattern
- **Severity**: MEDIUM (low impact but easy fix)
- **Impact**: Python 3.10+ warnings. Technical debt accumulation. Bad code example for contributors.
- **Effort**: 2 hours
- **Root Cause**: Task 2 updated pyproject.toml but didn't fix all code patterns. Early modules use `asyncio.get_event_loop()` instead of `asyncio.new_event_loop()`.
- **Evidence**:
  - Task 2 checked pyproject.toml only
  - Didn't audit actual code for deprecated patterns
- **Action**:
  1. Run: `grep -r "get_event_loop()" cynic/ --include="*.py"`
  2. Replace with: `asyncio.new_event_loop()` or `asyncio.get_running_loop()` (context-dependent)
  3. Add test: run code with Python 3.10+ warnings enabled, expect 0 warnings
  4. Target: Zero deprecation warnings

**Status**: Not started. Low priority.

---

### LOW (Cosmetic / Documentation)

#### DEBT-L1-001: Sparse Developer Documentation

- **Title**: New developers struggle to set up and understand architecture
- **Severity**: LOW
- **Impact**: Onboarding slow. High knowledge loss if key engineer leaves.
- **Effort**: 8 hours
- **Root Cause**: Focused on code delivery, documentation trailing. Architecture changed multiple times (Session 3, 10, etc.).
- **Evidence**:
  - No SETUP.md for developers
  - No architecture diagram
  - No runbook for common operations
- **Action**:
  1. Create `docs/SETUP.md`: Python version, dependencies, venv, environment variables, database init
  2. Create `docs/ARCHITECTURE.md`: ASCII diagram of kernel components, event buses, 11 dogs
  3. Create `docs/RUNBOOK.md`: how to start, stop, restart, debug, view logs
  4. Target: New developer can set up in 30 minutes without support

**Status**: Not started.

---

#### DEBT-L1-002: Tests for error paths incomplete

- **Title**: Happy path tests exist but error cases sparse
- **Severity**: LOW
- **Impact**: Fragile code. Unexpected errors slip through.
- **Effort**: 12 hours (systematic coverage improvement)
- **Root Cause**: Focus on delivery, test coverage lagging. Current ~9% (Session 10 noted).
- **Evidence**:
  - Task coverage estimates showed 9.1% baseline
  - Many modules have 0 tests (util/, helper functions)
  - Error paths not exercised
- **Action**:
  1. Audit test coverage: `pytest --cov=cynic --cov-report=html`
  2. Prioritize high-risk modules (database, event bus, chat)
  3. Add error path tests: mock failures, verify graceful degradation
  4. Target: 40%+ overall coverage, 70%+ for critical modules

**Status**: Not started. Will improve incrementally.

---

## Debt Reduction Strategy

### Recommended Fix Order (by ROI)

**Phase 2 Blocker Fixes (First Week)**:
1. DEBT-T1-001: Exception handlers (8h) — Unblocks debugging
2. DEBT-T1-002: Chat race conditions (4h) — Unblocks reliable chat
3. DEBT-T1-004: Event bus load test (6h) — Unblocks Phase 2 confidence

**Subtotal**: 18 hours

**Phase 2 High-Priority Fixes (Week 2-3)**:
4. DEBT-T1-003: SONA orchestrator (16h) — Unblocks learning
5. DEBT-H1-001: Chat encryption (4h) — Security baseline
6. DEBT-H1-002: ResidualDetector persistence (4h) — Detector reliability
7. DEBT-H1-005: MCP feedback integration (4h) — Learning signals

**Subtotal**: 28 hours

**Phase 2 Medium Fixes (As needed)**:
8. DEBT-H1-004: ContextCompressor (8h) — UX improvement
9. DEBT-M1-001: Event bus observability (6h) — Operational visibility
10. DEBT-M1-002: DB backups (4h) — Data safety

**Subtotal**: 18 hours

**Total Effort to Clear BLOCKER + HIGH + Key MEDIUM**: 64 hours (~2 weeks, 1 engineer)

---

## Monitoring Debt Growth

**Monthly Audit Checklist**:
- [ ] Run `grep -r "except Exception:" cynic/` — target: 0 matches
- [ ] Run `mypy cynic/ --strict` — target: 0 errors
- [ ] Run load test: verify p99 latency < 500ms
- [ ] Review GitHub issues labeled "technical-debt"
- [ ] Update TECHNICAL_DEBT.md with new findings

**Confidence**: 58% (φ-bounded)

*sniff* Debt is visible now. Phase 2 starts with clear sight of what's broken.
