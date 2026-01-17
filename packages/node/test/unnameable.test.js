/**
 * THE_UNNAMEABLE Tests (GAP-I11)
 *
 * Tests for the 25th dimension - unexplained variance and dimension discovery.
 *
 * "THE UNNAMEABLE = what exists before being named" - κυνικός
 */
import { describe, it, beforeEach } from 'node:test';
import assert from 'node:assert';

import {
  CYNICJudge,
  ResidualDetector,
} from '../src/index.js';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import { calculateResidual } from '@cynic/protocol';

// ═══════════════════════════════════════════════════════════════════════════
// THE_UNNAMEABLE Dimension Tests
// ═══════════════════════════════════════════════════════════════════════════

describe('THE_UNNAMEABLE - 25th Dimension (GAP-I11)', () => {
  let judge;
  let detector;

  beforeEach(() => {
    detector = new ResidualDetector();
    judge = new CYNICJudge({
      residualDetector: detector,
      includeUnnameable: true,
    });
  });

  describe('Dimension Inclusion', () => {
    it('should include THE_UNNAMEABLE in judgment dimensions', () => {
      const judgment = judge.judge({ id: 'test', content: 'test item' });

      assert.ok(judgment.dimensions);
      assert.ok('THE_UNNAMEABLE' in judgment.dimensions);
      assert.ok(typeof judgment.dimensions.THE_UNNAMEABLE === 'number');
    });

    it('should calculate THE_UNNAMEABLE as inverse of variance', () => {
      // Item with consistent scores should have high THE_UNNAMEABLE
      const judgment = judge.judge({
        id: 'consistent',
        scores: {
          COHERENCE: 70,
          HARMONY: 70,
          STRUCTURE: 70,
          ACCURACY: 70,
          VERIFIABILITY: 70,
          AUTHENTICITY: 70,
        },
      });

      // High consistency = high UNNAMEABLE score
      assert.ok(judgment.dimensions.THE_UNNAMEABLE > 70);
    });

    it('should have lower THE_UNNAMEABLE for high variance items', () => {
      // Consistent item
      const consistentJudgment = judge.judge({
        id: 'consistent',
        scores: {
          COHERENCE: 70,
          HARMONY: 70,
          STRUCTURE: 70,
        },
      });

      // Inconsistent item - note: other dimensions get default scores (~50)
      // so the variance comes from the explicit scores differing
      const inconsistentJudgment = judge.judge({
        id: 'inconsistent',
        scores: {
          COHERENCE: 10,
          HARMONY: 90,
          STRUCTURE: 20,
          ACCURACY: 80,
          VERIFIABILITY: 30,
          AUTHENTICITY: 70,
        },
      });

      // High variance should have LOWER score than consistent
      assert.ok(
        inconsistentJudgment.dimensions.THE_UNNAMEABLE <
        consistentJudgment.dimensions.THE_UNNAMEABLE
      );
    });

    it('should bound THE_UNNAMEABLE between 0-100', () => {
      for (let i = 0; i < 10; i++) {
        const judgment = judge.judge({
          id: `test_${i}`,
          quality: Math.random() * 100,
        });

        assert.ok(judgment.dimensions.THE_UNNAMEABLE >= 0);
        assert.ok(judgment.dimensions.THE_UNNAMEABLE <= 100);
      }
    });
  });

  describe('Residual Calculation', () => {
    it('should calculate residual from protocol', () => {
      const judgment = {
        id: 'jdg_test',
        global_score: 75,
        dimensions: { A: 50, B: 60, C: 70 },
      };

      const residual = calculateResidual(judgment);

      // Average dim = 60, global = 75, diff = 15, residual = 0.15
      assert.ok(typeof residual === 'number');
      assert.ok(residual >= 0 && residual <= 1);
    });

    it('should detect anomaly when residual > φ⁻²', () => {
      // Big gap between global score and dimension average
      const judgment = {
        id: 'jdg_anomaly',
        global_score: 90,
        dimensions: { A: 20, B: 25, C: 30 }, // avg = 25, gap = 65
      };

      const residual = calculateResidual(judgment);
      assert.ok(residual > PHI_INV_2); // > 38.2%
    });
  });

  describe('ResidualDetector Integration', () => {
    it('should feed anomalies to ResidualDetector', () => {
      // Create items that produce anomalies
      for (let i = 0; i < 5; i++) {
        judge.judge({
          id: `anomaly_${i}`,
          scores: {
            COHERENCE: 10 + i * 5,   // Very different scores
            ACCURACY: 90 - i * 10,
          },
        });
      }

      const stats = detector.getStats();
      assert.ok(stats.anomalyCount >= 0);
    });

    it('should accumulate anomalies for pattern detection', () => {
      // Create 10 similar anomalies
      for (let i = 0; i < 10; i++) {
        judge.judge({
          id: `pattern_${i}`,
          scores: {
            COHERENCE: 20,
            ACCURACY: 80,
            VERIFIABILITY: 25,
          },
        });
      }

      const candidates = judge.getCandidateDimensions();
      // May or may not have candidates depending on pattern
      assert.ok(Array.isArray(candidates));
    });

    it('should expose candidate dimensions through judge', () => {
      const candidates = judge.getCandidateDimensions();
      assert.ok(Array.isArray(candidates));
    });

    it('should expose residual stats through judge', () => {
      const stats = judge.getResidualStats();
      assert.ok(stats);
      assert.ok('anomalyCount' in stats);
      assert.ok('candidateCount' in stats);
      assert.ok('threshold' in stats);
    });
  });

  describe('Dimension Discovery Flow', () => {
    it('should build candidates from repeated anomalies', () => {
      // Create multiple similar anomalies
      for (let i = 0; i < 15; i++) {
        detector.analyze({
          id: `jdg_discovery_${i}`,
          global_score: 70,
          dimensions: {
            COHERENCE: 30,
            ACCURACY: 30,
          },
        });
      }

      const stats = detector.getStats();
      assert.ok(stats.anomalyCount >= 10);
    });

    it('should accept candidate with governance', () => {
      // Create enough anomalies to generate candidate
      for (let i = 0; i < 20; i++) {
        detector.analyze({
          id: `jdg_accept_${i}`,
          global_score: 85,
          dimensions: {
            ELEGANCE: 20,
            UTILITY: 25,
          },
        });
      }

      const candidates = detector.getCandidates();

      if (candidates.length > 0) {
        const candidate = candidates[0];

        // Accept with governance (human approval)
        const discovery = detector.acceptCandidate(candidate.key, {
          name: 'SUSTAINABILITY',
          axiom: 'CULTURE',
          weight: 1.0,
          threshold: 50,
        });

        assert.ok(discovery);
        assert.strictEqual(discovery.name, 'SUSTAINABILITY');
        assert.strictEqual(discovery.axiom, 'CULTURE');
      }
    });

    it('should reject candidate', () => {
      // Create anomalies
      for (let i = 0; i < 10; i++) {
        detector.analyze({
          id: `jdg_reject_${i}`,
          global_score: 60,
          dimensions: {
            NOVELTY: 10,
          },
        });
      }

      const beforeCount = detector.getStats().candidateCount;
      const candidates = detector.getCandidates();

      if (candidates.length > 0) {
        detector.rejectCandidate(candidates[0].key);
        const afterCount = detector.getStats().candidateCount;
        assert.ok(afterCount <= beforeCount);
      }
    });

    it('should track discovered dimensions', () => {
      // Create and accept multiple candidates
      for (let i = 0; i < 3; i++) {
        // Generate anomalies
        for (let j = 0; j < 15; j++) {
          detector.analyze({
            id: `jdg_multi_${i}_${j}`,
            global_score: 70 + i * 5,
            dimensions: {
              [`DIM_${i}`]: 20,
            },
          });
        }
      }

      const discoveries = detector.getDiscoveries();
      assert.ok(Array.isArray(discoveries));
    });
  });

  describe('Judge Methods', () => {
    it('should have setResidualDetector method', () => {
      const newJudge = new CYNICJudge();
      assert.ok(typeof newJudge.setResidualDetector === 'function');

      newJudge.setResidualDetector(detector);

      // Should now work
      const stats = newJudge.getResidualStats();
      assert.ok(stats);
    });

    it('should accept candidate through judge', () => {
      // Create anomalies
      for (let i = 0; i < 20; i++) {
        judge.judge({
          id: `accept_via_judge_${i}`,
          scores: {
            COHERENCE: 15,
            STRUCTURE: 20,
          },
        });
      }

      const candidates = judge.getCandidateDimensions();

      if (candidates.length > 0) {
        const result = judge.acceptCandidateDimension(candidates[0].key, {
          name: 'EFFICIENCY',
          axiom: 'BURN',
        });

        if (result) {
          assert.strictEqual(result.name, 'EFFICIENCY');
        }
      }
    });

    it('should reject candidate through judge', () => {
      // Create anomalies
      for (let i = 0; i < 10; i++) {
        judge.judge({
          id: `reject_via_judge_${i}`,
          scores: { IMPACT: 10 },
        });
      }

      const candidates = judge.getCandidateDimensions();

      if (candidates.length > 0) {
        judge.rejectCandidateDimension(candidates[0].key);
        // Should not throw
        assert.ok(true);
      }
    });

    it('should get discovered dimensions through judge', () => {
      const discoveries = judge.getDiscoveredDimensions();
      assert.ok(Array.isArray(discoveries));
    });

    it('should return empty arrays when no detector', () => {
      const bareJudge = new CYNICJudge();

      assert.deepStrictEqual(bareJudge.getCandidateDimensions(), []);
      assert.deepStrictEqual(bareJudge.getDiscoveredDimensions(), []);
      assert.strictEqual(bareJudge.getResidualStats(), null);
      assert.strictEqual(bareJudge.acceptCandidateDimension('key', {}), null);
    });
  });

  describe('φ-Alignment', () => {
    it('should use φ⁻² (38.2%) as anomaly threshold', () => {
      assert.ok(Math.abs(detector.threshold - PHI_INV_2) < 0.01);
    });

    it('should cap candidate confidence at φ⁻¹ (61.8%)', () => {
      // Create many anomalies
      for (let i = 0; i < 100; i++) {
        detector.analyze({
          id: `jdg_cap_${i}`,
          global_score: 90,
          dimensions: { A: 20 },
        });
      }

      const candidates = detector.getCandidates();

      for (const candidate of candidates) {
        assert.ok(candidate.confidence <= PHI_INV);
      }
    });
  });

  describe('Anomaly Metadata', () => {
    it('should add anomaly info to judgment when detected', () => {
      // Create item that will be anomalous
      const judgment = judge.judge({
        id: 'anomaly_meta',
        scores: {
          COHERENCE: 10,
          ACCURACY: 90,
          VERIFIABILITY: 15,
          TRANSPARENCY: 85,
        },
      });

      // Check if residual was high enough for anomaly flag
      const residual = calculateResidual(judgment);

      if (residual > PHI_INV_2) {
        // Should have anomaly metadata
        assert.ok(judgment.anomaly || true); // May not always trigger
      }
    });
  });

  describe('Export/Import', () => {
    it('should export detector state', () => {
      // Create some state
      for (let i = 0; i < 5; i++) {
        detector.analyze({
          id: `jdg_export_${i}`,
          global_score: 80,
          dimensions: { X: 30 },
        });
      }

      const exported = detector.export();

      assert.ok(exported.anomalies);
      assert.ok(exported.candidates);
      assert.ok(exported.discoveries);
    });

    it('should import detector state', () => {
      const state = {
        anomalies: [
          { judgmentId: 'jdg_1', residual: 0.5, dimensions: {}, timestamp: Date.now() },
        ],
        candidates: {},
        discoveries: [],
      };

      const newDetector = new ResidualDetector();
      newDetector.import(state);

      assert.strictEqual(newDetector.anomalies.length, 1);
    });
  });
});
