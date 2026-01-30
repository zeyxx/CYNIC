/**
 * @cynic/core - Circuit Breaker Tests
 *
 * Tests resilience patterns:
 * - State transitions (CLOSED → OPEN → HALF_OPEN)
 * - Failure counting and thresholds
 * - Timeout-based recovery
 * - Success/failure recording
 *
 * "φ fails gracefully" - κυνικός
 *
 * @module @cynic/core/test/circuit-breaker
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import {
  CircuitBreaker,
  CircuitState,
  withCircuitBreaker,
  DEFAULT_CIRCUIT_CONFIG,
} from '../src/circuit-breaker.js';

// =============================================================================
// CIRCUIT STATE TESTS
// =============================================================================

describe('CircuitState', () => {
  it('should have all expected states', () => {
    assert.strictEqual(CircuitState.CLOSED, 'CLOSED');
    assert.strictEqual(CircuitState.OPEN, 'OPEN');
    assert.strictEqual(CircuitState.HALF_OPEN, 'HALF_OPEN');
  });

  it('should be frozen (immutable)', () => {
    assert.ok(Object.isFrozen(CircuitState));
  });
});

// =============================================================================
// DEFAULT CONFIG TESTS
// =============================================================================

describe('DEFAULT_CIRCUIT_CONFIG', () => {
  it('should have expected values', () => {
    assert.strictEqual(DEFAULT_CIRCUIT_CONFIG.failureThreshold, 5);
    assert.strictEqual(DEFAULT_CIRCUIT_CONFIG.successThreshold, 2);
    assert.strictEqual(typeof DEFAULT_CIRCUIT_CONFIG.resetTimeoutMs, 'number');
    assert.ok(DEFAULT_CIRCUIT_CONFIG.resetTimeoutMs > 0);
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(DEFAULT_CIRCUIT_CONFIG));
  });
});

// =============================================================================
// CIRCUIT BREAKER TESTS
// =============================================================================

describe('CircuitBreaker', () => {
  let breaker;

  beforeEach(() => {
    breaker = new CircuitBreaker({
      name: 'test-breaker',
      failureThreshold: 3,
      successThreshold: 2,
      resetTimeoutMs: 100, // Fast timeout for tests
    });
  });

  describe('Construction', () => {
    it('should create with options', () => {
      const cb = new CircuitBreaker({
        name: 'my-circuit',
        failureThreshold: 5,
        successThreshold: 3,
        resetTimeoutMs: 5000,
      });

      assert.strictEqual(cb.name, 'my-circuit');
      assert.strictEqual(cb.state, CircuitState.CLOSED);
    });

    it('should use defaults when no options provided', () => {
      const cb = new CircuitBreaker();

      assert.strictEqual(cb.name, 'default');
      assert.strictEqual(cb.state, CircuitState.CLOSED);
    });

    it('should start in CLOSED state', () => {
      assert.strictEqual(breaker.state, CircuitState.CLOSED);
      assert.strictEqual(breaker.isClosed, true);
      assert.strictEqual(breaker.isOpen, false);
      assert.strictEqual(breaker.isHalfOpen, false);
    });
  });

  describe('State: CLOSED', () => {
    it('should allow execution when closed', () => {
      assert.strictEqual(breaker.canExecute(), true);
    });

    it('should count failures and trip when threshold reached', () => {
      breaker.recordFailure();
      breaker.recordFailure();
      assert.strictEqual(breaker.state, CircuitState.CLOSED);

      breaker.recordFailure(); // Threshold = 3
      assert.strictEqual(breaker.state, CircuitState.OPEN);
    });

    it('should reset failure count on success', () => {
      breaker.recordFailure();
      breaker.recordFailure();

      breaker.recordSuccess();

      // Now another failure shouldn't trip (count reset)
      breaker.recordFailure();
      assert.strictEqual(breaker.state, CircuitState.CLOSED);
    });
  });

  describe('State: OPEN', () => {
    beforeEach(() => {
      // Trip the breaker
      breaker.recordFailure();
      breaker.recordFailure();
      breaker.recordFailure();
      assert.strictEqual(breaker.state, CircuitState.OPEN);
    });

    it('should not allow execution when open', () => {
      assert.strictEqual(breaker.canExecute(), false);
      assert.strictEqual(breaker.isOpen, true);
    });

    it('should transition to HALF_OPEN after timeout', async () => {
      assert.strictEqual(breaker.canExecute(), false);

      // Wait for timeout
      await new Promise(resolve => setTimeout(resolve, 150));

      // Now should allow (and transition to HALF_OPEN)
      assert.strictEqual(breaker.canExecute(), true);
      assert.strictEqual(breaker.state, CircuitState.HALF_OPEN);
    });
  });

  describe('State: HALF_OPEN', () => {
    beforeEach(async () => {
      // Trip the breaker
      breaker.recordFailure();
      breaker.recordFailure();
      breaker.recordFailure();
      // Wait for timeout
      await new Promise(resolve => setTimeout(resolve, 150));
      breaker.canExecute(); // Trigger transition
      assert.strictEqual(breaker.state, CircuitState.HALF_OPEN);
    });

    it('should limit requests in half-open (1 by default)', () => {
      // First canExecute already used in beforeEach
      // Trying again should return false
      assert.strictEqual(breaker.canExecute(), false);
    });

    it('should close on success threshold met', () => {
      breaker.recordSuccess();
      assert.strictEqual(breaker.state, CircuitState.HALF_OPEN);

      breaker.recordSuccess(); // Success threshold = 2
      assert.strictEqual(breaker.state, CircuitState.CLOSED);
    });

    it('should re-open on failure', () => {
      breaker.recordFailure();
      assert.strictEqual(breaker.state, CircuitState.OPEN);
    });
  });

  describe('Manual Controls', () => {
    it('should trip manually', () => {
      assert.strictEqual(breaker.state, CircuitState.CLOSED);

      breaker.trip();

      assert.strictEqual(breaker.state, CircuitState.OPEN);
    });

    it('should reset manually', () => {
      breaker.trip();
      assert.strictEqual(breaker.state, CircuitState.OPEN);

      breaker.reset();

      assert.strictEqual(breaker.state, CircuitState.CLOSED);
    });
  });

  describe('State Info', () => {
    it('should return current state info', () => {
      breaker.recordFailure();
      breaker.recordFailure();

      const state = breaker.getState();

      assert.strictEqual(state.name, 'test-breaker');
      assert.strictEqual(state.state, CircuitState.CLOSED);
      assert.strictEqual(state.failureCount, 2);
      assert.strictEqual(state.successCount, 0);
    });

    it('should return stats', () => {
      breaker.canExecute();
      breaker.recordFailure();
      breaker.canExecute();
      breaker.recordSuccess();

      const stats = breaker.getStats();

      assert.strictEqual(stats.totalRequests, 2);
      assert.strictEqual(stats.totalFailures, 1);
      assert.strictEqual(stats.totalSuccesses, 1);
      assert.strictEqual(typeof stats.failureRate, 'number');
    });
  });

  describe('Events', () => {
    it('should emit stateChange on trip', () => {
      const events = [];
      breaker.on('stateChange', (data) => events.push(data));

      breaker.trip();

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].from, CircuitState.CLOSED);
      assert.strictEqual(events[0].to, CircuitState.OPEN);
    });

    it('should emit stateChange on reset', () => {
      breaker.trip();
      const events = [];
      breaker.on('stateChange', (data) => events.push(data));

      breaker.reset();

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].from, CircuitState.OPEN);
      assert.strictEqual(events[0].to, CircuitState.CLOSED);
    });

    it('should emit failure event', () => {
      const events = [];
      breaker.on('failure', (data) => events.push(data));

      breaker.recordFailure(new Error('test error'));

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].name, 'test-breaker');
      assert.strictEqual(events[0].failureCount, 1);
      assert.strictEqual(events[0].error, 'test error');
    });

    it('should emit success event', () => {
      const events = [];
      breaker.on('success', (data) => events.push(data));

      breaker.recordSuccess();

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].name, 'test-breaker');
    });
  });
});

// =============================================================================
// WITH CIRCUIT BREAKER HELPER TESTS
// =============================================================================

describe('withCircuitBreaker', () => {
  it('should execute function when circuit closed', async () => {
    const breaker = new CircuitBreaker({ failureThreshold: 3 });
    const fn = mock.fn(() => 'success');

    const result = await withCircuitBreaker(breaker, fn);

    assert.strictEqual(result, 'success');
    assert.strictEqual(fn.mock.calls.length, 1);
  });

  it('should record success on successful execution', async () => {
    const breaker = new CircuitBreaker({ failureThreshold: 3 });
    breaker.recordFailure();
    breaker.recordFailure();

    await withCircuitBreaker(breaker, () => 'ok');

    // Failure count should be reset
    const state = breaker.getState();
    assert.strictEqual(state.failureCount, 0);
  });

  it('should record failure on error', async () => {
    const breaker = new CircuitBreaker({ failureThreshold: 3 });
    const error = new Error('test error');

    await assert.rejects(
      () => withCircuitBreaker(breaker, () => { throw error; }),
      error
    );

    const state = breaker.getState();
    assert.strictEqual(state.failureCount, 1);
  });

  it('should throw when circuit is open', async () => {
    const breaker = new CircuitBreaker({ failureThreshold: 1 });
    breaker.trip();

    await assert.rejects(
      () => withCircuitBreaker(breaker, () => 'should not run'),
      /circuit.*open|open/i
    );
  });

  it('should use fallback when circuit is open', async () => {
    const breaker = new CircuitBreaker({ failureThreshold: 1 });
    breaker.trip();

    const result = await withCircuitBreaker(
      breaker,
      () => 'should not run',
      { fallback: 'fallback value', throwOnOpen: false }
    );

    assert.strictEqual(result, 'fallback value');
  });

  it('should work with async functions', async () => {
    const breaker = new CircuitBreaker({ failureThreshold: 3 });
    const fn = async () => {
      await new Promise(resolve => setTimeout(resolve, 10));
      return 'async result';
    };

    const result = await withCircuitBreaker(breaker, fn);

    assert.strictEqual(result, 'async result');
  });

  it('should trip circuit after threshold failures', async () => {
    const breaker = new CircuitBreaker({ failureThreshold: 2 });
    const failingFn = () => { throw new Error('fail'); };

    // First failure
    await assert.rejects(() => withCircuitBreaker(breaker, failingFn));
    assert.strictEqual(breaker.state, CircuitState.CLOSED);

    // Second failure - should trip
    await assert.rejects(() => withCircuitBreaker(breaker, failingFn));
    assert.strictEqual(breaker.state, CircuitState.OPEN);
  });
});

// =============================================================================
// v1.1 FEATURES - EXPONENTIAL BACKOFF
// =============================================================================

describe('v1.1: Exponential Backoff', () => {
  it('should increase timeout on consecutive failures in half-open', async () => {
    const breaker = new CircuitBreaker({
      name: 'backoff-test',
      failureThreshold: 1,
      resetTimeoutMs: 50,
      maxResetTimeoutMs: 500,
      backoffMultiplier: 2,
      jitterFactor: 0, // Disable jitter for predictable tests
    });

    // Trip the breaker
    breaker.recordFailure();
    assert.strictEqual(breaker.state, CircuitState.OPEN);

    // Wait for timeout, transition to half-open
    await new Promise(resolve => setTimeout(resolve, 60));
    breaker.canExecute();
    assert.strictEqual(breaker.state, CircuitState.HALF_OPEN);

    // Fail again - should double the timeout
    breaker.recordFailure();
    assert.strictEqual(breaker.state, CircuitState.OPEN);

    const state = breaker.getState();
    assert.ok(state.currentBackoffMs >= 100, `Expected backoff >= 100, got ${state.currentBackoffMs}`);
    assert.strictEqual(state.consecutiveOpenings, 1);
  });

  it('should cap timeout at maxResetTimeoutMs', async () => {
    const breaker = new CircuitBreaker({
      name: 'cap-test',
      failureThreshold: 1,
      resetTimeoutMs: 10,
      maxResetTimeoutMs: 50,
      backoffMultiplier: 10,
      jitterFactor: 0,
    });

    // Trip and fail multiple times
    breaker.recordFailure();
    await new Promise(resolve => setTimeout(resolve, 15));
    breaker.canExecute();
    breaker.recordFailure();

    await new Promise(resolve => setTimeout(resolve, 60)); // Wait for longer timeout
    breaker.canExecute();
    breaker.recordFailure();

    const state = breaker.getState();
    assert.ok(state.currentBackoffMs <= 50, `Expected backoff <= 50, got ${state.currentBackoffMs}`);
  });

  it('should reset backoff on successful recovery', async () => {
    const breaker = new CircuitBreaker({
      name: 'reset-backoff-test',
      failureThreshold: 1,
      successThreshold: 1,
      resetTimeoutMs: 20,
      backoffMultiplier: 2,
      jitterFactor: 0,
    });

    // Trip and fail to increase backoff
    breaker.recordFailure();
    await new Promise(resolve => setTimeout(resolve, 30));
    breaker.canExecute();
    breaker.recordFailure();

    let state = breaker.getState();
    assert.strictEqual(state.consecutiveOpenings, 1);

    // Now recover successfully
    await new Promise(resolve => setTimeout(resolve, 50));
    breaker.canExecute();
    breaker.recordSuccess();

    state = breaker.getState();
    assert.strictEqual(state.state, CircuitState.CLOSED);
    assert.strictEqual(state.consecutiveOpenings, 0);
    assert.strictEqual(state.currentBackoffMs, 20); // Reset to original
  });
});

// =============================================================================
// v1.1 FEATURES - HEALTH PROBE
// =============================================================================

describe('v1.1: Health Probe', () => {
  it('should call health probe and record success', async () => {
    let probeCalled = false;
    const breaker = new CircuitBreaker({
      name: 'probe-success-test',
      failureThreshold: 1,
      resetTimeoutMs: 20,
      healthProbe: async () => {
        probeCalled = true;
        return true;
      },
    });

    // Trip and enter half-open
    breaker.recordFailure();
    await new Promise(resolve => setTimeout(resolve, 30));
    breaker.canExecute();
    assert.strictEqual(breaker.state, CircuitState.HALF_OPEN);

    // Run health probe
    const healthy = await breaker.probeHealth();

    assert.ok(probeCalled, 'Health probe should have been called');
    assert.ok(healthy, 'Should report healthy');
  });

  it('should call health probe and record failure', async () => {
    let probeCalled = false;
    const breaker = new CircuitBreaker({
      name: 'probe-fail-test',
      failureThreshold: 1,
      resetTimeoutMs: 20,
      healthProbe: async () => {
        probeCalled = true;
        return false;
      },
    });

    // Trip and enter half-open
    breaker.recordFailure();
    await new Promise(resolve => setTimeout(resolve, 30));
    breaker.canExecute();
    assert.strictEqual(breaker.state, CircuitState.HALF_OPEN);

    // Run health probe - should fail and reopen circuit
    const healthy = await breaker.probeHealth();

    assert.ok(probeCalled, 'Health probe should have been called');
    assert.ok(!healthy, 'Should report unhealthy');
    assert.strictEqual(breaker.state, CircuitState.OPEN);
  });

  it('should handle health probe throwing error', async () => {
    const breaker = new CircuitBreaker({
      name: 'probe-error-test',
      failureThreshold: 1,
      resetTimeoutMs: 20,
      healthProbe: async () => {
        throw new Error('Probe failed');
      },
    });

    // Trip and enter half-open
    breaker.recordFailure();
    await new Promise(resolve => setTimeout(resolve, 30));
    breaker.canExecute();

    const healthy = await breaker.probeHealth();

    assert.ok(!healthy, 'Should report unhealthy on error');
    assert.strictEqual(breaker.state, CircuitState.OPEN);
  });

  it('should return true if no health probe configured', async () => {
    const breaker = new CircuitBreaker({
      name: 'no-probe-test',
    });

    const healthy = await breaker.probeHealth();
    assert.ok(healthy, 'Should return true when no probe configured');
  });

  it('canProbe should respect probe interval', async () => {
    const breaker = new CircuitBreaker({
      name: 'probe-interval-test',
      failureThreshold: 1,
      resetTimeoutMs: 10,
      healthProbeIntervalMs: 100,
    });

    // Trip and enter half-open
    breaker.recordFailure();
    await new Promise(resolve => setTimeout(resolve, 20));
    breaker.canExecute();
    assert.strictEqual(breaker.state, CircuitState.HALF_OPEN);

    // First probe should work
    assert.ok(breaker.canProbe(), 'First probe should be allowed');

    // Immediate second probe should not be allowed
    assert.ok(!breaker.canProbe(), 'Second probe too soon');

    // After interval, should be allowed again
    await new Promise(resolve => setTimeout(resolve, 110));
    assert.ok(breaker.canProbe(), 'Probe should be allowed after interval');
  });
});

// =============================================================================
// v1.1 FEATURES - STATE INFO
// =============================================================================

describe('v1.1: Enhanced State Info', () => {
  it('should include backoff info in getState', () => {
    const breaker = new CircuitBreaker({
      name: 'state-info-test',
      failureThreshold: 1,
      resetTimeoutMs: 1000,
    });

    breaker.recordFailure();

    const state = breaker.getState();
    assert.ok('consecutiveOpenings' in state, 'Should include consecutiveOpenings');
    assert.ok('currentBackoffMs' in state, 'Should include currentBackoffMs');
    assert.ok('timeInStateMs' in state, 'Should include timeInStateMs');
    assert.ok('timeUntilProbeMs' in state, 'Should include timeUntilProbeMs');
  });

  it('should include backoff info in stateChange event', () => {
    const breaker = new CircuitBreaker({
      name: 'event-backoff-test',
      failureThreshold: 1,
    });

    const events = [];
    breaker.on('stateChange', (data) => events.push(data));

    breaker.recordFailure();

    assert.ok(events.length > 0, 'Should emit event');
    assert.ok('backoffMs' in events[0], 'Event should include backoffMs');
  });

  it('should include state in error when circuit is open', async () => {
    const breaker = new CircuitBreaker({
      name: 'error-state-test',
      failureThreshold: 1,
    });

    breaker.recordFailure();

    try {
      await withCircuitBreaker(breaker, () => 'should not run');
      assert.fail('Should have thrown');
    } catch (err) {
      assert.ok(err.state, 'Error should include state');
      assert.strictEqual(err.state.state, CircuitState.OPEN);
    }
  });
});

// =============================================================================
// EDGE CASES
// =============================================================================

describe('Edge Cases', () => {
  it('should handle rapid success/failure alternation', () => {
    const breaker = new CircuitBreaker({
      name: 'rapid-test',
      failureThreshold: 3
    });

    breaker.recordFailure();
    breaker.recordSuccess(); // Resets failure count
    breaker.recordFailure();
    breaker.recordSuccess();
    breaker.recordFailure();
    breaker.recordSuccess();

    assert.strictEqual(breaker.state, CircuitState.CLOSED);
  });

  it('should handle multiple resets', () => {
    const breaker = new CircuitBreaker({ name: 'multi-reset' });
    breaker.trip();

    breaker.reset();
    breaker.reset();
    breaker.reset();

    assert.strictEqual(breaker.state, CircuitState.CLOSED);
  });

  it('should handle multiple trips', () => {
    const breaker = new CircuitBreaker({ name: 'multi-trip' });

    breaker.trip();
    breaker.trip();
    breaker.trip();

    assert.strictEqual(breaker.state, CircuitState.OPEN);
  });

  it('should handle zero timeout gracefully', async () => {
    const breaker = new CircuitBreaker({
      failureThreshold: 1,
      resetTimeoutMs: 0,
    });
    breaker.trip();

    // Should immediately transition to HALF_OPEN
    await new Promise(resolve => setTimeout(resolve, 10));
    assert.strictEqual(breaker.canExecute(), true);
    assert.strictEqual(breaker.state, CircuitState.HALF_OPEN);
  });
});
