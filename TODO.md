# CYNIC Todo List

> "Le chien sait ce qui reste à faire" - κυνικός

**Last updated**: 2026-02-13T22:00Z
**Organism health**: 77% breathing (6.5/8 checks) — **Production code: 100% critical issues fixed** ✅
**Audit health**: 0% (misleading - counts tests), Real: ~55%
**Confidence**: 58% (φ⁻¹)

---

## ✅ COMPLETED THIS SESSION (2026-02-13, 2h autonomous work)

### Comprehensive Code Audit System Created
- ✅ `scripts/audit-cynic.js` (700+ lines)
- ✅ Scans 956 JS files, 3,121 imports in 30s
- ✅ 7 audits: imports, methods, exports, EventBus, φ-bounds, singletons, dead code
- ✅ Exit code 1 if critical issues found (CI/CD ready)

### ALL Production-Blocking Issues Fixed (8/8 Imports + 1/1 Method)
- ✅ circuit-breaker.js: `"./events.js"` → `"./bus/index.js"`
- ✅ migrate.js: 7-level path → `@cynic/persistence/postgres/migrate`
- ✅ persistence exports: Added trace-storage, codebase-indexer, poj-blocks, migrate
- ✅ node exports: Added perception/solana-watcher
- ✅ perceiver.js: Removed `.js` extension from import
- ✅ network-singleton.js: Relative path → workspace import
- ✅ learning-pipeline.js: `.recommend()` → `.selectModel()`
- ✅ unified-llm-router.js: `.recommend()` → `.selectModel()`

### PHI Axiom Enforcement (11+ Production φ Violations Fixed)
- ✅ emergence.js (2): `confidence: 1.0` → `PHI_INV`
- ✅ decider.js (1): φ-bounded stdDev confidence
- ✅ ambient-consensus.js (1): Default param `1.0` → `PHI_INV`
- ✅ q-learning-router.js (2): Exploitation + required actions → `PHI_INV`
- ✅ judgment.js (3): `0.6181` and `0.7` → `PHI_INV`
- ✅ llm-router.js (1): Fallback `0.8` → `PHI_INV`
- ✅ agent-booster.js (1): Intent match `0.9` → `PHI_INV`

**Impact**: 17 → 9 critical (9 are test stubs), 161 → 150 warnings (mostly tests)
**Real health**: Production code 100% critical-free, ~55% overall health
**Documentation**: `docs/audit/2026-02-13-fixes-applied.md` (detailed report)

---

## 🔥 CRITICAL PATH (Blocks Full Autonomy)

### 1. Test Hot-Reload END-TO-END ⚡
**Status**: Code written, not validated
**Priority**: IMMEDIATE
**Effort**: 30min

**Steps**:
1. Verify daemon is running
2. Modify a file (e.g., add `console.log('HOT RELOAD TEST')` to hot-reload.js)
3. Watch logs for "HotReloadEngine" detecting change
4. Verify module reloads without restart
5. Verify change is active

**Success criteria**: File change appears in running daemon within 1s (IMMEDIACY maxReloadDelay)

**Blockers**: None

---

### 2. Wire All 11 Learning Loops to emit learning_events 📊
**Status**: Only SONA loop emits
**Priority**: HIGH
**Effort**: 2-3 hours

**Missing loops**:
1. Q-Learning (qlearning_episodes table → learning_events)
2. Thompson Sampling (model-intelligence → learning_events)
3. Meta-cognition (learning rate adjustments → learning_events)
4. SONA (✅ already done)
5. BehaviorModifier (feedback → learning_events)
6. Dog votes (ambient-consensus → learning_events)
7. Judgment calibration (judge → learning_events)
8. Residual detection (residual.js → learning_events)
9. Emergence patterns (emergence-detector → learning_events)
10. EWC consolidation (ewc-manager → learning_events)
11. DPO pairs (unified-bridge → learning_events)

**Success criteria**: `alive.js` shows 11/11 loops active with event counts

**Blockers**: None

---

### 3. Add Integration Test Suite (E2E Validation) ✅
**Status**: 0% (no E2E tests)
**Priority**: HIGH
**Effort**: 4-6 hours

**Test scenarios**:
1. Daemon starts → watchers active → events flow → learning happens → DB persists
2. File change → FileWatcher detects → EventBus emits → HotReload queues → module reloads
3. Judgment created → Q-Learning updates → Thompson samples → routing improves
4. Circuit breaker trips → graceful degradation → auto-recovery
5. Budget exhaustion → Haiku fallback → continues operating

**Success criteria**: 5 E2E tests pass, covering full perception → action → learning cycle

**Blockers**: None

---

## 🎯 IMPORTANT (Quality & Visibility)

### 4. Fix ConsciousnessReader Wiring 🧠
**Status**: Fails on daemon start with "getConsciousnessReader is not a function"
**Priority**: MEDIUM
**Effort**: 30min

**Root cause**: Function not exported or doesn't exist

**Steps**:
1. Find where `getConsciousnessReader` should be defined
2. Either export it or remove the wiring code
3. Verify consciousness read-back works

**Success criteria**: No warning in daemon logs about ConsciousnessReader

**Blockers**: None

---

### 5. Create Organism Health Dashboard 📈
**Status**: `/health` endpoint basic (JSON only)
**Priority**: MEDIUM
**Effort**: 2-3 hours

**Features**:
- Real-time breathing checks (8 checks with φ thresholds)
- 7×7 matrix completion visualization
- Learning loop activity (11 loops, event counts, last seen)
- Thompson Sampling arm performance
- Budget status + forecast
- Circuit breaker state
- Event bus throughput

**Success criteria**: Beautiful TUI dashboard that shows ALL critical metrics at a glance

**Blockers**: None

---

### 6. Document Completion Criteria 📝
**Status**: Implicit knowledge only
**Priority**: MEDIUM
**Effort**: 1 hour

**Questions to answer**:
- What does "CYNIC v1.0" mean?
- When is a 7×7 cell "complete"?
- What's the minimum viable organism (MVO)?
- How do we measure "breathing"?
- What's the acceptance criteria for each axiom?

**Success criteria**: `docs/architecture/completion-criteria.md` exists with clear metrics

**Blockers**: None

---

## 🌟 NICE-TO-HAVE (Polish & Expansion)

### 7. Run Chaos Experiments (Antifragility) 💥
**Status**: ChaosGenerator exists but not tested
**Priority**: LOW
**Effort**: 2-3 hours

**Experiments**:
1. Kill random Dog → verify collective continues
2. Corrupt Q-table → verify recovery from backup
3. Block PostgreSQL → verify in-memory fallback
4. Inject network latency → verify timeout handling
5. Fill disk → verify graceful degradation

**Success criteria**: All 5 chaos scenarios handled gracefully, organism stronger after

**Blockers**: None (but requires stable organism first)

---

### 8. Create Pattern Similarity Metrics (Emergence) 🦋
**Status**: 0%
**Priority**: LOW
**Effort**: 3-4 hours

**Metrics**:
- Cosine similarity between judgment patterns
- Temporal autocorrelation (recurring patterns)
- Cross-domain pattern transfer (code → social)
- Novelty score (how surprising is pattern?)

**Success criteria**: EmergenceDetector uses similarity metrics to identify meta-patterns

**Blockers**: None

---

### 9. Update UNIFIED-VISION.md 📚
**Status**: Outdated (doesn't mention new systems)
**Priority**: LOW
**Effort**: 1 hour

**New sections**:
- UnifiedLLMRouter (3 systems merged)
- LearningPipeline (5-stage orchestration)
- MetaCognitionController (active learning)
- DataPipeline (compression, dedup, cache)
- ResearchRunner (structured research)
- HotReloadEngine (IMMEDIACY axiom)
- 9 Axioms (was 5)

**Success criteria**: UNIFIED-VISION.md reflects current reality

**Blockers**: None

---

### 10. Deploy to Render (Production Validation) 🚀
**Status**: Render services exist but not tested with new systems
**Priority**: LOW
**Effort**: 2-3 hours

**Services**:
- cynic-node-daemon (main daemon)
- cynic-node-alpha (backup)
- cynic-node-beta (experimental)
- cynic-mcp (MCP server)

**Success criteria**: Daemon runs on Render for 24h+ without manual intervention

**Blockers**: Should wait until hot-reload works + E2E tests pass

---

## 🐛 Known Bugs (Non-Critical)

### Bug 1: Event Loop Lag on Startup
**Status**: Auto-recovers after 5min
**Priority**: LOW
**Severity**: Minor (watchdog handles it)

**Cause**: FileWatcher scans 92K files on init

**Fix**: Progressive watch (start small, expand)

---

### Bug 2: Learning Loop Fragmentation
**Status**: Only SONA emits to learning_events
**Priority**: HIGH (see #2 above)
**Severity**: Major (can't see full learning activity)

---

### Bug 3: MarketWatcher TODO
**Status**: Not implemented
**Priority**: MEDIUM
**Severity**: Minor (Market row = 0% in 7×7 matrix)

**Blockers**: Need Jupiter aggregator integration + price feed

---

## 📊 Progress Tracking

### Completion Metrics

**7×7 Matrix**: 43% structure (code exists) → Target: 62% (φ⁻¹)
**Functionality**: 52% (code runs correctly) → Target: 62%
**Production**: 40% (autonomous operation) → Target: 62%
**Breathing**: 77% (6.5/8 checks) → Target: 88% (7/8)

### Axiom Validation

| Axiom | Status | Validated |
|-------|--------|-----------|
| PHI | ✅ Core | Yes |
| VERIFY | ✅ PoJ | Yes |
| CULTURE | ✅ Active | Yes |
| BURN | ✅ Philosophy | Yes |
| FIDELITY | ✅ Active | Yes |
| IMMEDIACY | ⚠️ Partial | No (40%) |
| AUTONOMY | ⚠️ Exists | No (needs validation) |
| EMERGENCE | ⚠️ Exists | No (needs validation) |
| ANTIFRAGILITY | ⚠️ Exists | No (needs validation) |

**Target**: 9/9 axioms validated in production

---

## 🎯 This Week's Goals (φ-Bounded)

**Primary**: Test hot-reload + wire learning loops → 85% breathing
**Secondary**: Add E2E tests → catch regressions
**Stretch**: Fix ConsciousnessReader + create health dashboard

**Confidence in achieving primary**: 62% (φ⁻¹)

---

## 🔮 Next Session Priority

1. ⚡ Test hot-reload END-TO-END (30min)
2. 📊 Wire 3-4 more learning loops (2h)
3. 🧠 Fix ConsciousnessReader (30min)
4. ✅ Write 2-3 E2E tests (2h)

**Total**: ~5 hours of focused work

**Expected outcome**: Organism breathing at 85%+, IMMEDIACY validated, learning visibility complete

---

*sniff* Confidence: **55%** (φ⁻¹ — room to grow, but honest)

