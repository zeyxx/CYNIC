/**
 * Tests for Shannon Entropy in Judgments
 * "L'incertitude est information" - κυνικός
 */

import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import {
  shannonEntropy,
  normalizedEntropy,
  scoresToProbabilities,
  judgmentEntropy,
  informationGain,
  adjustConfidenceByEntropy,
  optimalConfidence,
  ENTROPY_THRESHOLDS,
} from '../src/judge/entropy.js';
import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

describe('Shannon Entropy', () => {
  describe('shannonEntropy', () => {
    it('should return 0 for single certain outcome', () => {
      const H = shannonEntropy([1]);
      assert.strictEqual(H, 0);
    });

    it('should return 1 bit for fair coin', () => {
      const H = shannonEntropy([0.5, 0.5]);
      assert.ok(Math.abs(H - 1) < 0.001, `Expected ~1, got ${H}`);
    });

    it('should return log2(n) for uniform distribution', () => {
      const n = 4;
      const uniform = Array(n).fill(1 / n);
      const H = shannonEntropy(uniform);
      const expected = Math.log2(n);
      assert.ok(Math.abs(H - expected) < 0.001, `Expected ${expected}, got ${H}`);
    });

    it('should handle skewed distribution', () => {
      // [0.9, 0.1] should have low entropy
      const H = shannonEntropy([0.9, 0.1]);
      assert.ok(H < 1, `Expected < 1 bit, got ${H}`);
      assert.ok(H > 0, `Expected > 0 bits, got ${H}`);
    });
  });

  describe('normalizedEntropy', () => {
    it('should return 1 for uniform distribution', () => {
      const uniform = [0.25, 0.25, 0.25, 0.25];
      const Hnorm = normalizedEntropy(uniform);
      assert.ok(Math.abs(Hnorm - 1) < 0.001, `Expected 1, got ${Hnorm}`);
    });

    it('should return 0 for single element', () => {
      const Hnorm = normalizedEntropy([1]);
      assert.strictEqual(Hnorm, 0);
    });

    it('should be between 0 and 1', () => {
      const Hnorm = normalizedEntropy([0.7, 0.2, 0.1]);
      assert.ok(Hnorm >= 0 && Hnorm <= 1, `Expected [0,1], got ${Hnorm}`);
    });
  });

  describe('scoresToProbabilities', () => {
    it('should normalize scores to sum to 1', () => {
      const probs = scoresToProbabilities([80, 60, 40, 20]);
      const sum = probs.reduce((a, b) => a + b, 0);
      assert.ok(Math.abs(sum - 1) < 0.001, `Expected sum=1, got ${sum}`);
    });

    it('should handle uniform scores', () => {
      const probs = scoresToProbabilities([50, 50, 50, 50]);
      probs.forEach(p => {
        assert.ok(Math.abs(p - 0.25) < 0.001, `Expected 0.25, got ${p}`);
      });
    });

    it('should handle all zeros as uniform', () => {
      const probs = scoresToProbabilities([0, 0, 0, 0]);
      probs.forEach(p => {
        assert.strictEqual(p, 0.25);
      });
    });
  });
});

describe('Judgment Entropy', () => {
  describe('judgmentEntropy', () => {
    it('should detect DECISIVE category for polarized scores', () => {
      const scores = { PHI: 95, VERIFY: 20, CULTURE: 15, BURN: 10 };
      const result = judgmentEntropy(scores);

      // Very polarized = low entropy = DECISIVE
      assert.ok(result.normalizedEntropy < PHI_INV_2, `Expected H < 38.2%, got ${result.normalizedEntropy}`);
    });

    it('should detect CHAOTIC category for uniform scores', () => {
      const scores = { PHI: 50, VERIFY: 50, CULTURE: 50, BURN: 50 };
      const result = judgmentEntropy(scores);

      // Uniform = max entropy = CHAOTIC
      assert.strictEqual(result.category, 'CHAOTIC');
      assert.ok(result.shouldTriggerConsensus, 'Should trigger consensus');
    });

    it('should include confidence in result', () => {
      const scores = { PHI: 80, VERIFY: 60, CULTURE: 70, BURN: 50 };
      const result = judgmentEntropy(scores);

      assert.ok(result.confidence > 0, 'Should have positive confidence');
      assert.ok(result.confidence <= PHI_INV, `Confidence should be ≤ φ⁻¹, got ${result.confidence}`);
    });

    it('should return probabilities for each axiom', () => {
      const scores = { PHI: 80, VERIFY: 20 };
      const result = judgmentEntropy(scores);

      assert.ok(result.probabilities.PHI > result.probabilities.VERIFY);
      const sum = Object.values(result.probabilities).reduce((a, b) => a + b, 0);
      assert.ok(Math.abs(sum - 1) < 0.001, `Probabilities should sum to 1, got ${sum}`);
    });
  });

  describe('informationGain', () => {
    it('should be positive when entropy decreases', () => {
      const before = { PHI: 50, VERIFY: 50, CULTURE: 50 };
      const after = { PHI: 90, VERIFY: 30, CULTURE: 20 };

      const IG = informationGain(before, after);
      assert.ok(IG > 0, `Expected positive IG, got ${IG}`);
    });

    it('should be negative when entropy increases', () => {
      const before = { PHI: 90, VERIFY: 30, CULTURE: 20 };
      const after = { PHI: 50, VERIFY: 50, CULTURE: 50 };

      const IG = informationGain(before, after);
      assert.ok(IG < 0, `Expected negative IG, got ${IG}`);
    });

    it('should be zero for identical distributions', () => {
      const scores = { PHI: 70, VERIFY: 60, CULTURE: 50 };
      const IG = informationGain(scores, scores);
      assert.ok(Math.abs(IG) < 0.001, `Expected IG=0, got ${IG}`);
    });
  });
});

describe('Confidence Adjustment', () => {
  describe('adjustConfidenceByEntropy', () => {
    it('should not exceed φ⁻¹', () => {
      const adjusted = adjustConfidenceByEntropy(1.0, 0);
      assert.ok(adjusted <= PHI_INV, `Expected ≤ ${PHI_INV}, got ${adjusted}`);
    });

    it('should reduce confidence with high entropy', () => {
      const lowEntropy = adjustConfidenceByEntropy(0.5, 0.2);
      const highEntropy = adjustConfidenceByEntropy(0.5, 0.8);

      assert.ok(highEntropy < lowEntropy, 'High entropy should reduce confidence');
    });

    it('should maintain minimum confidence at φ⁻³', () => {
      const adjusted = adjustConfidenceByEntropy(0.1, 1.0);
      assert.ok(adjusted >= PHI_INV_3 * 0.1, `Expected ≥ ${PHI_INV_3 * 0.1}, got ${adjusted}`);
    });
  });

  describe('optimalConfidence', () => {
    it('should return higher confidence for decisive judgments', () => {
      const decisive = { PHI: 95, VERIFY: 20, CULTURE: 15, BURN: 10 };
      const chaotic = { PHI: 50, VERIFY: 50, CULTURE: 50, BURN: 50 };

      const confDecisive = optimalConfidence(75, decisive);
      const confChaotic = optimalConfidence(75, chaotic);

      assert.ok(confDecisive > confChaotic, 'Decisive should have higher confidence');
    });

    it('should never exceed φ⁻¹', () => {
      const scores = { PHI: 100, VERIFY: 100, CULTURE: 100, BURN: 100 };
      const conf = optimalConfidence(100, scores);

      assert.ok(conf <= PHI_INV, `Expected ≤ ${PHI_INV}, got ${conf}`);
    });

    it('should scale with Q-Score', () => {
      const scores = { PHI: 70, VERIFY: 60, CULTURE: 65, BURN: 55 };

      const confHigh = optimalConfidence(80, scores);
      const confLow = optimalConfidence(40, scores);

      assert.ok(confHigh > confLow, 'Higher Q-Score should give higher confidence');
    });
  });
});

describe('φ-Alignment', () => {
  it('should use φ-based thresholds', () => {
    assert.strictEqual(ENTROPY_THRESHOLDS.DECISIVE, PHI_INV_3);
    assert.strictEqual(ENTROPY_THRESHOLDS.MODERATE, PHI_INV_2);
    assert.strictEqual(ENTROPY_THRESHOLDS.UNCERTAIN, PHI_INV);
  });

  it('should trigger consensus at φ⁻¹ entropy threshold', () => {
    // Scores that should give entropy > 61.8%
    const uniform = { PHI: 50, VERIFY: 50, CULTURE: 50, BURN: 50, META: 50 };
    const result = judgmentEntropy(uniform);

    assert.ok(result.normalizedEntropy > PHI_INV, 'Uniform scores should exceed φ⁻¹ entropy');
    assert.ok(result.shouldTriggerConsensus, 'Should trigger consensus');
  });
});
