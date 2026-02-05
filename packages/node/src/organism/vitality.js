/**
 * Vitality - Composite Life Force Index
 *
 * "Le chien vit quand il court" - κυνικός
 *
 * Combines all 6 organism dimensions into a single vitality score:
 * - Metabolism (energy consumption efficiency)
 * - Homeostasis (stability)
 * - Temperature (thermodynamic state)
 * - Efficiency (useful work / total energy)
 * - Growth (adaptation rate)
 * - Resilience (recovery capability)
 *
 * Vitality = φ-weighted geometric mean of all dimensions
 * Max vitality = φ⁻¹ (61.8%) - never claim perfect health
 *
 * @module @cynic/node/organism/vitality
 */

'use strict';

import { PHI, PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

import { getMetabolismTracker } from './metabolism.js';
import { getThermodynamicState } from './thermodynamics.js';
import { getHomeostasisTracker } from './homeostasis.js';
import { getGrowthTracker } from './growth.js';
import { getResilienceTracker } from './resilience.js';

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

export const VITALITY_CONFIG = {
  // Dimension weights (φ-derived, sum to 1)
  WEIGHTS: {
    metabolism: PHI_INV_3,       // 0.236
    homeostasis: PHI_INV_2,      // 0.382
    temperature: PHI_INV_3,      // 0.236 (inverse - lower is better)
    efficiency: PHI_INV,         // 0.618
    growth: PHI_INV_2,           // 0.382
    resilience: PHI_INV,         // 0.618
  },

  // Normalize weights to sum to 1
  get NORMALIZED_WEIGHTS() {
    const total = Object.values(this.WEIGHTS).reduce((a, b) => a + b, 0);
    const normalized = {};
    for (const [key, value] of Object.entries(this.WEIGHTS)) {
      normalized[key] = value / total;
    }
    return normalized;
  },

  // Vitality thresholds
  VITALITY_THRIVING: PHI_INV,     // > 61.8% = thriving
  VITALITY_HEALTHY: PHI_INV_2,    // > 38.2% = healthy
  VITALITY_STRUGGLING: PHI_INV_3, // > 23.6% = struggling
  // < 23.6% = critical

  // Maximum vitality (φ-bounded)
  MAX_VITALITY: PHI_INV,

  // Update interval for continuous monitoring (ms)
  UPDATE_INTERVAL: 5000,
};

// ═══════════════════════════════════════════════════════════════════════════════
// DIMENSION SCORES
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Get all dimension scores
 * @returns {Object} Scores for each dimension
 */
export function getDimensionScores() {
  const metabolism = getMetabolismTracker();
  const thermo = getThermodynamicState();
  const homeostasis = getHomeostasisTracker();
  const growth = getGrowthTracker();
  const resilience = getResilienceTracker();

  return {
    metabolism: metabolism.getHealth().score,
    homeostasis: homeostasis.getHealth().score,
    temperature: thermo.getHealth().score,
    efficiency: thermo.getEfficiency(),
    growth: growth.getHealth().score,
    resilience: resilience.getHealth().score,
  };
}

/**
 * Get dimension statuses
 * @returns {Object}
 */
export function getDimensionStatuses() {
  const metabolism = getMetabolismTracker();
  const thermo = getThermodynamicState();
  const homeostasis = getHomeostasisTracker();
  const growth = getGrowthTracker();
  const resilience = getResilienceTracker();

  return {
    metabolism: metabolism.getHealth().status,
    homeostasis: homeostasis.getStatus(),
    temperature: thermo.getTemperatureStatus(),
    efficiency: thermo.getEfficiency() >= PHI_INV_2 ? 'efficient' : 'inefficient',
    growth: growth.getStatus(),
    resilience: resilience.getStatus(),
  };
}

// ═══════════════════════════════════════════════════════════════════════════════
// VITALITY CALCULATOR
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Calculate composite vitality score
 * Uses weighted geometric mean, φ-bounded
 * @returns {number} 0 to φ⁻¹
 */
export function calculateVitality() {
  const scores = getDimensionScores();
  const weights = VITALITY_CONFIG.NORMALIZED_WEIGHTS;

  // Geometric mean: ∏(score_i ^ weight_i)
  let product = 1;
  let totalWeight = 0;

  for (const [dimension, score] of Object.entries(scores)) {
    const weight = weights[dimension] || 0;
    if (weight > 0 && score > 0) {
      product *= Math.pow(score, weight);
      totalWeight += weight;
    }
  }

  // Normalize if not all dimensions present
  const vitality = totalWeight > 0 ? Math.pow(product, 1 / totalWeight) : 0;

  // φ-bound
  return Math.min(VITALITY_CONFIG.MAX_VITALITY, vitality);
}

/**
 * Get vitality status
 * @returns {string}
 */
export function getVitalityStatus() {
  const v = calculateVitality();

  if (v >= VITALITY_CONFIG.VITALITY_THRIVING) return 'thriving';
  if (v >= VITALITY_CONFIG.VITALITY_HEALTHY) return 'healthy';
  if (v >= VITALITY_CONFIG.VITALITY_STRUGGLING) return 'struggling';
  return 'critical';
}

// ═══════════════════════════════════════════════════════════════════════════════
// VITALITY MONITOR
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Continuous vitality monitoring
 */
export class VitalityMonitor {
  constructor(options = {}) {
    this.interval = options.interval || VITALITY_CONFIG.UPDATE_INTERVAL;
    this.history = [];
    this.maxHistory = options.maxHistory || 1000;

    // Listeners
    this.listeners = new Set();

    // Monitoring state
    this.isMonitoring = false;
    this.timer = null;

    // Session
    this.sessionStart = Date.now();
  }

  /**
   * Start monitoring
   */
  start() {
    if (this.isMonitoring) return;

    this.isMonitoring = true;
    this._tick();
  }

  /**
   * Stop monitoring
   */
  stop() {
    this.isMonitoring = false;
    if (this.timer) {
      clearTimeout(this.timer);
      this.timer = null;
    }
  }

  /**
   * Single monitoring tick
   * @private
   */
  _tick() {
    if (!this.isMonitoring) return;

    const snapshot = this.getSnapshot();
    this.history.push(snapshot);

    // Prune history
    while (this.history.length > this.maxHistory) {
      this.history.shift();
    }

    // Notify listeners
    for (const listener of this.listeners) {
      try {
        listener(snapshot);
      } catch (e) {
        // Ignore listener errors
      }
    }

    // Schedule next tick
    this.timer = setTimeout(() => this._tick(), this.interval);
  }

  /**
   * Get current snapshot
   * @returns {Object}
   */
  getSnapshot() {
    return {
      timestamp: Date.now(),
      vitality: calculateVitality(),
      status: getVitalityStatus(),
      dimensions: getDimensionScores(),
      statuses: getDimensionStatuses(),
    };
  }

  /**
   * Add listener for vitality updates
   * @param {Function} callback
   */
  addListener(callback) {
    this.listeners.add(callback);
  }

  /**
   * Remove listener
   * @param {Function} callback
   */
  removeListener(callback) {
    this.listeners.delete(callback);
  }

  /**
   * Get vitality trend
   * @param {number} [windowSize=10] - Number of snapshots to analyze
   * @returns {string} 'improving', 'stable', 'declining'
   */
  getTrend(windowSize = 10) {
    if (this.history.length < 2) return 'stable';

    const recent = this.history.slice(-windowSize);
    if (recent.length < 2) return 'stable';

    // Calculate slope
    const values = recent.map(s => s.vitality);
    const n = values.length;
    const sumX = (n * (n - 1)) / 2;
    const sumY = values.reduce((a, b) => a + b, 0);
    const sumXY = values.reduce((sum, y, x) => sum + x * y, 0);
    const sumX2 = (n * (n - 1) * (2 * n - 1)) / 6;

    const slope = (n * sumXY - sumX * sumY) / (n * sumX2 - sumX * sumX);

    // Classify trend
    const threshold = 0.01; // 1% change per snapshot
    if (slope > threshold) return 'improving';
    if (slope < -threshold) return 'declining';
    return 'stable';
  }

  /**
   * Get average vitality over window
   * @param {number} [windowMs=60000] - Time window in ms
   * @returns {number}
   */
  getAverageVitality(windowMs = 60000) {
    const cutoff = Date.now() - windowMs;
    const recent = this.history.filter(s => s.timestamp >= cutoff);

    if (recent.length === 0) return calculateVitality();

    const sum = recent.reduce((a, s) => a + s.vitality, 0);
    return sum / recent.length;
  }

  /**
   * Get minimum vitality over window
   * @param {number} [windowMs=60000]
   * @returns {number}
   */
  getMinVitality(windowMs = 60000) {
    const cutoff = Date.now() - windowMs;
    const recent = this.history.filter(s => s.timestamp >= cutoff);

    if (recent.length === 0) return calculateVitality();

    return Math.min(...recent.map(s => s.vitality));
  }

  /**
   * Check if vitality is declining
   * @returns {boolean}
   */
  isDeclining() {
    return this.getTrend() === 'declining';
  }

  /**
   * Check if vitality is critical
   * @returns {boolean}
   */
  isCritical() {
    return calculateVitality() < VITALITY_CONFIG.VITALITY_STRUGGLING;
  }

  /**
   * Get comprehensive health report
   * @returns {Object}
   */
  getHealthReport() {
    const current = this.getSnapshot();
    const trend = this.getTrend();
    const avgVitality = this.getAverageVitality();
    const minVitality = this.getMinVitality();

    // Find weakest dimension
    const dimensions = current.dimensions;
    let weakestDimension = null;
    let weakestScore = Infinity;

    for (const [dim, score] of Object.entries(dimensions)) {
      if (score < weakestScore) {
        weakestScore = score;
        weakestDimension = dim;
      }
    }

    // Recommendations based on state
    const recommendations = [];

    if (current.status === 'critical') {
      recommendations.push('Immediate intervention needed');
    }

    if (trend === 'declining') {
      recommendations.push('Vitality declining - investigate root cause');
    }

    if (weakestDimension) {
      const statusMap = {
        metabolism: 'Optimize token usage and reduce latency',
        homeostasis: 'Stabilize performance variance',
        temperature: 'Allow system to cool down',
        efficiency: 'Reduce errors and retries',
        growth: 'Encourage learning and pattern acquisition',
        resilience: 'Address unrecovered incidents',
      };
      recommendations.push(`Weakest: ${weakestDimension} - ${statusMap[weakestDimension]}`);
    }

    return {
      current,
      trend,
      statistics: {
        avgVitality,
        minVitality,
        snapshotCount: this.history.length,
      },
      weakest: {
        dimension: weakestDimension,
        score: weakestScore,
      },
      recommendations,
      session: {
        durationMs: Date.now() - this.sessionStart,
        isMonitoring: this.isMonitoring,
      },
    };
  }

  /**
   * Reset monitor
   */
  reset() {
    this.stop();
    this.history = [];
    this.sessionStart = Date.now();
  }

  /**
   * Serialize
   */
  toJSON() {
    return {
      history: this.history.slice(-100), // Keep last 100 only
      sessionStart: this.sessionStart,
    };
  }

  /**
   * Restore
   */
  static fromJSON(data) {
    const monitor = new VitalityMonitor();
    monitor.history = data.history || [];
    monitor.sessionStart = data.sessionStart || Date.now();
    return monitor;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FACTORY & SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

export function createVitalityMonitor(options = {}) {
  return new VitalityMonitor(options);
}

let _vitalityMonitor = null;

export function getVitalityMonitor() {
  if (!_vitalityMonitor) {
    _vitalityMonitor = new VitalityMonitor();
  }
  return _vitalityMonitor;
}

export function resetVitalityMonitor() {
  if (_vitalityMonitor) {
    _vitalityMonitor.stop();
  }
  _vitalityMonitor = null;
}

// ═══════════════════════════════════════════════════════════════════════════════
// QUICK ACCESS
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Get quick vitality summary
 * @returns {Object}
 */
export function getVitalitySummary() {
  return {
    vitality: calculateVitality(),
    status: getVitalityStatus(),
    dimensions: getDimensionScores(),
  };
}

export default {
  VitalityMonitor,
  calculateVitality,
  getVitalityStatus,
  getDimensionScores,
  getDimensionStatuses,
  getVitalitySummary,
  createVitalityMonitor,
  getVitalityMonitor,
  resetVitalityMonitor,
  VITALITY_CONFIG,
};
