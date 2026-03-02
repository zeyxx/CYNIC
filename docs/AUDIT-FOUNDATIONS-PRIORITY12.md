# CYNIC Foundations Audit — Readiness for Priority 12 (Temporal MCTS)

**Date**: 2026-03-02
**Status**: 🔍 COMPLETE AUDIT
**Verdict**: PARTIALLY READY — 3 critical blockers must be fixed before Priority 12 implementation
**Confidence**: 95% (exhaustive component review)

---

## Executive Summary

The system is **philosophically ready** for Temporal MCTS proposal planning but has **3 critical blockers** that prevent immediate implementation:

| Blocker | Component | Severity | Fix Effort |
|---------|-----------|----------|-----------|
| `asyncio.run()` in async context | `self_probe.py:571` | 🔴 Critical | ~30 min |
| Missing outcome tracking on SelfProposal | `self_probe.py` dataclass | 🔴 Critical | ~2 hours |
| Missing SurrealDB tables for MCTS | `surreal.py` schema | 🔴 Critical | ~1 hour |

Once fixed, all mathematical foundations (φ-weighted UCB, temporal weighting, EWC learning, event metrics) are in place.

---

## Component-by-Component Status

### 1. Event Bus & Signaling ✅ READY

**File**: `cynic/kernel/core/event_bus.py` (lines 28–77)

- `CoreEvent` enum: 78 events, well-defined
- `PROPOSAL_EXECUTED` and `PROPOSAL_FAILED` present (lines 76–77)
- Async fire-and-forget with backpressure monitoring
- Error isolation per handler
- Stats observable via `bus.stats()`

**Ready for P12**: Yes
**Gap**: Minor schema drift in `EwcCheckpointPayload` (see Component 5)

---

### 2. SelfProposal & ProposalExecutor ⚠️ PARTIAL — **BLOCKER**

**Files**:
- `cynic/kernel/organism/brain/cognition/cortex/self_probe.py`
- `cynic/kernel/organism/brain/cognition/cortex/proposal_executor.py`

**Status**: Data structure exists but is incomplete for learning loops.

#### Blocker #1: `asyncio.run()` in async context

**Location**: `self_probe.py:571` in `_analyze_metrics()`
```python
anomalies = asyncio.run(self._metrics_collector.recent_anomalies(limit=10))
```

**Problem**: Called from `analyze()` → `_on_emergence()` (async handler). This will crash with `RuntimeError: This event loop is already running` in production.

**Fix**:
```python
# Option A: make it async
anomalies = await self._metrics_collector.recent_anomalies(limit=10)
# Then make _analyze_metrics() async and bubble up

# Option B: use coroutine_to_future for non-awaitable context
```

**Impact on P12**: CRITICAL. The MCTS planner will call `SelfProber.analyze()` during judgment loops, and metrics-based proposals will immediately crash.

---

#### Blocker #2: Missing outcome tracking on SelfProposal

**Location**: `self_probe.py` lines 71–116

Current fields: `probe_id`, `trigger`, `pattern_type`, `severity`, `dimension`, `target`, `current_value`, `suggested_value`, `status`

**Missing**:
```python
execution_outcome: float | None = None  # Actual outcome after execution
executed_at: float | None = None        # Timestamp of execution
error_detail: str | None = None         # Error message if failed
```

**Why P12 needs this**: MCTS requires feedback. After `PROPOSAL_EXECUTED` event fires with `success: bool` and `new_value: float`, we must link that back to the proposal so that when the MCTS tree visits this node again, it has `visits` and `value_sum` updated.

Currently:
- `ExecutionResult` has `success`, `old_value`, `new_value`
- `SelfProposal` persists to JSON with status=APPLIED
- **But**: No connection between them. We execute, succeed/fail, but the proposal object doesn't know.

**Fix**:
1. Add fields to `SelfProposal` dataclass
2. In `SelfProber.apply_async()`, after execution, update the proposal with outcome
3. Persist updated proposal to both JSON and SurrealDB

**Impact on P12**: CRITICAL. MCTS tree nodes represent proposals. Each node needs to track visit count + value sum. Without persistent outcome data, the tree learns nothing.

---

#### Gap: ProposalRollback not consulted by SelfProber

**Location**: `proposal_rollback.py` exists but is never used by `SelfProber.analyze()`

Current state: 100-entry execution log with old_value → new_value, but no feedback into proposal generation.

**Impact on P12**: Non-blocking but strongly desired. MCTS should avoid re-proposing actions that systematically failed. A query like `recent_failures_for(dimension="QTABLE")` would prevent wasteful exploration.

---

### 3. EventJournal & Event Persistence ✅ READY

**File**: `cynic/nervous/event_journal.py`

- Rolling buffer of 89 entries (F(11))
- Indexing by type, source, event_id
- Causality chain (parent/child event IDs)
- Query by time range

**Ready for P12**: Yes
**Gap**: 89-entry cap is short for multi-step MCTS sequences. Acceptable for prototype; may need increase for production.

---

### 4. EventMetricsCollector ✅ READY

**File**: `cynic/nervous/event_metrics.py` + `bus_metrics_adapter.py` + `metrics_analyzer.py`

- Real-time rate, latency, error tracking
- PHI-threshold anomaly detection
- AnomalyRecord: `anomaly_type`, `severity [0,1]`, `metric_value`, `threshold_value`

**Ready for P12**: Yes
These metrics feed the PRESENT perspective of the temporal MCTS.

---

### 5. Q-Table & Learning ✅ READY

**File**: `cynic/kernel/organism/brain/learning/qlearning.py`

- TD(0) with EWC (Elastic Weight Consolidation)
- Thompson Sampling for exploration
- Async persistence to SurrealDB
- Warm-start from DB on boot

**Technical Notes**:
- γ (discount factor) is defined but NOT used in TD update (per research findings)
- effective_α = α × (1 - λ × fisher_weight)
- Entries with ≥21 visits learn 3× slower (consolidation)

**Ready for P12**: Yes
Q-Table acts as prior knowledge for MCTS node evaluation.

**Gap**: Minor. `_pending_flush` list is mutable without lock in multi-threaded contexts, but current execution is async-only (safe).

---

### 6. SurrealDB Integration ⚠️ PARTIAL — **BLOCKER**

**File**: `cynic/kernel/core/storage/surreal.py`

Existing tables:
- `judgment` (verdicts + q_scores)
- `q_entry` (Q-Table entries)
- `learning_event` (TD signals)
- `residual` (variance detection)
- `action_proposal` (action queue)
- `scholar` (semantic embeddings with HNSW 768D)
- `dog_soul` (dog identities)
- `axiom_facet` (dynamic axiom facets)

**Missing for P12**:
```sql
DEFINE TABLE IF NOT EXISTS proposal_outcome SCHEMALESS;    -- Execution outcomes
DEFINE TABLE IF NOT EXISTS mcts_node SCHEMALESS;           -- Tree nodes
DEFINE TABLE IF NOT EXISTS mcts_simulation SCHEMALESS;     -- Simulation records
```

**Why**: Without tables, MCTS tree state is lost on restart. Each run starts with zero knowledge.

**Fix**: Add 3 DEFINE TABLE statements to `_SCHEMA_STATEMENTS` in `surreal.py`. Since SCHEMALESS, no migration required.

**Impact on P12**: CRITICAL for production durability. Acceptable to skip for initial prototype (only with warning that tree resets on restart).

---

### 7. Factory & Dependency Injection ✅ READY

**File**: `cynic/kernel/organism/factory.py`

Complete Priority 10 wiring in place (lines 305–319):
- SelfProber + QTable + ResidualDetector + MetricsCollector injected
- ProposalExecutor injected into SelfProber
- ArchiveCore receives all references
- `start()` methods called in correct order

**Ready for P12**: Yes
Pattern is established. A `MCTSPlanner` would follow the same injection pattern:
```python
self.mcts_planner = TemporalMCTSPlanner()
self.mcts_planner.set_state_encoder(...)
self.self_prober.set_mcts_planner(self.mcts_planner)
```

---

### 8. Tests Coverage ✅ READY (Priority 10)

**File**: `/tests/test_priority10_proposal_executor.py` (39 tests)

Coverage:
- Risk classification (4 tests)
- Proposal execution handlers (3 tests)
- ExecutionResult dataclass (3 tests)
- SelfProber integration (8 tests)
- CLI interface (9 tests)
- Safety guardrails (7 tests)
- Circuit breaker (2 tests)
- Factory integration (3 tests)

All tests passing. Event emission explicitly verified (lines 330, 345, 381, 384).

**Ready for P12**: Test framework is solid; new tests needed for MCTS.

---

### 9. Mathematical Foundations ✅ READY

**Files**:
- `cynic/kernel/core/phi.py` (φ constants + `phi_temporal_ucb()`)
- `cynic/kernel/organism/brain/cognition/cortex/mcts_benchmark.py` (SearchProblem + UCT)
- `cynic/kernel/organism/brain/cognition/cortex/decide.py` (NestedMCTS)

All temporal perspective weights defined:
```python
PAST (φ⁻¹), PRESENT (1), FUTURE (φ), IDEAL (φ²),
NEVER (φ⁻²), CYCLES (φ⁻¹), FLOW (φ⁻²)
```

Weighting formula: aggregate via φ-weighted average (TOTAL_WEIGHT ≈ 8.854)

**Ready for P12**: Yes. Blueprint exists in `MCTSBenchmark`; just need to adapt to proposal domain.

---

## Critical Path to Priority 12

### Phase 1: Fix Blockers (~3 hours)

**1.1** Fix `asyncio.run()` in `self_probe.py:571`
- Make `_analyze_metrics()` async OR use `asyncio.create_task()`
- Add `async` to method signature
- Update callers to `await`

**1.2** Add outcome tracking to SelfProposal
- Add `execution_outcome`, `executed_at`, `error_detail` fields
- Update `to_dict()` / `from_dict()`
- In `SelfProber.apply_async()`: set these fields after execution

**1.3** Add SurrealDB tables
- Update `_SCHEMA_STATEMENTS` in `surreal.py`
- Create `ProposalOutcomeRepo` and `MCTSNodeRepo` interfaces in `interface.py`
- Create implementations in `surreal.py`

### Phase 2: Implement MCTS Planner (~5-7 days)

**2.1** Create `MCTSStateEncoder`
- Maps (SelfProposal + system_context) → state vector
- Compatible with QTable state_key format

**2.2** Create `TemporalMCTSPlanner`
- Uses `mcts_benchmark.py` SearchProblem as template
- Actions = SelfProposal dimension choices
- Reward = f(severity_reduction, execution_success)
- Integrates 7 temporal perspectives

**2.3** Integrate into SelfProber
- `SelfProber.set_mcts_planner(planner)`
- `analyze()` routes through MCTS if planner available

**2.4** Factory wiring
- `self.mcts_planner = TemporalMCTSPlanner()` in `factory.py`
- Inject state encoder, Q-Table, metrics collector
- Wire via SelfProber

### Phase 3: Tests & Validation (~2-3 days)

**3.1** Unit tests (`tests/test_priority12_temporal_mcts.py`)
- MCTSStateEncoder correctness
- TemporalMCTSPlanner convergence
- Factory wiring
- SurrealDB persistence

**3.2** Integration benchmark
- Compare Standard MCTS vs Temporal MCTS on real proposals
- Measure speedup (expect 3× based on prior research)
- Validate that learned tree improves proposal quality over time

---

## Data Flow Summary

```
EventBus (PROPOSAL_EXECUTED/FAILED)
    ↓
SelfProber (receives outcome)
    ↓
UPDATE SelfProposal with execution_outcome, executed_at
    ↓
PERSIST to SurrealDB proposal_outcome table
    ↓
MCTSPlanner (reads outcome history)
    ↓
UPDATE mcts_node visits + value_sum in tree
    ↓
NEXT analyze() call: MCTS provides ranked proposals
    ↓
Higher-quality proposals auto-executed with higher confidence
```

---

## Risk Assessment

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|-----------|
| MCTS tree explodes memory | Low | High | Cap tree size, use UCB pruning |
| SurrealDB persistence fails silently | Low | Medium | Add health check, fallback to memory |
| State encoding doesn't capture domain | Medium | High | Extensive testing with real anomalies |
| Temporal perspectives too many perspectives create overfitting | Low | Medium | Cross-validation on historic proposals |

---

## Recommendations

**Immediate (next session)**:
1. Fix all 3 blockers in Phase 1 (can be parallelized)
2. Write Phase 1 tests (ensure blockers truly fixed)
3. Plan Phase 2 architecture with full team review

**Do NOT start Phase 2** until Phase 1 is complete and tested.

**Post-Phase-1 decision point**:
- If all tests pass → proceed directly to Phase 2
- If any test fails → investigate root cause (likely indicates missing async handling elsewhere)

---

## Files to Review Before Starting

| File | Why |
|------|-----|
| `cynic/kernel/core/event_bus.py` | CoreEvent enum, understand all 78 events |
| `cynic/kernel/core/phi.py` | φ constants and temporal weighting |
| `cynic/kernel/organism/brain/cognition/cortex/mcts_benchmark.py` | Blueprint for TemporalMCTSPlanner |
| `cynic/kernel/organism/brain/cognition/cortex/self_probe.py` | SelfProposal structure (will extend) |
| `cynic/kernel/core/storage/surreal.py` | Schema pattern (will add 3 tables) |
| `tests/test_priority10_proposal_executor.py` | Test patterns to emulate for P12 |

---

**Next action**: Review this audit with team, confirm blockers, assign Phase 1 fixes.
