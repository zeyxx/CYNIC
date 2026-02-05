/**
 * Tests for Inference Entropy Module
 *
 * "φ mesure l'incertitude" - κυνικός
 *
 * Information theory utilities: Shannon entropy, KL divergence, etc.
 * Tests for packages/node/src/inference/entropy.js
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { PHI_INV } from '@cynic/core';

import {
  shannonEntropy,
  normalizedEntropy,
  crossEntropy,
  klDivergence,
  jsDivergence,
  scoresToProbabilities,
  entropyConfidence,
  EntropyTracker,
  getEntropyTracker,
  resetEntropyTracker,
} from '../src/inference/entropy.js';

describe('Inference Entropy: Shannon Entropy', () => {
  it('should return 0 for deterministic distribution', () => {
    const probs = [1, 0, 0, 0];
    const entropy = shannonEntropy(probs);
    assert.equal(entropy, 0);
  });

  it('should return log2(n) for uniform distribution', () => {
    const probs = [0.25, 0.25, 0.25, 0.25];
    const entropy = shannonEntropy(probs);
    assert.ok(Math.abs(entropy - 2) < 1e-10); // log2(4) = 2
  });

  it('should return log2(2) = 1 for fair coin', () => {
    const probs = [0.5, 0.5];
    const entropy = shannonEntropy(probs);
    assert.ok(Math.abs(entropy - 1) < 1e-10);
  });

  it('should handle empty array', () => {
    assert.equal(shannonEntropy([]), 0);
    assert.equal(shannonEntropy(null), 0);
  });

  it('should handle zeros in distribution', () => {
    const probs = [0.5, 0, 0.5, 0];
    const entropy = shannonEntropy(probs);
    assert.ok(Math.abs(entropy - 1) < 1e-10); // Same as fair coin
  });

  it('should increase with more uniform distributions', () => {
    const skewed = [0.9, 0.1];
    const uniform = [0.5, 0.5];
    assert.ok(shannonEntropy(uniform) > shannonEntropy(skewed));
  });
});

describe('Inference Entropy: Normalized Entropy', () => {
  it('should return 1 for uniform distribution', () => {
    const probs = [0.25, 0.25, 0.25, 0.25];
    const normalized = normalizedEntropy(probs);
    assert.ok(Math.abs(normalized - 1) < 1e-10);
  });

  it('should return 0 for deterministic distribution', () => {
    const probs = [1, 0, 0];
    const normalized = normalizedEntropy(probs);
    assert.equal(normalized, 0);
  });

  it('should be between 0 and 1', () => {
    const probs = [0.7, 0.2, 0.1];
    const normalized = normalizedEntropy(probs);
    assert.ok(normalized >= 0 && normalized <= 1);
  });

  it('should handle single element', () => {
    assert.equal(normalizedEntropy([1]), 0);
  });

  it('should handle empty array', () => {
    assert.equal(normalizedEntropy([]), 0);
  });
});

describe('Inference Entropy: Cross-Entropy', () => {
  it('should equal entropy when P = Q', () => {
    const p = [0.5, 0.5];
    const ce = crossEntropy(p, p);
    const entropy = shannonEntropy(p);
    assert.ok(Math.abs(ce - entropy) < 1e-10);
  });

  it('should be >= entropy (Gibbs inequality)', () => {
    const p = [0.5, 0.5];
    const q = [0.9, 0.1];
    const ce = crossEntropy(p, q);
    const entropy = shannonEntropy(p);
    assert.ok(ce >= entropy - 1e-10);
  });

  it('should return Infinity when q=0 but p>0', () => {
    const p = [0.5, 0.5];
    const q = [1, 0];
    assert.equal(crossEntropy(p, q), Infinity);
  });

  it('should handle mismatched lengths', () => {
    assert.equal(crossEntropy([0.5, 0.5], [0.5]), Infinity);
  });
});

describe('Inference Entropy: KL Divergence', () => {
  it('should return 0 when P = Q', () => {
    const p = [0.5, 0.5];
    const kl = klDivergence(p, p);
    assert.ok(Math.abs(kl) < 1e-10);
  });

  it('should be >= 0 (always non-negative)', () => {
    const p = [0.3, 0.7];
    const q = [0.6, 0.4];
    const kl = klDivergence(p, q);
    assert.ok(kl >= -1e-10);
  });

  it('should be asymmetric', () => {
    const p = [0.9, 0.1];
    const q = [0.5, 0.5];
    const klPQ = klDivergence(p, q);
    const klQP = klDivergence(q, p);
    assert.notEqual(klPQ, klQP);
  });

  it('should return Infinity when q=0 but p>0', () => {
    const p = [0.5, 0.5];
    const q = [1, 0];
    assert.equal(klDivergence(p, q), Infinity);
  });

  it('should handle p=0 gracefully', () => {
    const p = [0, 1];
    const q = [0.5, 0.5];
    const kl = klDivergence(p, q);
    assert.ok(isFinite(kl));
  });
});

describe('Inference Entropy: JS Divergence', () => {
  it('should return 0 when P = Q', () => {
    const p = [0.5, 0.5];
    const js = jsDivergence(p, p);
    assert.ok(Math.abs(js) < 1e-10);
  });

  it('should be symmetric', () => {
    const p = [0.9, 0.1];
    const q = [0.5, 0.5];
    const jsPQ = jsDivergence(p, q);
    const jsQP = jsDivergence(q, p);
    assert.ok(Math.abs(jsPQ - jsQP) < 1e-10);
  });

  it('should be between 0 and 1', () => {
    const p = [0.9, 0.1];
    const q = [0.1, 0.9];
    const js = jsDivergence(p, q);
    assert.ok(js >= 0 && js <= 1);
  });

  it('should handle extreme distributions', () => {
    const p = [1, 0];
    const q = [0, 1];
    const js = jsDivergence(p, q);
    assert.ok(js > 0 && js <= 1);
  });
});

describe('Inference Entropy: Scores to Probabilities (Softmax)', () => {
  it('should sum to 1', () => {
    const scores = [1, 2, 3, 4];
    const probs = scoresToProbabilities(scores);
    const sum = probs.reduce((a, b) => a + b, 0);
    assert.ok(Math.abs(sum - 1) < 1e-10);
  });

  it('should preserve order', () => {
    const scores = [1, 2, 3];
    const probs = scoresToProbabilities(scores);
    assert.ok(probs[0] < probs[1]);
    assert.ok(probs[1] < probs[2]);
  });

  it('should return uniform for equal scores', () => {
    const scores = [5, 5, 5, 5];
    const probs = scoresToProbabilities(scores);
    for (const p of probs) {
      assert.ok(Math.abs(p - 0.25) < 1e-10);
    }
  });

  it('should handle temperature scaling', () => {
    const scores = [1, 2];
    const probsT1 = scoresToProbabilities(scores, 1.0);
    const probsT2 = scoresToProbabilities(scores, 2.0);
    // Higher temperature = more uniform
    const diffT1 = Math.abs(probsT1[1] - probsT1[0]);
    const diffT2 = Math.abs(probsT2[1] - probsT2[0]);
    assert.ok(diffT2 < diffT1);
  });

  it('should handle empty array', () => {
    assert.deepEqual(scoresToProbabilities([]), []);
  });

  it('should handle large values (numerical stability)', () => {
    const scores = [1000, 1001, 1002];
    const probs = scoresToProbabilities(scores);
    const sum = probs.reduce((a, b) => a + b, 0);
    assert.ok(Math.abs(sum - 1) < 1e-10);
  });
});

describe('Inference Entropy: Entropy Confidence', () => {
  it('should return high confidence for low entropy', () => {
    const scores = [100, 10, 5, 2, 1]; // Very skewed
    const analysis = entropyConfidence(scores);
    assert.ok(analysis.confidence > 0.5);
  });

  it('should return low confidence for high entropy', () => {
    const scores = [50, 50, 50, 50, 50]; // Uniform
    const analysis = entropyConfidence(scores);
    assert.ok(analysis.confidence < 0.2);
  });

  it('FALSIFIABLE: confidence cannot exceed φ⁻¹', () => {
    // Even with extremely skewed distribution
    const scores = [1000, 1, 1, 1, 1];
    const analysis = entropyConfidence(scores);
    assert.ok(analysis.confidence <= PHI_INV + 1e-10);
  });

  it('should include distribution in result', () => {
    const scores = [80, 70, 60];
    const analysis = entropyConfidence(scores);
    assert.ok(Array.isArray(analysis.distribution));
    assert.equal(analysis.distribution.length, 3);
  });

  it('should calculate max entropy correctly', () => {
    const scores = [1, 2, 3, 4, 5];
    const analysis = entropyConfidence(scores);
    assert.ok(Math.abs(analysis.maxEntropy - Math.log2(5)) < 1e-10);
  });

  it('should handle empty scores', () => {
    const analysis = entropyConfidence([]);
    assert.equal(analysis.confidence, 0);
    assert.equal(analysis.entropy, 0);
  });
});

describe('Inference Entropy: EntropyTracker', () => {
  let tracker;

  beforeEach(() => {
    tracker = new EntropyTracker({ windowSize: 10 });
  });

  it('should record entropy observations', () => {
    tracker.record(1.5);
    tracker.record(1.6);
    tracker.record(1.7);

    const summary = tracker.getSummary();
    assert.equal(summary.count, 3);
  });

  it('should calculate average', () => {
    tracker.record(1.0);
    tracker.record(2.0);
    tracker.record(3.0);

    const avg = tracker.getAverage();
    assert.ok(Math.abs(avg - 2.0) < 1e-10);
  });

  it('should detect increasing trend', () => {
    for (let i = 0; i < 10; i++) {
      tracker.record(i * 0.5);
    }
    assert.equal(tracker.getTrend(), 'increasing');
  });

  it('should detect decreasing trend', () => {
    for (let i = 10; i > 0; i--) {
      tracker.record(i * 0.5);
    }
    assert.equal(tracker.getTrend(), 'decreasing');
  });

  it('should detect stable trend', () => {
    for (let i = 0; i < 10; i++) {
      tracker.record(1.5);
    }
    assert.equal(tracker.getTrend(), 'stable');
  });

  it('should respect window size', () => {
    const smallTracker = new EntropyTracker({ windowSize: 3 });
    for (let i = 0; i < 10; i++) {
      smallTracker.record(i);
    }
    assert.equal(smallTracker.getSummary().count, 3);
  });

  it('should reset correctly', () => {
    tracker.record(1.0);
    tracker.record(2.0);
    tracker.reset();
    assert.equal(tracker.getSummary().count, 0);
  });

  it('should return 0 average for empty tracker', () => {
    assert.equal(tracker.getAverage(), 0);
  });

  it('should return stable for insufficient data', () => {
    tracker.record(1.0);
    assert.equal(tracker.getTrend(), 'stable');
  });
});

describe('Inference Entropy: Singleton Tracker', () => {
  afterEach(() => {
    resetEntropyTracker();
  });

  it('should return same instance', () => {
    const a = getEntropyTracker();
    const b = getEntropyTracker();
    assert.equal(a, b);
  });

  it('should reset singleton', () => {
    const a = getEntropyTracker();
    a.record(1.5);
    resetEntropyTracker();
    const b = getEntropyTracker();
    assert.notEqual(a, b);
    assert.equal(b.getSummary().count, 0);
  });
});

describe('Inference Entropy: φ-Alignment', () => {
  it('FALSIFIABLE: all confidence values bounded by φ⁻¹', () => {
    const testCases = [
      [100, 1, 1, 1, 1],      // Extremely skewed
      [50, 50],               // Fair coin
      [33, 33, 34],           // Uniform-ish
      [99, 1],                // Very skewed
      [1, 1, 1, 1, 1, 1, 1],  // Very uniform
    ];

    for (const scores of testCases) {
      const analysis = entropyConfidence(scores);
      assert.ok(
        analysis.confidence <= PHI_INV + 1e-10,
        `Confidence ${analysis.confidence} exceeds φ⁻¹ for ${scores}`
      );
    }
  });

  it('FALSIFIABLE: entropy increases with uncertainty', () => {
    const certain = entropyConfidence([100, 1, 1]);
    const uncertain = entropyConfidence([33, 33, 34]);

    assert.ok(
      uncertain.entropy > certain.entropy,
      'More uniform distribution should have higher entropy'
    );
  });

  it('FALSIFIABLE: KL divergence is non-negative', () => {
    const distributions = [
      [[0.5, 0.5], [0.9, 0.1]],
      [[0.25, 0.25, 0.25, 0.25], [0.1, 0.2, 0.3, 0.4]],
      [[0.7, 0.3], [0.3, 0.7]],
    ];

    for (const [p, q] of distributions) {
      const kl = klDivergence(p, q);
      assert.ok(
        kl >= -1e-10,
        `KL divergence ${kl} is negative for P=${p}, Q=${q}`
      );
    }
  });
});

describe('Inference Entropy: Edge Cases', () => {
  it('should handle single probability', () => {
    assert.equal(shannonEntropy([1]), 0);
    assert.equal(normalizedEntropy([1]), 0);
  });

  it('should handle very small probabilities', () => {
    const probs = [0.999, 0.001];
    const entropy = shannonEntropy(probs);
    assert.ok(isFinite(entropy));
    assert.ok(entropy > 0);
  });

  it('should handle numerical precision', () => {
    // Probabilities that almost sum to 1
    const probs = [0.33333333, 0.33333333, 0.33333334];
    const entropy = shannonEntropy(probs);
    assert.ok(isFinite(entropy));
  });
});
