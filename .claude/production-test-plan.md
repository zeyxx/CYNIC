# Production Testing Plan — Validate Budget Control + Wiring

> Goal: Stress-test watchdog, budget control, and wiring under realistic conditions

## Test 1: Watchdog Memory Pressure Recovery

**Objective**: Trigger memory pressure, verify automatic recovery

### Setup:
```javascript
// Create memory-intensive scenario
const stressTest = async () => {
  const huge = new Array(1e7).fill('x'.repeat(100)); // ~1GB
  // Trigger operations that push heap >80%
};
```

### Expected Behavior:
1. Watchdog detects heap >80% → emits `daemon:memory:pressure`
2. ContextCompressor clears caches (topics, outcomes, maturity signals)
3. ModelIntelligence forces Haiku selection
4. KabbalisticRouter sees health degradation → forces LIGHT tier
5. Memory pressure resolves within 60s

### Verification:
- `/health` endpoint shows heap ratio drop
- Logs show cache clearing
- Next routing decision uses Haiku

---

## Test 2: Budget Circuit Breaker Under High Velocity

**Objective**: Simulate rapid LLM calls, verify velocity governor

### Setup:
```bash
# Rapid-fire requests to daemon
for i in {1..20}; do
  curl -X POST http://localhost:6180/llm/ask \
    -H "Content-Type: application/json" \
    -d '{"prompt":"test query"}' &
done
```

### Expected Behavior:
1. CostLedger tracks burn rate → velocity >φ⁻¹ (61.8%)
2. KabbalisticRouter emits `velocity:alarm`
3. Router applies 500ms throttle + downgrades to LIGHT tier
4. CostLedger records alarm in history
5. EmergenceDetector detects velocity spike pattern (if 3+ in 10min)

### Verification:
- Check CostLedger: `_velocityAlarms` array populated
- Check EmergenceDetector: `pattern:detected` event emitted
- Check logs: throttle messages visible

---

## Test 3: Health Degradation → Tier Forcing

**Objective**: Verify health circuit breaker overrides budget

### Setup:
```javascript
// Manually trigger health degradation via globalEventBus
globalEventBus.emit('daemon:health:degraded', {
  level: 'critical',
  issues: [{ subsystem: 'heap', level: 'critical', message: 'Test' }],
  timestamp: Date.now(),
});
```

### Expected Behavior:
1. KabbalisticRouter receives event → sets `_healthLevel = 'critical'`
2. Next route() call checks health BEFORE budget
3. Forces LOCAL tier (no LLM calls) on CRITICAL
4. Health overrides budget priority (health > budget > velocity)

### Verification:
- Router `_healthLevel` updated
- Next routing returns LOCAL tier
- Cost ledger shows $0 cost for forced-LOCAL operations

---

## Test 4: Error Clustering Detection

**Objective**: Trigger same error 3+ times, verify pattern detection

### Setup:
```javascript
// Emit same error 3 times in 5 minutes
for (let i = 0; i < 3; i++) {
  globalEventBus.emit('ERROR_OCCURRED', {
    error: new Error('Test error: connection timeout'),
    context: 'test',
    tool: 'Bash',
    command: 'curl',
  });
}
```

### Expected Behavior:
1. EmergenceDetector records errors in `_errors` array
2. Detects clustering: same error 3+ times in 5min window
3. Emits `pattern:detected` event with type='error_clustering'
4. Logs warning with error message + count

### Verification:
- Check EmergenceDetector `_errors` array length
- Listen for `pattern:detected` event
- Check logs for "Error clustering detected"

---

## Test 5: Subagent Spawn Storm Detection

**Objective**: Spawn 5+ subagents in 30s, verify storm detection

### Setup:
```javascript
// Emit 5 SUBAGENT_STARTED events rapidly
for (let i = 0; i < 5; i++) {
  globalEventBus.emit('SUBAGENT_STARTED', {
    subagentId: `test-${i}`,
    taskType: 'exploration',
    parentId: 'test-parent',
  });
}
```

### Expected Behavior:
1. UnifiedOrchestrator tracks active subagents in `_activeSubagents` Set
2. Detects 5+ starts in 30s window
3. Emits `spawn:storm` event
4. Logs warning with activeCount + recentStarts

### Verification:
- Check UnifiedOrchestrator `_activeSubagents.size`
- Listen for `spawn:storm` event
- Check `_subagentHistory` array (last 89, F(11))

---

## Test 6: Learning Stagnation Detection

**Objective**: Emit 5 non-converged learning cycles, verify stagnation alert

### Setup:
```javascript
// Emit 5 non-converged cycles
for (let i = 0; i < 5; i++) {
  globalEventBus.emit('learning:cycle:complete', {
    module: 'test-module',
    converged: false,
    maturity: 0.3,
  });
}
```

### Expected Behavior:
1. EmergenceDetector tracks cycles in `_learningCycles` array
2. Detects 5 consecutive non-converged cycles
3. Emits `pattern:detected` with type='learning_stagnation'
4. Logs warning with module name

### Verification:
- Check EmergenceDetector `_learningCycles` array
- Listen for `pattern:detected` event
- Check logs for "Learning stagnation detected"

---

## Test 7: Model Recommendation Usage

**Objective**: Verify router uses budget-aware model recommendation

### Setup:
```javascript
// Emit model recommendation
globalEventBus.emit('model:recommendation', {
  model: 'haiku',
  reason: 'budget cautious',
  budgetLevel: 'moderate',
});

// Then route a task
router.route({ taskType: 'code', payload: { input: 'test' } });
```

### Expected Behavior:
1. KabbalisticRouter receives recommendation → sets `_recommendedModel = 'haiku'`
2. Next route() call checks `_recommendedModel`
3. Maps haiku→LIGHT tier
4. Uses LIGHT tier for routing (if no health/budget override)

### Verification:
- Check router `_recommendedModel` value
- Verify routing result uses LIGHT tier
- Check cost ledger for haiku model usage

---

## Test Execution Order

1. **Test 7** (model recommendation) — quick, validates event flow
2. **Test 4** (error clustering) — quick, validates pattern detection
3. **Test 6** (learning stagnation) — quick, validates learning monitoring
4. **Test 5** (spawn storm) — quick, validates orchestration awareness
5. **Test 3** (health degradation) — medium, validates circuit breaker priority
6. **Test 2** (velocity governor) — medium, requires multiple requests
7. **Test 1** (memory pressure) — complex, requires heap stress

---

## Success Criteria

✅ All 7 tests pass
✅ No crashes or unhandled errors
✅ Events flow correctly through wiring
✅ Circuit breakers trigger as expected
✅ Pattern detection works in real-time
✅ Logs show appropriate warnings/info

**Confidence target**: >φ⁻¹ (61.8%) that production deployment will be stable

---

*sniff* On lance les tests?
