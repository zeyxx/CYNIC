/**
 * φ-Utility Library Tests
 *
 * Tests the consolidated utility patterns.
 * These patterns were duplicated across 150+ files.
 * Now they live in one place and are tested ONCE.
 *
 * @module @cynic/core/test/phi-utils
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { EventEmitter } from 'events';

import {
  PHI_INV,
  PHI_INV_2,
  THRESHOLDS,
  phiBound,
  phiBoundTo,
  clamp,
  phiClassify,
  phiHealthStatus,
  roundTo,
  pushHistory,
  createSingleton,
  createStatsTracker,
  updateStats,
  createCooldownTracker,
  applyCalibration,
} from '../src/index.js';

// =============================================================================
// phiBound
// =============================================================================

describe('phiBound', () => {
  it('clamps value to [0, PHI_INV]', () => {
    assert.equal(phiBound(0.5), 0.5);
    assert.equal(phiBound(0), 0);
    assert.equal(phiBound(PHI_INV), PHI_INV);
  });

  it('caps values above PHI_INV', () => {
    assert.equal(phiBound(0.9), PHI_INV);
    assert.equal(phiBound(1.0), PHI_INV);
    assert.equal(phiBound(999), PHI_INV);
  });

  it('floors values below 0', () => {
    assert.equal(phiBound(-0.1), 0);
    assert.equal(phiBound(-999), 0);
  });

  it('handles NaN and non-numbers', () => {
    assert.equal(phiBound(NaN), 0);
    assert.equal(phiBound(undefined), 0);
    assert.equal(phiBound(null), 0);
    assert.equal(phiBound('hello'), 0);
  });

  it('preserves precision for mid-range values', () => {
    const value = 0.42;
    assert.equal(phiBound(value), 0.42);
  });
});

// =============================================================================
// phiBoundTo
// =============================================================================

describe('phiBoundTo', () => {
  it('defaults to PHI_INV ceiling', () => {
    assert.equal(phiBoundTo(0.5), phiBound(0.5));
    assert.equal(phiBoundTo(0.9), PHI_INV);
  });

  it('uses custom ceiling', () => {
    assert.equal(phiBoundTo(0.5, 0.4), 0.4);
    assert.equal(phiBoundTo(0.3, 0.4), 0.3);
    assert.equal(phiBoundTo(1.0, 1.0), 1.0);
  });

  it('handles NaN', () => {
    assert.equal(phiBoundTo(NaN, 0.5), 0);
  });
});

// =============================================================================
// clamp
// =============================================================================

describe('clamp', () => {
  it('clamps between min and max', () => {
    assert.equal(clamp(5, 0, 10), 5);
    assert.equal(clamp(-5, 0, 10), 0);
    assert.equal(clamp(15, 0, 10), 10);
  });

  it('handles edge cases', () => {
    assert.equal(clamp(0, 0, 0), 0);
    assert.equal(clamp(5, 5, 5), 5);
  });

  it('handles NaN — returns min', () => {
    assert.equal(clamp(NaN, 0, 10), 0);
  });

  it('works with negative ranges', () => {
    assert.equal(clamp(-5, -10, -1), -5);
    assert.equal(clamp(-20, -10, -1), -10);
    assert.equal(clamp(0, -10, -1), -1);
  });
});

// =============================================================================
// phiClassify
// =============================================================================

describe('phiClassify', () => {
  it('returns HOWL for scores >= 80', () => {
    assert.equal(phiClassify(80), 'HOWL');
    assert.equal(phiClassify(100), 'HOWL');
    assert.equal(phiClassify(95.5), 'HOWL');
  });

  it('returns WAG for scores >= 50 and < 80', () => {
    assert.equal(phiClassify(50), 'WAG');
    assert.equal(phiClassify(79.9), 'WAG');
    assert.equal(phiClassify(65), 'WAG');
  });

  it('returns GROWL for scores >= GROWL threshold and < 50', () => {
    assert.equal(phiClassify(THRESHOLDS.GROWL), 'GROWL');
    assert.equal(phiClassify(49.9), 'GROWL');
    assert.equal(phiClassify(40), 'GROWL');
  });

  it('returns BARK for scores below GROWL threshold', () => {
    assert.equal(phiClassify(0), 'BARK');
    assert.equal(phiClassify(10), 'BARK');
    assert.equal(phiClassify(THRESHOLDS.GROWL - 0.1), 'BARK');
  });
});

// =============================================================================
// phiHealthStatus
// =============================================================================

describe('phiHealthStatus', () => {
  it('returns healthy for confidence >= PHI_INV', () => {
    assert.equal(phiHealthStatus(PHI_INV), 'healthy');
    assert.equal(phiHealthStatus(0.7), 'healthy');
  });

  it('returns warning for confidence >= PHI_INV_2 and < PHI_INV', () => {
    assert.equal(phiHealthStatus(PHI_INV_2), 'warning');
    assert.equal(phiHealthStatus(0.5), 'warning');
  });

  it('returns critical for confidence > 0 and < PHI_INV_2', () => {
    assert.equal(phiHealthStatus(0.1), 'critical');
    assert.equal(phiHealthStatus(0.01), 'critical');
  });

  it('returns failing for confidence <= 0', () => {
    assert.equal(phiHealthStatus(0), 'failing');
    assert.equal(phiHealthStatus(-1), 'failing');
  });
});

// =============================================================================
// roundTo
// =============================================================================

describe('roundTo', () => {
  it('rounds to 3 decimal places by default', () => {
    assert.equal(roundTo(0.12345), 0.123);
    assert.equal(roundTo(0.9999), 1);
    assert.equal(roundTo(3.14159), 3.142);
  });

  it('rounds to custom precision', () => {
    assert.equal(roundTo(0.12345, 2), 0.12);
    assert.equal(roundTo(0.12345, 4), 0.1235);
    assert.equal(roundTo(0.12345, 0), 0);
    assert.equal(roundTo(3.14159, 1), 3.1);
  });

  it('handles NaN and non-numbers', () => {
    assert.equal(roundTo(NaN), 0);
    assert.equal(roundTo(undefined), 0);
    assert.equal(roundTo('hello'), 0);
  });

  it('handles integers', () => {
    assert.equal(roundTo(42), 42);
    assert.equal(roundTo(42, 2), 42);
  });
});

// =============================================================================
// pushHistory
// =============================================================================

describe('pushHistory', () => {
  it('pushes and trims to maxSize', () => {
    const arr = [1, 2, 3];
    pushHistory(arr, 4, 3);
    assert.deepEqual(arr, [2, 3, 4]);
  });

  it('does not trim when under max', () => {
    const arr = [];
    pushHistory(arr, 'a', 5);
    pushHistory(arr, 'b', 5);
    assert.deepEqual(arr, ['a', 'b']);
  });

  it('returns the same array', () => {
    const arr = [];
    const result = pushHistory(arr, 1, 10);
    assert.equal(result, arr);
  });

  it('handles maxSize of 1', () => {
    const arr = [];
    pushHistory(arr, 'a', 1);
    pushHistory(arr, 'b', 1);
    pushHistory(arr, 'c', 1);
    assert.deepEqual(arr, ['c']);
  });

  it('handles Fibonacci maxSize (233)', () => {
    const arr = [];
    for (let i = 0; i < 250; i++) {
      pushHistory(arr, i, 233);
    }
    assert.equal(arr.length, 233);
    assert.equal(arr[0], 17); // 250 - 233 = 17
    assert.equal(arr[232], 249);
  });
});

// =============================================================================
// createSingleton
// =============================================================================

describe('createSingleton', () => {
  it('creates and returns same instance', () => {
    class TestClass extends EventEmitter {
      constructor(opts) {
        super();
        this.value = opts.value || 0;
      }
    }

    const { getInstance, resetInstance } = createSingleton(TestClass);

    const a = getInstance({ value: 42 });
    const b = getInstance({ value: 99 }); // should return same
    assert.equal(a, b);
    assert.equal(a.value, 42);

    resetInstance();
  });

  it('resets instance', () => {
    class TestClass extends EventEmitter {
      constructor(opts) {
        super();
        this.value = opts.value || 0;
      }
    }

    const { getInstance, resetInstance } = createSingleton(TestClass);

    const a = getInstance({ value: 42 });
    resetInstance();
    const b = getInstance({ value: 99 });
    assert.notEqual(a, b);
    assert.equal(b.value, 99);

    resetInstance();
  });

  it('calls removeAllListeners on reset', () => {
    let removed = false;

    class TestClass {
      removeAllListeners() {
        removed = true;
      }
    }

    const { getInstance, resetInstance } = createSingleton(TestClass);
    getInstance();
    resetInstance();
    assert.equal(removed, true);
  });

  it('accepts custom cleanup', () => {
    let customCalled = false;

    class TestClass {}

    const { getInstance, resetInstance } = createSingleton(TestClass, {
      cleanup: () => { customCalled = true; },
    });

    getInstance();
    resetInstance();
    assert.equal(customCalled, true);
  });

  it('handles reset without instance (no-op)', () => {
    class TestClass {}
    const { resetInstance } = createSingleton(TestClass);
    assert.doesNotThrow(() => resetInstance());
  });
});

// =============================================================================
// createStatsTracker + updateStats
// =============================================================================

describe('createStatsTracker', () => {
  const TypeEnum = { A: 'alpha', B: 'beta', C: 'gamma' };

  it('initializes all type counters to 0', () => {
    const stats = createStatsTracker(TypeEnum);
    assert.equal(stats.total, 0);
    assert.equal(stats.byType.alpha, 0);
    assert.equal(stats.byType.beta, 0);
    assert.equal(stats.byType.gamma, 0);
    assert.equal(stats.lastTimestamp, null);
  });

  it('initializes extra fields to 0', () => {
    const stats = createStatsTracker(TypeEnum, ['delivered', 'blocked']);
    assert.equal(stats.delivered, 0);
    assert.equal(stats.blocked, 0);
  });
});

describe('updateStats', () => {
  const TypeEnum = { A: 'alpha', B: 'beta' };

  it('increments total and byType', () => {
    const stats = createStatsTracker(TypeEnum);
    updateStats(stats, { type: 'alpha', timestamp: 1000 });
    assert.equal(stats.total, 1);
    assert.equal(stats.byType.alpha, 1);
    assert.equal(stats.byType.beta, 0);
    assert.equal(stats.lastTimestamp, 1000);
  });

  it('ignores unknown types gracefully', () => {
    const stats = createStatsTracker(TypeEnum);
    updateStats(stats, { type: 'unknown', timestamp: 2000 });
    assert.equal(stats.total, 1);
    assert.equal(stats.byType.alpha, 0);
    assert.equal(stats.lastTimestamp, 2000);
  });

  it('handles multiple updates', () => {
    const stats = createStatsTracker(TypeEnum);
    updateStats(stats, { type: 'alpha' });
    updateStats(stats, { type: 'alpha' });
    updateStats(stats, { type: 'beta' });
    assert.equal(stats.total, 3);
    assert.equal(stats.byType.alpha, 2);
    assert.equal(stats.byType.beta, 1);
  });
});

// =============================================================================
// createCooldownTracker
// =============================================================================

describe('createCooldownTracker', () => {
  it('returns false when no previous action', () => {
    const tracker = createCooldownTracker({ action: 5000 });
    assert.equal(tracker.isOnCooldown('action'), false);
  });

  it('returns true during cooldown', () => {
    const tracker = createCooldownTracker({ action: 60000 });
    tracker.record('action');
    assert.equal(tracker.isOnCooldown('action'), true);
  });

  it('uses default cooldown for unknown types', () => {
    const tracker = createCooldownTracker({}, 60000);
    tracker.record('unknown');
    assert.equal(tracker.isOnCooldown('unknown'), true);
  });

  it('reset clears all cooldowns', () => {
    const tracker = createCooldownTracker({ action: 60000 });
    tracker.record('action');
    tracker.reset();
    assert.equal(tracker.isOnCooldown('action'), false);
  });

  it('getState returns a copy of the map', () => {
    const tracker = createCooldownTracker({ action: 60000 });
    tracker.record('action');
    const state = tracker.getState();
    assert.equal(state instanceof Map, true);
    assert.equal(state.has('action'), true);
  });

  it('different action types are independent', () => {
    const tracker = createCooldownTracker({ fast: 1, slow: 60000 });
    tracker.record('slow');
    assert.equal(tracker.isOnCooldown('slow'), true);
    assert.equal(tracker.isOnCooldown('fast'), false);
  });
});

// =============================================================================
// applyCalibration
// =============================================================================

describe('applyCalibration', () => {
  it('returns phiBound(base) when too few samples', () => {
    const outcomes = [{ result: 'success' }];
    const result = applyCalibration(outcomes, 0.5, { minSamples: 5 });
    assert.equal(result, 0.5);
  });

  it('returns phiBound(base) for null/empty outcomes', () => {
    assert.equal(applyCalibration(null, 0.5), 0.5);
    assert.equal(applyCalibration([], 0.5), 0.5);
  });

  it('increases confidence when success rate > base', () => {
    const outcomes = Array.from({ length: 10 }, () => ({ result: 'success' }));
    const result = applyCalibration(outcomes, 0.3, { minSamples: 5 });
    assert.ok(result > 0.3, `Expected > 0.3, got ${result}`);
  });

  it('decreases confidence when success rate < base', () => {
    const outcomes = Array.from({ length: 10 }, () => ({ result: 'failure' }));
    const result = applyCalibration(outcomes, 0.5, { minSamples: 5 });
    assert.ok(result < 0.5, `Expected < 0.5, got ${result}`);
  });

  it('never exceeds PHI_INV', () => {
    const outcomes = Array.from({ length: 100 }, () => ({ result: 'success' }));
    const result = applyCalibration(outcomes, 0.6, { minSamples: 5 });
    assert.ok(result <= PHI_INV, `Expected <= ${PHI_INV}, got ${result}`);
  });

  it('never goes below 0', () => {
    const outcomes = Array.from({ length: 100 }, () => ({ result: 'failure' }));
    const result = applyCalibration(outcomes, 0.1, { minSamples: 5 });
    assert.ok(result >= 0, `Expected >= 0, got ${result}`);
  });

  it('respects maxAdjust limit', () => {
    const allSuccess = Array.from({ length: 20 }, () => ({ result: 'success' }));
    const result = applyCalibration(allSuccess, 0.3, { minSamples: 5, maxAdjust: 0.1 });
    // gap = 1.0 - 0.3 = 0.7, scaled by PHI_INV = 0.432, clamped to 0.1
    // result = 0.3 + 0.1 = 0.4
    assert.ok(result <= 0.4 + 0.001, `Expected <= 0.401, got ${result}`);
  });

  it('supports custom successValue', () => {
    const outcomes = Array.from({ length: 10 }, () => ({ result: 'pass' }));
    const result = applyCalibration(outcomes, 0.3, { minSamples: 5, successValue: 'pass' });
    assert.ok(result > 0.3, `Expected > 0.3, got ${result}`);
  });
});

// =============================================================================
// Integration: imported through @cynic/core
// =============================================================================

describe('phi-utils re-export integration', () => {
  it('all utilities are importable from @cynic/core index', () => {
    assert.equal(typeof phiBound, 'function');
    assert.equal(typeof phiBoundTo, 'function');
    assert.equal(typeof clamp, 'function');
    assert.equal(typeof phiClassify, 'function');
    assert.equal(typeof phiHealthStatus, 'function');
    assert.equal(typeof roundTo, 'function');
    assert.equal(typeof pushHistory, 'function');
    assert.equal(typeof createSingleton, 'function');
    assert.equal(typeof createStatsTracker, 'function');
    assert.equal(typeof updateStats, 'function');
    assert.equal(typeof createCooldownTracker, 'function');
    assert.equal(typeof applyCalibration, 'function');
  });
});
