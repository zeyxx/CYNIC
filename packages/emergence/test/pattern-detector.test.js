/**
 * PatternDetector Tests
 *
 * "Patterns emerge from chaos" - κυνικός
 *
 * @module @cynic/emergence/test/pattern-detector
 */

import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert/strict';

import {
  PatternDetector,
  createPatternDetector,
  PatternType,
  SIGNIFICANCE_THRESHOLDS,
} from '../src/pattern-detector.js';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

// =============================================================================
// CONSTANTS
// =============================================================================

describe('PatternType', () => {
  it('should have all pattern types', () => {
    assert.strictEqual(PatternType.SEQUENCE, 'SEQUENCE');
    assert.strictEqual(PatternType.ANOMALY, 'ANOMALY');
    assert.strictEqual(PatternType.TREND, 'TREND');
    assert.strictEqual(PatternType.CLUSTER, 'CLUSTER');
    assert.strictEqual(PatternType.CORRELATION, 'CORRELATION');
    assert.strictEqual(PatternType.CYCLE, 'CYCLE');
    assert.strictEqual(PatternType.EMERGENCE, 'EMERGENCE');
  });
});

describe('SIGNIFICANCE_THRESHOLDS', () => {
  it('should have φ-aligned thresholds', () => {
    assert.ok(Math.abs(SIGNIFICANCE_THRESHOLDS.TRIVIAL - PHI_INV_3) < 0.001);
    assert.ok(Math.abs(SIGNIFICANCE_THRESHOLDS.NOTABLE - PHI_INV_2) < 0.001);
    assert.ok(Math.abs(SIGNIFICANCE_THRESHOLDS.SIGNIFICANT - PHI_INV) < 0.001);
    assert.strictEqual(SIGNIFICANCE_THRESHOLDS.CRITICAL, 0.9);
  });
});

// =============================================================================
// PATTERN DETECTOR
// =============================================================================

describe('PatternDetector', () => {
  let detector;

  beforeEach(() => {
    detector = createPatternDetector();
  });

  describe('Construction', () => {
    it('should create with factory', () => {
      assert.ok(detector instanceof PatternDetector);
    });

    it('should have default options', () => {
      assert.strictEqual(detector.windowSize, 50);
      assert.strictEqual(detector.minOccurrences, 3);
      assert.strictEqual(detector.anomalyThreshold, 2);
    });

    it('should accept custom options', () => {
      const custom = createPatternDetector({
        windowSize: 100,
        minOccurrences: 5,
        anomalyThreshold: 3,
      });
      assert.strictEqual(custom.windowSize, 100);
      assert.strictEqual(custom.minOccurrences, 5);
      assert.strictEqual(custom.anomalyThreshold, 3);
    });

    it('should start with empty data', () => {
      assert.strictEqual(detector.dataPoints.length, 0);
      assert.strictEqual(detector.patterns.size, 0);
    });
  });

  describe('observe()', () => {
    it('should add data points', () => {
      detector.observe({ type: 'TEST', value: 50 });
      assert.strictEqual(detector.dataPoints.length, 1);
    });

    it('should add timestamp to data points', () => {
      const before = Date.now();
      detector.observe({ type: 'TEST' });
      const after = Date.now();

      const point = detector.dataPoints[0];
      assert.ok(point.timestamp >= before);
      assert.ok(point.timestamp <= after);
    });

    it('should update statistics for numeric values', () => {
      detector.observe({ value: 10 });
      detector.observe({ value: 20 });
      detector.observe({ value: 30 });

      assert.strictEqual(detector.stats.count, 3);
      assert.strictEqual(detector.stats.mean, 20);
    });

    it('should maintain buffer size', () => {
      const detector = createPatternDetector({ windowSize: 5 });
      for (let i = 0; i < 100; i++) {
        detector.observe({ value: i });
      }
      // maxDataPoints = windowSize * 10 = 50
      assert.ok(detector.dataPoints.length <= 50);
    });

    it('should detect immediate anomalies', () => {
      // Build baseline
      for (let i = 0; i < 20; i++) {
        detector.observe({ value: 50 });
      }

      // Extreme value
      const patterns = detector.observe({ value: 150 });

      assert.ok(patterns.length > 0);
      assert.strictEqual(patterns[0].type, PatternType.ANOMALY);
    });
  });

  describe('detect()', () => {
    it('should return empty array with no data', () => {
      const patterns = detector.detect();
      assert.ok(Array.isArray(patterns));
    });

    it('should detect trends', () => {
      // Increasing trend
      for (let i = 0; i < 20; i++) {
        detector.observe({ value: i * 10 });
      }

      const patterns = detector.detect();
      const trend = patterns.find(p => p.type === PatternType.TREND);

      if (trend) {
        assert.strictEqual(trend.data.direction, 'INCREASING');
      }
    });

    it('should detect clusters', () => {
      // Cluster around 50
      for (let i = 0; i < 15; i++) {
        detector.observe({ value: 50 + Math.random() * 5 });
      }

      const patterns = detector.detect();
      const cluster = patterns.find(p => p.type === PatternType.CLUSTER);

      assert.ok(cluster, 'Should detect cluster');
      assert.ok(cluster.data.center >= 45 && cluster.data.center <= 60);
    });

    it('should detect sequences', () => {
      // Repeating sequence
      for (let i = 0; i < 15; i++) {
        detector.observe({ verdict: 'WAG' });
        detector.observe({ verdict: 'GROWL' });
      }

      const patterns = detector.detect();
      const sequence = patterns.find(p => p.type === PatternType.SEQUENCE);

      assert.ok(sequence, 'Should detect repeating sequence');
    });
  });

  describe('getPatterns()', () => {
    beforeEach(() => {
      // Add various patterns
      for (let i = 0; i < 20; i++) {
        detector.observe({ value: 50 });
      }
      detector.observe({ value: 150 }); // Anomaly
      detector.detect();
    });

    it('should return all patterns without filter', () => {
      const all = detector.getPatterns();
      assert.ok(all.length > 0);
    });

    it('should filter by type', () => {
      const anomalies = detector.getPatterns(PatternType.ANOMALY);
      for (const p of anomalies) {
        assert.strictEqual(p.type, PatternType.ANOMALY);
      }
    });

    it('should filter by minimum significance', () => {
      const significant = detector.getPatterns(null, 0.5);
      for (const p of significant) {
        assert.ok(p.significance >= 0.5);
      }
    });
  });

  describe('getTopPatterns()', () => {
    it('should return patterns sorted by significance', () => {
      // Create multiple patterns
      for (let i = 0; i < 30; i++) {
        detector.observe({ value: 50 + Math.random() * 10 });
      }
      detector.observe({ value: 200 }); // Big anomaly
      detector.detect();

      const top = detector.getTopPatterns(5);

      for (let i = 1; i < top.length; i++) {
        assert.ok(top[i - 1].significance >= top[i].significance);
      }
    });

    it('should respect limit', () => {
      for (let i = 0; i < 50; i++) {
        detector.observe({ value: Math.random() * 100 });
      }
      detector.detect();

      const top = detector.getTopPatterns(3);
      assert.ok(top.length <= 3);
    });
  });

  describe('hasPattern()', () => {
    it('should return pattern if exists', () => {
      for (let i = 0; i < 20; i++) {
        detector.observe({ value: 50 });
      }
      detector.observe({ value: 150 });
      detector.detect();

      const all = detector.getPatterns();
      if (all.length > 0) {
        const found = detector.hasPattern(all[0].id);
        assert.ok(found);
        assert.strictEqual(found.id, all[0].id);
      }
    });

    it('should return null if not exists', () => {
      const found = detector.hasPattern('nonexistent');
      assert.strictEqual(found, null);
    });
  });

  describe('Statistics', () => {
    it('should calculate variance correctly', () => {
      detector.observe({ value: 10 });
      detector.observe({ value: 20 });
      detector.observe({ value: 30 });

      const variance = detector._getVariance();
      // Variance of [10, 20, 30] = 100
      assert.ok(Math.abs(variance - 100) < 1);
    });

    it('should calculate standard deviation', () => {
      detector.observe({ value: 10 });
      detector.observe({ value: 20 });
      detector.observe({ value: 30 });

      const stdDev = detector._getStdDev();
      // StdDev of [10, 20, 30] = 10
      assert.ok(Math.abs(stdDev - 10) < 1);
    });
  });

  describe('getStats()', () => {
    it('should return statistics summary', () => {
      detector.observe({ value: 50 });
      detector.observe({ value: 60 });
      detector.detect();

      const stats = detector.getStats();
      assert.ok('dataPoints' in stats);
      assert.ok('patterns' in stats);
      assert.ok('mean' in stats);
      assert.ok('stdDev' in stats);
      assert.ok('distribution' in stats);
    });
  });

  describe('export/import', () => {
    it('should export state', () => {
      detector.observe({ value: 50 });
      detector.detect();

      const exported = detector.export();
      assert.ok('dataPoints' in exported);
      assert.ok('patterns' in exported);
      assert.ok('stats' in exported);
      assert.ok('exportedAt' in exported);
    });

    it('should import state', () => {
      detector.observe({ value: 50 });
      detector.detect();
      const exported = detector.export();

      const newDetector = createPatternDetector();
      newDetector.import(exported);

      assert.strictEqual(newDetector.stats.count, detector.stats.count);
    });
  });

  describe('clear()', () => {
    it('should clear all data', () => {
      detector.observe({ value: 50 });
      detector.detect();

      detector.clear();

      assert.strictEqual(detector.dataPoints.length, 0);
      assert.strictEqual(detector.patterns.size, 0);
      assert.strictEqual(detector.stats.count, 0);
    });
  });

  describe('φ-alignment', () => {
    it('should cap confidence at φ⁻¹', () => {
      // Create high-confidence pattern
      for (let i = 0; i < 30; i++) {
        detector.observe({ value: 50 });
      }
      detector.detect();

      const patterns = detector.getPatterns();
      for (const p of patterns) {
        assert.ok(p.confidence <= PHI_INV + 0.001, `Confidence ${p.confidence} exceeds φ⁻¹`);
      }
    });
  });
});
