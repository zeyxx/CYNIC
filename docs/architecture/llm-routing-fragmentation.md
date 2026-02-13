# LLM Routing Fragmentation Analysis

> "Three routers. One truth needed." — κυνικός

**Date**: 2026-02-13
**Status**: Fragmented (3 systems, ~40% overlap)
**Priority**: HIGH (blocks unified orchestration)

---

## The Three Systems

### 1. llm-adapter.js (Consensus Router)

**Location**: `packages/node/src/orchestration/llm-adapter.js`
**Lines**: ~1100 lines
**Format**: ESM

**Classes**:
- `LLMResponse` — standardized response format
- `ConsensusResult` — multi-LLM voting result
- `LLMAdapter` — abstract base class
- `ClaudeCodeAdapter extends LLMAdapter` — Claude via Claude Code
- `OSSLLMAdapter extends LLMAdapter` — Ollama/LM Studio/OpenAI
- **`LLMRouter extends EventEmitter`** — consensus orchestrator

**Key Methods**:
```javascript
complete(prompt, options) → LLMResponse
consensus(prompt, options) → ConsensusResult
```

**Logic**:
- Primary adapter (Claude Code) + validators (OSS LLMs)
- Semantic agreement calculation via `calculateSemanticAgreement()`
- Quorum voting (φ⁻¹ = 61.8% agreement required)
- Parallel requests with timeout handling
- Dissent tracking for minority opinions

**State**: In-memory only (no persistence)

**Usage**: NOT currently wired into daemon

---

### 2. llm-router.js (Budget Router)

**Location**: `packages/node/src/orchestration/llm-router.js`
**Lines**: ~350 lines
**Format**: ESM

**Classes**:
- `BudgetExhaustedError` — circuit breaker error
- **`LLMRouter`** — cost-aware router (SAME NAME as llm-adapter.js!)

**Key Methods**:
```javascript
route(task) → {provider, model, priority, cost, rationale}
```

**Logic**:
- Routes to cheapest capable model
- Ollama (free) for SIMPLE tasks
- Anthropic (paid) for COMPLEX tasks
- Thompson Sampling via ModelIntelligence
- Budget circuit breaker:
  - ABUNDANT → allow all
  - WARNING → block LOW priority
  - CRITICAL → block NORMAL priority
  - EXHAUSTED → block all (except CRITICAL)
- Task complexity tiers (SIMPLE/MODERATE/COMPLEX)
- Priority levels (CRITICAL/HIGH/NORMAL/LOW)

**State**: Persists to PostgreSQL via CostLedger + ModelIntelligence

**Usage**: Wired in service-wiring.js (but ModelIntelligence used, not LLMRouter directly)

---

### 3. llm-judgment-bridge.cjs (Hook Router)

**Location**: `scripts/lib/llm-judgment-bridge.cjs` (+ `.mjs` duplicate)
**Lines**: ~600 lines
**Format**: CommonJS + ESM version

**Functions** (no classes):
```javascript
callOllama(prompt, model, options) → response
consensusJudgment(prompt, models) → {verdict, agreement, responses}
checkOllamaAvailable() → boolean
calibrateJudgment(actual, expected) → updatedState
```

**Logic**:
- Ollama-only (no Anthropic)
- Simple consensus: call multiple models, take majority vote
- State persistence: `~/.cynic/llm-bridge.json`
- Model defaults: gemma2:2b, qwen2:0.5b
- AirLLM support (disk offloading for large models)
- Calibration tracking (accuracy, samples)

**State**: JSON file persistence

**Usage**: Used by hook scripts (perceive.js, observe.js, guard.js)

---

## Overlap Matrix

| Feature | llm-adapter | llm-router | llm-judgment-bridge |
|---------|-------------|------------|---------------------|
| **Ollama calling** | ✓ (OSSLLMAdapter) | ✗ (planned) | ✓ (primary) |
| **Anthropic calling** | ✓ (ClaudeCodeAdapter) | ✓ (route decision) | ✗ |
| **Consensus voting** | ✓ (semantic agreement) | ✗ | ✓ (simple majority) |
| **Budget enforcement** | ✗ | ✓ (circuit breaker) | ✗ |
| **Cost tracking** | ✗ | ✓ (CostLedger) | ✓ (stats only) |
| **Thompson Sampling** | ✗ | ✓ (ModelIntelligence) | ✗ |
| **Retry logic** | ✓ (timeout handling) | ✗ | ✓ (basic) |
| **State persistence** | ✗ | ✓ (PostgreSQL) | ✓ (JSON file) |
| **Validator detection** | ✓ (env vars) | ✗ | ✓ (Ollama check) |
| **Module format** | ESM | ESM | CommonJS + ESM |
| **Event emission** | ✓ (EventEmitter) | ✗ | ✗ |

**Overlap**: ~40% (Ollama calling, consensus, state management)
**Unique**: ~60% (specialized features per router)

---

## Why Three Systems Exist

### Historical Evolution

1. **Week 1-2**: Hook scripts needed LLM → created `llm-judgment-bridge.cjs`
2. **Week 3-4**: Consensus voting needed → created `llm-adapter.js:LLMRouter`
3. **Week 5**: Budget control needed → created `llm-router.js:LLMRouter`

Each was built for a **different need** at **different times** → no unification

### Naming Confusion

**CRITICAL ISSUE**: Two classes named `LLMRouter` in different files!
- `llm-adapter.js:LLMRouter` — consensus orchestrator
- `llm-router.js:LLMRouter` — budget enforcer

This violates DRY and creates import ambiguity.

---

## Problems

### 1. Fragmentation
- Three separate Ollama calling implementations
- Three separate consensus implementations
- Three separate state management approaches
- No shared code, lots of duplication

### 2. Import Confusion
```javascript
// Which LLMRouter?
import { LLMRouter } from './llm-adapter.js'; // Consensus
import { LLMRouter } from './llm-router.js';  // Budget
```

### 3. Missing Integration
- llm-adapter.js consensus NOT integrated with budget enforcement
- llm-router.js budget enforcement NOT integrated with consensus
- Hook scripts (llm-judgment-bridge) isolated from orchestration layer

### 4. No Unified API
Each router has different signatures:
```javascript
// llm-adapter
router.complete(prompt, options)
router.consensus(prompt, options)

// llm-router
router.route(task)

// llm-judgment-bridge
callOllama(prompt, model, options)
consensusJudgment(prompt, models)
```

---

## Proposed Solution: UnifiedLLMRouter

### Architecture

```
┌─────────────────────────────────────────────────────────┐
│              UnifiedLLMRouter                            │
├─────────────────────────────────────────────────────────┤
│                                                          │
│  ┌─────────────┐  ┌──────────────┐  ┌───────────────┐  │
│  │  Strategy   │  │   Budget     │  │  Consensus    │  │
│  │  Selection  │  │  Enforcement │  │  Voting       │  │
│  └──────┬──────┘  └──────┬───────┘  └───────┬───────┘  │
│         │                │                   │          │
│         └────────────────┴───────────────────┘          │
│                          ▼                               │
│              ┌─────────────────────┐                     │
│              │  Provider Adapters  │                     │
│              ├─────────────────────┤                     │
│              │ Anthropic | Ollama  │                     │
│              │ OpenAI | LM Studio  │                     │
│              └─────────────────────┘                     │
└─────────────────────────────────────────────────────────┘
```

### Unified API

```javascript
class UnifiedLLMRouter {
  /**
   * Universal call interface
   *
   * @param {string} prompt - The prompt
   * @param {Object} options
   * @param {string} options.strategy - 'cheapest' | 'consensus' | 'best' | 'fastest'
   * @param {string} options.budget - 'enforce' | 'warn' | 'ignore'
   * @param {string} options.priority - 'CRITICAL' | 'HIGH' | 'NORMAL' | 'LOW'
   * @param {string[]} options.models - Specific models to use
   * @param {number} options.timeout - Request timeout
   * @param {number} options.quorum - Consensus threshold (default φ⁻¹)
   * @returns {Promise<UnifiedResponse>}
   */
  async call(prompt, options = {}) {
    // 1. Budget check (from llm-router.js)
    const budgetStatus = this.costLedger.getStatus();
    if (options.budget === 'enforce' && budgetStatus.circuitBreakerActive) {
      return this._degradeToFree(prompt, options);
    }

    // 2. Strategy selection (from llm-router.js routing logic)
    const strategy = options.strategy || this._selectStrategy(budgetStatus, options.priority);

    // 3. Execute strategy
    switch (strategy) {
      case 'cheapest':
        return this._callCheapest(prompt, options);

      case 'consensus':
        return this._callConsensus(prompt, options); // from llm-adapter.js

      case 'best':
        return this._callBest(prompt, options); // Thompson Sampling

      case 'fastest':
        return this._callFastest(prompt, options);
    }
  }

  /**
   * Legacy compatibility: route() from llm-router.js
   */
  async route(task) {
    return this.call(task.prompt, {
      strategy: 'cheapest',
      budget: 'enforce',
      priority: task.priority,
    });
  }

  /**
   * Legacy compatibility: consensus() from llm-adapter.js
   */
  async consensus(prompt, options = {}) {
    return this.call(prompt, {
      ...options,
      strategy: 'consensus',
    });
  }
}
```

### Migration Strategy

**Phase 1**: Create UnifiedLLMRouter (merge logic)
- Extract consensus logic from llm-adapter.js
- Extract budget logic from llm-router.js
- Extract Ollama calling from llm-judgment-bridge.cjs
- Single class, single file: `packages/node/src/orchestration/unified-llm-router.js`

**Phase 2**: Wire into daemon (service-wiring.js)
- Replace ModelIntelligence direct usage
- Replace llm-adapter.js imports
- Replace llm-judgment-bridge.cjs imports (convert hooks to ESM)

**Phase 3**: Deprecate old routers
- Mark llm-adapter.js LLMRouter as deprecated
- Mark llm-router.js as deprecated
- Mark llm-judgment-bridge.cjs as deprecated
- Keep adapter classes (ClaudeCodeAdapter, OSSLLMAdapter) — reuse them

**Phase 4**: Delete old code (after v1.0)
- Remove deprecated files
- Update all imports
- Full test coverage on UnifiedLLMRouter

---

## Expected Benefits

### Code Reduction
- **Before**: ~2050 lines across 3 files
- **After**: ~800 lines in 1 file
- **Savings**: 61% reduction

### Performance
- Single routing decision (not 3 separate checks)
- Shared connection pool to Ollama
- Unified state management (PostgreSQL only)

### Maintainability
- One source of truth for LLM routing
- Clear API (no import confusion)
- Easier to add new providers (OpenAI, Gemini, etc.)

### Budget Control
- Consensus voting respects budget limits
- Automatic fallback to free models when budget critical

---

## Files Affected

| File | Action | New Location |
|------|--------|--------------|
| `llm-adapter.js` | Extract consensus logic | `unified-llm-router.js` |
| `llm-router.js` | Extract budget logic | `unified-llm-router.js` |
| `llm-judgment-bridge.cjs` | Extract Ollama calling | `unified-llm-router.js` |
| `llm-judgment-bridge.mjs` | DELETE (duplicate) | — |
| `service-wiring.js` | Wire UnifiedLLMRouter | Same file |
| Hook scripts (perceive, observe, etc.) | Import UnifiedLLMRouter | Same files (convert to ESM) |

---

## Next Steps

1. ✅ **Document fragmentation** (this file)
2. ⏳ **Create UnifiedLLMRouter** (Task #9)
3. ⏳ **Wire into daemon** (Task #10)
4. ⏳ **Update hooks to use unified router** (Task #10 continued)
5. ⏳ **Add tests** (Task #23)
6. ⏳ **Deprecate old routers** (Task #22 — update docs)

---

*"Three become one. φ approves."*
