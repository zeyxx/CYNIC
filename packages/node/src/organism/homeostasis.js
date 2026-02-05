/**
 * Homeostasis - Stability and Equilibrium
 *
 * "Le chien cherche l'équilibre, pas l'excès" - κυνικός
 *
 * Measures how well CYNIC maintains internal stability:
 * - Variance in performance metrics
 * - Deviation from baseline
 * - Recovery after perturbation
 *
 * High homeostasis = stable, predictable behavior
 * Low homeostasis = erratic, unstable behavior
 *
 * @module @cynic/node/organism/homeostasis
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import { computeStats, GaussianDistribution } from '../inference/gaussian.js';

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

export const HOMEOSTASIS_CONFIG = {
  // Stability thresholds (coefficient of variation)
  CV_STABLE: PHI_INV_3,      // CV < 23.6% = highly stable
  CV_MODERATE: PHI_INV_2,    // CV < 38.2% = moderately stable
  CV_UNSTABLE: PHI_INV,      // CV < 61.8% = unstable
  // CV > 61.8% = chaotic

  // Baseline learning rate
  BASELINE_ALPHA: PHI_INV_3, // 23.6% weight for new observations

  // Minimum samples for valid statistics
  MIN_SAMPLES: 5,

  // Maximum history size
  MAX_HISTORY: 500,

  // Metric names
  METRICS: [
    'latency',
    'tokenRate',
    'errorRate',
    'successRate',
    'qScore',
  ],
};

// ═══════════════════════════════════════════════════════════════════════════════
// METRIC BASELINE
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Tracks baseline for a single metric using exponential moving average
 */
export class MetricBaseline {
  /**
   * @param {string} name - Metric name
   * @param {Object} [options]
   */
  constructor(name, options = {}) {
    this.name = name;
    this.alpha = options.alpha || HOMEOSTASIS_CONFIG.BASELINE_ALPHA;

    // Baseline statistics (exponentially weighted)
    this.mean = null;
    this.variance = null;

    // Raw history for detailed analysis
    this.history = [];
    this.maxHistory = options.maxHistory || HOMEOSTASIS_CONFIG.MAX_HISTORY;
  }

  /**
   * Update baseline with new observation
   * @param {number} value
   */
  update(value) {
    // Store in history
    this.history.push({
      timestamp: Date.now(),
      value,
    });

    // Prune history
    while (this.history.length > this.maxHistory) {
      this.history.shift();
    }

    // Initialize or update EMA
    if (this.mean === null) {
      this.mean = value;
      this.variance = 0;
    } else {
      // Exponential moving average
      const diff = value - this.mean;
      this.mean += this.alpha * diff;
      this.variance = (1 - this.alpha) * (this.variance + this.alpha * diff * diff);
    }
  }

  /**
   * Get current standard deviation
   */
  getStd() {
    return Math.sqrt(this.variance || 0);
  }

  /**
   * Get coefficient of variation (CV = std / mean)
   * @returns {number}
   */
  getCV() {
    if (this.mean === null || this.mean === 0) return 0;
    return this.getStd() / Math.abs(this.mean);
  }

  /**
   * Check if value is within normal range
   * @param {number} value
   * @param {number} [sigmas=2] - Number of standard deviations
   * @returns {boolean}
   */
  isNormal(value, sigmas = 2) {
    if (this.mean === null) return true;

    const std = this.getStd();
    const zScore = Math.abs(value - this.mean) / (std || 1);

    return zScore <= sigmas;
  }

  /**
   * Get deviation from baseline (z-score)
   * @param {number} value
   * @returns {number}
   */
  getDeviation(value) {
    if (this.mean === null) return 0;

    const std = this.getStd();
    if (std === 0) return 0;

    return (value - this.mean) / std;
  }

  /**
   * Get stability score (1 - CV), φ-bounded
   * @returns {number}
   */
  getStability() {
    const cv = this.getCV();
    const raw = Math.max(0, 1 - cv);
    return Math.min(PHI_INV, raw);
  }

  /**
   * Get recent history
   * @param {number} [n=10]
   */
  getRecent(n = 10) {
    return this.history.slice(-n);
  }

  toJSON() {
    return {
      name: this.name,
      mean: this.mean,
      variance: this.variance,
      historyLength: this.history.length,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// HOMEOSTASIS TRACKER
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Tracks overall system homeostasis across multiple metrics
 */
export class HomeostasisTracker {
  constructor(options = {}) {
    /** @type {Map<string, MetricBaseline>} */
    this.baselines = new Map();

    // Initialize default metrics
    for (const metric of HOMEOSTASIS_CONFIG.METRICS) {
      this.baselines.set(metric, new MetricBaseline(metric, options));
    }

    // Perturbation tracking
    this.perturbations = [];
    this.maxPerturbations = options.maxPerturbations || 100;

    // Session
    this.sessionStart = Date.now();
  }

  /**
   * Update a metric
   * @param {string} name - Metric name
   * @param {number} value - New value
   */
  update(name, value) {
    if (!this.baselines.has(name)) {
      this.baselines.set(name, new MetricBaseline(name));
    }

    const baseline = this.baselines.get(name);

    // Check for perturbation before updating
    if (baseline.mean !== null && !baseline.isNormal(value, 3)) {
      this.perturbations.push({
        timestamp: Date.now(),
        metric: name,
        value,
        expected: baseline.mean,
        deviation: baseline.getDeviation(value),
      });

      // Prune perturbations
      while (this.perturbations.length > this.maxPerturbations) {
        this.perturbations.shift();
      }
    }

    baseline.update(value);
  }

  /**
   * Update multiple metrics at once
   * @param {Object} metrics - {name: value, ...}
   */
  updateBatch(metrics) {
    for (const [name, value] of Object.entries(metrics)) {
      this.update(name, value);
    }
  }

  /**
   * Get baseline for a metric
   * @param {string} name
   * @returns {MetricBaseline|null}
   */
  getBaseline(name) {
    return this.baselines.get(name) || null;
  }

  /**
   * Get stability for a specific metric
   * @param {string} name
   * @returns {number}
   */
  getMetricStability(name) {
    const baseline = this.baselines.get(name);
    return baseline ? baseline.getStability() : 0;
  }

  /**
   * Get overall homeostasis score
   * Geometric mean of individual metric stabilities, φ-bounded
   * @returns {number}
   */
  getHomeostasis() {
    const stabilities = [];

    for (const baseline of this.baselines.values()) {
      if (baseline.mean !== null) {
        stabilities.push(baseline.getStability());
      }
    }

    if (stabilities.length === 0) return 0;

    // Geometric mean
    const product = stabilities.reduce((a, b) => a * b, 1);
    const geoMean = Math.pow(product, 1 / stabilities.length);

    return Math.min(PHI_INV, geoMean);
  }

  /**
   * Get homeostasis status
   * @returns {string}
   */
  getStatus() {
    const h = this.getHomeostasis();

    if (h >= HOMEOSTASIS_CONFIG.CV_STABLE) return 'stable';
    if (h >= HOMEOSTASIS_CONFIG.CV_MODERATE) return 'moderate';
    if (h >= HOMEOSTASIS_CONFIG.CV_UNSTABLE) return 'unstable';
    return 'chaotic';
  }

  /**
   * Get health assessment
   * @returns {Object}
   */
  getHealth() {
    const score = this.getHomeostasis();
    const status = this.getStatus();

    const metricDetails = {};
    for (const [name, baseline] of this.baselines) {
      if (baseline.mean !== null) {
        metricDetails[name] = {
          mean: baseline.mean,
          std: baseline.getStd(),
          cv: baseline.getCV(),
          stability: baseline.getStability(),
        };
      }
    }

    return {
      score,
      status,
      details: {
        metrics: metricDetails,
        perturbationCount: this.perturbations.length,
        recentPerturbations: this.perturbations.slice(-5),
      },
    };
  }

  /**
   * Detect if system is in perturbation
   * @param {number} [windowMs=60000] - Time window
   * @returns {boolean}
   */
  isPerturbed(windowMs = 60000) {
    const cutoff = Date.now() - windowMs;
    const recentPerturbations = this.perturbations.filter(
      p => p.timestamp >= cutoff
    );

    // More than 3 perturbations in window = perturbed
    return recentPerturbations.length > 3;
  }

  /**
   * Get statistics
   * @returns {Object}
   */
  getStats() {
    const metricStats = {};
    for (const [name, baseline] of this.baselines) {
      metricStats[name] = baseline.toJSON();
    }

    return {
      homeostasis: this.getHomeostasis(),
      status: this.getStatus(),
      metrics: metricStats,
      perturbations: {
        total: this.perturbations.length,
        recent: this.perturbations.slice(-10),
      },
      session: {
        durationMs: Date.now() - this.sessionStart,
      },
    };
  }

  /**
   * Reset tracker
   */
  reset() {
    for (const baseline of this.baselines.values()) {
      baseline.mean = null;
      baseline.variance = null;
      baseline.history = [];
    }
    this.perturbations = [];
    this.sessionStart = Date.now();
  }

  /**
   * Serialize for persistence
   */
  toJSON() {
    const baselines = {};
    for (const [name, baseline] of this.baselines) {
      baselines[name] = {
        mean: baseline.mean,
        variance: baseline.variance,
        history: baseline.history,
      };
    }

    return {
      baselines,
      perturbations: this.perturbations,
      sessionStart: this.sessionStart,
    };
  }

  /**
   * Restore from persistence
   */
  static fromJSON(data) {
    const tracker = new HomeostasisTracker();

    for (const [name, saved] of Object.entries(data.baselines || {})) {
      const baseline = new MetricBaseline(name);
      baseline.mean = saved.mean;
      baseline.variance = saved.variance;
      baseline.history = saved.history || [];
      tracker.baselines.set(name, baseline);
    }

    tracker.perturbations = data.perturbations || [];
    tracker.sessionStart = data.sessionStart || Date.now();

    return tracker;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FACTORY & SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

export function createHomeostasisTracker(options = {}) {
  return new HomeostasisTracker(options);
}

let _homeostasisTracker = null;

export function getHomeostasisTracker() {
  if (!_homeostasisTracker) {
    _homeostasisTracker = new HomeostasisTracker();
  }
  return _homeostasisTracker;
}

export function resetHomeostasisTracker() {
  _homeostasisTracker = null;
}

export default {
  MetricBaseline,
  HomeostasisTracker,
  createHomeostasisTracker,
  getHomeostasisTracker,
  resetHomeostasisTracker,
  HOMEOSTASIS_CONFIG,
};
