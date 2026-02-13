/**
 * @cynic/core - Parallel Event Bus Tests
 *
 * Tests the parallel event dispatch mechanism that prevents
 * slow listeners from blocking the event pipeline.
 *
 * "The pack runs together, not in line" - κυνικός
 *
 * @module @cynic/core/test/parallel-event-bus
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  ParallelEventBus,
  createParallelEventBus,
} from '../src/bus/parallel-event-bus.js';

describe('ParallelEventBus', () => {
  let bus;

  beforeEach(() => {
    bus = new ParallelEventBus();
  });

  afterEach(() => {
    bus.removeAllListeners();
  });

  describe('Construction', () => {
    it('should create bus with default options', () => {
      const b = new ParallelEventBus();
      assert.ok(b instanceof ParallelEventBus);
      assert.strictEqual(b.getMaxListeners(), 55); // φ-aligned default
    });

    it('should accept custom maxListeners', () => {
      const b = new ParallelEventBus({ maxListeners: 100 });
      assert.strictEqual(b.getMaxListeners(), 100);
    });

    it('should accept custom error handler', () => {
      const errors = [];
      const b = new ParallelEventBus({
        onError: (event, err) => {
          errors.push({ event, error: err.message });
        },
      });

      b.on('test', () => {
        throw new Error('handler error');
      });

      b.emit('test');

      // Wait for async handlers
      return new Promise(resolve => {
        setTimeout(() => {
          assert.strictEqual(errors.length, 1);
          assert.strictEqual(errors[0].event, 'test');
          assert.strictEqual(errors[0].error, 'handler error');
          resolve();
        }, 50);
      });
    });
  });

  describe('emit() - Fire and Forget', () => {
    it('should return true if event had listeners', () => {
      bus.on('test', () => {});
      const result = bus.emit('test');
      assert.strictEqual(result, true);
    });

    it('should return false if no listeners', () => {
      const result = bus.emit('test');
      assert.strictEqual(result, false);
    });

    it('should invoke listeners asynchronously', () => {
      let invoked = false;

      bus.on('test', () => {
        invoked = true;
      });

      bus.emit('test');

      // Should NOT be invoked synchronously
      assert.strictEqual(invoked, false);

      // Wait for async execution
      return new Promise(resolve => {
        setTimeout(() => {
          assert.strictEqual(invoked, true);
          resolve();
        }, 50);
      });
    });

    it('should invoke all listeners in parallel', () => {
      const order = [];
      const delays = [100, 50, 150]; // Different delays

      delays.forEach((delay, i) => {
        bus.on('test', async () => {
          await new Promise(r => setTimeout(r, delay));
          order.push(i);
        });
      });

      bus.emit('test');

      // Wait for all handlers
      return new Promise(resolve => {
        setTimeout(() => {
          // Fastest listener should finish first (index 1 with 50ms delay)
          assert.strictEqual(order[0], 1);
          assert.strictEqual(order[1], 0);
          assert.strictEqual(order[2], 2);
          resolve();
        }, 200);
      });
    });

    it('should pass arguments to listeners', () => {
      let receivedArgs;

      bus.on('test', (...args) => {
        receivedArgs = args;
      });

      bus.emit('test', 'arg1', 'arg2', { key: 'value' });

      return new Promise(resolve => {
        setTimeout(() => {
          assert.deepStrictEqual(receivedArgs, ['arg1', 'arg2', { key: 'value' }]);
          resolve();
        }, 50);
      });
    });

    it('should not crash if one listener fails', () => {
      const results = [];

      bus.on('test', async () => {
        results.push(1);
      });

      bus.on('test', async () => {
        throw new Error('Listener 2 failed');
      });

      bus.on('test', async () => {
        results.push(3);
      });

      bus.emit('test');

      return new Promise(resolve => {
        setTimeout(() => {
          // First and third listeners should still execute
          assert.strictEqual(results.length, 2);
          assert.ok(results.includes(1));
          assert.ok(results.includes(3));
          resolve();
        }, 50);
      });
    });

    it('should handle synchronous listeners', () => {
      let syncCalled = false;

      bus.on('test', () => {
        syncCalled = true;
      });

      bus.emit('test');

      return new Promise(resolve => {
        setTimeout(() => {
          assert.strictEqual(syncCalled, true);
          resolve();
        }, 50);
      });
    });

    it('should handle async listeners', () => {
      let asyncCalled = false;

      bus.on('test', async () => {
        await new Promise(r => setTimeout(r, 10));
        asyncCalled = true;
      });

      bus.emit('test');

      return new Promise(resolve => {
        setTimeout(() => {
          assert.strictEqual(asyncCalled, true);
          resolve();
        }, 50);
      });
    });

    it('should update statistics', () => {
      bus.on('test1', () => {});
      bus.on('test1', () => {});
      bus.on('test2', () => {});

      bus.emit('test1');
      bus.emit('test2');

      return new Promise(resolve => {
        setTimeout(() => {
          const stats = bus.getStats();
          assert.strictEqual(stats.eventsEmitted, 2);
          assert.strictEqual(stats.listenersInvoked, 3);
          assert.strictEqual(stats.parallelBatches, 2);
          resolve();
        }, 50);
      });
    });
  });

  describe('emitAndWait() - Wait for Completion', () => {
    it('should wait for all listeners to complete', async () => {
      const order = [];

      bus.on('test', async () => {
        await new Promise(r => setTimeout(r, 50));
        order.push(1);
      });

      bus.on('test', async () => {
        await new Promise(r => setTimeout(r, 20));
        order.push(2);
      });

      const result = await bus.emitAndWait('test');

      // All listeners should have completed
      assert.strictEqual(result.success, 2);
      assert.strictEqual(result.failed, 0);
      assert.strictEqual(result.errors.length, 0);
      assert.strictEqual(order.length, 2);
    });

    it('should report failed listeners', async () => {
      bus.on('test', async () => {
        // Success
      });

      bus.on('test', async () => {
        throw new Error('Failed');
      });

      const result = await bus.emitAndWait('test');

      assert.strictEqual(result.success, 1);
      assert.strictEqual(result.failed, 1);
      assert.strictEqual(result.errors.length, 1);
      assert.strictEqual(result.errors[0].message, 'Failed');
    });

    it('should return immediately if no listeners', async () => {
      const result = await bus.emitAndWait('test');

      assert.strictEqual(result.success, 0);
      assert.strictEqual(result.failed, 0);
      assert.strictEqual(result.errors.length, 0);
    });

    it('should pass arguments to listeners', async () => {
      let receivedArgs;

      bus.on('test', (...args) => {
        receivedArgs = args;
      });

      await bus.emitAndWait('test', 'a', 'b', 'c');

      assert.deepStrictEqual(receivedArgs, ['a', 'b', 'c']);
    });

    it('should update statistics', async () => {
      bus.on('test', () => {});
      bus.on('test', () => { throw new Error('fail'); });

      await bus.emitAndWait('test');

      const stats = bus.getStats();
      assert.strictEqual(stats.eventsEmitted, 1);
      assert.strictEqual(stats.listenersInvoked, 2);
      assert.strictEqual(stats.listenersFailed, 1);
    });
  });

  describe('Performance - Parallel vs Sequential', () => {
    it('should execute listeners in parallel (not sequential)', async () => {
      const startTime = Date.now();
      const executionTimes = [];

      // Add 3 listeners that each take 50ms
      for (let i = 0; i < 3; i++) {
        bus.on('test', async () => {
          const start = Date.now();
          await new Promise(r => setTimeout(r, 50));
          executionTimes.push(Date.now() - start);
        });
      }

      await bus.emitAndWait('test');
      const totalTime = Date.now() - startTime;

      // Parallel: ~50ms total (all run together)
      // Sequential: ~150ms total (one after another)
      assert.ok(totalTime < 100, `Expected < 100ms (parallel), got ${totalTime}ms`);

      // All listeners should have taken ~50ms each
      executionTimes.forEach(time => {
        assert.ok(time >= 40 && time < 70, `Listener took ${time}ms`);
      });
    });

    it('should not block emit() caller', () => {
      const startTime = Date.now();

      bus.on('test', async () => {
        await new Promise(r => setTimeout(r, 100));
      });

      bus.emit('test'); // Fire and forget

      const emitTime = Date.now() - startTime;

      // Should return almost immediately (< 10ms)
      assert.ok(emitTime < 10, `emit() took ${emitTime}ms (should be < 10ms)`);
    });
  });

  describe('Error Handling', () => {
    it('should log errors but not throw', () => {
      bus.on('test', () => {
        throw new Error('Listener error');
      });

      // Should not throw
      assert.doesNotThrow(() => {
        bus.emit('test');
      });
    });

    it('should call custom error handler', () => {
      const errors = [];

      bus.setErrorHandler((event, err, listener) => {
        errors.push({ event, message: err.message });
      });

      bus.on('test', () => {
        throw new Error('Custom error');
      });

      bus.emit('test');

      return new Promise(resolve => {
        setTimeout(() => {
          assert.strictEqual(errors.length, 1);
          assert.strictEqual(errors[0].event, 'test');
          assert.strictEqual(errors[0].message, 'Custom error');
          resolve();
        }, 50);
      });
    });

    it('should handle error in error handler gracefully', () => {
      bus.setErrorHandler(() => {
        throw new Error('Error handler error');
      });

      bus.on('test', () => {
        throw new Error('Listener error');
      });

      // Should not throw
      assert.doesNotThrow(() => {
        bus.emit('test');
      });
    });

    it('should clear error handler', () => {
      const errors = [];

      bus.setErrorHandler((event, err) => {
        errors.push(err.message);
      });

      bus.clearErrorHandler();

      bus.on('test', () => {
        throw new Error('Error');
      });

      bus.emit('test');

      return new Promise(resolve => {
        setTimeout(() => {
          // Custom handler should not have been called
          assert.strictEqual(errors.length, 0);
          resolve();
        }, 50);
      });
    });
  });

  describe('Statistics', () => {
    it('should track events emitted', () => {
      bus.on('test', () => {});

      bus.emit('test');
      bus.emit('test');
      bus.emit('other', 'data');

      const stats = bus.getStats();
      assert.strictEqual(stats.eventsEmitted, 2); // 'test' emitted twice, 'other' has no listeners
    });

    it('should track listeners invoked', () => {
      bus.on('test', () => {});
      bus.on('test', () => {});
      bus.on('test', () => {});

      bus.emit('test');

      return new Promise(resolve => {
        setTimeout(() => {
          const stats = bus.getStats();
          assert.strictEqual(stats.listenersInvoked, 3);
          resolve();
        }, 50);
      });
    });

    it('should track listeners failed', () => {
      bus.on('test', () => {});
      bus.on('test', () => { throw new Error('fail'); });
      bus.on('test', () => { throw new Error('fail'); });

      bus.emit('test');

      return new Promise(resolve => {
        setTimeout(() => {
          const stats = bus.getStats();
          assert.strictEqual(stats.listenersFailed, 2);
          resolve();
        }, 50);
      });
    });

    it('should calculate average listeners per event', () => {
      bus.on('test1', () => {});
      bus.on('test1', () => {});
      bus.on('test2', () => {});

      bus.emit('test1'); // 2 listeners
      bus.emit('test2'); // 1 listener

      return new Promise(resolve => {
        setTimeout(() => {
          const stats = bus.getStats();
          assert.strictEqual(stats.averageListenersPerEvent, '1.50');
          resolve();
        }, 50);
      });
    });

    it('should calculate failure rate', () => {
      bus.on('test', () => {});
      bus.on('test', () => { throw new Error('fail'); });

      bus.emit('test');

      return new Promise(resolve => {
        setTimeout(() => {
          const stats = bus.getStats();
          assert.strictEqual(stats.failureRate, '50.00%');
          resolve();
        }, 50);
      });
    });

    it('should reset statistics', () => {
      bus.on('test', () => {});
      bus.emit('test');

      return new Promise(resolve => {
        setTimeout(() => {
          bus.resetStats();

          const stats = bus.getStats();
          assert.strictEqual(stats.eventsEmitted, 0);
          assert.strictEqual(stats.listenersInvoked, 0);
          assert.strictEqual(stats.listenersFailed, 0);
          assert.strictEqual(stats.parallelBatches, 0);
          resolve();
        }, 50);
      });
    });
  });

  describe('EventEmitter Compatibility', () => {
    it('should support on() method', () => {
      let called = false;
      bus.on('test', () => { called = true; });
      bus.emit('test');

      return new Promise(resolve => {
        setTimeout(() => {
          assert.strictEqual(called, true);
          resolve();
        }, 50);
      });
    });

    it('should support once() method', () => {
      let count = 0;
      bus.once('test', () => { count++; });

      bus.emit('test');
      bus.emit('test');

      return new Promise(resolve => {
        setTimeout(() => {
          assert.strictEqual(count, 1);
          resolve();
        }, 50);
      });
    });

    it('should support off() method', () => {
      const handler = () => {};
      bus.on('test', handler);
      bus.off('test', handler);

      const result = bus.emit('test');
      assert.strictEqual(result, false); // No listeners
    });

    it('should support removeAllListeners()', () => {
      bus.on('test', () => {});
      bus.on('test', () => {});

      bus.removeAllListeners('test');

      const result = bus.emit('test');
      assert.strictEqual(result, false);
    });

    it('should support listenerCount()', () => {
      bus.on('test', () => {});
      bus.on('test', () => {});

      assert.strictEqual(bus.listenerCount('test'), 2);
    });

    it('should support listeners() method', () => {
      const h1 = () => {};
      const h2 = () => {};

      bus.on('test', h1);
      bus.on('test', h2);

      const listeners = bus.listeners('test');
      assert.strictEqual(listeners.length, 2);
      assert.strictEqual(listeners[0], h1);
      assert.strictEqual(listeners[1], h2);
    });
  });

  describe('createParallelEventBus() Factory', () => {
    it('should create a ParallelEventBus instance', () => {
      const b = createParallelEventBus();
      assert.ok(b instanceof ParallelEventBus);
    });

    it('should accept options', () => {
      const b = createParallelEventBus({ maxListeners: 200 });
      assert.strictEqual(b.getMaxListeners(), 200);
    });
  });

  describe('Edge Cases', () => {
    it('should handle listener that returns promise', async () => {
      let resolved = false;

      bus.on('test', () => {
        return new Promise(resolve => {
          setTimeout(() => {
            resolved = true;
            resolve();
          }, 20);
        });
      });

      await bus.emitAndWait('test');

      assert.strictEqual(resolved, true);
    });

    it('should handle empty arguments', () => {
      let called = false;
      bus.on('test', () => { called = true; });

      bus.emit('test');

      return new Promise(resolve => {
        setTimeout(() => {
          assert.strictEqual(called, true);
          resolve();
        }, 50);
      });
    });

    it('should handle many listeners (stress test)', async () => {
      const count = 50;
      let invocations = 0;

      for (let i = 0; i < count; i++) {
        bus.on('stress', () => {
          invocations++;
        });
      }

      await bus.emitAndWait('stress');

      assert.strictEqual(invocations, count);
    });

    it('should handle rapid-fire emissions', async () => {
      let count = 0;
      bus.on('rapid', () => { count++; });

      // Emit 10 events rapidly
      for (let i = 0; i < 10; i++) {
        bus.emit('rapid');
      }

      // Wait for all to process
      await new Promise(r => setTimeout(r, 100));

      assert.strictEqual(count, 10);
    });
  });
});
