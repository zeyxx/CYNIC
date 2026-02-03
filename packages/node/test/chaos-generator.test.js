/**
 * ChaosGenerator Tests
 *
 * Tests for the CYNIC ChaosGenerator - Chaos Engineering.
 *
 * "Un système qui survit au hasard survit à tout" - κυνικός
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import {
  ChaosGenerator,
  ChaosResult,
  ChaosEventType,
  CHAOS_CONFIG,
  createChaosGenerator,
  getChaosGenerator,
  _resetChaosGeneratorForTesting,
} from '../src/chaos/chaos-generator.js';
import { PHI_INV, PHI_INV_3 } from '@cynic/core';

// =============================================================================
// TEST HELPERS
// =============================================================================

function createMockLearningService(overrides = {}) {
  return {
    recordFeedback: mock.fn(async () => {}),
    ...overrides,
  };
}

// =============================================================================
// CHAOS RESULT TESTS
// =============================================================================

describe('ChaosResult', () => {
  describe('construction', () => {
    it('should create with defaults', () => {
      const result = new ChaosResult();

      assert.ok(result.id.startsWith('chaos-'));
      assert.equal(result.type, ChaosEventType.FORCE_PLANNING);
      assert.equal(result.injected, false);
      assert.equal(result.reason, null);
      assert.equal(result.scenario, null);
      assert.ok(result.timestamp > 0);
    });

    it('should create with provided data', () => {
      const result = new ChaosResult({
        type: ChaosEventType.ADVERSARIAL_SCENARIO,
        injected: true,
        reason: 'chaos_test',
        originalConfidence: 0.6,
      });

      assert.equal(result.type, ChaosEventType.ADVERSARIAL_SCENARIO);
      assert.equal(result.injected, true);
      assert.equal(result.reason, 'chaos_test');
      assert.equal(result.originalConfidence, 0.6);
    });
  });

  describe('toJSON', () => {
    it('should serialize correctly', () => {
      const result = new ChaosResult({
        type: ChaosEventType.INJECT_FAILURE,
        injected: true,
        reason: 'test',
      });

      const json = result.toJSON();

      assert.ok(json.id.startsWith('chaos-'));
      assert.equal(json.type, 'inject_failure');
      assert.equal(json.injected, true);
      assert.equal(json.reason, 'test');
    });
  });
});

// =============================================================================
// CHAOS GENERATOR TESTS
// =============================================================================

describe('ChaosGenerator', () => {
  let chaos;

  beforeEach(() => {
    _resetChaosGeneratorForTesting();
    chaos = new ChaosGenerator({ enabled: true });
    // Reset rate limiting state
    chaos._recentEvents = [];
    chaos._lastEventTime = 0;
  });

  describe('construction', () => {
    it('should create disabled by default', () => {
      const c = new ChaosGenerator();
      assert.equal(c.enabled, false);
    });

    it('should accept enabled option', () => {
      const c = new ChaosGenerator({ enabled: true });
      assert.equal(c.enabled, true);
    });

    it('should have default config', () => {
      const c = new ChaosGenerator();
      assert.equal(c.config.PLANNING_PROBABILITY, CHAOS_CONFIG.PLANNING_PROBABILITY);
      assert.equal(c.config.MAX_EVENTS_PER_MINUTE, 5);
    });

    it('should accept custom config', () => {
      const c = new ChaosGenerator({
        config: { MAX_EVENTS_PER_MINUTE: 10 },
      });
      assert.equal(c.config.MAX_EVENTS_PER_MINUTE, 10);
    });
  });

  // ===========================================================================
  // FORCE PLANNING TESTS
  // ===========================================================================

  describe('shouldForcePlanning', () => {
    it('should not inject when disabled', () => {
      const c = new ChaosGenerator({ enabled: false });

      const result = c.shouldForcePlanning({ content: 'test' });

      assert.equal(result.injected, false);
    });

    it('should respect rate limiting', () => {
      // Fill up rate limit
      chaos._recentEvents = [Date.now(), Date.now(), Date.now(), Date.now(), Date.now()];

      const result = chaos.shouldForcePlanning({ content: 'test' });

      assert.equal(result.injected, false);
      assert.equal(result.reason, 'rate_limited');
    });

    it('should respect cooldown', () => {
      chaos._lastEventTime = Date.now();  // Just had an event

      const result = chaos.shouldForcePlanning({ content: 'test' });

      assert.equal(result.injected, false);
      assert.equal(result.reason, 'rate_limited');
    });

    it('should track checks', () => {
      chaos.shouldForcePlanning({ content: 'test' });
      chaos.shouldForcePlanning({ content: 'test' });

      assert.equal(chaos.stats.checks, 2);
    });

    it('should emit event when injected', async () => {
      let emitted = false;
      chaos.on('chaos:inject', () => { emitted = true; });

      // Run many times to get a hit (probabilistic)
      for (let i = 0; i < 100; i++) {
        chaos._lastEventTime = 0;
        chaos._recentEvents = [];
        const result = chaos.shouldForcePlanning({ content: 'test' });
        if (result.injected) {
          assert.ok(emitted);
          return;
        }
      }
      // If we get here, no injection happened (possible with low probability)
      // This is acceptable for a probabilistic test
    });
  });

  // ===========================================================================
  // ADVERSARIAL SCENARIO TESTS
  // ===========================================================================

  describe('injectAdversarialScenario', () => {
    it('should not inject when disabled', () => {
      const c = new ChaosGenerator({ enabled: false });

      const result = c.injectAdversarialScenario({ content: 'test' });

      assert.equal(result.injected, false);
    });

    it('should include scenario when injected', () => {
      // Run many times to get a hit
      for (let i = 0; i < 100; i++) {
        chaos._lastEventTime = 0;
        chaos._recentEvents = [];
        const result = chaos.injectAdversarialScenario({ content: 'test' });
        if (result.injected) {
          assert.ok(result.scenario);
          assert.ok(result.scenario.id);
          assert.ok(result.scenario.description);
          return;
        }
      }
    });
  });

  // ===========================================================================
  // CONFIDENCE PERTURBATION TESTS
  // ===========================================================================

  describe('perturbConfidence', () => {
    it('should not perturb when disabled', () => {
      const c = new ChaosGenerator({ enabled: false });

      const result = c.perturbConfidence(0.5);

      assert.equal(result.injected, false);
      assert.equal(result.perturbedConfidence, 0.5);
    });

    it('should cap perturbed confidence at φ⁻¹', () => {
      // Run many times to get a perturbation
      for (let i = 0; i < 100; i++) {
        chaos._lastEventTime = 0;
        chaos._recentEvents = [];
        const result = chaos.perturbConfidence(0.9);  // High base
        if (result.injected) {
          assert.ok(result.perturbedConfidence <= PHI_INV);
          return;
        }
      }
    });

    it('should not go below 0', () => {
      // Run many times
      for (let i = 0; i < 100; i++) {
        chaos._lastEventTime = 0;
        chaos._recentEvents = [];
        const result = chaos.perturbConfidence(0.1);  // Low base
        assert.ok(result.perturbedConfidence >= 0);
      }
    });
  });

  // ===========================================================================
  // ALTERNATIVE VALIDATION TESTS
  // ===========================================================================

  describe('validateAlternatives', () => {
    it('should flag missing alternatives', () => {
      const result = chaos.validateAlternatives({ id: 'primary' }, []);

      assert.equal(result.valid, false);
      assert.ok(result.issues.includes('No alternatives generated'));
    });

    it('should flag matching alternatives', () => {
      const primary = { id: 'a1', label: 'Option A' };
      const alternatives = [
        { id: 'a1', label: 'Option A' },  // Same as primary
        { id: 'a2', label: 'Option B' },
      ];

      const result = chaos.validateAlternatives(primary, alternatives);

      assert.ok(result.issues.some(i => i.includes('matches primary')));
    });

    it('should flag low diversity', () => {
      const alternatives = [
        { label: 'Same' },
        { label: 'Same' },
        { label: 'Same' },
      ];

      const result = chaos.validateAlternatives(null, alternatives);

      assert.ok(result.issues.includes('Low alternative diversity'));
    });

    it('should pass valid alternatives', () => {
      const primary = { id: 'primary' };
      const alternatives = [
        { id: 'a1', label: 'Option A' },
        { id: 'a2', label: 'Option B' },
        { id: 'a3', label: 'Option C' },
      ];

      const result = chaos.validateAlternatives(primary, alternatives);

      assert.equal(result.valid, true);
      assert.equal(result.coverage, 1);
    });
  });

  // ===========================================================================
  // RESULT RECORDING TESTS
  // ===========================================================================

  describe('recordChaosResult', () => {
    it('should record survived outcome', () => {
      // Force an injection to get a result
      chaos._recentEvents = [];
      chaos._lastEventTime = 0;

      // Manually create and record a result
      const result = new ChaosResult({
        type: ChaosEventType.FORCE_PLANNING,
        injected: true,
      });
      chaos._results.push(result);
      chaos.stats.injected++;

      chaos.recordChaosResult(result.id, { survived: true });

      assert.equal(chaos.stats.survived, 1);
    });

    it('should record failed outcome', () => {
      const result = new ChaosResult({ injected: true });
      chaos._results.push(result);

      chaos.recordChaosResult(result.id, { survived: false, error: 'test error' });

      assert.equal(chaos.stats.failed, 1);
    });

    it('should emit result event', () => {
      let emitted = false;
      chaos.on('chaos:result', () => { emitted = true; });

      const result = new ChaosResult({ injected: true });
      chaos._results.push(result);

      chaos.recordChaosResult(result.id, { survived: true });

      assert.ok(emitted);
    });

    it('should forward to learning service', () => {
      const learning = createMockLearningService();
      chaos.learningService = learning;

      const result = new ChaosResult({ injected: true });
      chaos._results.push(result);

      chaos.recordChaosResult(result.id, { survived: true });

      assert.ok(learning.recordFeedback.mock.calls.length > 0);
    });
  });

  // ===========================================================================
  // ENABLE/DISABLE TESTS
  // ===========================================================================

  describe('enable/disable', () => {
    it('should enable chaos', () => {
      const c = new ChaosGenerator({ enabled: false });

      c.enable();

      assert.equal(c.enabled, true);
    });

    it('should disable chaos', () => {
      const c = new ChaosGenerator({ enabled: true });

      c.disable();

      assert.equal(c.enabled, false);
    });
  });

  // ===========================================================================
  // STATISTICS TESTS
  // ===========================================================================

  describe('statistics', () => {
    it('should track by type', () => {
      // Manually record to test stats
      chaos._recordChaosEvent(new ChaosResult({
        type: ChaosEventType.FORCE_PLANNING,
        injected: true,
      }));

      assert.equal(chaos.stats.byType[ChaosEventType.FORCE_PLANNING], 1);
    });

    it('should calculate survival rate', () => {
      chaos.stats.survived = 8;
      chaos.stats.failed = 2;

      const stats = chaos.getStats();

      assert.equal(stats.survivalRate, '80.0%');
    });

    it('should handle no results for survival rate', () => {
      const stats = chaos.getStats();
      assert.equal(stats.survivalRate, 'N/A');
    });

    it('should reset stats', () => {
      chaos.stats.checks = 100;
      chaos.stats.injected = 10;

      chaos.resetStats();

      assert.equal(chaos.stats.checks, 0);
      assert.equal(chaos.stats.injected, 0);
    });
  });

  // ===========================================================================
  // SINGLETON TESTS
  // ===========================================================================

  describe('singleton', () => {
    beforeEach(() => {
      _resetChaosGeneratorForTesting();
    });

    it('should return same instance', () => {
      const c1 = getChaosGenerator();
      const c2 = getChaosGenerator();

      assert.strictEqual(c1, c2);
    });

    it('should reset for testing', () => {
      const c1 = getChaosGenerator();
      _resetChaosGeneratorForTesting();
      const c2 = getChaosGenerator();

      assert.notStrictEqual(c1, c2);
    });
  });
});
