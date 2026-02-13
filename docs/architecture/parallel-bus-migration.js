/**
 * Parallel Event Bus Migration Examples
 *
 * Copy-paste integration patterns for converting existing event buses
 * to use ParallelEventBus for non-blocking dispatch.
 *
 * "The pack runs together, not in line" - κυνικός
 */

// ═══════════════════════════════════════════════════════════════════════════
// EXAMPLE 1: Services Bus (Direct Replacement)
// ═══════════════════════════════════════════════════════════════════════════

// BEFORE (Sequential):
// packages/node/src/services/event-bus.js

import { EventEmitter } from 'events';

export class EventBus extends EventEmitter {
  constructor(options = {}) {
    super();
    // ... existing setup ...
  }

  publish(eventType, data, meta = {}) {
    // ... existing logic ...
    this.emit(eventType, event); // <-- SEQUENTIAL
    // ...
  }
}

// AFTER (Parallel):
// packages/node/src/services/event-bus.js

import { ParallelEventBus } from '@cynic/core';

export class EventBus extends ParallelEventBus {
  constructor(options = {}) {
    super(options);
    // ... existing setup UNCHANGED ...
  }

  publish(eventType, data, meta = {}) {
    // ... existing logic UNCHANGED ...
    this.emit(eventType, event); // <-- NOW PARALLEL (inherited)
    // ...
  }
}

// That's it! Just change the base class.

// ═══════════════════════════════════════════════════════════════════════════
// EXAMPLE 2: Core Bus (Composition Pattern — Safer)
// ═══════════════════════════════════════════════════════════════════════════

// BEFORE:
// packages/core/src/bus/event-bus.js

import { EventEmitter } from 'node:events';

export class CYNICEventBus extends EventEmitter {
  #history = [];
  #subscriptions = new Map();
  #middlewares = [];

  publish(type, payload, options = {}) {
    const event = new CYNICEvent(type, payload, options);

    // Middlewares
    for (const middleware of this.#middlewares) {
      const result = middleware(event);
      if (result === false) return event;
    }

    // History
    this.#addToHistory(event);

    // Emit (SEQUENTIAL)
    this.emit(type, event);
    this.emit(`${namespace}:*`, event);
    this.emit('*', event);

    return event;
  }

  subscribe(type, handler) {
    const wrappedHandler = (event) => {
      try {
        handler(event);
      } catch (error) {
        log.error('Handler error', { type, error: error.message });
      }
    };

    this.on(type, wrappedHandler);
    // ... subscription tracking ...
  }
}

// AFTER (Composition):
// packages/core/src/bus/event-bus.js

import { EventEmitter } from 'node:events';
import { ParallelEventBus } from './parallel-event-bus.js';

export class CYNICEventBus extends EventEmitter {
  #history = [];
  #subscriptions = new Map();
  #middlewares = [];
  #parallelBus = null; // <-- NEW

  constructor(options = {}) {
    super();
    this.#historyLimit = options.historyLimit || 1000;
    this.setMaxListeners(100);

    // NEW: Create parallel dispatcher
    this.#parallelBus = new ParallelEventBus({
      maxListeners: 100,
      onError: (event, error, listener) => {
        // Integrate with existing error handling
        this.publish(EventType.COMPONENT_ERROR, {
          eventType: event,
          error: error.message,
        });
      },
    });
  }

  publish(type, payload, options = {}) {
    const event = new CYNICEvent(type, payload, options);

    // Middlewares (UNCHANGED)
    for (const middleware of this.#middlewares) {
      try {
        const result = middleware(event);
        if (result === false) return event;
      } catch (error) {
        log.error('Middleware error', { error: error.message });
      }
    }

    // History (UNCHANGED)
    this.#addToHistory(event);

    // CHANGED: Parallel emit
    const namespace = type.split(':')[0];
    this.#parallelBus.emit(type, event);
    this.#parallelBus.emit(`${namespace}:*`, event);
    this.#parallelBus.emit('*', event);

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

    // CHANGED: Subscribe to parallel bus
    this.#parallelBus.on(type, wrappedHandler);

    // Track subscription (UNCHANGED)
    if (!this.#subscriptions.has(type)) {
      this.#subscriptions.set(type, new Set());
    }
    this.#subscriptions.get(type).add(wrappedHandler);

    // Return unsubscribe function
    return () => {
      this.#parallelBus.off(type, wrappedHandler); // <-- CHANGED
      this.#subscriptions.get(type)?.delete(wrappedHandler);
    };
  }

  subscribeOnce(type, handler) {
    const wrappedHandler = (event) => {
      try {
        handler(event);
      } catch (error) {
        log.error('Handler error', { type, error: error.message });
      }
    };

    // CHANGED: Use parallel bus
    this.#parallelBus.once(type, wrappedHandler);
  }

  clear() {
    this.#parallelBus.removeAllListeners(); // <-- CHANGED
    this.#subscriptions.clear();
    this.#history = [];
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// EXAMPLE 3: Global Bus Instances
// ═══════════════════════════════════════════════════════════════════════════

// BEFORE:
export const globalEventBus = new CYNICEventBus();

// AFTER (No change needed — uses updated CYNICEventBus):
export const globalEventBus = new CYNICEventBus();

// ═══════════════════════════════════════════════════════════════════════════
// EXAMPLE 4: Agent Bus (No Change — Already Parallel)
// ═══════════════════════════════════════════════════════════════════════════

// packages/node/src/agents/event-bus.js
// Current implementation already does parallel dispatch manually:

async publish(event) {
  const matchingSubscriptions = this._getMatchingSubscriptions(event);

  const errors = [];
  let delivered = 0;

  // Already parallel via async for loop
  for (const subscription of matchingSubscriptions) {
    try {
      await subscription.invoke(event); // <-- Async, non-blocking
      delivered++;
    } catch (error) {
      errors.push(error);
    }
  }

  return { delivered, errors };
}

// NO CHANGE NEEDED HERE

// ═══════════════════════════════════════════════════════════════════════════
// BENCHMARK: Measure Impact
// ═══════════════════════════════════════════════════════════════════════════

async function benchmarkParallelBus() {
  const { getEventBus } = await import('@cynic/node/services/event-bus.js');
  const bus = getEventBus();

  const listenerCount = 10;
  const delayMs = 5;

  // Add slow listeners
  for (let i = 0; i < listenerCount; i++) {
    bus.subscribe('benchmark', async () => {
      await new Promise(resolve => setTimeout(resolve, delayMs));
      console.log(`Listener ${i + 1} completed`);
    });
  }

  console.log('=== PARALLEL BUS BENCHMARK ===');
  console.log(`Listeners: ${listenerCount}`);
  console.log(`Delay per listener: ${delayMs}ms`);
  console.log(`Sequential would take: ${listenerCount * delayMs}ms`);
  console.log('---');

  const start = Date.now();
  bus.publish('benchmark', { test: true });
  const emitTime = Date.now() - start;

  console.log(`emit() returned in: ${emitTime}ms (fire-and-forget)`);

  // Wait for all listeners to complete
  setTimeout(() => {
    const totalTime = Date.now() - start;
    const savings = (listenerCount * delayMs) - totalTime;
    const speedup = ((listenerCount * delayMs) / totalTime).toFixed(2);

    console.log('---');
    console.log(`All listeners completed in: ${totalTime}ms`);
    console.log(`Time saved: ${savings}ms`);
    console.log(`Speedup: ${speedup}x`);

    const stats = bus.getStats();
    console.log('---');
    console.log('Bus Stats:', stats);
  }, 100);
}

// Run: node docs/architecture/parallel-bus-migration.js
// benchmarkParallelBus();

// ═══════════════════════════════════════════════════════════════════════════
// TESTING: Verify Backward Compatibility
// ═══════════════════════════════════════════════════════════════════════════

import assert from 'node:assert/strict';

async function testBackwardCompatibility() {
  const { ParallelEventBus } = await import('@cynic/core');

  const bus = new ParallelEventBus();

  // Test 1: Basic emit/on
  let received = false;
  bus.on('test', () => { received = true; });
  bus.emit('test');

  await new Promise(resolve => setTimeout(resolve, 50));
  assert.strictEqual(received, true, 'Basic emit/on should work');

  // Test 2: Multiple listeners
  let count = 0;
  bus.on('multi', () => { count++; });
  bus.on('multi', () => { count++; });
  bus.on('multi', () => { count++; });
  bus.emit('multi');

  await new Promise(resolve => setTimeout(resolve, 50));
  assert.strictEqual(count, 3, 'All listeners should be invoked');

  // Test 3: Error isolation
  let errorCaught = false;
  let afterErrorCalled = false;

  bus.on('error-test', () => {
    throw new Error('Intentional error');
  });
  bus.on('error-test', () => {
    afterErrorCalled = true;
  });

  bus.setErrorHandler((event, error) => {
    errorCaught = true;
  });

  bus.emit('error-test');

  await new Promise(resolve => setTimeout(resolve, 50));
  assert.strictEqual(errorCaught, true, 'Error should be caught');
  assert.strictEqual(afterErrorCalled, true, 'Subsequent listeners should run');

  console.log('✓ All backward compatibility tests passed');
}

// Run: node docs/architecture/parallel-bus-migration.js
// testBackwardCompatibility();

// ═══════════════════════════════════════════════════════════════════════════
// ROLLBACK: If Issues Arise
// ═══════════════════════════════════════════════════════════════════════════

/*
SERVICES BUS ROLLBACK:
  - Change `extends ParallelEventBus` back to `extends EventEmitter`
  - No other changes needed

CORE BUS ROLLBACK:
  - Remove `#parallelBus` composition
  - Replace `this.#parallelBus.emit()` with `this.emit()`
  - Replace `this.#parallelBus.on()` with `this.on()`
  - Done

Zero data loss. Zero API breakage.
*/
