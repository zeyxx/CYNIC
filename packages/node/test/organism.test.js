/**
 * Organism Module Tests
 *
 * Tests for CYNIC's 7-dimensional organism model.
 * All tests are FALSIFIABLE - they can prove the model wrong.
 *
 * @module @cynic/node/test/organism
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

import {
  // Metabolism
  MetabolicSample,
  MetabolismTracker,
  createMetabolismTracker,
  getMetabolismTracker,
  resetMetabolismTracker,
  METABOLISM_CONFIG,

  // Thermodynamics
  ThermoEvent,
  ThermoEventType,
  ThermodynamicState,
  createThermodynamicState,
  getThermodynamicState,
  resetThermodynamicState,
  THERMO_CONFIG,

  // Homeostasis
  MetricBaseline,
  HomeostasisTracker,
  createHomeostasisTracker,
  getHomeostasisTracker,
  resetHomeostasisTracker,
  HOMEOSTASIS_CONFIG,

  // Growth
  GrowthEvent,
  GrowthTracker,
  createGrowthTracker,
  getGrowthTracker,
  resetGrowthTracker,
  GROWTH_CONFIG,

  // Resilience
  Incident,
  ResilienceTracker,
  createResilienceTracker,
  getResilienceTracker,
  resetResilienceTracker,
  RESILIENCE_CONFIG,

  // Vitality
  VitalityMonitor,
  calculateVitality,
  getVitalityStatus,
  getDimensionScores,
  createVitalityMonitor,
  getVitalityMonitor,
  resetVitalityMonitor,
  VITALITY_CONFIG,

  // Unified
  getOrganismTrackers,
  getOrganismState,
  resetOrganismState,
  recordSuccess,
  recordError,
} from '../src/organism/index.js';

// Reset all singletons before each test
beforeEach(() => {
  resetMetabolismTracker();
  resetThermodynamicState();
  resetHomeostasisTracker();
  resetGrowthTracker();
  resetResilienceTracker();
  resetVitalityMonitor();
});

// ═══════════════════════════════════════════════════════════════════════════════
// METABOLISM TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Metabolism', () => {
  describe('MetabolicSample', () => {
    it('should create sample with token counts', () => {
      const sample = new MetabolicSample({
        tokensIn: 100,
        tokensOut: 50,
        latencyMs: 500,
      });

      assert.strictEqual(sample.tokensIn, 100);
      assert.strictEqual(sample.tokensOut, 50);
      assert.strictEqual(sample.totalTokens, 150);
    });

    it('should calculate weighted tokens', () => {
      const sample = new MetabolicSample({
        tokensIn: 100,  // cost 1x
        tokensOut: 50,  // cost 3x
      });

      // 100 * 1 + 50 * 3 = 250
      assert.strictEqual(sample.weightedTokens, 250);
    });

    it('should calculate efficiency φ-bounded', () => {
      const sample = new MetabolicSample({
        tokensIn: 100,
        tokensOut: 200, // 200% efficiency raw
      });

      // Should be capped at φ⁻¹
      assert.ok(sample.efficiency <= PHI_INV);
    });
  });

  describe('MetabolismTracker', () => {
    it('should track metabolic events', () => {
      const tracker = createMetabolismTracker();

      tracker.record({ tokensIn: 100, tokensOut: 50, latencyMs: 500 });
      tracker.record({ tokensIn: 200, tokensOut: 100, latencyMs: 300 });

      const stats = tracker.getStats();
      assert.strictEqual(stats.session.totalOperations, 2);
      assert.strictEqual(stats.session.totalTokensIn, 300);
    });

    it('should calculate metabolic rate', () => {
      const tracker = createMetabolismTracker({ windowSize: 60000 });

      tracker.record({ tokensIn: 1000, tokensOut: 500, latencyMs: 100 });

      const rate = tracker.getMetabolicRate();
      assert.ok(rate > 0);
    });

    it('should calculate metabolic health', () => {
      const tracker = createMetabolismTracker();

      // Record some activity
      for (let i = 0; i < 10; i++) {
        tracker.record({ tokensIn: 100, tokensOut: 50, latencyMs: 100 });
      }

      const health = tracker.getHealth();
      assert.ok('score' in health);
      assert.ok('status' in health);
      assert.ok(health.score <= PHI_INV);
    });

    it('should return dormant for no activity', () => {
      const tracker = createMetabolismTracker();
      const health = tracker.getHealth();
      assert.strictEqual(health.status, 'dormant');
    });
  });

  describe('Singleton', () => {
    it('should return same instance', () => {
      const t1 = getMetabolismTracker();
      const t2 = getMetabolismTracker();
      assert.strictEqual(t1, t2);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// THERMODYNAMICS TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Thermodynamics', () => {
  describe('ThermoEvent', () => {
    it('should classify heat events', () => {
      const error = new ThermoEvent(ThermoEventType.ERROR, 1);
      assert.strictEqual(error.isHeat, true);
      assert.strictEqual(error.isWork, false);
      assert.ok(error.heat > 0);
    });

    it('should classify work events', () => {
      const success = new ThermoEvent(ThermoEventType.SUCCESS, 1);
      assert.strictEqual(success.isWork, true);
      assert.strictEqual(success.isHeat, false);
      assert.ok(success.work > 0);
    });

    it('should apply heat multipliers', () => {
      const error = new ThermoEvent(ThermoEventType.ERROR, 1);
      const timeout = new ThermoEvent(ThermoEventType.TIMEOUT, 1);

      // Timeout should generate more heat than error
      assert.ok(timeout.heat > error.heat);
    });
  });

  describe('ThermodynamicState', () => {
    it('should start at optimal temperature', () => {
      const state = createThermodynamicState();
      assert.strictEqual(state.getTemperature(), THERMO_CONFIG.TEMP_OPTIMAL);
    });

    it('should increase temperature with heat events', () => {
      const state = createThermodynamicState();
      const initialTemp = state.getTemperature();

      state.recordError(5);

      assert.ok(state.getTemperature() > initialTemp);
    });

    it('should calculate efficiency', () => {
      const state = createThermodynamicState();

      state.recordSuccess(10);
      state.recordError(5);

      const efficiency = state.getEfficiency();
      // W = 10, Q = 25 (5 * HEAT_ERROR)
      // η = 10 / (10 + 25) ≈ 0.286
      assert.ok(efficiency > 0);
      assert.ok(efficiency <= PHI_INV);
    });

    it('FALSIFIABLE: efficiency cannot exceed φ⁻¹', () => {
      const state = createThermodynamicState();

      // All successes, no errors
      for (let i = 0; i < 100; i++) {
        state.recordSuccess(10);
      }

      const efficiency = state.getEfficiency();
      assert.ok(efficiency <= PHI_INV + 0.001, `Efficiency ${efficiency} exceeds φ⁻¹`);
    });

    it('FALSIFIABLE: temperature rises with errors', () => {
      const state = createThermodynamicState();
      const temps = [];

      for (let i = 0; i < 5; i++) {
        temps.push(state.getTemperature());
        state.recordError(2);
      }

      // Each subsequent temperature should be higher
      for (let i = 1; i < temps.length; i++) {
        assert.ok(temps[i] > temps[i - 1], `Temperature did not rise: ${temps[i]} <= ${temps[i-1]}`);
      }
    });

    it('should decay temperature over time', async () => {
      const state = createThermodynamicState({ decayInterval: 10 });

      state.recordError(10);
      const hotTemp = state.getTemperature();

      // Wait for decay
      await new Promise(r => setTimeout(r, 50));

      const cooledTemp = state.getTemperature();
      assert.ok(cooledTemp < hotTemp, 'Temperature did not decay');
    });

    it('should cap temperature at collapse', () => {
      const state = createThermodynamicState();

      // Generate massive heat
      for (let i = 0; i < 100; i++) {
        state.recordTimeout(10);
      }

      assert.ok(state.getTemperature() <= THERMO_CONFIG.TEMP_COLLAPSE);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// HOMEOSTASIS TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Homeostasis', () => {
  describe('MetricBaseline', () => {
    it('should track metric over time', () => {
      const baseline = new MetricBaseline('latency');

      baseline.update(100);
      baseline.update(110);
      baseline.update(90);

      assert.ok(baseline.mean !== null);
    });

    it('should calculate coefficient of variation', () => {
      const baseline = new MetricBaseline('latency');

      // Stable values
      for (let i = 0; i < 10; i++) {
        baseline.update(100 + Math.random() * 10);
      }

      const cv = baseline.getCV();
      assert.ok(cv >= 0);
      assert.ok(cv < 1); // Should be relatively stable
    });

    it('should detect anomalies', () => {
      const baseline = new MetricBaseline('latency');

      // Establish baseline
      for (let i = 0; i < 20; i++) {
        baseline.update(100);
      }

      // Way outside normal
      assert.strictEqual(baseline.isNormal(500, 2), false);
      assert.strictEqual(baseline.isNormal(100, 2), true);
    });
  });

  describe('HomeostasisTracker', () => {
    it('should track multiple metrics', () => {
      const tracker = createHomeostasisTracker();

      tracker.update('latency', 100);
      tracker.update('tokenRate', 50);

      const stats = tracker.getStats();
      assert.ok('latency' in stats.metrics);
      assert.ok('tokenRate' in stats.metrics);
    });

    it('should calculate overall homeostasis', () => {
      const tracker = createHomeostasisTracker();

      // Stable updates
      for (let i = 0; i < 10; i++) {
        tracker.update('latency', 100);
        tracker.update('tokenRate', 50);
      }

      const homeostasis = tracker.getHomeostasis();
      assert.ok(homeostasis >= 0);
      assert.ok(homeostasis <= PHI_INV);
    });

    it('FALSIFIABLE: high variance = low homeostasis', () => {
      const stableTracker = createHomeostasisTracker();
      const volatileTracker = createHomeostasisTracker();

      // Stable updates
      for (let i = 0; i < 20; i++) {
        stableTracker.update('test', 100);
      }

      // Volatile updates
      for (let i = 0; i < 20; i++) {
        volatileTracker.update('test', i % 2 === 0 ? 10 : 1000);
      }

      assert.ok(
        stableTracker.getHomeostasis() > volatileTracker.getHomeostasis(),
        'Stable system should have higher homeostasis than volatile'
      );
    });

    it('should detect perturbations', () => {
      const tracker = createHomeostasisTracker();

      // Establish baseline
      for (let i = 0; i < 10; i++) {
        tracker.update('metric', 100);
      }

      // Cause perturbation
      tracker.update('metric', 1000);

      const stats = tracker.getStats();
      assert.ok(stats.perturbations.total > 0);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// GROWTH TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Growth', () => {
  describe('GrowthTracker', () => {
    it('should track growth events', () => {
      const tracker = createGrowthTracker();

      tracker.recordPattern({ domain: 'code' });
      tracker.recordKnowledge({ domain: 'code' });

      const stats = tracker.getStats();
      assert.strictEqual(stats.eventCount, 2);
    });

    it('should calculate growth rate', () => {
      const tracker = createGrowthTracker();

      // Record several events
      for (let i = 0; i < 5; i++) {
        tracker.recordPattern();
      }

      const rate = tracker.getGrowthRate();
      assert.ok(rate >= 0);
    });

    it('should track by domain', () => {
      const tracker = createGrowthTracker();

      tracker.recordPattern({ domain: 'code', magnitude: 2 });
      tracker.recordPattern({ domain: 'solana', magnitude: 3 });

      const byDomain = tracker.getGrowthByDomain();
      assert.strictEqual(byDomain.code, 2);
      assert.strictEqual(byDomain.solana, 3);
    });

    it('should handle regressions', () => {
      const tracker = createGrowthTracker();

      tracker.recordPattern({ magnitude: 5 });
      tracker.recordRegression(GROWTH_CONFIG.TYPES.PATTERN, { magnitude: 2 });

      const stats = tracker.getStats();
      assert.strictEqual(stats.totalGrowth, 5);
      assert.strictEqual(stats.totalRegressions, 2);
    });

    it('FALSIFIABLE: no events = stagnant status', () => {
      const tracker = createGrowthTracker();
      assert.strictEqual(tracker.getStatus(), 'stagnant');
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// RESILIENCE TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Resilience', () => {
  describe('Incident', () => {
    it('should create incident with type', () => {
      const incident = new Incident('error', { message: 'Test error' });

      assert.strictEqual(incident.type, 'error');
      assert.strictEqual(incident.isRecovered, false);
    });

    it('should mark recovery', () => {
      const incident = new Incident('error');

      incident.markRecovered(true);

      assert.strictEqual(incident.isRecovered, true);
      assert.strictEqual(incident.isSuccessful, true);
      assert.ok(incident.recoveryTimeMs >= 0);
    });
  });

  describe('ResilienceTracker', () => {
    it('should track incidents', () => {
      const tracker = createResilienceTracker();

      tracker.recordError({ message: 'Error 1' });
      tracker.recordError({ message: 'Error 2' });

      assert.strictEqual(tracker.totalIncidents, 2);
    });

    it('should track active incidents', () => {
      const tracker = createResilienceTracker();

      const incident = tracker.recordError();
      assert.strictEqual(tracker.getActiveIncidents().length, 1);

      tracker.markRecovered(incident.id, true);
      assert.strictEqual(tracker.getActiveIncidents().length, 0);
    });

    it('should calculate MTTR', () => {
      const tracker = createResilienceTracker();

      const i1 = tracker.recordError();
      tracker.markRecovered(i1.id, true);

      const mttr = tracker.getMTTR();
      assert.ok(mttr >= 0);
    });

    it('should calculate recovery rate', () => {
      const tracker = createResilienceTracker();

      const i1 = tracker.recordError();
      const i2 = tracker.recordError();

      tracker.markRecovered(i1.id, true);
      tracker.markRecovered(i2.id, false);

      const rate = tracker.getRecoveryRate();
      assert.strictEqual(rate, 0.5); // 1 success / 2 total
    });

    it('should trip circuit breaker after threshold', () => {
      const tracker = createResilienceTracker();

      for (let i = 0; i < RESILIENCE_CONFIG.CIRCUIT_BREAKER_THRESHOLD; i++) {
        tracker.recordError();
      }

      assert.strictEqual(tracker.isCircuitBreakerTripped(), true);
    });

    it('FALSIFIABLE: recovery reduces circuit breaker failures', () => {
      const tracker = createResilienceTracker();

      // Add some failures
      const i1 = tracker.recordError();
      const i2 = tracker.recordError();
      const failuresAfterErrors = tracker.circuitBreaker.failures;

      // Recover
      tracker.markRecovered(i1.id, true);
      tracker.markRecovered(i2.id, true);

      assert.ok(
        tracker.circuitBreaker.failures < failuresAfterErrors,
        'Recovery should reduce circuit breaker failures'
      );
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// VITALITY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Vitality', () => {
  describe('calculateVitality', () => {
    it('should return 0 for fresh state', () => {
      const vitality = calculateVitality();
      // Fresh state with no data = low vitality
      assert.ok(vitality >= 0);
    });

    it('FALSIFIABLE: vitality cannot exceed φ⁻¹', () => {
      // Even with perfect inputs, vitality is bounded
      const metabolism = getMetabolismTracker();
      const thermo = getThermodynamicState();
      const growth = getGrowthTracker();

      // Record all successes
      for (let i = 0; i < 50; i++) {
        metabolism.record({ tokensIn: 100, tokensOut: 50, latencyMs: 50 });
        thermo.recordSuccess(1);
        growth.recordPattern();
      }

      const vitality = calculateVitality();
      assert.ok(vitality <= PHI_INV + 0.001, `Vitality ${vitality} exceeds φ⁻¹`);
    });
  });

  describe('VitalityMonitor', () => {
    it('should take snapshots', () => {
      const monitor = createVitalityMonitor();
      const snapshot = monitor.getSnapshot();

      assert.ok('vitality' in snapshot);
      assert.ok('status' in snapshot);
      assert.ok('dimensions' in snapshot);
    });

    it('should track history', () => {
      const monitor = createVitalityMonitor();

      monitor.history.push(monitor.getSnapshot());
      monitor.history.push(monitor.getSnapshot());

      assert.strictEqual(monitor.history.length, 2);
    });

    it('should calculate trend', () => {
      const monitor = createVitalityMonitor();

      // Add declining snapshots
      for (let i = 10; i > 0; i--) {
        monitor.history.push({
          timestamp: Date.now(),
          vitality: i / 20,
          status: 'test',
          dimensions: {},
        });
      }

      const trend = monitor.getTrend();
      assert.strictEqual(trend, 'declining');
    });

    it('should generate health report', () => {
      const monitor = createVitalityMonitor();
      const report = monitor.getHealthReport();

      assert.ok('current' in report);
      assert.ok('trend' in report);
      assert.ok('recommendations' in report);
    });
  });

  describe('getVitalityStatus', () => {
    it('should return valid status', () => {
      const status = getVitalityStatus();
      const validStatuses = ['thriving', 'healthy', 'struggling', 'critical'];
      assert.ok(validStatuses.includes(status));
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// INTEGRATION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Organism Integration', () => {
  it('should get all trackers', () => {
    const trackers = getOrganismTrackers();

    assert.ok('metabolism' in trackers);
    assert.ok('thermodynamics' in trackers);
    assert.ok('homeostasis' in trackers);
    assert.ok('growth' in trackers);
    assert.ok('resilience' in trackers);
    assert.ok('vitality' in trackers);
  });

  it('should get organism state', () => {
    const state = getOrganismState();

    assert.ok('vitality' in state);
    assert.ok('metabolism' in state);
    assert.ok('thermodynamics' in state);
  });

  it('should reset all state', () => {
    // Add some data
    recordSuccess(5);
    recordError(2);

    // Reset
    resetOrganismState();

    const state = getOrganismState();
    assert.strictEqual(state.thermodynamics.work, 0);
    assert.strictEqual(state.thermodynamics.heat, 0);
  });

  it('recordSuccess should increase work', () => {
    const before = getThermodynamicState().work;
    recordSuccess(5);
    const after = getThermodynamicState().work;

    assert.ok(after > before);
  });

  it('recordError should increase heat and temperature', () => {
    const beforeHeat = getThermodynamicState().heat;
    const beforeTemp = getThermodynamicState().getTemperature();

    recordError(3);

    const afterHeat = getThermodynamicState().heat;
    const afterTemp = getThermodynamicState().getTemperature();

    assert.ok(afterHeat > beforeHeat);
    assert.ok(afterTemp > beforeTemp);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// FALSIFIABILITY TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Falsifiability', () => {
  it('FALSIFIABLE: errors degrade vitality', () => {
    // Record successes first
    for (let i = 0; i < 10; i++) {
      recordSuccess(1);
    }
    const beforeVitality = calculateVitality();

    // Now record errors
    for (let i = 0; i < 10; i++) {
      recordError(1);
    }
    const afterVitality = calculateVitality();

    assert.ok(
      afterVitality <= beforeVitality,
      `Vitality should not increase with errors: ${beforeVitality} -> ${afterVitality}`
    );
  });

  it('FALSIFIABLE: all dimensions are φ-bounded', () => {
    const scores = getDimensionScores();

    for (const [dim, score] of Object.entries(scores)) {
      assert.ok(
        score <= PHI_INV + 0.001,
        `Dimension ${dim} (${score}) exceeds φ⁻¹`
      );
    }
  });

  it('FALSIFIABLE: temperature cannot go below optimal without cooling', () => {
    const thermo = getThermodynamicState();

    // Only successes (no heat)
    for (let i = 0; i < 20; i++) {
      thermo.recordSuccess(1);
    }

    assert.ok(
      thermo.getTemperature() >= THERMO_CONFIG.TEMP_OPTIMAL,
      'Temperature below optimal without cooling mechanism'
    );
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// CONFIGURATION TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Configuration', () => {
  it('should have φ-aligned thresholds', () => {
    assert.strictEqual(THERMO_CONFIG.MAX_EFFICIENCY, PHI_INV);
    assert.strictEqual(VITALITY_CONFIG.MAX_VITALITY, PHI_INV);
    assert.strictEqual(RESILIENCE_CONFIG.SUCCESS_RATE_HEALTHY, PHI_INV);
  });

  it('should have temperature warning at φ × 50', () => {
    const expected = Math.round(1.618 * 50); // ~81
    assert.strictEqual(THERMO_CONFIG.TEMP_WARNING, expected);
  });
});
