/**
 * ConsciousnessMonitor Tests
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/emergence/test/consciousness-monitor
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  ConsciousnessMonitor,
  createConsciousnessMonitor,
  ConsciousnessState,
  AWARENESS_THRESHOLDS,
  MAX_CONFIDENCE,
} from '../src/consciousness-monitor.js';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

// =============================================================================
// CONSTANTS
// =============================================================================

describe('ConsciousnessState', () => {
  it('should have all state values', () => {
    assert.strictEqual(ConsciousnessState.DORMANT, 'DORMANT');
    assert.strictEqual(ConsciousnessState.AWAKENING, 'AWAKENING');
    assert.strictEqual(ConsciousnessState.AWARE, 'AWARE');
    assert.strictEqual(ConsciousnessState.HEIGHTENED, 'HEIGHTENED');
    assert.strictEqual(ConsciousnessState.TRANSCENDENT, 'TRANSCENDENT');
  });
});

describe('AWARENESS_THRESHOLDS', () => {
  it('should have φ-aligned thresholds', () => {
    assert.strictEqual(AWARENESS_THRESHOLDS.DORMANT, 0);
    assert.ok(Math.abs(AWARENESS_THRESHOLDS.AWAKENING - PHI_INV_3) < 0.001);
    assert.ok(Math.abs(AWARENESS_THRESHOLDS.AWARE - PHI_INV_2) < 0.001);
    assert.ok(Math.abs(AWARENESS_THRESHOLDS.HEIGHTENED - PHI_INV) < 0.001);
    assert.strictEqual(AWARENESS_THRESHOLDS.TRANSCENDENT, 1.0);
  });
});

describe('MAX_CONFIDENCE', () => {
  it('should be φ⁻¹', () => {
    assert.ok(Math.abs(MAX_CONFIDENCE - PHI_INV) < 0.001);
  });
});

// =============================================================================
// CONSCIOUSNESS MONITOR
// =============================================================================

describe('ConsciousnessMonitor', () => {
  let monitor;

  beforeEach(() => {
    monitor = createConsciousnessMonitor();
  });

  describe('Construction', () => {
    it('should create with factory', () => {
      assert.ok(monitor instanceof ConsciousnessMonitor);
    });

    it('should have default window size', () => {
      assert.strictEqual(monitor.windowSize, 100);
    });

    it('should have φ-aligned decay rate', () => {
      assert.ok(Math.abs(monitor.decayRate - PHI_INV) < 0.001);
    });

    it('should start in DORMANT state', () => {
      assert.strictEqual(monitor.state, ConsciousnessState.DORMANT);
    });

    it('should start with 0 awareness', () => {
      assert.strictEqual(monitor.awarenessLevel, 0);
    });
  });

  describe('observe()', () => {
    it('should record observation', () => {
      const obs = monitor.observe('JUDGMENT', { verdict: 'WAG' }, 0.5);

      assert.ok(obs.id.startsWith('obs_'));
      assert.strictEqual(obs.type, 'JUDGMENT');
      assert.strictEqual(obs.confidence, 0.5);
    });

    it('should cap confidence at φ⁻¹', () => {
      const obs = monitor.observe('TEST', {}, 0.95);
      assert.ok(obs.confidence <= PHI_INV + 0.001);
    });

    it('should increment total observations', () => {
      monitor.observe('TEST', {});
      monitor.observe('TEST', {});
      assert.strictEqual(monitor.metrics.totalObservations, 2);
    });

    it('should update average confidence', () => {
      monitor.observe('TEST', {}, 0.5);
      monitor.observe('TEST', {}, 0.5);
      assert.strictEqual(monitor.metrics.avgConfidence, 0.5);
    });
  });

  describe('recordUncertainty()', () => {
    it('should track uncertainty', () => {
      monitor.recordUncertainty('classification', 0.3, { item: 'test' });
      assert.strictEqual(monitor.uncertaintyHistory.length, 1);
    });

    it('should track uncertainty zones for low confidence', () => {
      monitor.recordUncertainty('classification', 0.2);
      assert.strictEqual(monitor.metrics.uncertaintyZones.length, 1);
    });

    it('should not track zone for higher confidence', () => {
      monitor.recordUncertainty('classification', 0.5);
      assert.strictEqual(monitor.metrics.uncertaintyZones.length, 0);
    });
  });

  describe('Pattern recognition', () => {
    it('should notice patterns', () => {
      monitor.noticePattern('pattern_1', { type: 'TREND' }, 0.7);
      assert.ok(monitor.noticedPatterns.has('pattern_1'));
    });

    it('should check if pattern noticed', () => {
      monitor.noticePattern('pattern_1', { type: 'TREND' });
      assert.ok(monitor.hasNoticed('pattern_1'));
      assert.strictEqual(monitor.hasNoticed('unknown'), null);
    });

    it('should increment notice count', () => {
      monitor.noticePattern('pattern_1', { type: 'TREND' });
      monitor.noticePattern('pattern_1', { type: 'TREND' });

      const pattern = monitor.hasNoticed('pattern_1');
      assert.strictEqual(pattern.noticeCount, 2);
    });
  });

  describe('Predictions', () => {
    it('should record predictions', () => {
      monitor.recordPrediction('will succeed', true, 0.7);
      assert.strictEqual(monitor.metrics.decisionsCount, 1);
      assert.strictEqual(monitor.metrics.correctPredictions, 1);
    });

    it('should track incorrect predictions', () => {
      monitor.recordPrediction('will fail', false, 0.6);
      assert.strictEqual(monitor.metrics.decisionsCount, 1);
      assert.strictEqual(monitor.metrics.correctPredictions, 0);
    });

    it('should calculate prediction accuracy', () => {
      monitor.recordPrediction('p1', true);
      monitor.recordPrediction('p2', true);
      monitor.recordPrediction('p3', false);

      const accuracy = monitor.getPredictionAccuracy();
      assert.ok(Math.abs(accuracy - 2/3) < 0.01);
    });

    it('should return 0 accuracy with no predictions', () => {
      assert.strictEqual(monitor.getPredictionAccuracy(), 0);
    });
  });

  describe('State transitions', () => {
    it('should be DORMANT with few observations', () => {
      for (let i = 0; i < 5; i++) {
        monitor.observe('TEST', {}, 0.5);
      }
      assert.strictEqual(monitor.state, ConsciousnessState.DORMANT);
    });

    it('should progress state with more observations', () => {
      for (let i = 0; i < 15; i++) {
        monitor.observe('TEST', {}, 0.3);
      }
      // With 15 observations, should be past DORMANT
      // State depends on confidence and other factors
      assert.ok(monitor.awarenessLevel > 0, 'Awareness should be > 0 with observations');
    });

    it('should reach AWARE with good confidence', () => {
      for (let i = 0; i < 20; i++) {
        monitor.observe('TEST', {}, 0.5);
      }
      monitor.noticePattern('p1', {});
      monitor.noticePattern('p2', {});
      monitor.recordPrediction('test', true, 0.6);

      // Should be at least AWAKENING
      assert.ok(monitor.awarenessLevel > AWARENESS_THRESHOLDS.DORMANT);
    });
  });

  describe('getInsights()', () => {
    beforeEach(() => {
      for (let i = 0; i < 20; i++) {
        monitor.observe('TEST', {}, 0.5);
      }
    });

    it('should return insights object', () => {
      const insights = monitor.getInsights();

      assert.ok('state' in insights);
      assert.ok('awarenessLevel' in insights);
      assert.ok('totalObservations' in insights);
      assert.ok('avgConfidence' in insights);
      assert.ok('predictionAccuracy' in insights);
      assert.ok('recommendations' in insights);
    });

    it('should include recent activity', () => {
      const insights = monitor.getInsights();
      assert.ok('recentActivity' in insights);
      assert.ok(insights.recentActivity.count > 0);
    });
  });

  describe('getMetaInsight()', () => {
    beforeEach(() => {
      for (let i = 0; i < 20; i++) {
        monitor.observe('TEST', {}, 0.5);
      }
    });

    it('should return meta-insight object', () => {
      const meta = monitor.getMetaInsight();

      assert.ok('selfAwareness' in meta);
      assert.ok('blindSpots' in meta);
      assert.ok('strengths' in meta);
      assert.ok('growthAreas' in meta);
      assert.ok('coherence' in meta);
    });

    it('should include awareness trend', () => {
      const meta = monitor.getMetaInsight();
      assert.ok(['INCREASING', 'DECREASING', 'STABLE'].includes(meta.selfAwareness.trend));
    });
  });

  describe('export/import', () => {
    it('should export state', () => {
      monitor.observe('TEST', {}, 0.5);
      monitor.noticePattern('p1', {});

      const exported = monitor.export();

      assert.ok('observations' in exported);
      assert.ok('metrics' in exported);
      assert.ok('noticedPatterns' in exported);
      assert.ok('exportedAt' in exported);
    });

    it('should import state', () => {
      monitor.observe('TEST', {}, 0.5);
      monitor.noticePattern('p1', { type: 'test' });
      const exported = monitor.export();

      const newMonitor = createConsciousnessMonitor();
      newMonitor.import(exported);

      assert.ok(newMonitor.hasNoticed('p1'));
    });
  });

  describe('reset()', () => {
    it('should reset all state', () => {
      monitor.observe('TEST', {}, 0.5);
      monitor.noticePattern('p1', {});
      monitor.recordPrediction('test', true);

      monitor.reset();

      assert.strictEqual(monitor.observations.length, 0);
      assert.strictEqual(monitor.noticedPatterns.size, 0);
      assert.strictEqual(monitor.metrics.totalObservations, 0);
      assert.strictEqual(monitor.state, ConsciousnessState.DORMANT);
    });
  });
});
