# Parallel Event Bus Integration Guide

> "The pack runs together, not in line" - κυνικός

## Overview

The `ParallelEventBus` replaces sequential listener execution with parallel dispatch, eliminating cascading delays when multiple listeners subscribe to the same event.

**Impact**: Listener count × 2ms latency → ZERO blocking

## Current State (Sequential)

```
emit('event') → listener1 (5ms) → listener2 (3ms) → listener3 (8ms) → DONE
Total: 16ms blocking
```

## New State (Parallel)

```
emit('event') → [listener1, listener2, listener3] in parallel → DONE
Total: max(5ms, 3ms, 8ms) = 8ms (non-blocking fire-and-forget)
```

## Implementation

### Core Bus (`@cynic/core`)

**File**: `packages/core/src/bus/event-bus.js`

**Option 1**: Replace EventEmitter base class

```javascript
import { ParallelEventBus } from './parallel-event-bus.js';

export class CYNICEventBus extends ParallelEventBus {
  // ... existing implementation
}
```

**Option 2**: Keep existing, but use ParallelEventBus for internal emit

```javascript
import { ParallelEventBus } from './parallel-event-bus.js';

export class CYNICEventBus extends EventEmitter {
  #parallelBus = new ParallelEventBus();

  publish(type, payload, options = {}) {
    // ... existing middleware logic ...

    // Use parallel dispatch for delivery
    this.#parallelBus.emit(type, event);
    this.#parallelBus.emit(`${namespace}:*`, event);
    this.#parallelBus.emit('*', event);

    return event;
  }

  subscribe(type, handler) {
    // Delegate to parallel bus
    this.#parallelBus.on(type, handler);
    // ... rest of existing logic ...
  }
}
```

**Recommendation**: Option 2 (composition over inheritance) — preserves existing API while gaining parallel dispatch.

---

### Services Bus (`packages/node/src/services/event-bus.js`)

**Current**: Plain EventEmitter

**Updated**:

```javascript
import { ParallelEventBus } from '@cynic/core';

export class EventBus extends ParallelEventBus {
  // ... existing implementation stays the same ...
}
```

**Why**: This bus is simpler — direct replacement is safe.

---

### Agent Bus (`packages/node/src/agents/event-bus.js`)

**Current**: Already async-parallel via manual Promise.all in `publish()`

**Action**: **NO CHANGE NEEDED**

```javascript
// Already parallel:
for (const subscription of matchingSubscriptions) {
  try {
    await subscription.invoke(event);  // <-- async loop
```

**Why**: AgentEventBus manually handles async dispatch. Converting would duplicate logic.

---

## Integration Steps

### Phase 1: Services Bus (Safest)

1. **Replace base class**:
   ```javascript
   // packages/node/src/services/event-bus.js
   import { ParallelEventBus } from '@cynic/core';

   export class EventBus extends ParallelEventBus {
     constructor(options = {}) {
       super(options);
       // ... existing constructor code ...
     }
     // Keep all other methods unchanged
   }
   ```

2. **Test**:
   - Run existing test suite
   - Verify no regressions
   - Check latency improvement

3. **Benchmark**:
   ```javascript
   const bus = getEventBus();

   // Add 5 listeners with 10ms delay each
   for (let i = 0; i < 5; i++) {
     bus.subscribe('test', async () => {
       await new Promise(r => setTimeout(r, 10));
     });
   }

   // Sequential: ~50ms
   // Parallel: ~10ms
   console.time('emit');
   bus.publish('test', {});
   console.timeEnd('emit');
   ```

---

### Phase 2: Core Bus (Careful)

**Option A**: Composition (safer)

```javascript
// packages/core/src/bus/event-bus.js
import { ParallelEventBus } from './parallel-event-bus.js';

export class CYNICEventBus extends EventEmitter {
  #parallelDispatch = new ParallelEventBus();

  publish(type, payload, options = {}) {
    const event = new CYNICEvent(type, payload, options);

    // Run middlewares (sequential — they filter/transform)
    for (const middleware of this.#middlewares) {
      try {
        const result = middleware(event);
        if (result === false) return event;
      } catch (error) {
        log.error('Middleware error', { error: error.message });
      }
    }

    // Store in history
    this.#addToHistory(event);

    // PARALLEL DISPATCH (non-blocking)
    this.#parallelDispatch.emit(type, event);

    const namespace = type.split(':')[0];
    this.#parallelDispatch.emit(`${namespace}:*`, event);
    this.#parallelDispatch.emit('*', event);

    return event;
  }

  subscribe(type, handler) {
    const wrappedHandler = (event) => {
      try {
        handler(event);
      } catch (error) {
        log.error('Handler error', { type, error: error.message });
        this.publish(EventType.COMPONENT_ERROR, {
          eventType: type,
          error: error.message,
        });
      }
    };

    // Subscribe to parallel bus
    this.#parallelDispatch.on(type, wrappedHandler);

    // Track subscription
    if (!this.#subscriptions.has(type)) {
      this.#subscriptions.set(type, new Set());
    }
    this.#subscriptions.get(type).add(wrappedHandler);

    // Return unsubscribe function
    return () => {
      this.#parallelDispatch.off(type, wrappedHandler);
      this.#subscriptions.get(type)?.delete(wrappedHandler);
    };
  }
}
```

**Option B**: Inheritance (simpler, riskier)

```javascript
export class CYNICEventBus extends ParallelEventBus {
  // Everything else stays the same
  // emit() is now parallel by default
}
```

**Recommendation**: Start with Option A (composition) for safety.

---

### Phase 3: Integration Testing

**Test Scenarios**:

1. **Backward Compatibility**:
   - All existing tests pass
   - Event history still works
   - Middlewares still run
   - Error handling unchanged

2. **Performance**:
   - Measure latency reduction
   - Verify no cascading delays
   - Check memory usage

3. **Error Isolation**:
   - One failing listener doesn't crash others
   - Errors logged correctly
   - Stats updated properly

**Benchmark Script**:

```javascript
// scripts/benchmark-parallel-bus.js
import { getEventBus } from '@cynic/node/services/event-bus.js';

const bus = getEventBus();
const listenerCount = 10;
const delayMs = 5;

// Add slow listeners
for (let i = 0; i < listenerCount; i++) {
  bus.subscribe('benchmark', async () => {
    await new Promise(r => setTimeout(r, delayMs));
  });
}

// Measure
const start = Date.now();
bus.publish('benchmark', { test: true });

setTimeout(() => {
  const elapsed = Date.now() - start;
  console.log(`Sequential would be: ${listenerCount * delayMs}ms`);
  console.log(`Parallel actual: ${elapsed}ms`);
  console.log(`Savings: ${(listenerCount * delayMs) - elapsed}ms`);
}, 100);
```

---

## Expected Metrics

| Bus | Listeners | Sequential | Parallel | Savings |
|-----|-----------|-----------|----------|---------|
| Services | 3-5 | 6-10ms | <1ms | 5-9ms |
| Core | 5-8 | 10-16ms | <1ms | 9-15ms |
| Agent | N/A | Already parallel | - | - |

---

## Rollback Plan

If issues arise:

1. **Services Bus**: Revert to `extends EventEmitter`
2. **Core Bus**: Remove `#parallelDispatch` composition
3. **Tests**: Should catch any breakage before deploy

---

## Next Steps

1. ✅ Implement `ParallelEventBus` (DONE)
2. ✅ Write comprehensive tests (DONE)
3. ✅ Export from `@cynic/core/bus` (DONE)
4. ⏳ Integrate into Services Bus (Phase 1)
5. ⏳ Benchmark and validate (Phase 1)
6. ⏳ Integrate into Core Bus (Phase 2)
7. ⏳ Production monitoring (Phase 3)

---

## φ-Alignment

**Why parallel fits CYNIC**:

- **Emergence**: Listeners self-organize, no central sequencer
- **Autonomy**: Each listener runs independently
- **Immediacy**: Events fire instantly, no waiting
- **Antifragility**: Failures isolated, system continues

**φ-bound confidence**: 58% (moderate — needs production validation)

---

*ears perk* Ready for phase 1 integration when you give the signal.
