/**
 * @cynic/node - Differential Privacy
 *
 * ε-differential privacy for pattern aggregation.
 * Guarantees that removing any single user doesn't significantly change output.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/privacy/differential
 */

'use strict';

import { createHash } from 'crypto';
import { PHI, PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * φ-aligned constants for differential privacy
 */
export const PRIVACY_CONSTANTS = {
  /** Privacy budget ε (φ⁻¹ = 0.618) - never fully reveal */
  EPSILON: PHI_INV,

  /** Default sensitivity (1 per user) */
  DEFAULT_SENSITIVITY: 1,

  /** Noise multiplier (φ = 1.618) */
  NOISE_MULTIPLIER: PHI,

  /** Minimum noise floor (φ⁻² = 0.382) */
  MIN_NOISE_FLOOR: PHI_INV_2,

  /** Max queries per budget period (Fib(11) = 89) */
  MAX_QUERIES_PER_PERIOD: 89,

  /** Budget refresh period in hours (Fib(8) = 21) */
  BUDGET_REFRESH_HOURS: 21,
};

/**
 * Generate Laplacian noise
 *
 * Laplace distribution centered at 0 with scale b = sensitivity/ε
 *
 * @param {number} scale - Scale parameter (b)
 * @returns {number} Random noise from Laplace(0, scale)
 */
export function laplacianNoise(scale) {
  // Use inverse CDF method
  // If U ~ Uniform(0,1), then X = -b * sign(U-0.5) * ln(1-2|U-0.5|) ~ Laplace(0,b)
  const u = Math.random() - 0.5;
  const sign = u >= 0 ? 1 : -1;
  const noise = -scale * sign * Math.log(1 - 2 * Math.abs(u));

  return noise;
}

/**
 * Generate Gaussian noise (for composed queries)
 *
 * @param {number} sigma - Standard deviation
 * @returns {number} Random noise from Normal(0, sigma²)
 */
export function gaussianNoise(sigma) {
  // Box-Muller transform
  const u1 = Math.random();
  const u2 = Math.random();
  const noise = Math.sqrt(-2 * Math.log(u1)) * Math.cos(2 * Math.PI * u2) * sigma;

  return noise;
}

/**
 * Differential Privacy mechanism for aggregating statistics
 */
export class DifferentialPrivacy {
  /**
   * @param {object} options
   * @param {number} [options.epsilon] - Privacy parameter (default: φ⁻¹)
   * @param {number} [options.sensitivity] - Query sensitivity (default: 1)
   */
  constructor(options = {}) {
    this.epsilon = options.epsilon ?? PRIVACY_CONSTANTS.EPSILON;
    this.sensitivity = options.sensitivity ?? PRIVACY_CONSTANTS.DEFAULT_SENSITIVITY;

    // Privacy budget tracking
    this.budgetUsed = 0;
    this.queryCount = 0;
    this.lastBudgetReset = Date.now();
  }

  /**
   * Get current privacy budget status
   * @returns {object} Budget status
   */
  getBudgetStatus() {
    this._maybeResetBudget();

    return {
      epsilon: this.epsilon,
      budgetUsed: this.budgetUsed,
      budgetRemaining: Math.max(0, this.epsilon - this.budgetUsed),
      queryCount: this.queryCount,
      maxQueries: PRIVACY_CONSTANTS.MAX_QUERIES_PER_PERIOD,
      percentUsed: (this.budgetUsed / this.epsilon) * 100,
    };
  }

  /**
   * Check if budget allows another query
   * @returns {boolean}
   */
  canQuery() {
    this._maybeResetBudget();
    return this.queryCount < PRIVACY_CONSTANTS.MAX_QUERIES_PER_PERIOD &&
           this.budgetUsed < this.epsilon;
  }

  /**
   * Reset budget if refresh period has passed
   * @private
   */
  _maybeResetBudget() {
    const refreshMs = PRIVACY_CONSTANTS.BUDGET_REFRESH_HOURS * 60 * 60 * 1000;
    if (Date.now() - this.lastBudgetReset > refreshMs) {
      this.budgetUsed = 0;
      this.queryCount = 0;
      this.lastBudgetReset = Date.now();
    }
  }

  /**
   * Add Laplacian noise to a value
   *
   * @param {number} trueValue - The true value to protect
   * @param {number} [sensitivity] - Query sensitivity (default: instance sensitivity)
   * @returns {number} Noised value
   */
  addNoise(trueValue, sensitivity = null) {
    if (!this.canQuery()) {
      throw new Error('Privacy budget exhausted');
    }

    const s = sensitivity ?? this.sensitivity;
    const scale = (s / this.epsilon) * PRIVACY_CONSTANTS.NOISE_MULTIPLIER;
    const noise = laplacianNoise(scale);

    // Track budget usage
    this.budgetUsed += s / this.epsilon;
    this.queryCount++;

    // Apply minimum noise floor
    const noisedValue = trueValue + noise;
    const minNoise = trueValue * PRIVACY_CONSTANTS.MIN_NOISE_FLOOR;

    if (Math.abs(noise) < minNoise) {
      return trueValue + (noise >= 0 ? minNoise : -minNoise);
    }

    return noisedValue;
  }

  /**
   * Add noise to a count (non-negative result)
   *
   * @param {number} count - True count
   * @returns {number} Noised count (>= 0)
   */
  addNoiseToCount(count) {
    const noised = this.addNoise(count);
    return Math.max(0, Math.round(noised));
  }

  /**
   * Add noise to a ratio (bounded [0,1])
   *
   * @param {number} ratio - True ratio
   * @returns {number} Noised ratio in [0,1]
   */
  addNoiseToRatio(ratio) {
    const noised = this.addNoise(ratio, 0.1); // Lower sensitivity for ratios
    return Math.max(0, Math.min(1, noised));
  }
}

/**
 * Pattern aggregator with differential privacy
 */
export class PrivatePatternAggregator {
  /**
   * @param {object} options
   * @param {number} [options.epsilon] - Privacy parameter
   */
  constructor(options = {}) {
    this.dp = new DifferentialPrivacy(options);
    this.patternCounts = new Map();
    this.totalPatterns = 0;
  }

  /**
   * Add a pattern (hashed, not raw)
   *
   * @param {string} patternHash - Hash of the pattern
   * @param {string} [category] - Pattern category
   */
  addPattern(patternHash, category = 'general') {
    const key = `${category}:${patternHash}`;
    const current = this.patternCounts.get(key) || 0;
    this.patternCounts.set(key, current + 1);
    this.totalPatterns++;
  }

  /**
   * Get private count for a pattern
   *
   * @param {string} patternHash - Pattern hash
   * @param {string} [category] - Category
   * @returns {number} Noised count
   */
  getPrivateCount(patternHash, category = 'general') {
    const key = `${category}:${patternHash}`;
    const trueCount = this.patternCounts.get(key) || 0;
    return this.dp.addNoiseToCount(trueCount);
  }

  /**
   * Get top patterns by noised count
   *
   * @param {number} [limit=10] - Max patterns to return
   * @param {string} [category] - Filter by category
   * @returns {Array<{ hash: string, category: string, count: number }>}
   */
  getTopPatterns(limit = 10, category = null) {
    const results = [];

    for (const [key, count] of this.patternCounts) {
      const [cat, hash] = key.split(':');
      if (category && cat !== category) continue;

      // Add noise to each count
      const noisedCount = this.dp.addNoiseToCount(count);
      if (noisedCount > 0) {
        results.push({ hash, category: cat, count: noisedCount });
      }
    }

    // Sort by noised count and return top
    return results
      .sort((a, b) => b.count - a.count)
      .slice(0, limit);
  }

  /**
   * Get aggregate statistics with privacy
   *
   * @returns {object} Private statistics
   */
  getPrivateStats() {
    return {
      totalPatterns: this.dp.addNoiseToCount(this.totalPatterns),
      uniquePatterns: this.dp.addNoiseToCount(this.patternCounts.size),
      budgetStatus: this.dp.getBudgetStatus(),
    };
  }

  /**
   * Get category distribution with privacy
   *
   * @returns {Map<string, number>} Category -> noised count
   */
  getCategoryDistribution() {
    const categories = new Map();

    for (const [key, count] of this.patternCounts) {
      const [category] = key.split(':');
      const current = categories.get(category) || 0;
      categories.set(category, current + count);
    }

    // Add noise to each category count
    const privateDistribution = new Map();
    for (const [category, count] of categories) {
      privateDistribution.set(category, this.dp.addNoiseToCount(count));
    }

    return privateDistribution;
  }
}

/**
 * Knowledge aggregator with differential privacy
 *
 * Used for collective learning without exposing individual contributions.
 */
export class PrivateKnowledgeAggregator {
  constructor(options = {}) {
    this.dp = new DifferentialPrivacy({
      epsilon: options.epsilon ?? PRIVACY_CONSTANTS.EPSILON,
      sensitivity: options.sensitivity ?? 1,
    });

    // Aggregate knowledge buckets (using golden angle for distribution)
    this.buckets = new Map();
    this.totalContributions = 0;
  }

  /**
   * Hash content to bucket using golden angle distribution
   *
   * @param {string} content - Content to hash
   * @returns {number} Bucket index (0-359)
   */
  _getBucket(content) {
    const hash = createHash('sha256').update(content).digest();
    const angle = (hash.readUInt32BE(0) % 360000) / 1000; // 0-359.999
    // Apply golden angle offset for even distribution
    const goldenAngle = 137.5;
    const bucket = Math.floor((angle + goldenAngle) % 360);
    return bucket;
  }

  /**
   * Contribute knowledge (privately)
   *
   * @param {string} contentHash - Hash of knowledge content
   * @param {object} metadata - Non-identifying metadata
   */
  contribute(contentHash, metadata = {}) {
    const bucket = this._getBucket(contentHash);
    const entry = this.buckets.get(bucket) || { count: 0, categories: new Map() };

    entry.count++;
    if (metadata.category) {
      const catCount = entry.categories.get(metadata.category) || 0;
      entry.categories.set(metadata.category, catCount + 1);
    }

    this.buckets.set(bucket, entry);
    this.totalContributions++;
  }

  /**
   * Get private aggregate
   *
   * @returns {object} Privacy-preserved aggregate
   */
  getPrivateAggregate() {
    const bucketCounts = [];
    const categoryTotals = new Map();

    for (const [bucket, entry] of this.buckets) {
      bucketCounts.push({
        bucket,
        count: this.dp.addNoiseToCount(entry.count),
      });

      for (const [cat, count] of entry.categories) {
        const current = categoryTotals.get(cat) || 0;
        categoryTotals.set(cat, current + count);
      }
    }

    // Privatize category totals
    const privateCategories = new Map();
    for (const [cat, count] of categoryTotals) {
      privateCategories.set(cat, this.dp.addNoiseToCount(count));
    }

    return {
      totalContributions: this.dp.addNoiseToCount(this.totalContributions),
      activeBuckets: this.dp.addNoiseToCount(this.buckets.size),
      bucketDistribution: bucketCounts.filter(b => b.count > 0),
      categoryDistribution: Object.fromEntries(privateCategories),
      budgetStatus: this.dp.getBudgetStatus(),
    };
  }
}

export default {
  PRIVACY_CONSTANTS,
  laplacianNoise,
  gaussianNoise,
  DifferentialPrivacy,
  PrivatePatternAggregator,
  PrivateKnowledgeAggregator,
};
