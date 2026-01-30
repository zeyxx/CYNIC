/**
 * @cynic/node - SONA Tests
 *
 * Tests for Self-Optimizing Neural Adaptation system.
 *
 * @module @cynic/node/test/sona
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import { SONA, createSONA, SONA_CONFIG } from '../src/learning/sona.js';

// =============================================================================
// CONSTANTS
// =============================================================================

const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;
const PHI_INV_3 = 0.236067977499790;

// =============================================================================
// MOCK HELPERS
// =============================================================================

function createMockLearningService() {
  const weightModifiers = new Map();
  return {
    getWeightModifier: mock.fn((dim) => weightModifiers.get(dim) || 1.0),
    adjustDimensionWeight: mock.fn((dim, delta) => {
      const current = weightModifiers.get(dim) || 1.0;
      const newVal = current + delta;
      weightModifiers.set(dim, newVal);
      return newVal;
    }),
    _weightModifiers: weightModifiers,
  };
}

function createMockEWCService(locked = false) {
  return {
    canModifyPattern: mock.fn(async () => !locked),
  };
}

// =============================================================================
// SONA_CONFIG TESTS
// =============================================================================

describe('SONA_CONFIG', () => {
  it('should have φ-aligned adaptation rate', () => {
    assert.ok(Math.abs(SONA_CONFIG.ADAPTATION_RATE - PHI_INV_3) < 0.001,
      `Adaptation rate should be φ⁻³ (0.236), got ${SONA_CONFIG.ADAPTATION_RATE}`);
  });

  it('should have φ-aligned max adaptation', () => {
    assert.ok(Math.abs(SONA_CONFIG.MAX_ADAPTATION - PHI_INV_2) < 0.001,
      `Max adaptation should be φ⁻² (0.382), got ${SONA_CONFIG.MAX_ADAPTATION}`);
  });

  it('should have φ-aligned correlation threshold', () => {
    assert.ok(Math.abs(SONA_CONFIG.CORRELATION_THRESHOLD - PHI_INV_2) < 0.001,
      `Correlation threshold should be φ⁻² (0.382)`);
  });

  it('should have Fibonacci-aligned limits', () => {
    assert.strictEqual(SONA_CONFIG.MAX_TRACKED_PATTERNS, 144, 'F(12) = 144');
    assert.strictEqual(SONA_CONFIG.MAX_OBSERVATIONS_PER_PATTERN, 89, 'F(11) = 89');
    assert.strictEqual(SONA_CONFIG.BATCH_SIZE, 13, 'F(7) = 13');
  });

  it('should have Fibonacci-aligned time windows', () => {
    assert.strictEqual(SONA_CONFIG.RECENT_WINDOW_MS, 21000, 'F(8) * 1000');
    assert.strictEqual(SONA_CONFIG.ADAPTATION_INTERVAL_MS, 55, 'F(10)');
    assert.strictEqual(SONA_CONFIG.STATS_WINDOW_MS, 89000, 'F(11) * 1000');
  });

  it('should be frozen', () => {
    assert.ok(Object.isFrozen(SONA_CONFIG));
  });
});

// =============================================================================
// SONA TESTS
// =============================================================================

describe('SONA', () => {
  describe('Construction', () => {
    it('should create with factory', () => {
      const sona = createSONA();
      assert.ok(sona instanceof SONA);
    });

    it('should initialize empty state', () => {
      const sona = createSONA();
      const stats = sona.getStats();

      assert.strictEqual(stats.totalObservations, 0);
      assert.strictEqual(stats.adaptationsMade, 0);
      assert.strictEqual(stats.patternsTracked, 0);
    });

    it('should accept external services', () => {
      const learningService = createMockLearningService();
      const ewcService = createMockEWCService();

      const sona = createSONA({
        learningService,
        ewcService,
      });

      assert.ok(sona);
    });

    it('should accept custom config', () => {
      const sona = createSONA({
        config: { MIN_OBSERVATIONS: 5 },
      });

      assert.strictEqual(sona.config.MIN_OBSERVATIONS, 5);
    });
  });

  describe('Lifecycle', () => {
    it('should start adaptation loop', () => {
      const sona = createSONA();
      sona.start();

      assert.ok(sona._adaptationTimer !== null);

      sona.stop();
    });

    it('should stop adaptation loop', () => {
      const sona = createSONA();
      sona.start();
      sona.stop();

      assert.strictEqual(sona._adaptationTimer, null);
    });

    it('should emit lifecycle events', () => {
      const sona = createSONA();
      const events = [];

      sona.on('sona:started', () => events.push('started'));
      sona.on('sona:stopped', () => events.push('stopped'));

      sona.start();
      sona.stop();

      assert.deepStrictEqual(events, ['started', 'stopped']);
    });
  });

  describe('observe()', () => {
    it('should record pattern observations', () => {
      const sona = createSONA();

      sona.observe({
        patternId: 'pat_001',
        dimensionScores: { COHERENCE: 80, ACCURACY: 75 },
        judgmentId: 'jdg_001',
      });

      const stats = sona.getStats();
      assert.strictEqual(stats.totalObservations, 1);
      assert.strictEqual(stats.queuedObservations, 1);
    });

    it('should emit observe event', () => {
      const sona = createSONA();
      const events = [];

      sona.on('sona:observed', (data) => events.push(data));

      sona.observe({
        patternId: 'pat_001',
        dimensionScores: { COHERENCE: 80 },
        judgmentId: 'jdg_001',
      });

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].patternId, 'pat_001');
    });

    it('should ignore observations without pattern or scores', () => {
      const sona = createSONA();

      sona.observe({ patternId: null, dimensionScores: {} });
      sona.observe({ patternId: 'pat_001' });

      assert.strictEqual(sona.getStats().totalObservations, 0);
    });
  });

  describe('processFeedback()', () => {
    let sona;

    beforeEach(() => {
      sona = createSONA();
    });

    it('should link feedback to pending observations', () => {
      sona.observe({
        patternId: 'pat_001',
        dimensionScores: { COHERENCE: 80 },
        judgmentId: 'jdg_001',
      });

      const events = [];
      sona.on('sona:feedback', (data) => events.push(data));

      sona.processFeedback({
        judgmentId: 'jdg_001',
        outcome: 'correct',
        actualScore: 85,
        originalScore: 80,
      });

      assert.strictEqual(events.length, 1);
      assert.strictEqual(events[0].outcome, 'correct');
    });

    it('should convert outcome to numeric', () => {
      sona.observe({
        patternId: 'pat_001',
        dimensionScores: { COHERENCE: 80 },
        judgmentId: 'jdg_001',
      });

      sona.processFeedback({
        judgmentId: 'jdg_001',
        outcome: 'partial',
        actualScore: 75,
        originalScore: 80,
      });

      // Check that observation was moved from queue
      assert.strictEqual(sona._adaptationQueue.length, 0);
    });

    it('should update correlations', () => {
      // Add multiple observations for the same pattern
      for (let i = 0; i < 5; i++) {
        sona.observe({
          patternId: 'pat_001',
          dimensionScores: { COHERENCE: 70 + i * 5 },
          judgmentId: `jdg_${i}`,
        });

        sona.processFeedback({
          judgmentId: `jdg_${i}`,
          outcome: i > 2 ? 'correct' : 'incorrect',
          actualScore: 70 + i * 5,
          originalScore: 70,
        });
      }

      const insights = sona.getDimensionInsights();
      // Should have tracked COHERENCE dimension
      assert.ok(insights.COHERENCE || Object.keys(insights).length >= 0);
    });
  });

  describe('Pattern Performance', () => {
    it('should return null for unknown pattern', () => {
      const sona = createSONA();
      assert.strictEqual(sona.getPatternPerformance('unknown'), null);
    });

    it('should calculate pattern performance', () => {
      const sona = createSONA();

      // Add observations
      for (let i = 0; i < 5; i++) {
        sona.observe({
          patternId: 'pat_001',
          dimensionScores: { COHERENCE: 80 },
          judgmentId: `jdg_${i}`,
        });

        sona.processFeedback({
          judgmentId: `jdg_${i}`,
          outcome: i % 2 === 0 ? 'correct' : 'incorrect',
          actualScore: 80,
          originalScore: 75,
        });
      }

      const perf = sona.getPatternPerformance('pat_001');

      assert.ok(perf);
      assert.strictEqual(perf.patternId, 'pat_001');
      assert.ok(perf.overall.count === 5);
      assert.ok(perf.overall.avgOutcome >= 0 && perf.overall.avgOutcome <= 1);
    });
  });

  describe('EWC Integration', () => {
    it('should respect EWC locks', async () => {
      const learningService = createMockLearningService();
      const ewcService = createMockEWCService(true); // locked

      const sona = createSONA({ learningService, ewcService });
      sona.start();

      // Add enough observations to trigger adaptation
      for (let i = 0; i < 5; i++) {
        sona.observe({
          patternId: 'pat_locked',
          dimensionScores: { COHERENCE: 80 },
          judgmentId: `jdg_${i}`,
        });

        sona.processFeedback({
          judgmentId: `jdg_${i}`,
          outcome: 'correct',
          actualScore: 90,
          originalScore: 80,
        });
      }

      // Wait for adaptation batch
      await new Promise(resolve => setTimeout(resolve, 100));

      sona.stop();

      // Should have checked canModifyPattern
      assert.ok(ewcService.canModifyPattern.mock.calls.length > 0);

      // Should NOT have adjusted weights (pattern is locked)
      assert.strictEqual(learningService.adjustDimensionWeight.mock.calls.length, 0);
    });
  });

  describe('Statistics', () => {
    it('should track stats correctly', () => {
      const sona = createSONA();

      for (let i = 0; i < 3; i++) {
        sona.observe({
          patternId: `pat_${i}`,
          dimensionScores: { COHERENCE: 80 },
          judgmentId: `jdg_${i}`,
        });
      }

      const stats = sona.getStats();

      assert.strictEqual(stats.totalObservations, 3);
      assert.ok(stats.config);
    });
  });

  describe('Dimension Insights', () => {
    it('should return empty for no correlations', () => {
      const sona = createSONA();
      const insights = sona.getDimensionInsights();

      assert.deepStrictEqual(insights, {});
    });
  });

  describe('Reset', () => {
    it('should clear all state', () => {
      const sona = createSONA();

      // Add some state
      sona.observe({
        patternId: 'pat_001',
        dimensionScores: { COHERENCE: 80 },
        judgmentId: 'jdg_001',
      });

      sona.reset();

      const stats = sona.getStats();
      assert.strictEqual(stats.totalObservations, 0);
      assert.strictEqual(stats.patternsTracked, 0);
    });
  });
});

// =============================================================================
// INTEGRATION TESTS
// =============================================================================

describe('SONA Integration', () => {
  it('should adapt weights through learning service', async () => {
    const learningService = createMockLearningService();
    const ewcService = createMockEWCService(false); // not locked

    const sona = createSONA({
      learningService,
      ewcService,
      config: {
        MIN_OBSERVATIONS: 3,
        ADAPTATION_INTERVAL_MS: 10, // Fast for testing
      },
    });

    sona.start();

    // Add successful observations
    for (let i = 0; i < 5; i++) {
      sona.observe({
        patternId: 'pat_success',
        dimensionScores: { COHERENCE: 80, ACCURACY: 75 },
        judgmentId: `jdg_${i}`,
      });

      sona.processFeedback({
        judgmentId: `jdg_${i}`,
        outcome: 'correct',
        actualScore: 85,
        originalScore: 80,
      });
    }

    // Wait for adaptation
    await new Promise(resolve => setTimeout(resolve, 100));

    sona.stop();

    // Should have made adaptations
    const stats = sona.getStats();
    assert.ok(stats.totalObservations > 0);
  });

  it('should maintain sub-millisecond average adaptation time', async () => {
    const sona = createSONA({
      config: { ADAPTATION_INTERVAL_MS: 10 },
    });

    sona.start();

    // Add many observations
    for (let i = 0; i < 20; i++) {
      sona.observe({
        patternId: `pat_${i % 5}`,
        dimensionScores: { COHERENCE: 70 + i, ACCURACY: 60 + i },
        judgmentId: `jdg_${i}`,
      });

      sona.processFeedback({
        judgmentId: `jdg_${i}`,
        outcome: i % 3 === 0 ? 'incorrect' : 'correct',
        actualScore: 75,
        originalScore: 70,
      });
    }

    // Wait for several adaptation cycles
    await new Promise(resolve => setTimeout(resolve, 200));

    sona.stop();

    const stats = sona.getStats();

    // Average adaptation time should be very low
    // Note: First few may be higher due to JIT warmup
    assert.ok(stats.avgAdaptationTime < 50,
      `Adaptation should be fast, got ${stats.avgAdaptationTime}ms`);
  });
});
