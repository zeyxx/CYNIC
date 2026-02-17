# LLM Routing Unification Roadmap

> "Three become one. The pack moves as unity." — κυνικός

**Date**: 2026-02-13
**Status**: PLANNED
**Priority**: HIGH
**Estimated Effort**: 17-25 hours
**Target**: One unified, tested, production-ready LLM router

---

## Current State Summary

**Active System**: `unified-llm-router.js` (731 lines, wired in daemon)
**Dormant Systems**:
- `llm-adapter.js` (1086 lines, valuable consensus logic)
- `llm-router.js` (400 lines, valuable budget enforcement)
- `llm-judgment-bridge.cjs` (805 lines, ORPHANED — delete)

**Duplication**: ~40% (Ollama calling, consensus, state management)
**Test Coverage**: 0% (no automated tests for any system)

See `multi-llm-audit-2026-02-13.md` for full analysis.

---

## Phase 1: Feature Consolidation (4-6 hours)

### Goal
Merge valuable features from fragmented systems into `unified-llm-router.js`.

### Tasks

#### 1.1 Add AirLLMAdapter (2h)

**Source**: `llm-adapter.js` lines 948-1084

**Target**: `unified-llm-router.js` (new adapter class)

**Code to extract**:
```javascript
class AirLLMAdapter extends OllamaAdapter {
  constructor(options = {}) {
    super({
      provider: 'airllm',
      model: options.model || 'mistral:7b-instruct-q4_0',
      timeout: options.timeout || 120000, // 2 min for large models
      ...options,
    });
    this.deepAnalysis = options.deepAnalysis !== false;
  }

  async complete(prompt, options = {}) {
    const deepOptions = {
      temperature: options.temperature ?? 0.2,
      maxTokens: options.maxTokens ?? 1000,
      ...options,
    };
    return super.complete(prompt, deepOptions);
  }
}
```

**Integration**:
- Add to `unified-llm-router.js` adapters object
- Create factory: `createAirLLMAdapter(options)`
- Add to adapter registry in constructor
- Wire to Strategy.BEST (deep analysis mode)

**Test**: AirLLM validator creation, timeout, deep analysis flag

#### 1.2 Import Semantic Similarity (1h)

**Source**: `@cynic/llm` package

**Current issue**: `_calculateConsensusAgreement()` uses simple pairwise comparison

**Fix**:
```javascript
import { calculateSemanticAgreement } from '@cynic/llm';

_calculateConsensusAgreement(responses) {
  if (responses.length < 2) return 1.0;

  // Use semantic similarity instead of simple comparison
  const result = calculateSemanticAgreement(responses, SimilarityThresholds.HIGH);

  return result.agreement;
}
```

**Test**: Consensus with similar vs dissimilar responses

#### 1.3 Add PostgreSQL Persistence (2h)

**Source**: `llm-router.js` lines 303-333

**Target**: `unified-llm-router.js._callProvider()`

**Code to add**:
```javascript
async _callProvider(providerName, prompt, options) {
  const adapter = this.adapters[providerName];
  const start = Date.now();
  const response = await adapter.complete(prompt, options);
  const duration = Date.now() - start;

  // Track stats
  this.stats.byProvider[providerName] = (this.stats.byProvider[providerName] || 0) + 1;

  // Track cost
  if (response.cost > 0) {
    await this.costLedger.track({
      category: 'llm',
      operation: providerName,
      amount: response.cost,
      metadata: { model: response.model, tokens: response.tokens },
    });
  }

  // NEW: Persist to routing_accuracy table
  try {
    const pool = await this.pool;
    await pool.query(`
      INSERT INTO routing_accuracy (
        router_type, event_type, confidence, metadata
      ) VALUES ($1, $2, $3, $4)
    `, [
      'llm',
      options.taskType || 'default',
      response.confidence,
      JSON.stringify({
        provider: providerName,
        model: response.model,
        strategy: options.strategy,
        duration,
        tokens: response.tokens,
        cost: response.cost,
      })
    ]);
  } catch (err) {
    // Non-blocking
    log.debug('Failed to persist routing decision', { error: err.message });
  }

  this.emit('call:complete', { provider: providerName, duration, response });
  return response;
}
```

**Test**: PostgreSQL record insertion, non-blocking on failure

#### 1.4 Add Latency Tracking (1h)

**Target**: Enable _callFastest() to select lowest latency provider

**Code to add**:
```javascript
constructor(options = {}) {
  // ... existing code ...

  // Latency tracking per provider
  this.latencyStats = {
    anthropic: { p50: null, p95: null, p99: null, samples: [] },
    ollama: { p50: null, p95: null, p99: null, samples: [] },
  };
}

_recordLatency(providerName, duration) {
  const stats = this.latencyStats[providerName];
  if (!stats) return;

  stats.samples.push(duration);

  // Keep last 100 samples (φ-bounded)
  if (stats.samples.length > 100) {
    stats.samples = stats.samples.slice(-100);
  }

  // Calculate percentiles
  const sorted = [...stats.samples].sort((a, b) => a - b);
  stats.p50 = sorted[Math.floor(sorted.length * 0.50)];
  stats.p95 = sorted[Math.floor(sorted.length * 0.95)];
  stats.p99 = sorted[Math.floor(sorted.length * 0.99)];
}

async _callFastest(prompt, options) {
  // Select provider with lowest p95 latency
  const latencies = Object.entries(this.latencyStats)
    .filter(([name]) => this.adapters[name]?.enabled)
    .map(([name, stats]) => ({ name, p95: stats.p95 || Infinity }))
    .sort((a, b) => a.p95 - b.p95);

  const fastest = latencies[0]?.name || 'ollama';
  return this._callProvider(fastest, prompt, options);
}
```

**Test**: Latency tracking, percentile calculation, fastest selection

#### 1.5 Add Hybrid Strategy (1h)

**Source**: `llm-judgment-bridge.cjs` lines 656-704 (hybridJudge)

**Target**: New strategy in `unified-llm-router.js`

**Code to add**:
```javascript
export const Strategy = {
  CHEAPEST: 'cheapest',
  CONSENSUS: 'consensus',
  BEST: 'best',
  FASTEST: 'fastest',
  HYBRID: 'hybrid',  // NEW
};

async _callHybrid(prompt, options) {
  // Step 1: Try fast consensus
  try {
    const consensus = await this._callConsensus(prompt, {
      ...options,
      timeout: 10000,
    });

    if (consensus.hasConsensus) {
      return consensus;
    }
  } catch (err) {
    log.debug('Consensus failed, trying deep analysis', { error: err.message });
  }

  // Step 2: No consensus → try AirLLM (if available)
  const airllm = this.adapters.airllm;
  if (airllm?.enabled) {
    const deepResponse = await this._callProvider('airllm', prompt, {
      ...options,
      temperature: 0.2,
      maxTokens: 1000,
    });

    return {
      ...deepResponse,
      method: 'hybrid_deep',
      metadata: {
        ...deepResponse.metadata,
        consensusFailed: true,
      },
    };
  }

  // Step 3: Fallback to best provider
  return this._callBest(prompt, options);
}
```

**Test**: Consensus → AirLLM → fallback chain

---

## Phase 2: Feedback Loop Wiring (2-3 hours)

### Goal
Connect calibration feedback to Thompson Sampling.

### Tasks

#### 2.1 Add recordFeedback() Method (1h)

**Target**: `unified-llm-router.js`

**Code to add**:
```javascript
/**
 * Record feedback for a previous response
 * Wires to ModelIntelligence Thompson Sampling
 *
 * @param {string} responseId - LLMResponse.id
 * @param {boolean} wasCorrect - User feedback
 * @param {Object} [correction] - Optional correction data
 */
recordFeedback(responseId, wasCorrect, correction = null) {
  // TODO: Store responseId → {provider, model, taskType} mapping
  // For now, require explicit provider/model in correction

  if (!correction?.provider || !correction?.taskType) {
    log.warn('recordFeedback missing provider/taskType', { responseId });
    return;
  }

  // Wire to ModelIntelligence
  this.modelIntelligence.recordOutcome({
    taskType: correction.taskType,
    model: correction.provider,
    success: wasCorrect,
    qualityScore: wasCorrect ? 1.0 : 0.0,
  });

  // Emit event
  this.emit('feedback:recorded', {
    responseId,
    wasCorrect,
    provider: correction.provider,
    taskType: correction.taskType,
  });

  log.info('Feedback recorded', {
    responseId,
    wasCorrect,
    provider: correction.provider,
  });
}
```

**Test**: Feedback recording, ModelIntelligence integration

#### 2.2 Wire Daemon Endpoint (1h)

**Target**: `daemon/llm-endpoints.js`

**Update existing `/llm/feedback` endpoint**:
```javascript
app.post('/llm/feedback', (req, res) => {
  const { responseId, taskType, model, success, qualityScore } = req.body || {};

  if (!responseId) {
    return res.status(400).json({ error: 'Missing responseId' });
  }

  try {
    const router = getUnifiedLLMRouter();

    router.recordFeedback(responseId, success !== false, {
      provider: model,
      taskType,
      qualityScore,
    });

    res.json({
      recorded: true,
      stats: router.getStats(),
    });
  } catch (err) {
    log.error('LLM feedback failed', { error: err.message });
    res.status(500).json({ error: err.message });
  }
});
```

**Test**: HTTP endpoint, router integration

---

## Phase 3: Test Coverage (6-8 hours)

### Goal
Achieve 80%+ test coverage for UnifiedLLMRouter.

### Test File Structure

```
packages/node/test/orchestration/
  unified-llm-router.test.js      // Main test suite
  unified-llm-router-budget.test.js  // Budget enforcement
  unified-llm-router-consensus.test.js  // Multi-LLM voting
  unified-llm-router-integration.test.js  // Daemon integration
```

### Test Categories

#### 3.1 Budget Enforcement Tests (2h)

**Test cases**:
- ✓ ABUNDANT budget → allow all strategies
- ✓ MODERATE budget → allow all strategies
- ✓ CAUTIOUS budget → block LOW priority
- ✓ CRITICAL budget → block NORMAL/LOW priority
- ✓ EXHAUSTED budget → block all except CRITICAL
- ✓ CRITICAL priority → always allowed
- ✓ Budget degradation → fallback to Ollama
- ✓ Event emission: 'budget:block'

#### 3.2 Strategy Tests (2h)

**Test cases**:
- ✓ CHEAPEST: SIMPLE → Ollama, COMPLEX → Anthropic
- ✓ BEST: Thompson Sampling exploration/exploitation
- ✓ FASTEST: Lowest p95 latency selection
- ✓ CONSENSUS: Multi-adapter voting, quorum threshold
- ✓ HYBRID: Consensus → AirLLM → fallback

#### 3.3 Adapter Tests (1h)

**Test cases**:
- ✓ ClaudeCodeAdapter: API call, response parsing
- ✓ OllamaAdapter: API call, timeout, health check
- ✓ AirLLMAdapter: Deep analysis mode, extended timeout
- ✓ Health checks: healthy/unhealthy states

#### 3.4 Consensus Tests (1h)

**Test cases**:
- ✓ 2 adapters, 100% agreement → consensus
- ✓ 2 adapters, <61.8% agreement → no consensus
- ✓ Semantic similarity calculation
- ✓ Dissent tracking
- ✓ Event emission: 'consensus:reached', 'consensus:failed'

#### 3.5 Feedback Loop Tests (1h)

**Test cases**:
- ✓ recordFeedback() → ModelIntelligence.recordOutcome()
- ✓ Event emission: 'feedback:recorded'
- ✓ Missing provider/taskType handling

#### 3.6 Legacy API Tests (1h)

**Test cases**:
- ✓ route(task) → call() wrapper
- ✓ consensus(prompt) → call() wrapper
- ✓ Backward compatibility

#### 3.7 Integration Tests (1h)

**Test cases**:
- ✓ Daemon → router → Ollama (local)
- ✓ PostgreSQL persistence
- ✓ CostLedger integration
- ✓ ModelIntelligence integration

---

## Phase 4: Deprecation (1-2 hours)

### Goal
Mark old systems as deprecated, clean up codebase.

### Tasks

#### 4.1 Add Deprecation Notices (30m)

**llm-adapter.js** (top of file):
```javascript
/**
 * @deprecated This module is superseded by unified-llm-router.js
 *
 * Migration guide:
 * - Replace: import { LLMRouter } from './llm-adapter.js'
 * - With:    import { UnifiedLLMRouter } from './unified-llm-router.js'
 *
 * Removal target: v2.0.0 (after v1.0 stabilizes)
 */
```

**llm-router.js** (top of file):
```javascript
/**
 * @deprecated This module is superseded by unified-llm-router.js
 *
 * Budget enforcement has been integrated into UnifiedLLMRouter.
 *
 * Migration guide:
 * - Replace: router.route(task)
 * - With:    router.call(prompt, { strategy: 'cheapest', budget: 'enforce' })
 *
 * Removal target: v2.0.0
 */
```

#### 4.2 Delete Orphaned Files (30m)

**Delete**:
- `scripts/lib/llm-judgment-bridge.cjs` (805 lines)
- `scripts/lib/llm-judgment-bridge.mjs` (duplicate)

**Rationale**: Hook scripts no longer import this. Consensus logic merged into unified router.

#### 4.3 Update Documentation (30m)

**Files to update**:
- `docs/architecture/LLM-ROUTING-CURRENT.md` → mark as historical
- `docs/architecture/llm-routing-fragmentation.md` → mark as resolved
- `MEMORY.md` → remove llm-judgment-bridge references
- `README.md` → update LLM routing section

---

## Phase 5: Production Hardening (4-6 hours)

### Goal
Make UnifiedLLMRouter battle-ready for production.

### Tasks

#### 5.1 Retry Logic (1h)

**Add to adapters**:
```javascript
async complete(prompt, options = {}) {
  const maxRetries = options.maxRetries || 3;
  const baseDelay = options.retryDelay || 1000;

  for (let attempt = 1; attempt <= maxRetries; attempt++) {
    try {
      return await this._doComplete(prompt, options);
    } catch (err) {
      if (attempt === maxRetries) throw err;

      const delay = baseDelay * Math.pow(2, attempt - 1); // Exponential backoff
      log.warn(`Retry ${attempt}/${maxRetries} after ${delay}ms`, {
        error: err.message,
        provider: this.provider,
      });

      await new Promise(resolve => setTimeout(resolve, delay));
    }
  }
}
```

#### 5.2 Circuit Breaker per Provider (1h)

**Add to router**:
```javascript
constructor(options = {}) {
  // ... existing code ...

  this.circuitBreakers = {
    anthropic: { open: false, failures: 0, lastFailure: null },
    ollama: { open: false, failures: 0, lastFailure: null },
  };
}

_checkCircuitBreaker(providerName) {
  const breaker = this.circuitBreakers[providerName];
  if (!breaker) return true;

  // Open circuit after 5 failures in 5 minutes
  if (breaker.failures >= 5 &&
      breaker.lastFailure &&
      Date.now() - breaker.lastFailure < 300000) {
    breaker.open = true;
    return false;
  }

  // Auto-reset after 5 minutes
  if (breaker.open &&
      breaker.lastFailure &&
      Date.now() - breaker.lastFailure > 300000) {
    breaker.open = false;
    breaker.failures = 0;
    log.info('Circuit breaker reset', { provider: providerName });
  }

  return !breaker.open;
}
```

#### 5.3 Response Caching (1h)

**Add LRU cache**:
```javascript
import { LRUCache } from 'lru-cache';

constructor(options = {}) {
  // ... existing code ...

  this.cache = new LRUCache({
    max: 100, // φ-bounded
    ttl: 300000, // 5 min
  });
}

async call(prompt, options = {}) {
  // Check cache (if enabled)
  if (options.cache !== false) {
    const cacheKey = `${prompt}:${JSON.stringify(options)}`;
    const cached = this.cache.get(cacheKey);

    if (cached) {
      log.debug('Cache hit', { cacheKey });
      this.stats.cacheHits = (this.stats.cacheHits || 0) + 1;
      return cached;
    }
  }

  // ... existing routing logic ...

  // Store in cache
  if (options.cache !== false) {
    this.cache.set(cacheKey, response);
  }

  return response;
}
```

#### 5.4 Rate Limiting (1h)

**Add rate limiter per provider**:
```javascript
import { RateLimiter } from 'limiter';

constructor(options = {}) {
  // ... existing code ...

  this.rateLimiters = {
    anthropic: new RateLimiter({ tokensPerInterval: 10, interval: 'second' }),
    ollama: new RateLimiter({ tokensPerInterval: 100, interval: 'second' }),
  };
}

async _callProvider(providerName, prompt, options) {
  // Wait for rate limit
  const limiter = this.rateLimiters[providerName];
  if (limiter) {
    await limiter.removeTokens(1);
  }

  // ... existing code ...
}
```

#### 5.5 Streaming Support (2h)

**Add SSE streaming**:
```javascript
async stream(prompt, options = {}) {
  const adapter = this._selectAdapter(options);

  if (!adapter.supportsStreaming) {
    throw new Error(`${adapter.provider} does not support streaming`);
  }

  const stream = await adapter.stream(prompt, options);

  // Transform to SSE format
  return stream.on('data', (chunk) => {
    this.emit('stream:chunk', {
      provider: adapter.provider,
      chunk,
    });
  });
}
```

---

## Migration Guide for Existing Code

### Before (llm-adapter.js)
```javascript
import { getLLMRouter } from './llm-adapter.js';

const router = getLLMRouter();
const result = await router.consensus(prompt, { quorum: 0.618 });
```

### After (unified-llm-router.js)
```javascript
import { getUnifiedLLMRouter, Strategy } from './unified-llm-router.js';

const router = getUnifiedLLMRouter();
const result = await router.call(prompt, {
  strategy: Strategy.CONSENSUS,
  quorum: 0.618,
});
```

### Before (llm-router.js)
```javascript
import { getLLMRouter } from './llm-router.js';

const router = getLLMRouter();
const decision = await router.route({
  type: 'code',
  complexity: 'moderate',
  priority: 'NORMAL',
});
```

### After (unified-llm-router.js)
```javascript
import { getUnifiedLLMRouter, Complexity, Priority } from './unified-llm-router.js';

const router = getUnifiedLLMRouter();
const response = await router.call(prompt, {
  strategy: Strategy.CHEAPEST,
  complexity: Complexity.MODERATE,
  priority: Priority.NORMAL,
  budget: BudgetMode.ENFORCE,
});
```

---

## Success Metrics

| Metric | Before | Target | Validation |
|--------|--------|--------|------------|
| Lines of code | 3022 | 1080 | 64% reduction |
| Test coverage | 0% | 80%+ | Jest report |
| Files | 4 | 2 | File count |
| APIs | 4 | 1 | Import analysis |
| φ-alignment | 54% | 75%+ | Axiom scores |
| Duplication | 40% | <5% | Code analysis |
| Wiring | 25% (1/4) | 100% | Daemon check |

---

## Risk Assessment

| Risk | Impact | Mitigation |
|------|--------|------------|
| Breaking changes | HIGH | Legacy API wrappers (route(), consensus()) |
| Test gaps | HIGH | 80%+ coverage requirement |
| Performance regression | MEDIUM | Latency tracking, benchmarks |
| Integration bugs | MEDIUM | Integration tests with daemon |
| PostgreSQL failures | LOW | Non-blocking persistence |

**Overall Risk**: MODERATE (mitigated by phased approach)

---

## Timeline

| Phase | Week 1 | Week 2 | Week 3 | Week 4 |
|-------|--------|--------|--------|--------|
| Phase 1 | ████░░ | ░░░░░░ | ░░░░░░ | ░░░░░░ |
| Phase 2 | ░░░░░░ | ██░░░░ | ░░░░░░ | ░░░░░░ |
| Phase 3 | ░░░░░░ | ░░████ | ██░░░░ | ░░░░░░ |
| Phase 4 | ░░░░░░ | ░░░░░░ | ░░░░░░ | ██░░░░ |
| Phase 5 | ░░░░░░ | ░░░░░░ | ░░░░░░ | ░░████ |

**Target Completion**: 4 weeks (1 phase per week)

---

## Approval Gates

| Gate | Criteria | Approver |
|------|----------|----------|
| Phase 1 | All features merged, manual tests pass | ARCHITECT |
| Phase 2 | Feedback loop wired, daemon endpoint works | ARCHITECT |
| Phase 3 | 80%+ test coverage, all tests pass | ARCHITECT |
| Phase 4 | Old systems marked deprecated, docs updated | ARCHITECT |
| Phase 5 | Production hardening complete, benchmarks good | ARCHITECT |

---

*"Un seul routeur. Une seule vérité. φ guide le chemin."* — κυνικός

**Confidence**: 61% (φ⁻¹ limit)
