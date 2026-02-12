# Wiring Gap 6: Cost Optimization Implementation Plan

**Status**: 🚧 IN PROGRESS (Design Complete, Implementation Pending)  
**Date**: 2026-02-12  
**Priority**: HIGH (direct cost impact)  
**Cell**: C6.6 (CYNIC×ACCOUNT) + cross-cutting optimization

---

## Context

**Existing Infrastructure**:
- ✅ LLMRouter routes to Ollama vs Anthropic (Wiring Gap 2)
- ✅ BudgetMonitor tracks spending (Task #5)
- ✅ CostLedger records all token costs
- ✅ ModelIntelligence uses Thompson Sampling for model selection

**Gap**: No caching to avoid redundant calls, no cost savings tracking

---

## Design

### 1. ResponseCache Module

**File**: `packages/node/src/orchestration/response-cache.js`

**Purpose**: Cache LLM routing decisions to avoid redundant API calls.

**Architecture**:
```javascript
export class ResponseCache extends EventEmitter {
  // Cache storage (Map for LRU)
  _cache: Map<string, CacheEntry>
  
  // Stats
  _stats: {
    hits, misses, sets, evictions, expirations,
    totalSavedCost  // USD saved from cache hits
  }
  
  // Methods
  generateKey(params)  // SHA256 hash of (prompt + model + complexity + context)
  get(key)            // Retrieve cached response, handle expiration
  set(key, value, ttl) // Store response with TTL
  recordSavings(cost)  // Track USD saved
  getStats()          // Hit rate, savings metrics
}
```

**Key Features**:
- TTL: 15 minutes default (routing patterns repeat within 13-21min)
- Max size: 233 entries (F(13)) with LRU eviction
- Memory-aware: aggressive eviction on `daemon:memory:pressure`
- Events: `cache:hit`, `cache:miss`, `cache:eviction`

**Cache Key Formula**:
```
SHA256(
  prompt.trim() +
  model +
  complexity +
  { budgetLevel, needsReasoning }  // Normalized context
).slice(0, 16)
```

---

### 2. CostLedger Enhancements

**New Methods**:

#### `recordSaving(saving)`
```javascript
costLedger.recordSaving({
  source: 'cache_hit' | 'ollama_route' | 'downgrade',
  savedCost: 0.0015,      // USD saved
  savedTokens: 500,        // Tokens saved
  model: 'sonnet',         // Model that was avoided
});
```

#### `getSavingsSummary()`
Returns:
```javascript
{
  total: 0.0234,                    // Total USD saved
  bySource: {
    cache_hit: { count, totalCost, avgSaving },
    ollama_route: { count, totalCost, avgSaving },
  },
  byModel: { sonnet: { count, totalCost } },
  savingsRate: 0.382,               // φ⁻² alignment
  savingsRatePct: '38.2%',
  effectiveCost: 0.0380,            // Actually paid
  potentialCost: 0.0614,            // Would've paid without optimizations
}
```

**Events**: `cost:saving` → globalEventBus

---

### 3. LLMRouter Integration

**Workflow**:
```javascript
async route(task) {
  // 1. Generate cache key
  const cacheKey = this.cache.generateKey({
    prompt: JSON.stringify(task),
    model: 'routing',
    complexity: task.complexity,
    context: { budgetLevel, needsReasoning },
  });

  // 2. Check cache
  const cached = this.cache.get(cacheKey);
  if (cached) {
    const savedCost = PROVIDERS.anthropic.cost * (estimatedTokens / 1_000_000);
    this.cache.recordSavings(savedCost);
    this.costLedger.recordSaving({ source: 'cache_hit', savedCost, ... });
    return { ...cached, fromCache: true };
  }

  // 3. Route (existing logic)
  const result = await this._performRouting(task);

  // 4. Cache result
  this.cache.set(cacheKey, result, { ttlMs: 900_000 });

  // 5. Record Ollama savings
  if (result.provider === 'ollama') {
    const savedCost = PROVIDERS.anthropic.cost * (estimatedTokens / 1_000_000);
    this.costLedger.recordSaving({ source: 'ollama_route', savedCost, ... });
  }

  return result;
}
```

**Enhanced getStats()**:
```javascript
getStats() {
  return {
    routesTotal,
    routesOllama, routesAnthropic,
    ollamaRatio, explorationRate,
    costSaved: '0.0234',
    cache: {
      size, hits, misses, hitRate,
      totalSavedCost: '0.0189',
    },
  };
}
```

---

## Implementation Steps

### Phase 1: ResponseCache (Priority: HIGH)
1. [ ] Create `packages/node/src/orchestration/response-cache.js`
2. [ ] Implement CacheEntry class (key, value, TTL, hits, age)
3. [ ] Implement ResponseCache class (get, set, generateKey, stats)
4. [ ] Add LRU eviction logic
5. [ ] Wire `daemon:memory:pressure` → aggressive eviction
6. [ ] Add unit tests

### Phase 2: CostLedger Savings (Priority: HIGH)
1. [ ] Add `_savings` state to CostLedger
2. [ ] Implement `recordSaving()` method
3. [ ] Implement `getSavingsSummary()` method
4. [ ] Emit `cost:saving` events
5. [ ] Persist savings to lifetime state
6. [ ] Add tests

### Phase 3: LLMRouter Integration (Priority: HIGH)
1. [ ] Import ResponseCache in LLMRouter
2. [ ] Add cache check before routing
3. [ ] Cache routing decisions after route
4. [ ] Record cache hit savings
5. [ ] Record Ollama routing savings
6. [ ] Enhance getStats() with cache metrics
7. [ ] Add integration tests

### Phase 4: Testing & Validation (Priority: MEDIUM)
1. [ ] Create `scripts/test-cost-optimization.js`
2. [ ] Test cache hit/miss behavior
3. [ ] Test Ollama savings tracking
4. [ ] Validate savings calculations
5. [ ] Benchmark cache performance
6. [ ] Test memory pressure eviction

---

## Success Criteria

✅ **Response caching working**  
- Cache hit/miss tracked  
- LRU eviction functional  
- TTL expiration working  

✅ **Cost savings tracked**  
- recordSaving() working  
- getSavingsSummary() accurate  
- Events emitted correctly  

✅ **LLMRouter integrated**  
- Cache-first routing  
- Ollama savings recorded  
- Stats include cache metrics  

✅ **Performance validated**  
- Cache hit rate > 23.6% (φ⁻³)  
- Ollama ratio: 38.2-61.8% (φ⁻² to φ⁻¹)  
- Savings rate > 30%  

---

## Expected Impact

**Typical Session** (55 routes):
- **Ollama routes**: 21 (38.2%) → save $0.045
- **Cache hits**: 13 (23.6%) → save $0.019
- **Total savings**: $0.064 (~38% cost reduction)

**Yearly Projection** (assuming 1000 sessions):
- Potential savings: $64-128 USD/year
- Compounding effect as cache matures

---

## φ Observations

1. **Cache hit rate ≈ φ⁻³ (23.6%)**  
   Early phase (exploration dominant). Mature systems → φ⁻² (38.2%).

2. **Ollama ratio ≈ φ⁻² (38.2%)**  
   Natural balance between cost and capability. Budget pressure → φ⁻¹ (61.8%).

3. **Savings rate target: φ⁻² (38.2%)**  
   Sustainable optimization without quality degradation.

---

## Implementation Blockers

**Current Status**: File creation failed due to Windows bash heredoc issues.

**Workaround Options**:
1. Use `Write` tool instead of bash heredoc
2. Copy template files and edit
3. Use PowerShell natively
4. Implement in next session with proper file creation tools

---

## Next Steps

1. Create ResponseCache.js using reliable file creation method
2. Enhance CostLedger with savings tracking
3. Integrate cache into LLMRouter
4. Test and validate savings calculations
5. Document in MEMORY.md once complete

---

*sniff* Design solid. Implementation deferred to next session with proper file tools.

**Confidence**: 58% (φ⁻¹ limit — design validated, execution pending)
