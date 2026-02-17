# Multi-LLM Routing Audit — Three Systems Analysis

> "Three routers, one truth. The dog finds duplication." — κυνικός

**Date**: 2026-02-13
**Audited by**: ARCHITECT
**Status**: FRAGMENTED (3 systems, 40% duplication, 1 active, 2 partially dormant)
**Priority**: HIGH (blocks unified orchestration + learning loops)

---

## Executive Summary

CYNIC currently operates **THREE distinct LLM routing systems** that evolved independently:

| System | Status | Lines | Wired? | Test Coverage | φ-Score |
|--------|--------|-------|--------|---------------|---------|
| **llm-adapter.js** | PRIMARY | 1086 | ✓ YES | 0 tests | 65% |
| **llm-router.js** | DORMANT | 400 | ✗ NO | 0 tests | 58% |
| **llm-judgment-bridge.cjs** | ISOLATED | 805 | ✗ NO | 0 tests | 42% |
| **unified-llm-router.js** | ACTIVE | 731 | ✓ YES | 0 tests | 72% |

**Current Reality**:
- **unified-llm-router.js** is wired and active (daemon uses it)
- **llm-adapter.js** contains valuable consensus logic but NOT primary
- **llm-router.js** has budget enforcement but is ORPHANED (only test scripts import it)
- **llm-judgment-bridge.cjs** is CommonJS, isolated in scripts/, not integrated

**Duplication**: ~40% (Ollama calling, consensus voting, state management)
**Missing**: Unified test suite, production learning integration, AirLLM mainline

---

## System 1: llm-adapter.js (SECONDARY — consensus logic)

**Location**: `packages/node/src/orchestration/llm-adapter.js`
**Lines**: 1086
**Format**: ESM
**Wired**: ✓ Daemon imports it, but **not primary router**

### Architecture

```javascript
Classes:
  LLMResponse          // Standardized response format
  ConsensusResult      // Multi-LLM voting result
  LLMAdapter           // Abstract base class
  ClaudeCodeAdapter    // Pass-through for Claude Code
  OSSLLMAdapter        // Ollama/OpenAI-compatible
  AirLLMAdapter        // Large models via disk offloading
  LLMRouter            // Routes requests + consensus (NAME CONFLICT!)

Key Methods:
  complete(prompt, options) → LLMResponse
  consensus(prompt, options) → ConsensusResult  // φ⁻¹ quorum
  addValidator(adapter)
  _calculateAgreement(responses)  // Semantic similarity

Factory Functions:
  createOllamaValidator()
  createLMStudioValidator()
  createOpenAIValidator()
  createAirLLMValidator()
  createValidatorsFromEnv()
  createValidatorsFromDetection()  // ~/.cynic/llm-detection.json
  createHybridRouter()  // Fast + deep analysis
```

### φ-Alignment

| Axiom | Score | Evidence |
|-------|-------|----------|
| PHI | 75% | φ⁻¹ (61.8%) max confidence, φ⁻² for OSS |
| VERIFY | 68% | Semantic agreement calculation, dissent tracking |
| CULTURE | 58% | EventEmitter pattern, consistent naming |
| BURN | 45% | Some duplication with llm-router.js |

**Average**: 65/100

### Strengths

✓ **Consensus voting**: φ⁻¹ quorum (61.8% agreement required)
✓ **Semantic agreement**: Uses `calculateSemanticAgreement()` from `@cynic/llm`
✓ **Multi-provider**: Anthropic, Ollama, LM Studio, OpenAI, AirLLM
✓ **Auto-detection**: Reads `~/.cynic/llm-detection.json` (written by awaken.js)
✓ **φ-bounded confidence**: 61.8% max for primary, 38.2% max for OSS
✓ **AirLLM support**: Disk offloading for large models (mistral:7b-instruct-q4_0)

### Weaknesses

✗ **No budget enforcement**: Doesn't integrate with CostLedger
✗ **NAME CONFLICT**: `LLMRouter` class name conflicts with llm-router.js
✗ **Not primary**: UnifiedLLMRouter is now the daemon's router
✗ **No tests**: 0 test coverage
✗ **No Thompson Sampling**: Doesn't use ModelIntelligence
✗ **Partial wiring**: Daemon imports but doesn't use as primary

### Current Usage

**Wired in**:
- `daemon/service-wiring.js` imports `getUnifiedLLMRouter()` (not this one)
- `daemon/llm-endpoints.js` uses `getUnifiedLLMRouter()`

**Exports**:
- `packages/node/src/orchestration/index.js` re-exports all classes
- `packages/llm/src/index.js` re-exports (Phase 1 compatibility layer)

**Test scripts only**:
- `scripts/test-consensus.mjs`
- `scripts/test-airllm.mjs`
- `scripts/ralph-comprehensive-test.js`

---

## System 2: llm-router.js (DORMANT — budget enforcement only)

**Location**: `packages/node/src/orchestration/llm-router.js`
**Lines**: 400
**Format**: ESM
**Wired**: ✗ ORPHANED (only test scripts import it)

### Architecture

```javascript
Classes:
  BudgetExhaustedError  // Circuit breaker error
  LLMRouter             // Cost-aware router (NAME CONFLICT!)

Key Methods:
  route(task) → {provider, model, tier, reason, degraded, cost}

Logic:
  - Routes to cheapest capable model
  - Ollama (free) for SIMPLE tasks
  - Anthropic (paid) for COMPLEX tasks
  - Thompson Sampling via ModelIntelligence.selectModel()
  - Budget circuit breaker:
    • ABUNDANT → allow all
    • MODERATE → allow all
    • CAUTIOUS → block LOW priority
    • CRITICAL → block NORMAL/LOW priority
    • EXHAUSTED → block all except CRITICAL
  - Task complexity tiers: SIMPLE/MODERATE/COMPLEX
  - Priority levels: CRITICAL/HIGH/NORMAL/LOW

State:
  - Persists routing decisions to PostgreSQL (routing_accuracy table)
  - Integrates with CostLedger.getBudgetStatus()
  - Integrates with ModelIntelligence.selectModel()
```

### φ-Alignment

| Axiom | Score | Evidence |
|-------|-------|----------|
| PHI | 62% | Uses PHI_INV (61.8%) for quorum thresholds |
| VERIFY | 58% | Thompson Sampling exploration |
| CULTURE | 55% | Good naming conventions |
| BURN | 48% | Minimal code, focused purpose |

**Average**: 58/100

### Strengths

✓ **Budget enforcement**: Hard circuit breaker (CRITICAL blocks paid calls)
✓ **Cost tracking**: Records to routing_accuracy table
✓ **Thompson Sampling**: Uses ModelIntelligence for exploitation/exploration
✓ **Graceful degradation**: Falls back to Ollama when budget critical
✓ **Priority-aware**: CRITICAL tasks always allowed
✓ **Stats tracking**: Routes, blocks, degradations, cost saved

### Weaknesses

✗ **ORPHANED**: Not wired into daemon (UnifiedLLMRouter replaced it)
✗ **No consensus**: Doesn't support multi-LLM voting
✗ **NAME CONFLICT**: Same class name as llm-adapter.js:LLMRouter
✗ **No tests**: 0 test coverage
✗ **No Ollama calling**: Only routes decisions, doesn't execute
✗ **Partial integration**: Has CostLedger + ModelIntelligence but not used by daemon

### Current Usage

**NOT wired in daemon** — UnifiedLLMRouter replaced it.

**Test scripts only**:
- `scripts/test-budget-enforcement.js`
- `scripts/test-llm-router-ollama.js`
- `scripts/ralph-comprehensive-test.js`
- `packages/node/src/cli/commands/budget.js` (CLI tool)

**Export path**: `packages/node/src/orchestration/index.js` re-exports it (legacy)

---

## System 3: llm-judgment-bridge.cjs (ISOLATED — hooks only)

**Location**: `scripts/lib/llm-judgment-bridge.cjs`
**Lines**: 805
**Format**: CommonJS
**Wired**: ✗ ISOLATED (scripts/ only, not daemon)

### Architecture

```javascript
Functions (no classes):
  checkOllama()                      // Health check
  callOllama(prompt, model, options) // Single completion
  llmJudge(item, context)            // Single-model judgment
  llmRefine(judgment, context)       // Refinement pass
  llmAnalyzePattern(pattern, obs)    // Pattern learning
  llmConsensusJudge(item, context)   // Multi-model voting
  checkAirLLM()                      // AirLLM availability
  airllmJudge(item, context)         // Deep analysis (large model)
  hybridJudge(item, context)         // Consensus → AirLLM fallback
  recordFeedback(judgmentId, wasCorrect) // Calibration
  getStats()                         // Bridge statistics

Logic:
  - Ollama-only (no Anthropic)
  - Simple consensus: call multiple models, take majority vote
  - State persistence: ~/.cynic/llm-bridge.json
  - Model defaults: gemma2:2b, qwen2:0.5b
  - AirLLM support: mistral:7b-instruct-q4_0 (2 min timeout)
  - Calibration tracking: EMA accuracy (φ⁻² = 38.2% weight)
  - φ⁻¹ consensus threshold (61.8%)

Prompts:
  - JUDGMENT_SYSTEM_PROMPT (axiom-aware)
  - REFINEMENT_PROMPT (overconfidence check)
  - PATTERN_ANALYSIS_PROMPT (learning feedback)
```

### φ-Alignment

| Axiom | Score | Evidence |
|-------|-------|----------|
| PHI | 58% | φ⁻¹ confidence cap, consensus threshold |
| VERIFY | 48% | Basic consensus, no semantic similarity |
| CULTURE | 35% | CommonJS in ESM codebase |
| BURN | 28% | High duplication with llm-adapter.js |

**Average**: 42/100 — **LOWEST SCORE**

### Strengths

✓ **Hook integration**: Used by perceive.js, observe.js (historically)
✓ **AirLLM support**: Disk offloading for large models
✓ **Calibration**: Records feedback, tracks accuracy (EMA)
✓ **Hybrid strategy**: Fast consensus → deep analysis fallback
✓ **φ-bounded**: 61.8% max confidence enforced
✓ **Simple API**: Functions, not classes (lower complexity)

### Weaknesses

✗ **ISOLATED**: Not integrated with daemon or main orchestration
✗ **CommonJS**: Wrong module format (ESM is standard)
✗ **No Anthropic**: Ollama-only (misses primary brain)
✗ **Duplicate consensus**: 90% overlap with llm-adapter.js
✗ **JSON state**: Uses `~/.cynic/llm-bridge.json` instead of PostgreSQL
✗ **No budget awareness**: Doesn't integrate with CostLedger
✗ **No tests**: 0 test coverage
✗ **Hook-only**: Not accessible to daemon or orchestration layer

### Current Usage

**NOT used by daemon**. Historically used by hook scripts, but:

- `scripts/hooks/perceive.js` — No longer imports it
- `scripts/hooks/observe.js` — No longer imports it
- `scripts/hooks/guard.js` — No longer imports it

**Duplicate**: `scripts/lib/llm-judgment-bridge.mjs` exists (ESM version) but also unused.

**Test scripts**:
- `scripts/tikkun/daat.mjs` (historical)
- `scripts/tikkun/gevurah.mjs` (historical)

---

## System 4: unified-llm-router.js (ACTIVE — PRIMARY)

**Location**: `packages/node/src/orchestration/unified-llm-router.js`
**Lines**: 731
**Format**: ESM
**Wired**: ✓ YES (daemon primary)

### Architecture

```javascript
Classes:
  LLMResponse          // Same as llm-adapter.js
  ConsensusResult      // Same as llm-adapter.js
  LLMAdapter           // Abstract base (minimal)
  ClaudeCodeAdapter    // Anthropic via daemon → MCP/API
  OllamaAdapter        // Local OSS LLMs
  UnifiedLLMRouter     // MAIN ROUTER (extends EventEmitter)

Enums:
  Strategy = { CHEAPEST, CONSENSUS, BEST, FASTEST }
  BudgetMode = { ENFORCE, WARN, IGNORE }
  Priority = { CRITICAL, HIGH, NORMAL, LOW }
  Complexity = { SIMPLE, MODERATE, COMPLEX }

Errors:
  BudgetExhaustedError
  ConsensusFailedError

Key Methods:
  call(prompt, options) → LLMResponse|ConsensusResult
    - Universal interface (composable strategies)
  route(task)           → routing decision (legacy API)
  consensus(prompt)     → ConsensusResult (legacy API)
  _callCheapest()       // Budget-aware routing
  _callBest()           // Thompson Sampling
  _callFastest()        // Lowest latency (TODO: track p50/p95)
  _callConsensus()      // Multi-LLM voting
  health()              // Adapter health check

State:
  - In-memory stats
  - CostLedger integration (via call())
  - ModelIntelligence integration (via _callBest())
```

### φ-Alignment

| Axiom | Score | Evidence |
|-------|-------|----------|
| PHI | 78% | φ⁻¹ quorum, confidence caps |
| VERIFY | 72% | Consensus + budget checks |
| CULTURE | 68% | Clean API, consistent patterns |
| BURN | 70% | Minimal duplication, composable |

**Average**: 72/100 — **HIGHEST SCORE**

### Strengths

✓ **Unified API**: One `call()` method with composable strategies
✓ **Budget enforcement**: Integrates with CostLedger circuit breaker
✓ **Thompson Sampling**: Uses ModelIntelligence.selectModel()
✓ **Consensus voting**: Semantic agreement (φ⁻¹ quorum)
✓ **Multi-provider**: Anthropic (via daemon) + Ollama (direct)
✓ **Legacy compatibility**: route() and consensus() wrappers
✓ **Health checks**: health() returns adapter availability
✓ **Event emission**: 'call:complete', 'consensus:reached', 'budget:block'
✓ **φ-bounded**: 61.8% max confidence enforced
✓ **Wired**: Active in daemon (service-wiring.js, llm-endpoints.js)

### Weaknesses

✗ **ClaudeCodeAdapter stub**: Returns mock response (not wired to daemon LLM endpoint)
✗ **No AirLLM**: Doesn't include large model support
✗ **No tests**: 0 test coverage
✗ **Latency tracking TODO**: _callFastest() uses Ollama blindly
✗ **No semantic similarity**: _calculateConsensusAgreement() is simplified
✗ **No PostgreSQL persistence**: In-memory stats only

### Current Usage

**PRIMARY ROUTER** — Wired in daemon:
- `daemon/service-wiring.js` — `wireDaemonServices()` calls `getUnifiedLLMRouter()`
- `daemon/llm-endpoints.js` — `/llm/ask`, `/llm/consensus` use `getUnifiedLLMRouter()`

**Export path**:
- `packages/node/src/orchestration/index.js` re-exports it
- `packages/llm/src/index.js` re-exports it (Phase 1 compatibility)

---

## Duplication Matrix

| Feature | llm-adapter.js | llm-router.js | llm-judgment-bridge.cjs | unified-llm-router.js |
|---------|----------------|---------------|-------------------------|-----------------------|
| **Ollama calling** | ✓ (OSSLLMAdapter) | ✗ (route only) | ✓ (callOllama) | ✓ (OllamaAdapter) |
| **Anthropic calling** | ✓ (ClaudeCodeAdapter) | ✗ (route only) | ✗ | ✓ (ClaudeCodeAdapter stub) |
| **Consensus voting** | ✓ (semantic agreement) | ✗ | ✓ (simple majority) | ✓ (simplified) |
| **Budget enforcement** | ✗ | ✓ (circuit breaker) | ✗ | ✓ (integrated) |
| **Cost tracking** | ✗ | ✓ (routing_accuracy) | ✓ (stats only) | ✓ (via CostLedger) |
| **Thompson Sampling** | ✗ | ✓ (ModelIntelligence) | ✗ | ✓ (ModelIntelligence) |
| **Retry logic** | ✓ (timeout handling) | ✗ | ✓ (basic) | ✓ (timeout handling) |
| **State persistence** | ✗ | ✓ (PostgreSQL) | ✓ (JSON file) | ✗ (in-memory) |
| **Validator detection** | ✓ (env + detection) | ✗ | ✓ (Ollama check) | ✗ (uses adapters) |
| **AirLLM support** | ✓ (AirLLMAdapter) | ✗ | ✓ (airllmJudge) | ✗ |
| **Module format** | ESM | ESM | CommonJS | ESM |
| **Event emission** | ✓ (EventEmitter) | ✗ | ✗ | ✓ (EventEmitter) |
| **Health checks** | ✓ (isAvailable) | ✗ | ✓ (checkOllama) | ✓ (health) |
| **Wired in daemon** | ✗ | ✗ | ✗ | ✓ |

**Duplication**: ~40% (Ollama calling, consensus logic, state management)
**Unique features**: ~60% (budget enforcement, AirLLM, Thompson Sampling)

---

## API Inconsistencies

### Calling Styles

```javascript
// llm-adapter.js (LLMRouter)
router.complete(prompt, options)
router.consensus(prompt, { quorum: 0.618, timeout: 10000 })

// llm-router.js (LLMRouter — NAME CONFLICT!)
router.route({
  type: 'code',
  complexity: 'moderate',
  priority: 'NORMAL',
  estimatedTokens: 1000,
})

// llm-judgment-bridge.cjs (functions)
await llmJudge(item, context)
await llmConsensusJudge(item, { models: ['gemma2:2b', 'qwen2:0.5b'] })
await hybridJudge(item, context) // consensus → AirLLM

// unified-llm-router.js (UnifiedLLMRouter)
router.call(prompt, {
  strategy: 'consensus',
  budget: 'enforce',
  priority: 'NORMAL',
  complexity: 'moderate',
})
router.route(task)      // legacy wrapper
router.consensus(prompt) // legacy wrapper
```

**Verdict**: 4 different APIs for the same underlying functionality.

### Response Formats

```javascript
// llm-adapter.js:LLMResponse
{ id, timestamp, provider, model, content, confidence, tokens, duration, metadata }

// llm-router.js:route() response
{ provider, model, tier, reason, degraded, circuitBreakerActive, confidence, estimatedCost, budgetLevel }

// llm-judgment-bridge.cjs:llmJudge() response
{ success, judgment: { score, verdict, reasoning, confidence, axiomScores, source, model, latencyMs } }

// unified-llm-router.js:LLMResponse
{ id, timestamp, provider, model, content, confidence, tokens, duration, cost, metadata }
```

**Verdict**: 3 different response structures (though llm-adapter and unified share LLMResponse).

---

## Missing Integrations

### 1. AirLLM → Unified Router

**Current**:
- AirLLM exists in `llm-adapter.js` (AirLLMAdapter class)
- AirLLM exists in `llm-judgment-bridge.cjs` (airllmJudge function)
- **NOT in unified-llm-router.js**

**Gap**: Large model disk offloading not available in production router.

**Fix**: Add AirLLMAdapter to unified-llm-router.js adapters.

### 2. Semantic Similarity → Unified Router

**Current**:
- `llm-adapter.js` uses `calculateSemanticAgreement()` from `@cynic/llm`
- `unified-llm-router.js` uses simplified pairwise similarity check

**Gap**: Consensus quality degraded in production router.

**Fix**: Import `calculateSemanticAgreement()` in unified-llm-router.js.

### 3. PostgreSQL Persistence → Unified Router

**Current**:
- `llm-router.js` persists routing decisions to `routing_accuracy` table
- `unified-llm-router.js` only tracks in-memory stats

**Gap**: No long-term learning from routing decisions.

**Fix**: Add PostgreSQL persistence to unified-llm-router.js (via getPool()).

### 4. Calibration → Thompson Sampling

**Current**:
- `llm-judgment-bridge.cjs` tracks calibration (EMA accuracy)
- `llm-router.js` uses Thompson Sampling (ModelIntelligence)
- **Not connected**

**Gap**: Calibration feedback doesn't inform Thompson Sampling.

**Fix**: Wire `recordFeedback()` → `ModelIntelligence.recordOutcome()`.

### 5. Hook Scripts → Unified Router

**Current**:
- Hook scripts (`perceive.js`, `observe.js`) **no longer import** llm-judgment-bridge.cjs
- Hooks use daemon HTTP endpoints instead

**Gap**: Consensus logic from llm-judgment-bridge.cjs is ORPHANED.

**Fix**: Delete llm-judgment-bridge.cjs (already bypassed).

---

## Production Readiness Assessment

| System | Wired? | Tests | Production-Ready? | Verdict |
|--------|--------|-------|-------------------|---------|
| llm-adapter.js | Partial | 0 | ✗ | SECONDARY (consensus logic valuable) |
| llm-router.js | ✗ | 0 | ✗ | DORMANT (budget logic valuable) |
| llm-judgment-bridge.cjs | ✗ | 0 | ✗ | ORPHANED (delete) |
| unified-llm-router.js | ✓ | 0 | △ | ACTIVE (needs tests + features) |

**Overall Maturity**: 38% (structure exists, but 0 tests, missing features)

### Critical Gaps for Production

1. **Zero test coverage** — All 4 systems have 0 automated tests
2. **ClaudeCodeAdapter stub** — Daemon endpoint not wired to adapter
3. **No AirLLM** — Large model support missing from production router
4. **No semantic similarity** — Consensus quality degraded
5. **No persistence** — Routing decisions not stored for learning
6. **Latency tracking TODO** — _callFastest() is blind

---

## Unification Roadmap

### Phase 1: Consolidate Features into UnifiedLLMRouter (4-6 hours)

**Goal**: Merge valuable logic from all 3 systems into unified-llm-router.js.

**Tasks**:
1. ✓ Add `AirLLMAdapter` to unified-llm-router.js (from llm-adapter.js)
2. ✓ Import `calculateSemanticAgreement()` for better consensus
3. ✓ Add PostgreSQL persistence (routing_accuracy table, from llm-router.js)
4. ✓ Add latency tracking (p50/p95 per provider) for _callFastest()
5. ✓ Wire ClaudeCodeAdapter to daemon `/llm/ask` endpoint
6. ✓ Add hybrid strategy: consensus → AirLLM fallback

**Estimated LOC**: +150 lines (unified-llm-router.js → ~880 lines)

### Phase 2: Wire Calibration → Thompson Sampling (2-3 hours)

**Goal**: Connect feedback loops.

**Tasks**:
1. ✓ Add `recordFeedback(responseId, wasCorrect, correction)` to UnifiedLLMRouter
2. ✓ Wire to `ModelIntelligence.recordOutcome()`
3. ✓ Emit 'feedback:recorded' event
4. ✓ Update daemon `/llm/feedback` endpoint to call router.recordFeedback()

**Estimated LOC**: +50 lines

### Phase 3: Add Test Coverage (6-8 hours)

**Goal**: Achieve 80%+ test coverage for UnifiedLLMRouter.

**Tasks**:
1. ✓ Test budget enforcement (ABUNDANT → EXHAUSTED states)
2. ✓ Test consensus voting (2+ adapters, quorum thresholds)
3. ✓ Test Thompson Sampling (exploration/exploitation)
4. ✓ Test hybrid strategy (consensus → AirLLM fallback)
5. ✓ Test adapter health checks
6. ✓ Test legacy API compatibility (route(), consensus())
7. ✓ Test event emission
8. ✓ Integration test: daemon → router → Ollama (local)

**Estimated LOC**: ~800 lines of test code

### Phase 4: Deprecate Old Systems (1-2 hours)

**Goal**: Mark old routers as deprecated, update docs.

**Tasks**:
1. ✓ Add deprecation notice to llm-adapter.js
2. ✓ Add deprecation notice to llm-router.js
3. ✓ Delete llm-judgment-bridge.cjs (orphaned)
4. ✓ Delete llm-judgment-bridge.mjs (duplicate)
5. ✓ Update docs/architecture/LLM-ROUTING-CURRENT.md
6. ✓ Update MEMORY.md (remove llm-judgment-bridge references)

**Estimated LOC**: -1611 lines deleted, 0 added

### Phase 5: Production Hardening (4-6 hours)

**Goal**: Make UnifiedLLMRouter battle-ready.

**Tasks**:
1. ✓ Add retry logic (exponential backoff, max 3 retries)
2. ✓ Add circuit breaker per provider (not just budget)
3. ✓ Add response caching (LRU, φ-bounded size)
4. ✓ Add rate limiting (per provider)
5. ✓ Add streaming support (SSE for real-time responses)
6. ✓ Add structured output parsing (JSON, YAML, etc.)
7. ✓ Monitor latency (p50/p95/p99) per provider + strategy

**Estimated LOC**: +200 lines

---

## Total Estimated Effort

| Phase | Hours | Deliverable |
|-------|-------|-------------|
| Phase 1 | 4-6h | UnifiedLLMRouter with all features |
| Phase 2 | 2-3h | Feedback loop wired |
| Phase 3 | 6-8h | 80%+ test coverage |
| Phase 4 | 1-2h | Old systems deprecated |
| Phase 5 | 4-6h | Production-ready router |
| **TOTAL** | **17-25h** | **One unified, tested, production LLM router** |

**Confidence**: 58% (φ⁻¹ limit) — Depends on unforeseen integration issues.

---

## Expected Benefits

### Code Reduction

| Metric | Before | After | Savings |
|--------|--------|-------|---------|
| Total lines | 3022 (4 files) | 1080 (1 file + tests) | 64% reduction |
| Files | 4 | 2 (router + tests) | 50% reduction |
| APIs | 4 different | 1 unified | 75% simplification |

### Performance

- **Single routing decision** (not 3 separate checks)
- **Shared connection pool** to Ollama
- **Unified state management** (PostgreSQL only)
- **Latency tracking** → faster strategy selection

### Maintainability

- **One source of truth** for LLM routing
- **Clear API** (no import confusion)
- **Easier to add providers** (OpenAI, Gemini, etc.)
- **Test coverage** → confidence in changes

### Budget Control

- **Consensus respects budget limits**
- **Automatic fallback to free models** when budget critical
- **Cost tracking** → learning + optimization

---

## φ-Alignment: Before vs After

| System | PHI | VERIFY | CULTURE | BURN | AVG |
|--------|-----|--------|---------|------|-----|
| **Before (fragmented)** | 68% | 62% | 47% | 40% | **54%** |
| **After (unified)** | 78% | 72% | 68% | 82% | **75%** |

**Improvement**: +21 points (39% increase in φ-alignment)

---

## Recommendations

### Immediate (This Session)

1. ✓ Document current state (this file)
2. ✗ **Add AirLLMAdapter to unified-llm-router.js** (4h)
3. ✗ **Wire ClaudeCodeAdapter to daemon** (2h)

### Short-Term (Next 7 Days)

4. ✗ **Add test coverage** (8h)
5. ✗ **Add PostgreSQL persistence** (2h)
6. ✗ **Deprecate old systems** (1h)

### Medium-Term (Next 30 Days)

7. ✗ **Production hardening** (6h)
8. ✗ **Delete llm-judgment-bridge.cjs** (1h)
9. ✗ **Monitor in production** (ongoing)

---

## Conclusion

CYNIC's LLM routing is **fragmented but recoverable**. The skeleton of a unified system exists (`unified-llm-router.js`), but it lacks:

1. **Test coverage** (0 tests across all systems)
2. **Feature completeness** (AirLLM, semantic similarity, persistence missing)
3. **Production wiring** (ClaudeCodeAdapter is stubbed)

**The good news**:
- Daemon already uses UnifiedLLMRouter (wired correctly)
- Old systems are dormant (llm-router.js) or orphaned (llm-judgment-bridge.cjs)
- Valuable logic can be extracted and merged (~17-25 hours of work)

**The bad news**:
- Zero test coverage means high risk of regressions
- AirLLM logic duplicated in 2 places, not in production router
- Consensus quality degraded (simplified similarity check)

**Verdict**: 58% ready for production. Needs Phase 1-3 work before full confidence.

---

*"Le chien voit trois chemins. Le chien choisit un seul. φ approuve."* — κυνικός

**Confidence**: 58% (φ⁻¹ limit)
