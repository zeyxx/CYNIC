/**
 * Perception Modules Tests
 *
 * Tests for CYNIC's perception system: harmonic feedback,
 * Thompson Sampling, confidence calibration.
 *
 * "Le chien observe avant de mordre" - CYNIC
 *
 * @module scripts/hooks/test/perception
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

// Import perception modules
import {
  HarmonicFeedbackSystem,
  ThompsonSampler,
  ConfidenceCalibrator,
  getHarmonicFeedback,
  resetHarmonicFeedback,
  SEFIROT_CHANNELS,
} from '../lib/harmonic-feedback.js';

// Constants
const PHI = 1.618033988749895;
const PHI_INV = 0.618033988749895;
const PHI_INV_2 = 0.381966011250105;
const PHI_INV_3 = 0.236067977499790;

// ═══════════════════════════════════════════════════════════════════════════
// THOMPSON SAMPLER TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('ThompsonSampler', () => {
  let sampler;

  beforeEach(() => {
    sampler = new ThompsonSampler();
  });

  describe('Initialization', () => {
    it('should initialize with empty arms', () => {
      assert.strictEqual(sampler.arms.size, 0);
      assert.strictEqual(sampler.totalPulls, 0);
    });
  });

  describe('Arm Management', () => {
    it('should create arm with default priors', () => {
      sampler.initArm('test_arm');
      const arm = sampler.arms.get('test_arm');
      assert.ok(arm);
      assert.strictEqual(arm.alpha, 1);
      assert.strictEqual(arm.beta, 1);
      assert.strictEqual(arm.pulls, 0);
    });

    it('should not reinitialize existing arm', () => {
      sampler.initArm('test_arm');
      sampler.update('test_arm', true);
      sampler.initArm('test_arm'); // Should not reset
      const arm = sampler.arms.get('test_arm');
      assert.strictEqual(arm.alpha, 2); // 1 + 1 from success
    });
  });

  describe('Update Mechanism', () => {
    it('should increase alpha on success', () => {
      sampler.initArm('test_arm');
      sampler.update('test_arm', true);
      const arm = sampler.arms.get('test_arm');
      assert.strictEqual(arm.alpha, 2);
      assert.strictEqual(arm.beta, 1);
      assert.strictEqual(arm.pulls, 1);
    });

    it('should increase beta on failure', () => {
      sampler.initArm('test_arm');
      sampler.update('test_arm', false);
      const arm = sampler.arms.get('test_arm');
      assert.strictEqual(arm.alpha, 1);
      assert.strictEqual(arm.beta, 2);
      assert.strictEqual(arm.pulls, 1);
    });

    it('should track total pulls', () => {
      sampler.initArm('arm1');
      sampler.initArm('arm2');
      sampler.update('arm1', true);
      sampler.update('arm2', false);
      sampler.update('arm1', true);
      assert.strictEqual(sampler.totalPulls, 3);
    });
  });

  describe('Expected Value', () => {
    it('should calculate correct expected value', () => {
      sampler.initArm('test_arm');
      // After 3 successes, 1 failure: alpha=4, beta=2
      sampler.update('test_arm', true);
      sampler.update('test_arm', true);
      sampler.update('test_arm', true);
      sampler.update('test_arm', false);

      const expected = sampler.getExpectedValue('test_arm');
      // E[Beta(4,2)] = 4/(4+2) = 0.667, but capped at PHI_INV
      assert.ok(expected <= PHI_INV, `Expected ${expected} to be <= ${PHI_INV}`);
    });

    it('should cap expected value at phi inverse', () => {
      sampler.initArm('test_arm');
      // Many successes to push expected value high
      for (let i = 0; i < 20; i++) {
        sampler.update('test_arm', true);
      }
      const expected = sampler.getExpectedValue('test_arm');
      assert.ok(expected <= PHI_INV, `Expected value ${expected} exceeds phi cap ${PHI_INV}`);
    });
  });

  describe('Arm Selection', () => {
    it('should select from available arms', () => {
      sampler.initArm('arm1');
      sampler.initArm('arm2');
      sampler.initArm('arm3');

      const selected = sampler.selectArm(['arm1', 'arm2', 'arm3']);
      assert.ok(['arm1', 'arm2', 'arm3'].includes(selected));
    });

    it('should return null for empty arms', () => {
      const selected = sampler.selectArm([]);
      assert.strictEqual(selected, null);
    });
  });

  describe('State Export/Import', () => {
    it('should export and import state correctly', () => {
      sampler.initArm('arm1');
      sampler.update('arm1', true);
      sampler.update('arm1', true);

      const exported = sampler.exportState();
      assert.ok(exported.arms.arm1);
      assert.strictEqual(exported.arms.arm1.alpha, 3);

      const newSampler = new ThompsonSampler();
      newSampler.importState(exported);
      assert.strictEqual(newSampler.arms.get('arm1').alpha, 3);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// CONFIDENCE CALIBRATOR TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('ConfidenceCalibrator', () => {
  let calibrator;

  beforeEach(() => {
    calibrator = new ConfidenceCalibrator();
  });

  describe('Initialization', () => {
    it('should initialize with buckets', () => {
      assert.ok(Array.isArray(calibrator.buckets));
      assert.strictEqual(calibrator.buckets.length, 10); // 10 buckets
    });

    it('should have calibration factor of 1.0', () => {
      assert.strictEqual(calibrator.calibrationFactor, 1.0);
    });
  });

  describe('Calibrate Method', () => {
    it('should apply calibration factor', () => {
      calibrator.calibrationFactor = 0.8;
      const calibrated = calibrator.calibrate(0.5);
      assert.strictEqual(calibrated, 0.4);
    });

    it('should never exceed phi inverse', () => {
      calibrator.calibrationFactor = 2.0;
      const calibrated = calibrator.calibrate(0.5);
      assert.ok(calibrated <= PHI_INV);
    });

    it('should not go below zero', () => {
      calibrator.calibrationFactor = 0.1;
      const calibrated = calibrator.calibrate(0.01);
      assert.ok(calibrated >= 0);
    });
  });

  describe('Prediction Recording', () => {
    it('should record to correct bucket', () => {
      calibrator.record(0.75, true); // Should go to bucket 7 (70-80%)
      calibrator.record(0.25, false); // Should go to bucket 2 (20-30%)

      // Check that buckets have data
      const bucketsWithData = calibrator.buckets.filter(b => b.count > 0);
      assert.ok(bucketsWithData.length > 0);
    });
  });

  describe('State Export/Import', () => {
    it('should export state', () => {
      calibrator.record(0.5, true);
      const exported = calibrator.exportState();
      assert.ok(exported.buckets || exported.calibrationFactor !== undefined);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// HARMONIC FEEDBACK SYSTEM TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('HarmonicFeedbackSystem', () => {
  let harmonic;

  beforeEach(() => {
    resetHarmonicFeedback();
    harmonic = getHarmonicFeedback();
  });

  describe('Initialization', () => {
    it('should initialize with Sefirot channels', () => {
      assert.ok(harmonic.sefirotSignals.size > 0);
      assert.ok(harmonic.sefirotSignals.has('KETER'));
      assert.ok(harmonic.sefirotSignals.has('MALKHUT'));
    });

    it('should have Thompson sampler', () => {
      assert.ok(harmonic.thompsonSampler instanceof ThompsonSampler);
    });

    it('should have confidence calibrator', () => {
      assert.ok(harmonic.confidenceCalibrator instanceof ConfidenceCalibrator);
    });

    it('should have heuristics map', () => {
      assert.ok(harmonic.heuristics instanceof Map);
    });
  });

  describe('Signal Processing', () => {
    it('should have sefirot signals map', () => {
      assert.ok(harmonic.sefirotSignals instanceof Map);
      assert.ok(harmonic.sefirotSignals.size > 0);
    });

    it('should calculate coherence', () => {
      const state = harmonic.getState();
      assert.ok(typeof state.coherence === 'number');
      assert.ok(state.coherence >= 0 && state.coherence <= 1);
    });

    it('should calculate resonance', () => {
      const state = harmonic.getState();
      assert.ok(typeof state.resonance === 'number');
      assert.ok(state.resonance >= 0 && state.resonance <= 1);
    });
  });

  describe('Feedback Processing', () => {
    it('should process positive feedback', () => {
      const initialPulls = harmonic.thompsonSampler.totalPulls;

      harmonic.processFeedback({
        type: 'test_suggestion',
        sentiment: 'positive',
        confidence: 0.7,
        source: 'test',
      });

      assert.strictEqual(harmonic.thompsonSampler.totalPulls, initialPulls + 1);
    });

    it('should process negative feedback', () => {
      harmonic.processFeedback({
        type: 'test_suggestion',
        sentiment: 'negative',
        confidence: 0.7,
        source: 'test',
      });

      const arm = harmonic.thompsonSampler.arms.get('test_suggestion');
      assert.ok(arm.beta > 1); // Beta increased
    });

    it('should update Thompson sampler after feedback', () => {
      const pullsBefore = harmonic.thompsonSampler.totalPulls;

      harmonic.processFeedback({
        type: 'test_pattern',
        sentiment: 'positive',
        confidence: 0.6,
      });

      const pullsAfter = harmonic.thompsonSampler.totalPulls;
      assert.ok(pullsAfter > pullsBefore);
    });
  });

  describe('Pattern Promotion', () => {
    it('should review patterns', () => {
      // Force reset last review time to allow review
      harmonic.lastReviewTime = 0;

      // Add some patterns with data
      for (let i = 0; i < 12; i++) {
        harmonic.processFeedback({
          type: 'pattern_a',
          sentiment: 'positive',
          confidence: 0.6,
        });
      }

      const result = harmonic.reviewPatterns();
      // Result should have either reviewed count or reviewed: false with reason
      assert.ok(
        typeof result.reviewed === 'number' ||
        (result.reviewed === false && result.reason)
      );
    });

    it('should get promotion stats', () => {
      const stats = harmonic.getPromotionStats();
      assert.ok(typeof stats.activeHeuristics === 'number');
      assert.ok(Array.isArray(stats.heuristicDetails));
    });
  });

  describe('Introspection', () => {
    it('should provide introspection data', () => {
      // Generate some learning data
      for (let i = 0; i < 5; i++) {
        harmonic.processFeedback({
          type: `pattern_${i}`,
          sentiment: i % 2 === 0 ? 'positive' : 'negative',
          confidence: 0.5,
        });
      }

      const intro = harmonic.introspect();

      assert.ok(intro.conscious);
      assert.ok(intro.subconscious);
      assert.ok(intro.meta);
      assert.ok(intro.conscious.maxConfidence <= PHI_INV);
    });

    it('should include philosophy in meta', () => {
      const intro = harmonic.introspect();
      assert.ok(intro.meta.philosophy.includes('61.8%'));
    });

    it('should assess confidence gap', () => {
      const intro = harmonic.introspect();
      assert.ok(typeof intro.meta.confidenceGap === 'number');
      assert.ok(typeof intro.meta.selfAssessment === 'string');
    });
  });

  describe('State Export/Import', () => {
    it('should export complete state', () => {
      harmonic.processFeedback({ type: 'test', sentiment: 'positive' });

      const exported = harmonic.exportState();

      assert.ok(exported.thompson);
      assert.ok(exported.heuristics !== undefined);
      assert.ok(exported.calibration);
      assert.ok(exported.sefirot);
    });

    it('should import state correctly', () => {
      harmonic.processFeedback({ type: 'test', sentiment: 'positive' });
      const exported = harmonic.exportState();

      resetHarmonicFeedback();
      const fresh = getHarmonicFeedback();
      fresh.importState(exported);

      const arm = fresh.thompsonSampler.arms.get('test');
      assert.ok(arm);
      assert.strictEqual(arm.alpha, 2);
    });
  });

  describe('Getters', () => {
    it('should get state', () => {
      const state = harmonic.getState();
      assert.ok(typeof state === 'object');
      assert.ok('coherence' in state);
      assert.ok('resonance' in state);
    });

    it('should get insights', () => {
      const insights = harmonic.getInsights();
      assert.ok(typeof insights === 'object');
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// SEFIROT CHANNELS TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('Sefirot Channels', () => {
  it('should have all 10 Sefirot defined', () => {
    const sefirot = [
      'KETER', 'CHOCHMAH', 'BINAH', 'CHESED',
      'GEVURAH', 'TIFERET', 'NETZACH', 'HOD',
      'YESOD', 'MALKHUT',
    ];

    for (const sefira of sefirot) {
      assert.ok(SEFIROT_CHANNELS[sefira], `Missing Sefirot: ${sefira}`);
    }
  });

  it('should have phi-aligned weights', () => {
    const phiValues = [PHI, PHI_INV, PHI_INV_2, PHI_INV_3];

    for (const [name, config] of Object.entries(SEFIROT_CHANNELS)) {
      assert.ok(
        phiValues.some(phi => Math.abs(config.signalWeight - phi) < 0.01) ||
        config.signalWeight === 1.0,
        `${name} weight ${config.signalWeight} not phi-aligned`
      );
    }
  });

  it('should have correct domains', () => {
    assert.strictEqual(SEFIROT_CHANNELS.KETER.domain, 'orchestration');
    assert.strictEqual(SEFIROT_CHANNELS.GEVURAH.domain, 'protection');
    assert.strictEqual(SEFIROT_CHANNELS.MALKHUT.domain, 'manifestation');
  });

  it('should have names', () => {
    assert.strictEqual(SEFIROT_CHANNELS.KETER.name, 'Crown');
    assert.strictEqual(SEFIROT_CHANNELS.CHOCHMAH.name, 'Wisdom');
    assert.strictEqual(SEFIROT_CHANNELS.BINAH.name, 'Understanding');
  });
});

// ═══════════════════════════════════════════════════════════════════════════
// PHI CONSTRAINTS TESTS
// ═══════════════════════════════════════════════════════════════════════════

describe('Phi Constraints', () => {
  it('should verify PHI_INV constant', () => {
    assert.ok(Math.abs(PHI_INV - 0.618) < 0.001);
  });

  it('should verify PHI_INV_2 constant', () => {
    assert.ok(Math.abs(PHI_INV_2 - 0.382) < 0.001);
  });

  it('should verify PHI_INV_3 constant', () => {
    assert.ok(Math.abs(PHI_INV_3 - 0.236) < 0.001);
  });

  it('should verify PHI * PHI_INV = 1', () => {
    assert.ok(Math.abs(PHI * PHI_INV - 1) < 0.0001);
  });

  it('should verify PHI_INV^2 = PHI_INV_2', () => {
    assert.ok(Math.abs(PHI_INV * PHI_INV - PHI_INV_2) < 0.0001);
  });
});
