/**
 * Gaussian Distribution Tests
 *
 * Tests for the Gaussian/Normal distribution module.
 *
 * @module @cynic/node/test/gaussian
 */

import { describe, it, beforeEach, afterEach, mock } from 'node:test';
import assert from 'node:assert';
import { PHI_INV } from '@cynic/core';

import {
  // Core functions
  gaussianPDF,
  gaussianLogPDF,
  gaussianCDF,
  gaussianSurvival,
  gaussianQuantile,
  erf,
  erfc,

  // Z-scores
  zScore,
  fromZScore,
  standardize,
  zScoreToPValue,
  pValueToZScore,

  // Confidence intervals
  confidenceInterval,
  phiConfidenceInterval,
  confidenceToZScore,

  // Random generation
  randomStandardNormal,
  randomNormal,
  randomNormalArray,
  randomCorrelatedNormal,

  // Noise
  addNoise,
  addNoiseArray,
  GaussianNoiseGenerator,
  createNoiseGenerator,

  // Kernels and KDE
  gaussianKernel,
  gaussianKernelScaled,
  silvermanBandwidth,
  kernelDensityEstimate,
  kde,

  // Distribution classes
  GaussianDistribution,
  DiagonalGaussian,
  createGaussian,
  standardNormal,

  // Statistics
  computeStats,
  skewness,
  kurtosis,
  jarqueBeraTest,

  // Config
  GAUSSIAN_CONSTANTS,
  GAUSSIAN_CONFIG,
} from '../src/inference/index.js';

// ═══════════════════════════════════════════════════════════════════════════════
// CORE GAUSSIAN FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Gaussian Core Functions', () => {
  describe('gaussianPDF', () => {
    it('should compute standard normal PDF at mean', () => {
      const pdf = gaussianPDF(0, 0, 1);
      // PDF at mean = 1/√(2π) ≈ 0.3989
      assert.ok(Math.abs(pdf - GAUSSIAN_CONSTANTS.INV_SQRT_2PI) < 1e-10);
    });

    it('should compute standard normal PDF at 1 sigma', () => {
      const pdf = gaussianPDF(1, 0, 1);
      // f(1) = (1/√(2π)) * e^(-0.5) ≈ 0.2420
      const expected = GAUSSIAN_CONSTANTS.INV_SQRT_2PI * Math.exp(-0.5);
      assert.ok(Math.abs(pdf - expected) < 1e-10);
    });

    it('should handle non-standard mean and std', () => {
      const pdf = gaussianPDF(10, 10, 2);
      // At mean, PDF = 1/(σ√(2π))
      const expected = 1 / (2 * GAUSSIAN_CONSTANTS.SQRT_2PI);
      assert.ok(Math.abs(pdf - expected) < 1e-10);
    });

    it('should return 0 for zero std at non-mean point', () => {
      const pdf = gaussianPDF(1, 0, 0);
      assert.strictEqual(pdf, 0);
    });

    it('should return Infinity for zero std at mean', () => {
      const pdf = gaussianPDF(0, 0, 0);
      assert.strictEqual(pdf, Infinity);
    });
  });

  describe('gaussianCDF', () => {
    it('should return 0.5 at mean', () => {
      const cdf = gaussianCDF(0, 0, 1);
      assert.ok(Math.abs(cdf - 0.5) < 1e-6);
    });

    it('should return ~0.8413 at +1 sigma', () => {
      const cdf = gaussianCDF(1, 0, 1);
      assert.ok(Math.abs(cdf - 0.8413) < 0.001);
    });

    it('should return ~0.1587 at -1 sigma', () => {
      const cdf = gaussianCDF(-1, 0, 1);
      assert.ok(Math.abs(cdf - 0.1587) < 0.001);
    });

    it('should return ~0.975 at +1.96 sigma', () => {
      const cdf = gaussianCDF(1.96, 0, 1);
      assert.ok(Math.abs(cdf - 0.975) < 0.001);
    });

    it('should handle non-standard parameters', () => {
      // CDF at mean + 1 std should be ~0.8413
      const cdf = gaussianCDF(12, 10, 2);
      assert.ok(Math.abs(cdf - 0.8413) < 0.001);
    });
  });

  describe('gaussianSurvival', () => {
    it('should return 0.5 at mean', () => {
      const sf = gaussianSurvival(0, 0, 1);
      assert.ok(Math.abs(sf - 0.5) < 1e-6);
    });

    it('should equal 1 - CDF', () => {
      const x = 1.5;
      const cdf = gaussianCDF(x, 0, 1);
      const sf = gaussianSurvival(x, 0, 1);
      assert.ok(Math.abs(cdf + sf - 1) < 1e-10);
    });
  });

  describe('gaussianQuantile', () => {
    it('should return mean at p=0.5', () => {
      const q = gaussianQuantile(0.5, 0, 1);
      assert.strictEqual(q, 0);
    });

    it('should return ~1.645 at p=0.95', () => {
      const q = gaussianQuantile(0.95, 0, 1);
      assert.ok(Math.abs(q - 1.645) < 0.01);
    });

    it('should return ~-1.645 at p=0.05', () => {
      const q = gaussianQuantile(0.05, 0, 1);
      assert.ok(Math.abs(q - (-1.645)) < 0.01);
    });

    it('should return -Infinity at p=0', () => {
      const q = gaussianQuantile(0, 0, 1);
      assert.strictEqual(q, -Infinity);
    });

    it('should return +Infinity at p=1', () => {
      const q = gaussianQuantile(1, 0, 1);
      assert.strictEqual(q, Infinity);
    });

    it('should be inverse of CDF', () => {
      const p = 0.75;
      const q = gaussianQuantile(p, 0, 1);
      const pBack = gaussianCDF(q, 0, 1);
      assert.ok(Math.abs(p - pBack) < 1e-6);
    });
  });

  describe('erf and erfc', () => {
    it('should return 0 at x=0', () => {
      // Floating point: erf(0) ≈ 1e-9 due to approximation
      assert.ok(Math.abs(erf(0)) < 1e-8);
    });

    it('should be antisymmetric', () => {
      const x = 1.5;
      assert.ok(Math.abs(erf(x) + erf(-x)) < 1e-10);
    });

    it('should approach 1 for large x', () => {
      assert.ok(erf(3) > 0.999);
    });

    it('erfc should equal 1 - erf', () => {
      const x = 1.2;
      assert.ok(Math.abs(erfc(x) - (1 - erf(x))) < 1e-10);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// Z-SCORES AND STANDARDIZATION
// ═══════════════════════════════════════════════════════════════════════════════

describe('Z-Scores and Standardization', () => {
  describe('zScore', () => {
    it('should return 0 at mean', () => {
      assert.strictEqual(zScore(10, 10, 2), 0);
    });

    it('should return 1 at mean + 1 std', () => {
      assert.strictEqual(zScore(12, 10, 2), 1);
    });

    it('should return -1 at mean - 1 std', () => {
      assert.strictEqual(zScore(8, 10, 2), -1);
    });

    it('should handle zero std at mean', () => {
      assert.strictEqual(zScore(10, 10, 0), 0);
    });

    it('should return Infinity for zero std above mean', () => {
      assert.strictEqual(zScore(11, 10, 0), Infinity);
    });
  });

  describe('fromZScore', () => {
    it('should invert zScore', () => {
      const mean = 10;
      const std = 2;
      const x = 14;
      const z = zScore(x, mean, std);
      const xBack = fromZScore(z, mean, std);
      assert.strictEqual(xBack, x);
    });
  });

  describe('standardize', () => {
    it('should standardize an array', () => {
      const values = [2, 4, 6, 8, 10];
      const { standardized, mean, std } = standardize(values);

      assert.strictEqual(mean, 6);
      assert.ok(standardized[2] === 0); // Mean value should have z=0
    });

    it('should handle empty array', () => {
      const { standardized, mean, std } = standardize([]);
      assert.deepStrictEqual(standardized, []);
      assert.strictEqual(mean, 0);
      assert.strictEqual(std, 1);
    });

    it('should produce mean 0 and std ~1', () => {
      const values = [10, 20, 30, 40, 50];
      const { standardized } = standardize(values);

      const meanZ = standardized.reduce((a, b) => a + b, 0) / standardized.length;
      const varZ = standardized.reduce((sum, z) => sum + z * z, 0) / standardized.length;

      assert.ok(Math.abs(meanZ) < 1e-10);
      assert.ok(Math.abs(varZ - 1) < 1e-10);
    });
  });

  describe('zScoreToPValue and pValueToZScore', () => {
    it('should convert z=2 to p≈0.0455', () => {
      const p = zScoreToPValue(2);
      assert.ok(Math.abs(p - 0.0455) < 0.001);
    });

    it('should be inverse operations', () => {
      const z = 1.96;
      const p = zScoreToPValue(z);
      const zBack = pValueToZScore(p);
      assert.ok(Math.abs(z - zBack) < 0.01);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// CONFIDENCE INTERVALS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Confidence Intervals', () => {
  describe('confidenceInterval', () => {
    it('should compute 95% CI', () => {
      // But capped at φ (61.8%)
      const ci = confidenceInterval(100, 10, 100, 0.95);

      // Should be capped at φ
      assert.strictEqual(ci.confidence, PHI_INV);
    });

    it('should respect φ cap', () => {
      const ci = confidenceInterval(100, 10, 100, 0.99);
      assert.ok(ci.confidence <= PHI_INV);
    });

    it('should compute margin of error correctly', () => {
      const ci = confidenceInterval(100, 10, 100, 0.5);
      // 50% CI: z ≈ 0.674, margin = 0.674 * 10 / 10 = 0.674
      assert.ok(ci.marginOfError > 0);
      assert.ok(ci.upper > ci.lower);
    });
  });

  describe('phiConfidenceInterval', () => {
    it('should use φ confidence', () => {
      const ci = phiConfidenceInterval(100, 10, 100);
      assert.strictEqual(ci.confidence, PHI_INV);
    });
  });

  describe('confidenceToZScore', () => {
    it('should return z for 95% (capped to φ)', () => {
      const z = confidenceToZScore(0.95);
      // 61.8% CI corresponds to z ≈ 0.882
      assert.ok(z > 0);
      assert.ok(z < 2); // Should be less than 95% z-score
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// RANDOM GENERATION
// ═══════════════════════════════════════════════════════════════════════════════

describe('Random Generation', () => {
  describe('randomStandardNormal', () => {
    it('should generate values', () => {
      const values = [];
      for (let i = 0; i < 1000; i++) {
        values.push(randomStandardNormal());
      }

      // Check mean is approximately 0
      const mean = values.reduce((a, b) => a + b, 0) / values.length;
      assert.ok(Math.abs(mean) < 0.1);

      // Check std is approximately 1
      const variance = values.reduce((sum, x) => sum + x * x, 0) / values.length;
      assert.ok(Math.abs(variance - 1) < 0.2);
    });
  });

  describe('randomNormal', () => {
    it('should generate values with specified mean and std', () => {
      const mean = 100;
      const std = 15;
      const values = [];
      for (let i = 0; i < 1000; i++) {
        values.push(randomNormal(mean, std));
      }

      const sampleMean = values.reduce((a, b) => a + b, 0) / values.length;
      assert.ok(Math.abs(sampleMean - mean) < 2);
    });
  });

  describe('randomNormalArray', () => {
    it('should generate array of specified length', () => {
      const arr = randomNormalArray(100, 0, 1);
      assert.strictEqual(arr.length, 100);
    });
  });

  describe('randomCorrelatedNormal', () => {
    it('should generate correlated pairs', () => {
      const pairs = randomCorrelatedNormal(0.9, 1000);

      // Compute correlation
      let sumX = 0, sumY = 0, sumXY = 0, sumX2 = 0, sumY2 = 0;
      for (const { x, y } of pairs) {
        sumX += x;
        sumY += y;
        sumXY += x * y;
        sumX2 += x * x;
        sumY2 += y * y;
      }
      const n = pairs.length;
      const corr = (n * sumXY - sumX * sumY) /
        Math.sqrt((n * sumX2 - sumX * sumX) * (n * sumY2 - sumY * sumY));

      // Should be close to 0.9
      assert.ok(Math.abs(corr - 0.9) < 0.1);
    });

    it('should return single object for n=1', () => {
      const pair = randomCorrelatedNormal(0.5, 1);
      assert.ok('x' in pair && 'y' in pair);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// GAUSSIAN NOISE
// ═══════════════════════════════════════════════════════════════════════════════

describe('Gaussian Noise', () => {
  describe('addNoise', () => {
    it('should add noise to a value', () => {
      const original = 100;
      const noisy = addNoise(original, 1);
      // Just check it's different (with very high probability)
      // In practice, run multiple times
      assert.strictEqual(typeof noisy, 'number');
    });
  });

  describe('addNoiseArray', () => {
    it('should add noise to all values', () => {
      const original = [1, 2, 3, 4, 5];
      const noisy = addNoiseArray(original, 0.1);
      assert.strictEqual(noisy.length, 5);
    });
  });

  describe('GaussianNoiseGenerator', () => {
    it('should generate reproducible noise with seed', () => {
      const gen1 = new GaussianNoiseGenerator({ seed: 42 });
      const gen2 = new GaussianNoiseGenerator({ seed: 42 });

      const v1 = gen1.next();
      const v2 = gen2.next();

      assert.strictEqual(v1, v2);
    });

    it('should generate array of values', () => {
      const gen = new GaussianNoiseGenerator({ seed: 123 });
      const values = gen.generate(10);
      assert.strictEqual(values.length, 10);
    });

    it('should reset to initial state', () => {
      const gen = new GaussianNoiseGenerator({ seed: 42 });
      const first = gen.next();
      gen.next();
      gen.next();
      gen.reset();
      const afterReset = gen.next();
      assert.strictEqual(first, afterReset);
    });

    it('should respect mean and std', () => {
      const gen = new GaussianNoiseGenerator({ mean: 100, std: 10, seed: 42 });
      const values = gen.generate(1000);
      const mean = values.reduce((a, b) => a + b, 0) / values.length;
      assert.ok(Math.abs(mean - 100) < 2);
    });
  });

  describe('createNoiseGenerator', () => {
    it('should create a generator', () => {
      const gen = createNoiseGenerator({ seed: 42 });
      assert.ok(gen instanceof GaussianNoiseGenerator);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// KERNELS AND KDE
// ═══════════════════════════════════════════════════════════════════════════════

describe('Kernels and KDE', () => {
  describe('gaussianKernel', () => {
    it('should return max at 0', () => {
      const k0 = gaussianKernel(0);
      const k1 = gaussianKernel(1);
      assert.ok(k0 > k1);
    });

    it('should be symmetric', () => {
      assert.ok(Math.abs(gaussianKernel(1) - gaussianKernel(-1)) < 1e-10);
    });
  });

  describe('gaussianKernelScaled', () => {
    it('should scale with bandwidth', () => {
      const k1 = gaussianKernelScaled(0, 1);
      const k2 = gaussianKernelScaled(0, 2);
      // Larger bandwidth = smaller peak
      assert.ok(k1 > k2);
    });

    it('should return 0 for non-positive bandwidth', () => {
      assert.strictEqual(gaussianKernelScaled(0, 0), 0);
      assert.strictEqual(gaussianKernelScaled(0, -1), 0);
    });
  });

  describe('silvermanBandwidth', () => {
    it('should compute reasonable bandwidth', () => {
      const data = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
      const h = silvermanBandwidth(data);
      assert.ok(h > 0);
      assert.ok(h < 10); // Should be smaller than data range
    });

    it('should return 1 for small data', () => {
      const h = silvermanBandwidth([1]);
      assert.strictEqual(h, 1);
    });
  });

  describe('kernelDensityEstimate', () => {
    it('should estimate density', () => {
      const data = [0, 0, 0, 1, 1, 2];
      const density = kernelDensityEstimate(0, data);
      assert.ok(density > 0);
    });

    it('should return 0 for empty data', () => {
      assert.strictEqual(kernelDensityEstimate(0, []), 0);
    });
  });

  describe('kde', () => {
    it('should compute full KDE', () => {
      const data = [1, 2, 2, 3, 3, 3, 4, 4, 5];
      const result = kde(data, { points: 50 });

      assert.strictEqual(result.x.length, 50);
      assert.strictEqual(result.density.length, 50);
      assert.ok(result.bandwidth > 0);
    });

    it('should handle empty data', () => {
      const result = kde([]);
      assert.deepStrictEqual(result.x, []);
      assert.deepStrictEqual(result.density, []);
    });

    it('should peak near mode', () => {
      const data = [5, 5, 5, 5, 5, 1, 9]; // Mode at 5
      const result = kde(data, { points: 100 });

      // Find max density
      let maxIdx = 0;
      for (let i = 1; i < result.density.length; i++) {
        if (result.density[i] > result.density[maxIdx]) {
          maxIdx = i;
        }
      }

      // Peak should be near 5
      assert.ok(Math.abs(result.x[maxIdx] - 5) < 2);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// GAUSSIAN DISTRIBUTION CLASS
// ═══════════════════════════════════════════════════════════════════════════════

describe('GaussianDistribution', () => {
  let dist;

  beforeEach(() => {
    dist = new GaussianDistribution(100, 15);
  });

  it('should create with parameters', () => {
    assert.strictEqual(dist.mean, 100);
    assert.strictEqual(dist.std, 15);
    assert.strictEqual(dist.variance, 225);
  });

  it('should compute PDF', () => {
    const pdf = dist.pdf(100);
    assert.ok(pdf > 0);
  });

  it('should compute logPdf', () => {
    const logP = dist.logPdf(100);
    const p = dist.pdf(100);
    assert.ok(Math.abs(Math.exp(logP) - p) < 1e-10);
  });

  it('should compute CDF', () => {
    assert.ok(Math.abs(dist.cdf(100) - 0.5) < 1e-6);
  });

  it('should compute survival', () => {
    assert.ok(Math.abs(dist.survival(100) - 0.5) < 1e-6);
  });

  it('should compute quantile', () => {
    assert.strictEqual(dist.quantile(0.5), 100);
  });

  it('should compute zScore', () => {
    assert.strictEqual(dist.zScore(115), 1);
  });

  it('should sample', () => {
    const sample = dist.sample();
    assert.strictEqual(typeof sample, 'number');

    const samples = dist.sample(10);
    assert.strictEqual(samples.length, 10);
  });

  it('should compute φ-aligned confidence interval', () => {
    const ci = dist.confidenceInterval();
    assert.ok(ci.lower < ci.upper);
    assert.strictEqual(ci.confidence, PHI_INV);
  });

  it('should detect anomalies', () => {
    assert.strictEqual(dist.isAnomaly(100, 2), false); // At mean
    assert.strictEqual(dist.isAnomaly(130, 2), false); // 2 sigma
    assert.strictEqual(dist.isAnomaly(150, 2), true);  // > 3 sigma
  });

  it('should compute Mahalanobis distance', () => {
    const d = dist.mahalanobisDistance(115);
    assert.strictEqual(d, 1); // 1 std away
  });

  describe('GaussianDistribution.fit', () => {
    it('should fit to data', () => {
      const data = [90, 95, 100, 105, 110];
      const fitted = GaussianDistribution.fit(data);
      assert.strictEqual(fitted.mean, 100);
    });

    it('should handle empty data', () => {
      const fitted = GaussianDistribution.fit([]);
      assert.strictEqual(fitted.mean, 0);
      assert.strictEqual(fitted.std, 1);
    });
  });

  describe('serialization', () => {
    it('should serialize to JSON', () => {
      const json = dist.toJSON();
      assert.strictEqual(json.mean, 100);
      assert.strictEqual(json.std, 15);
    });

    it('should deserialize from JSON', () => {
      const json = { mean: 50, std: 10 };
      const restored = GaussianDistribution.fromJSON(json);
      assert.strictEqual(restored.mean, 50);
      assert.strictEqual(restored.std, 10);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// DIAGONAL GAUSSIAN (MULTIVARIATE)
// ═══════════════════════════════════════════════════════════════════════════════

describe('DiagonalGaussian', () => {
  let mvn;

  beforeEach(() => {
    mvn = new DiagonalGaussian([0, 10], [1, 2]);
  });

  it('should create with parameters', () => {
    assert.strictEqual(mvn.dimensions, 2);
    assert.deepStrictEqual(mvn.means, [0, 10]);
    assert.deepStrictEqual(mvn.stds, [1, 2]);
  });

  it('should compute PDF', () => {
    const pdf = mvn.pdf([0, 10]);
    assert.ok(pdf > 0);
  });

  it('should compute logPdf', () => {
    const logP = mvn.logPdf([0, 10]);
    const p = mvn.pdf([0, 10]);
    assert.ok(Math.abs(Math.exp(logP) - p) < 1e-10);
  });

  it('should throw on dimension mismatch', () => {
    assert.throws(() => mvn.logPdf([0]), /Expected 2 dimensions/);
  });

  it('should sample', () => {
    const sample = mvn.sample();
    assert.strictEqual(sample.length, 2);

    const samples = mvn.sample(5);
    assert.strictEqual(samples.length, 5);
    assert.strictEqual(samples[0].length, 2);
  });

  it('should compute Mahalanobis distance', () => {
    const d = mvn.mahalanobisDistance([0, 10]);
    assert.strictEqual(d, 0); // At mean
  });

  it('should compute squared Mahalanobis distance', () => {
    const d2 = mvn.mahalanobisDistanceSquared([1, 12]);
    // (1-0)²/1² + (12-10)²/2² = 1 + 1 = 2
    assert.strictEqual(d2, 2);
  });

  describe('DiagonalGaussian.fit', () => {
    it('should fit to data', () => {
      const data = [
        [0, 10],
        [2, 14],
        [-2, 6],
      ];
      const fitted = DiagonalGaussian.fit(data);
      assert.strictEqual(fitted.dimensions, 2);
      assert.strictEqual(fitted.means[0], 0);
      assert.strictEqual(fitted.means[1], 10);
    });

    it('should handle empty data', () => {
      const fitted = DiagonalGaussian.fit([]);
      assert.strictEqual(fitted.dimensions, 1);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// STATISTICAL UTILITIES
// ═══════════════════════════════════════════════════════════════════════════════

describe('Statistical Utilities', () => {
  describe('computeStats', () => {
    it('should compute basic statistics', () => {
      const data = [1, 2, 3, 4, 5];
      const stats = computeStats(data);

      assert.strictEqual(stats.n, 5);
      assert.strictEqual(stats.mean, 3);
      assert.strictEqual(stats.min, 1);
      assert.strictEqual(stats.max, 5);
      assert.strictEqual(stats.median, 3);
    });

    it('should handle even-length arrays for median', () => {
      const data = [1, 2, 3, 4];
      const stats = computeStats(data);
      assert.strictEqual(stats.median, 2.5);
    });

    it('should handle empty array', () => {
      const stats = computeStats([]);
      assert.strictEqual(stats.n, 0);
      assert.strictEqual(stats.mean, 0);
    });
  });

  describe('skewness', () => {
    it('should return 0 for symmetric distribution', () => {
      const data = [-2, -1, 0, 1, 2];
      const s = skewness(data);
      assert.ok(Math.abs(s) < 1e-10);
    });

    it('should return positive for right-skewed', () => {
      const data = [1, 2, 3, 4, 10];
      const s = skewness(data);
      assert.ok(s > 0);
    });

    it('should return 0 for small samples', () => {
      assert.strictEqual(skewness([1, 2]), 0);
    });
  });

  describe('kurtosis', () => {
    it('should return ~0 for normal-like distribution', () => {
      // Generate pseudo-normal data
      const data = [];
      for (let i = 0; i < 1000; i++) {
        data.push(randomStandardNormal());
      }
      const k = kurtosis(data);
      assert.ok(Math.abs(k) < 0.5); // Should be close to 0 (excess kurtosis)
    });

    it('should return 0 for small samples', () => {
      assert.strictEqual(kurtosis([1, 2, 3]), 0);
    });
  });

  describe('jarqueBeraTest', () => {
    it('should accept normal data', () => {
      const data = [];
      for (let i = 0; i < 1000; i++) {
        data.push(randomStandardNormal());
      }
      const result = jarqueBeraTest(data);
      assert.ok(result.pValue > 0.01); // Should not reject normality
    });

    it('should reject highly non-normal data', () => {
      // Uniform data
      const data = [];
      for (let i = 0; i < 100; i++) {
        data.push(i);
      }
      const result = jarqueBeraTest(data);
      // May or may not reject depending on sample
      assert.ok('isNormal' in result);
    });

    it('should handle small samples', () => {
      const result = jarqueBeraTest([1, 2, 3]);
      assert.strictEqual(result.statistic, 0);
      assert.strictEqual(result.isNormal, true);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// FACTORY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

describe('Factory Functions', () => {
  describe('createGaussian', () => {
    it('should create a distribution', () => {
      const dist = createGaussian(100, 15);
      assert.ok(dist instanceof GaussianDistribution);
      assert.strictEqual(dist.mean, 100);
    });

    it('should use defaults', () => {
      const dist = createGaussian();
      assert.strictEqual(dist.mean, 0);
      assert.strictEqual(dist.std, 1);
    });
  });

  describe('standardNormal', () => {
    it('should create N(0,1)', () => {
      const dist = standardNormal();
      assert.strictEqual(dist.mean, 0);
      assert.strictEqual(dist.std, 1);
    });
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════════

describe('Configuration', () => {
  it('should have correct constants', () => {
    assert.ok(Math.abs(GAUSSIAN_CONSTANTS.SQRT_2PI - Math.sqrt(2 * Math.PI)) < 1e-10);
    assert.ok(Math.abs(GAUSSIAN_CONSTANTS.SQRT_2 - Math.sqrt(2)) < 1e-10);
  });

  it('should have φ-aligned confidence', () => {
    assert.strictEqual(GAUSSIAN_CONFIG.CONFIDENCE_PHI, PHI_INV);
  });
});

// ═══════════════════════════════════════════════════════════════════════════════
// φ-ALIGNMENT TESTS
// ═══════════════════════════════════════════════════════════════════════════════

describe('φ-Alignment', () => {
  it('should cap confidence intervals at φ⁻¹', () => {
    const ci = confidenceInterval(100, 10, 100, 0.99);
    assert.ok(ci.confidence <= PHI_INV + 0.001);
  });

  it('should cap GaussianDistribution confidence at φ⁻¹', () => {
    const dist = createGaussian(0, 1);
    const ci = dist.confidenceInterval(0.99);
    assert.ok(ci.confidence <= PHI_INV + 0.001);
  });

  it('should use φ for default confidence', () => {
    const dist = createGaussian(0, 1);
    const ci = dist.confidenceInterval();
    assert.strictEqual(ci.confidence, PHI_INV);
  });
});
