/**
 * Metabolism - Energy Consumption Tracking
 *
 * "Le chien mange pour vivre, pas l'inverse" - κυνικός
 *
 * Measures how CYNIC consumes resources:
 * - Tokens (LLM input/output)
 * - CPU cycles
 * - Latency (time cost)
 * - Memory usage
 *
 * Metabolic rate = resources consumed / time elapsed
 * High metabolism without output = inefficiency (disease)
 *
 * @module @cynic/node/organism/metabolism
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import { computeStats } from '../inference/gaussian.js';

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

export const METABOLISM_CONFIG = {
  // Sampling window (ms)
  WINDOW_SIZE: 60000, // 1 minute

  // Maximum samples to keep
  MAX_SAMPLES: 1000,

  // Metabolic rate thresholds (φ-aligned)
  RATE_HEALTHY: PHI_INV,      // 61.8% of max = healthy
  RATE_WARNING: PHI_INV_2,    // 38.2% of max = warning
  RATE_CRITICAL: PHI_INV_3,   // 23.6% of max = critical

  // Token costs (approximate)
  TOKEN_INPUT_COST: 1,
  TOKEN_OUTPUT_COST: 3,       // Output tokens cost more

  // Efficiency thresholds
  EFFICIENCY_MIN: PHI_INV_3,  // 23.6% minimum useful efficiency
  EFFICIENCY_MAX: PHI_INV,    // 61.8% max (φ-bounded)
};

// ═══════════════════════════════════════════════════════════════════════════════
// METABOLIC SAMPLE
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Single metabolic measurement
 */
export class MetabolicSample {
  /**
   * @param {Object} data
   * @param {number} data.tokensIn - Input tokens consumed
   * @param {number} data.tokensOut - Output tokens generated
   * @param {number} data.latencyMs - Time taken (ms)
   * @param {number} [data.cpuPercent] - CPU usage (0-100)
   * @param {number} [data.memoryMB] - Memory usage (MB)
   * @param {string} [data.operation] - What operation was performed
   */
  constructor(data) {
    this.timestamp = Date.now();
    this.tokensIn = data.tokensIn || 0;
    this.tokensOut = data.tokensOut || 0;
    this.latencyMs = data.latencyMs || 0;
    this.cpuPercent = data.cpuPercent ?? null;
    this.memoryMB = data.memoryMB ?? null;
    this.operation = data.operation || 'unknown';

    // Computed
    this.totalTokens = this.tokensIn + this.tokensOut;
    this.weightedTokens = this.tokensIn * METABOLISM_CONFIG.TOKEN_INPUT_COST +
                          this.tokensOut * METABOLISM_CONFIG.TOKEN_OUTPUT_COST;
  }

  /**
   * Metabolic cost (energy spent)
   * @returns {number}
   */
  get cost() {
    return this.weightedTokens + (this.latencyMs / 100);
  }

  /**
   * Output efficiency (output / input)
   * @returns {number}
   */
  get efficiency() {
    if (this.tokensIn === 0) return 0;
    const raw = this.tokensOut / this.tokensIn;
    return Math.min(PHI_INV, raw); // φ-bounded
  }

  toJSON() {
    return {
      timestamp: this.timestamp,
      tokensIn: this.tokensIn,
      tokensOut: this.tokensOut,
      latencyMs: this.latencyMs,
      cpuPercent: this.cpuPercent,
      memoryMB: this.memoryMB,
      operation: this.operation,
      cost: this.cost,
      efficiency: this.efficiency,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// METABOLISM TRACKER
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Tracks metabolic activity over time
 */
export class MetabolismTracker {
  constructor(options = {}) {
    this.windowSize = options.windowSize || METABOLISM_CONFIG.WINDOW_SIZE;
    this.maxSamples = options.maxSamples || METABOLISM_CONFIG.MAX_SAMPLES;

    /** @type {MetabolicSample[]} */
    this.samples = [];

    // Aggregates
    this.totalTokensIn = 0;
    this.totalTokensOut = 0;
    this.totalLatencyMs = 0;
    this.totalOperations = 0;

    // Session tracking
    this.sessionStart = Date.now();
  }

  /**
   * Record a metabolic event
   * @param {Object} data - Sample data
   * @returns {MetabolicSample}
   */
  record(data) {
    const sample = new MetabolicSample(data);

    this.samples.push(sample);
    this.totalTokensIn += sample.tokensIn;
    this.totalTokensOut += sample.tokensOut;
    this.totalLatencyMs += sample.latencyMs;
    this.totalOperations++;

    // Prune old samples
    this._prune();

    return sample;
  }

  /**
   * Remove samples outside window or over limit
   * @private
   */
  _prune() {
    const cutoff = Date.now() - this.windowSize;

    // Remove old samples
    while (this.samples.length > 0 && this.samples[0].timestamp < cutoff) {
      this.samples.shift();
    }

    // Limit total samples
    while (this.samples.length > this.maxSamples) {
      this.samples.shift();
    }
  }

  /**
   * Get samples within time window
   * @param {number} [windowMs] - Window size (default: configured window)
   * @returns {MetabolicSample[]}
   */
  getRecentSamples(windowMs = this.windowSize) {
    const cutoff = Date.now() - windowMs;
    return this.samples.filter(s => s.timestamp >= cutoff);
  }

  /**
   * Calculate current metabolic rate
   * Rate = total weighted tokens / time window (tokens per second)
   * @returns {number}
   */
  getMetabolicRate() {
    const recent = this.getRecentSamples();
    if (recent.length === 0) return 0;

    const totalWeighted = recent.reduce((sum, s) => sum + s.weightedTokens, 0);
    const windowSec = this.windowSize / 1000;

    return totalWeighted / windowSec;
  }

  /**
   * Calculate metabolic efficiency
   * Efficiency = useful output / total energy spent
   * @returns {number} 0 to φ⁻¹ (max 61.8%)
   */
  getEfficiency() {
    const recent = this.getRecentSamples();
    if (recent.length === 0) return 0;

    const totalOut = recent.reduce((sum, s) => sum + s.tokensOut, 0);
    const totalCost = recent.reduce((sum, s) => sum + s.cost, 0);

    if (totalCost === 0) return 0;

    const raw = totalOut / totalCost;
    return Math.min(PHI_INV, raw); // φ-bounded
  }

  /**
   * Calculate metabolic health score
   * @returns {Object} {score, status, details}
   */
  getHealth() {
    const recent = this.getRecentSamples();
    if (recent.length === 0) {
      return {
        score: 0,
        status: 'dormant',
        details: { message: 'No metabolic activity' },
      };
    }

    const rate = this.getMetabolicRate();
    const efficiency = this.getEfficiency();
    const costs = recent.map(s => s.cost);
    const stats = computeStats(costs);

    // Coefficient of variation (stability)
    const cv = stats.std / (stats.mean || 1);
    const stability = Math.max(0, 1 - cv);

    // Health score: efficiency × stability, φ-bounded
    const rawScore = efficiency * stability;
    const score = Math.min(PHI_INV, rawScore);

    // Status determination
    let status;
    if (score >= METABOLISM_CONFIG.RATE_HEALTHY * PHI_INV) {
      status = 'healthy';
    } else if (score >= METABOLISM_CONFIG.RATE_WARNING * PHI_INV) {
      status = 'stressed';
    } else if (score >= METABOLISM_CONFIG.RATE_CRITICAL * PHI_INV) {
      status = 'strained';
    } else {
      status = 'critical';
    }

    return {
      score,
      status,
      details: {
        rate,
        efficiency,
        stability,
        sampleCount: recent.length,
        avgLatencyMs: stats.mean,
        stdLatencyMs: stats.std,
      },
    };
  }

  /**
   * Get metabolic statistics
   * @returns {Object}
   */
  getStats() {
    const sessionDurationMs = Date.now() - this.sessionStart;
    const recent = this.getRecentSamples();

    return {
      session: {
        durationMs: sessionDurationMs,
        totalOperations: this.totalOperations,
        totalTokensIn: this.totalTokensIn,
        totalTokensOut: this.totalTokensOut,
        totalLatencyMs: this.totalLatencyMs,
        avgTokensPerOp: this.totalOperations > 0
          ? (this.totalTokensIn + this.totalTokensOut) / this.totalOperations
          : 0,
      },
      recent: {
        sampleCount: recent.length,
        windowMs: this.windowSize,
        rate: this.getMetabolicRate(),
        efficiency: this.getEfficiency(),
      },
      health: this.getHealth(),
    };
  }

  /**
   * Reset tracker
   */
  reset() {
    this.samples = [];
    this.totalTokensIn = 0;
    this.totalTokensOut = 0;
    this.totalLatencyMs = 0;
    this.totalOperations = 0;
    this.sessionStart = Date.now();
  }

  /**
   * Serialize for persistence
   */
  toJSON() {
    return {
      samples: this.samples.map(s => s.toJSON()),
      totalTokensIn: this.totalTokensIn,
      totalTokensOut: this.totalTokensOut,
      totalLatencyMs: this.totalLatencyMs,
      totalOperations: this.totalOperations,
      sessionStart: this.sessionStart,
    };
  }

  /**
   * Restore from persistence
   */
  static fromJSON(data) {
    const tracker = new MetabolismTracker();
    tracker.samples = (data.samples || []).map(s => {
      const sample = new MetabolicSample(s);
      sample.timestamp = s.timestamp;
      return sample;
    });
    tracker.totalTokensIn = data.totalTokensIn || 0;
    tracker.totalTokensOut = data.totalTokensOut || 0;
    tracker.totalLatencyMs = data.totalLatencyMs || 0;
    tracker.totalOperations = data.totalOperations || 0;
    tracker.sessionStart = data.sessionStart || Date.now();
    return tracker;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FACTORY
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Create metabolism tracker
 * @param {Object} [options]
 * @returns {MetabolismTracker}
 */
export function createMetabolismTracker(options = {}) {
  return new MetabolismTracker(options);
}

// Singleton
let _metabolismTracker = null;

/**
 * Get singleton metabolism tracker
 * @returns {MetabolismTracker}
 */
export function getMetabolismTracker() {
  if (!_metabolismTracker) {
    _metabolismTracker = new MetabolismTracker();
  }
  return _metabolismTracker;
}

/**
 * Reset singleton for testing
 */
export function resetMetabolismTracker() {
  _metabolismTracker = null;
}

export default {
  MetabolicSample,
  MetabolismTracker,
  createMetabolismTracker,
  getMetabolismTracker,
  resetMetabolismTracker,
  METABOLISM_CONFIG,
};
