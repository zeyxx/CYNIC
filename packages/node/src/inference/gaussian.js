/**
 * Gaussian (Normal) Distribution
 *
 * "Le bruit cache le signal, mais le révèle aussi" - κυνικός
 *
 * f(x) = (1/√(2πσ²)) × e^(-(x-μ)²/(2σ²))
 *
 * Uses Gaussian distribution for:
 * - Statistical inference (confidence intervals, z-scores)
 * - Noise generation (controlled randomness)
 * - Kernel density estimation (smoothing)
 * - Anomaly scoring (Mahalanobis distance)
 * - Central limit theorem applications
 *
 * φ-aligned: Confidence intervals bounded at 61.8%
 *
 * @module @cynic/node/inference/gaussian
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Mathematical constants
 */
export const GAUSSIAN_CONSTANTS = {
  // √(2π) ≈ 2.5066
  SQRT_2PI: Math.sqrt(2 * Math.PI),

  // √2 ≈ 1.4142
  SQRT_2: Math.sqrt(2),

  // 1/√(2π) ≈ 0.3989
  INV_SQRT_2PI: 1 / Math.sqrt(2 * Math.PI),

  // Log of √(2π)
  LOG_SQRT_2PI: 0.5 * Math.log(2 * Math.PI),
};

/**
 * φ-aligned Gaussian configuration
 */
export const GAUSSIAN_CONFIG = {
  // Standard deviations for confidence levels
  SIGMA_68: 1.0,               // 68.27% confidence
  SIGMA_PHI: 1 / PHI_INV,      // ≈ 1.618 - φ-aligned (~89%)
  SIGMA_95: 1.96,              // 95% confidence
  SIGMA_99: 2.576,             // 99% confidence

  // φ-aligned confidence level
  CONFIDENCE_PHI: PHI_INV,     // 61.8% confidence

  // Numerical stability
  EPSILON: 1e-15,
  MAX_ITERATIONS: 100,

  // Default parameters
  DEFAULT_MEAN: 0,
  DEFAULT_STD: 1,

  // Kernel bandwidths
  SILVERMAN_FACTOR: 1.06,      // Silverman's rule of thumb multiplier
};

// ═══════════════════════════════════════════════════════════════════════════════
// CORE GAUSSIAN FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Gaussian Probability Density Function (PDF)
 * f(x; μ, σ) = (1/√(2πσ²)) × e^(-(x-μ)²/(2σ²))
 *
 * @param {number} x - Value
 * @param {number} [mean=0] - Mean (μ)
 * @param {number} [std=1] - Standard deviation (σ)
 * @returns {number} Probability density
 */
export function gaussianPDF(x, mean = 0, std = 1) {
  if (std <= 0) return x === mean ? Infinity : 0;

  const z = (x - mean) / std;
  return GAUSSIAN_CONSTANTS.INV_SQRT_2PI * Math.exp(-0.5 * z * z) / std;
}

/**
 * Log of Gaussian PDF (for numerical stability)
 *
 * @param {number} x - Value
 * @param {number} [mean=0] - Mean
 * @param {number} [std=1] - Standard deviation
 * @returns {number} Log probability density
 */
export function gaussianLogPDF(x, mean = 0, std = 1) {
  if (std <= 0) return x === mean ? Infinity : -Infinity;

  const z = (x - mean) / std;
  return -GAUSSIAN_CONSTANTS.LOG_SQRT_2PI - Math.log(std) - 0.5 * z * z;
}

/**
 * Gaussian Cumulative Distribution Function (CDF)
 * Φ(x) = P(X ≤ x) = 0.5 × (1 + erf((x-μ)/(σ√2)))
 *
 * @param {number} x - Value
 * @param {number} [mean=0] - Mean
 * @param {number} [std=1] - Standard deviation
 * @returns {number} Cumulative probability
 */
export function gaussianCDF(x, mean = 0, std = 1) {
  if (std <= 0) return x >= mean ? 1 : 0;

  const z = (x - mean) / (std * GAUSSIAN_CONSTANTS.SQRT_2);
  return 0.5 * (1 + erf(z));
}

/**
 * Gaussian Survival Function (1 - CDF)
 * P(X > x)
 *
 * @param {number} x - Value
 * @param {number} [mean=0] - Mean
 * @param {number} [std=1] - Standard deviation
 * @returns {number} Survival probability
 */
export function gaussianSurvival(x, mean = 0, std = 1) {
  return 1 - gaussianCDF(x, mean, std);
}

/**
 * Gaussian Quantile Function (Inverse CDF / Percent Point Function)
 * Find x such that P(X ≤ x) = p
 *
 * Uses Abramowitz and Stegun approximation
 *
 * @param {number} p - Probability (0 to 1)
 * @param {number} [mean=0] - Mean
 * @param {number} [std=1] - Standard deviation
 * @returns {number} Quantile value
 */
export function gaussianQuantile(p, mean = 0, std = 1) {
  if (p <= 0) return -Infinity;
  if (p >= 1) return Infinity;
  if (p === 0.5) return mean;

  // Standard normal quantile
  const z = standardNormalQuantile(p);

  return mean + std * z;
}

/**
 * Standard normal quantile (probit function)
 * @private
 */
function standardNormalQuantile(p) {
  // Abramowitz and Stegun approximation
  const a = [
    -3.969683028665376e1,
    2.209460984245205e2,
    -2.759285104469687e2,
    1.383577518672690e2,
    -3.066479806614716e1,
    2.506628277459239e0,
  ];
  const b = [
    -5.447609879822406e1,
    1.615858368580409e2,
    -1.556989798598866e2,
    6.680131188771972e1,
    -1.328068155288572e1,
  ];
  const c = [
    -7.784894002430293e-3,
    -3.223964580411365e-1,
    -2.400758277161838e0,
    -2.549732539343734e0,
    4.374664141464968e0,
    2.938163982698783e0,
  ];
  const d = [
    7.784695709041462e-3,
    3.224671290700398e-1,
    2.445134137142996e0,
    3.754408661907416e0,
  ];

  const pLow = 0.02425;
  const pHigh = 1 - pLow;

  let q, r;

  if (p < pLow) {
    q = Math.sqrt(-2 * Math.log(p));
    return (((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5]) /
      ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1);
  } else if (p <= pHigh) {
    q = p - 0.5;
    r = q * q;
    return (((((a[0] * r + a[1]) * r + a[2]) * r + a[3]) * r + a[4]) * r + a[5]) * q /
      (((((b[0] * r + b[1]) * r + b[2]) * r + b[3]) * r + b[4]) * r + 1);
  } else {
    q = Math.sqrt(-2 * Math.log(1 - p));
    return -(((((c[0] * q + c[1]) * q + c[2]) * q + c[3]) * q + c[4]) * q + c[5]) /
      ((((d[0] * q + d[1]) * q + d[2]) * q + d[3]) * q + 1);
  }
}

/**
 * Error function (erf)
 * erf(x) = (2/√π) × ∫₀ˣ e^(-t²) dt
 *
 * @param {number} x - Value
 * @returns {number} Error function value
 */
export function erf(x) {
  // Constants
  const a1 = 0.254829592;
  const a2 = -0.284496736;
  const a3 = 1.421413741;
  const a4 = -1.453152027;
  const a5 = 1.061405429;
  const p = 0.3275911;

  // Save the sign
  const sign = x < 0 ? -1 : 1;
  x = Math.abs(x);

  // Abramowitz and Stegun approximation
  const t = 1 / (1 + p * x);
  const y = 1 - (((((a5 * t + a4) * t) + a3) * t + a2) * t + a1) * t * Math.exp(-x * x);

  return sign * y;
}

/**
 * Complementary error function (erfc)
 * erfc(x) = 1 - erf(x)
 *
 * @param {number} x - Value
 * @returns {number} Complementary error function value
 */
export function erfc(x) {
  return 1 - erf(x);
}

// ═══════════════════════════════════════════════════════════════════════════════
// Z-SCORES AND STANDARDIZATION
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Compute z-score (standardization)
 * z = (x - μ) / σ
 *
 * @param {number} x - Value
 * @param {number} mean - Population mean
 * @param {number} std - Population standard deviation
 * @returns {number} Z-score
 */
export function zScore(x, mean, std) {
  if (std === 0) return x === mean ? 0 : (x > mean ? Infinity : -Infinity);
  return (x - mean) / std;
}

/**
 * Convert z-score back to value
 * x = μ + z × σ
 *
 * @param {number} z - Z-score
 * @param {number} mean - Mean
 * @param {number} std - Standard deviation
 * @returns {number} Original value
 */
export function fromZScore(z, mean, std) {
  return mean + z * std;
}

/**
 * Standardize an array of values
 *
 * @param {number[]} values - Array of values
 * @returns {Object} {standardized, mean, std}
 */
export function standardize(values) {
  if (values.length === 0) {
    return { standardized: [], mean: 0, std: 1 };
  }

  const mean = values.reduce((a, b) => a + b, 0) / values.length;
  const variance = values.reduce((sum, x) => sum + Math.pow(x - mean, 2), 0) / values.length;
  const std = Math.sqrt(variance) || 1;

  const standardized = values.map(x => (x - mean) / std);

  return { standardized, mean, std };
}

/**
 * Compute p-value from z-score (two-tailed)
 *
 * @param {number} z - Z-score
 * @returns {number} P-value
 */
export function zScoreToPValue(z) {
  return 2 * gaussianSurvival(Math.abs(z), 0, 1);
}

/**
 * Compute z-score from p-value (two-tailed)
 *
 * @param {number} p - P-value
 * @returns {number} Z-score (absolute value)
 */
export function pValueToZScore(p) {
  return Math.abs(gaussianQuantile(p / 2, 0, 1));
}

// ═══════════════════════════════════════════════════════════════════════════════
// CONFIDENCE INTERVALS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Compute confidence interval for mean
 *
 * @param {number} mean - Sample mean
 * @param {number} std - Sample standard deviation
 * @param {number} n - Sample size
 * @param {number} [confidence=0.95] - Confidence level
 * @returns {Object} {lower, upper, mean, marginOfError}
 */
export function confidenceInterval(mean, std, n, confidence = 0.95) {
  // φ-bound the confidence
  confidence = Math.min(PHI_INV, confidence);

  const alpha = 1 - confidence;
  const z = gaussianQuantile(1 - alpha / 2, 0, 1);
  const marginOfError = z * std / Math.sqrt(n);

  return {
    lower: mean - marginOfError,
    upper: mean + marginOfError,
    mean,
    marginOfError,
    confidence,
  };
}

/**
 * Compute φ-aligned confidence interval (61.8% confidence)
 *
 * @param {number} mean - Sample mean
 * @param {number} std - Sample standard deviation
 * @param {number} n - Sample size
 * @returns {Object} {lower, upper, mean, marginOfError}
 */
export function phiConfidenceInterval(mean, std, n) {
  return confidenceInterval(mean, std, n, GAUSSIAN_CONFIG.CONFIDENCE_PHI);
}

/**
 * Get z-score for confidence level
 *
 * @param {number} confidence - Confidence level (0 to 1)
 * @returns {number} Z-score
 */
export function confidenceToZScore(confidence) {
  const alpha = 1 - Math.min(PHI_INV, confidence);
  return gaussianQuantile(1 - alpha / 2, 0, 1);
}

// ═══════════════════════════════════════════════════════════════════════════════
// RANDOM NUMBER GENERATION
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Generate standard normal random number using Box-Muller transform
 *
 * @returns {number} Random value from N(0,1)
 */
export function randomStandardNormal() {
  // Box-Muller transform
  const u1 = Math.random();
  const u2 = Math.random();

  const z = Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
  return z;
}

/**
 * Generate normal random number
 *
 * @param {number} [mean=0] - Mean
 * @param {number} [std=1] - Standard deviation
 * @returns {number} Random value from N(μ, σ²)
 */
export function randomNormal(mean = 0, std = 1) {
  return mean + std * randomStandardNormal();
}

/**
 * Generate array of normal random numbers
 *
 * @param {number} n - Count
 * @param {number} [mean=0] - Mean
 * @param {number} [std=1] - Standard deviation
 * @returns {number[]} Array of random values
 */
export function randomNormalArray(n, mean = 0, std = 1) {
  const result = new Array(n);
  for (let i = 0; i < n; i++) {
    result[i] = randomNormal(mean, std);
  }
  return result;
}

/**
 * Generate correlated normal random numbers (2D)
 *
 * @param {number} correlation - Correlation coefficient (-1 to 1)
 * @param {number} [n=1] - Number of pairs
 * @returns {Object[]} Array of {x, y} pairs
 */
export function randomCorrelatedNormal(correlation, n = 1) {
  const rho = Math.max(-1, Math.min(1, correlation));
  const results = [];

  for (let i = 0; i < n; i++) {
    const z1 = randomStandardNormal();
    const z2 = randomStandardNormal();

    // Cholesky decomposition for correlation
    const x = z1;
    const y = rho * z1 + Math.sqrt(1 - rho * rho) * z2;

    results.push({ x, y });
  }

  return n === 1 ? results[0] : results;
}

// ═══════════════════════════════════════════════════════════════════════════════
// GAUSSIAN NOISE
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Add Gaussian noise to a value
 *
 * @param {number} value - Original value
 * @param {number} [noiseLevel=0.1] - Noise standard deviation
 * @returns {number} Noisy value
 */
export function addNoise(value, noiseLevel = 0.1) {
  return value + randomNormal(0, noiseLevel);
}

/**
 * Add Gaussian noise to an array
 *
 * @param {number[]} values - Original values
 * @param {number} [noiseLevel=0.1] - Noise standard deviation
 * @returns {number[]} Noisy values
 */
export function addNoiseArray(values, noiseLevel = 0.1) {
  return values.map(v => addNoise(v, noiseLevel));
}

/**
 * Gaussian noise generator with consistent seed-like behavior
 */
export class GaussianNoiseGenerator {
  /**
   * @param {Object} [options={}]
   * @param {number} [options.mean=0] - Noise mean
   * @param {number} [options.std=1] - Noise standard deviation
   * @param {number} [options.seed] - Optional seed (uses simple LCG)
   */
  constructor(options = {}) {
    this.mean = options.mean ?? 0;
    this.std = options.std ?? 1;
    this.seed = options.seed;
    this._state = options.seed ?? Math.floor(Math.random() * 2147483647);
  }

  /**
   * Simple LCG random number generator
   * @private
   */
  _random() {
    // LCG parameters (same as glibc)
    this._state = (this._state * 1103515245 + 12345) % 2147483648;
    return this._state / 2147483648;
  }

  /**
   * Generate next noise value
   * @returns {number}
   */
  next() {
    // Box-Muller with seeded random
    const u1 = this._random() || 1e-10;
    const u2 = this._random();

    const z = Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2);
    return this.mean + this.std * z;
  }

  /**
   * Generate array of noise values
   * @param {number} n - Count
   * @returns {number[]}
   */
  generate(n) {
    const result = new Array(n);
    for (let i = 0; i < n; i++) {
      result[i] = this.next();
    }
    return result;
  }

  /**
   * Reset generator to initial state
   */
  reset() {
    this._state = this.seed ?? Math.floor(Math.random() * 2147483647);
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// KERNEL FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Gaussian kernel function
 * K(x) = (1/√(2π)) × e^(-x²/2)
 *
 * @param {number} x - Distance
 * @returns {number} Kernel value
 */
export function gaussianKernel(x) {
  return GAUSSIAN_CONSTANTS.INV_SQRT_2PI * Math.exp(-0.5 * x * x);
}

/**
 * Gaussian kernel with bandwidth
 * K_h(x) = (1/h) × K(x/h)
 *
 * @param {number} x - Distance
 * @param {number} bandwidth - Bandwidth (h)
 * @returns {number} Kernel value
 */
export function gaussianKernelScaled(x, bandwidth) {
  if (bandwidth <= 0) return 0;
  return gaussianKernel(x / bandwidth) / bandwidth;
}

/**
 * Compute optimal bandwidth using Silverman's rule of thumb
 * h = 1.06 × σ × n^(-1/5)
 *
 * @param {number[]} data - Data array
 * @returns {number} Optimal bandwidth
 */
export function silvermanBandwidth(data) {
  if (data.length < 2) return 1;

  const n = data.length;
  const { std } = standardize(data);

  // Also consider IQR for robustness
  const sorted = [...data].sort((a, b) => a - b);
  const q1 = sorted[Math.floor(n * 0.25)];
  const q3 = sorted[Math.floor(n * 0.75)];
  const iqr = q3 - q1;

  // Use minimum of std and IQR/1.34 for robustness
  const spread = Math.min(std, iqr / 1.34) || std || 1;

  return GAUSSIAN_CONFIG.SILVERMAN_FACTOR * spread * Math.pow(n, -0.2);
}

/**
 * Kernel Density Estimation (KDE) at a point
 *
 * @param {number} x - Point to estimate
 * @param {number[]} data - Data array
 * @param {number} [bandwidth] - Bandwidth (auto-computed if not provided)
 * @returns {number} Density estimate
 */
export function kernelDensityEstimate(x, data, bandwidth) {
  if (data.length === 0) return 0;

  const h = bandwidth ?? silvermanBandwidth(data);
  const n = data.length;

  let sum = 0;
  for (const xi of data) {
    sum += gaussianKernelScaled(x - xi, h);
  }

  return sum / n;
}

/**
 * Full KDE over a range
 *
 * @param {number[]} data - Data array
 * @param {Object} [options={}]
 * @param {number} [options.points=100] - Number of evaluation points
 * @param {number} [options.bandwidth] - Bandwidth
 * @param {number} [options.min] - Minimum x (auto if not provided)
 * @param {number} [options.max] - Maximum x (auto if not provided)
 * @returns {Object} {x: number[], density: number[], bandwidth}
 */
export function kde(data, options = {}) {
  if (data.length === 0) {
    return { x: [], density: [], bandwidth: 0 };
  }

  const { points = 100 } = options;
  const bandwidth = options.bandwidth ?? silvermanBandwidth(data);

  const dataMin = Math.min(...data);
  const dataMax = Math.max(...data);
  const range = dataMax - dataMin || 1;

  const min = options.min ?? dataMin - 0.5 * range;
  const max = options.max ?? dataMax + 0.5 * range;

  const step = (max - min) / (points - 1);
  const x = new Array(points);
  const density = new Array(points);

  for (let i = 0; i < points; i++) {
    x[i] = min + i * step;
    density[i] = kernelDensityEstimate(x[i], data, bandwidth);
  }

  return { x, density, bandwidth };
}

// ═══════════════════════════════════════════════════════════════════════════════
// GAUSSIAN DISTRIBUTION CLASS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Gaussian (Normal) Distribution class
 */
export class GaussianDistribution {
  /**
   * @param {number} [mean=0] - Mean (μ)
   * @param {number} [std=1] - Standard deviation (σ)
   */
  constructor(mean = 0, std = 1) {
    this.mean = mean;
    this.std = Math.max(GAUSSIAN_CONFIG.EPSILON, std);
    this.variance = this.std * this.std;
  }

  /**
   * Probability density at x
   * @param {number} x
   * @returns {number}
   */
  pdf(x) {
    return gaussianPDF(x, this.mean, this.std);
  }

  /**
   * Log probability density at x
   * @param {number} x
   * @returns {number}
   */
  logPdf(x) {
    return gaussianLogPDF(x, this.mean, this.std);
  }

  /**
   * Cumulative probability P(X ≤ x)
   * @param {number} x
   * @returns {number}
   */
  cdf(x) {
    return gaussianCDF(x, this.mean, this.std);
  }

  /**
   * Survival probability P(X > x)
   * @param {number} x
   * @returns {number}
   */
  survival(x) {
    return gaussianSurvival(x, this.mean, this.std);
  }

  /**
   * Quantile (inverse CDF)
   * @param {number} p
   * @returns {number}
   */
  quantile(p) {
    return gaussianQuantile(p, this.mean, this.std);
  }

  /**
   * Z-score for a value
   * @param {number} x
   * @returns {number}
   */
  zScore(x) {
    return zScore(x, this.mean, this.std);
  }

  /**
   * Sample from distribution
   * @param {number} [n=1]
   * @returns {number|number[]}
   */
  sample(n = 1) {
    if (n === 1) return randomNormal(this.mean, this.std);
    return randomNormalArray(n, this.mean, this.std);
  }

  /**
   * Confidence interval
   * @param {number} [confidence=0.618]
   * @returns {Object} {lower, upper}
   */
  confidenceInterval(confidence = PHI_INV) {
    // φ-bound: cap at 61.8%
    const cappedConfidence = Math.min(PHI_INV, confidence);
    const alpha = 1 - cappedConfidence;
    const lower = this.quantile(alpha / 2);
    const upper = this.quantile(1 - alpha / 2);
    return { lower, upper, confidence: cappedConfidence };
  }

  /**
   * Check if value is anomalous (outside n standard deviations)
   * @param {number} x
   * @param {number} [sigmas=2]
   * @returns {boolean}
   */
  isAnomaly(x, sigmas = 2) {
    const z = Math.abs(this.zScore(x));
    return z > sigmas;
  }

  /**
   * Mahalanobis distance (1D = absolute z-score)
   * @param {number} x
   * @returns {number}
   */
  mahalanobisDistance(x) {
    return Math.abs(this.zScore(x));
  }

  /**
   * Fit distribution to data
   * @param {number[]} data
   * @returns {GaussianDistribution} New fitted distribution
   */
  static fit(data) {
    if (data.length === 0) {
      return new GaussianDistribution(0, 1);
    }

    const mean = data.reduce((a, b) => a + b, 0) / data.length;
    const variance = data.reduce((sum, x) => sum + Math.pow(x - mean, 2), 0) / data.length;
    const std = Math.sqrt(variance) || 1;

    return new GaussianDistribution(mean, std);
  }

  /**
   * Serialize
   * @returns {Object}
   */
  toJSON() {
    return {
      mean: this.mean,
      std: this.std,
      variance: this.variance,
    };
  }

  /**
   * Deserialize
   * @param {Object} data
   * @returns {GaussianDistribution}
   */
  static fromJSON(data) {
    return new GaussianDistribution(data.mean, data.std);
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MULTIVARIATE GAUSSIAN (BASIC)
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Multivariate Gaussian with diagonal covariance (independent dimensions)
 */
export class DiagonalGaussian {
  /**
   * @param {number[]} means - Mean for each dimension
   * @param {number[]} stds - Standard deviation for each dimension
   */
  constructor(means, stds) {
    this.means = means;
    this.stds = stds;
    this.dimensions = means.length;
  }

  /**
   * Log probability density
   * @param {number[]} x - Point
   * @returns {number}
   */
  logPdf(x) {
    if (x.length !== this.dimensions) {
      throw new Error(`Expected ${this.dimensions} dimensions, got ${x.length}`);
    }

    let logP = 0;
    for (let i = 0; i < this.dimensions; i++) {
      logP += gaussianLogPDF(x[i], this.means[i], this.stds[i]);
    }
    return logP;
  }

  /**
   * Probability density
   * @param {number[]} x
   * @returns {number}
   */
  pdf(x) {
    return Math.exp(this.logPdf(x));
  }

  /**
   * Sample from distribution
   * @param {number} [n=1]
   * @returns {number[]|number[][]}
   */
  sample(n = 1) {
    const samples = [];
    for (let i = 0; i < n; i++) {
      const point = this.means.map((mean, j) =>
        randomNormal(mean, this.stds[j])
      );
      samples.push(point);
    }
    return n === 1 ? samples[0] : samples;
  }

  /**
   * Mahalanobis distance (squared)
   * @param {number[]} x
   * @returns {number}
   */
  mahalanobisDistanceSquared(x) {
    let sum = 0;
    for (let i = 0; i < this.dimensions; i++) {
      const z = (x[i] - this.means[i]) / this.stds[i];
      sum += z * z;
    }
    return sum;
  }

  /**
   * Mahalanobis distance
   * @param {number[]} x
   * @returns {number}
   */
  mahalanobisDistance(x) {
    return Math.sqrt(this.mahalanobisDistanceSquared(x));
  }

  /**
   * Fit to data
   * @param {number[][]} data - Array of points
   * @returns {DiagonalGaussian}
   */
  static fit(data) {
    if (data.length === 0) return new DiagonalGaussian([0], [1]);

    const dimensions = data[0].length;
    const n = data.length;

    const means = new Array(dimensions).fill(0);
    const stds = new Array(dimensions).fill(0);

    // Compute means
    for (const point of data) {
      for (let i = 0; i < dimensions; i++) {
        means[i] += point[i];
      }
    }
    for (let i = 0; i < dimensions; i++) {
      means[i] /= n;
    }

    // Compute stds
    for (const point of data) {
      for (let i = 0; i < dimensions; i++) {
        stds[i] += Math.pow(point[i] - means[i], 2);
      }
    }
    for (let i = 0; i < dimensions; i++) {
      stds[i] = Math.sqrt(stds[i] / n) || 1;
    }

    return new DiagonalGaussian(means, stds);
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// UTILITY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Compute sample statistics
 *
 * @param {number[]} data - Data array
 * @returns {Object} {mean, variance, std, min, max, median}
 */
export function computeStats(data) {
  if (data.length === 0) {
    return { mean: 0, variance: 0, std: 0, min: 0, max: 0, median: 0, n: 0 };
  }

  const n = data.length;
  const mean = data.reduce((a, b) => a + b, 0) / n;
  const variance = data.reduce((sum, x) => sum + Math.pow(x - mean, 2), 0) / n;
  const std = Math.sqrt(variance);

  const sorted = [...data].sort((a, b) => a - b);
  const median = n % 2 === 0
    ? (sorted[n / 2 - 1] + sorted[n / 2]) / 2
    : sorted[Math.floor(n / 2)];

  return {
    n,
    mean,
    variance,
    std,
    min: sorted[0],
    max: sorted[n - 1],
    median,
  };
}

/**
 * Compute skewness
 *
 * @param {number[]} data - Data array
 * @returns {number} Skewness
 */
export function skewness(data) {
  const { mean, std, n } = computeStats(data);
  if (n < 3 || std === 0) return 0;

  const m3 = data.reduce((sum, x) => sum + Math.pow(x - mean, 3), 0) / n;
  return m3 / Math.pow(std, 3);
}

/**
 * Compute kurtosis (excess kurtosis, normal = 0)
 *
 * @param {number[]} data - Data array
 * @returns {number} Excess kurtosis
 */
export function kurtosis(data) {
  const { mean, std, n } = computeStats(data);
  if (n < 4 || std === 0) return 0;

  const m4 = data.reduce((sum, x) => sum + Math.pow(x - mean, 4), 0) / n;
  return m4 / Math.pow(std, 4) - 3; // Excess kurtosis
}

/**
 * Test for normality using Jarque-Bera test
 *
 * @param {number[]} data - Data array
 * @returns {Object} {statistic, pValue, isNormal}
 */
export function jarqueBeraTest(data) {
  const n = data.length;
  if (n < 8) return { statistic: 0, pValue: 1, isNormal: true };

  const S = skewness(data);
  const K = kurtosis(data);

  // JB = n * (S²/6 + K²/24)
  const JB = n * (S * S / 6 + K * K / 24);

  // Chi-squared with 2 degrees of freedom
  // Approximate p-value
  const pValue = Math.exp(-JB / 2);

  return {
    statistic: JB,
    pValue,
    isNormal: pValue > 0.05,
    skewness: S,
    kurtosis: K,
  };
}

// ═══════════════════════════════════════════════════════════════════════════════
// FACTORY FUNCTIONS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Create a Gaussian distribution
 * @param {number} [mean=0]
 * @param {number} [std=1]
 * @returns {GaussianDistribution}
 */
export function createGaussian(mean = 0, std = 1) {
  return new GaussianDistribution(mean, std);
}

/**
 * Create a noise generator
 * @param {Object} [options={}]
 * @returns {GaussianNoiseGenerator}
 */
export function createNoiseGenerator(options = {}) {
  return new GaussianNoiseGenerator(options);
}

/**
 * Create standard normal distribution N(0,1)
 * @returns {GaussianDistribution}
 */
export function standardNormal() {
  return new GaussianDistribution(0, 1);
}

// ═══════════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════════

export default {
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

  // Constants
  GAUSSIAN_CONSTANTS,
  GAUSSIAN_CONFIG,
};
