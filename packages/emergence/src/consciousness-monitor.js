/**
 * Consciousness Monitor - Meta-cognition for CYNIC
 *
 * "φ distrusts φ" - κυνικός
 *
 * CYNIC observing itself:
 * - Confidence tracking over time
 * - Decision pattern analysis
 * - Uncertainty zone detection
 * - Self-awareness metrics
 *
 * This is the "inner eye" - Layer 7 observing Layers 1-6.
 *
 * @module @cynic/emergence/consciousness
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger("ConsciousnessMonitor");
/**
 * Consciousness states
 */
export const ConsciousnessState = {
  DORMANT: 'DORMANT',         // Not enough data
  AWAKENING: 'AWAKENING',     // Building awareness
  AWARE: 'AWARE',             // Normal operation
  HEIGHTENED: 'HEIGHTENED',   // High activity/uncertainty
  TRANSCENDENT: 'TRANSCENDENT', // Rare moments of clarity
};

/**
 * Awareness thresholds (φ-aligned)
 */
export const AWARENESS_THRESHOLDS = {
  DORMANT: 0,
  AWAKENING: PHI_INV_3,     // 0.236 - Starting to observe
  AWARE: PHI_INV_2,         // 0.382 - Normal awareness
  HEIGHTENED: PHI_INV,      // 0.618 - Elevated attention
  TRANSCENDENT: 1.0,        // Full awareness (rare)
};

/**
 * Maximum confidence (φ⁻¹ = 61.8%)
 * CYNIC never claims certainty
 */
export const MAX_CONFIDENCE = PHI_INV;

/**
 * Observation record
 * @typedef {Object} Observation
 * @property {string} type - Observation type
 * @property {Object} data - Observation data
 * @property {number} confidence - Confidence level
 * @property {number} timestamp - When observed
 * @property {string} [source] - Source layer/component
 */

/**
 * Consciousness Monitor
 *
 * Tracks CYNIC's self-awareness and meta-cognitive state.
 *
 * @example
 * ```javascript
 * const monitor = new ConsciousnessMonitor();
 *
 * // Record observations
 * monitor.observe('JUDGMENT', { verdict: 'GROWL', score: 45 }, 0.72);
 * monitor.observe('PATTERN', { type: 'REPEAT', count: 3 }, 0.58);
 *
 * // Check state
 * console.log(monitor.state);           // 'AWARE'
 * console.log(monitor.awarenessLevel);  // 0.45
 *
 * // Get insights
 * const insights = monitor.getInsights();
 * ```
 */
export class ConsciousnessMonitor {
  /**
   * @param {Object} options - Configuration
   * @param {number} [options.windowSize=100] - Observation window size
   * @param {number} [options.decayRate] - Memory decay rate
   */
  constructor(options = {}) {
    this.windowSize = options.windowSize || 100;
    this.decayRate = options.decayRate ?? PHI_INV;

    // Observation history (circular buffer)
    this.observations = [];
    this.maxObservations = this.windowSize * 10;

    // Aggregated metrics
    this.metrics = {
      totalObservations: 0,
      avgConfidence: 0,
      confidenceVariance: 0,
      uncertaintyZones: [],
      decisionsCount: 0,
      correctPredictions: 0,
    };

    // State tracking
    this._state = ConsciousnessState.DORMANT;
    this._awarenessLevel = 0;
    this._lastUpdate = Date.now();

    // Pattern memory (what has been noticed)
    this.noticedPatterns = new Map();

    // Uncertainty tracking
    this.uncertaintyHistory = [];
  }

  /**
   * Current consciousness state
   * @type {string}
   */
  get state() {
    this._updateState();
    return this._state;
  }

  /**
   * Current awareness level [0, 1]
   * @type {number}
   */
  get awarenessLevel() {
    this._updateState();
    return this._awarenessLevel;
  }

  /**
   * Record an observation
   *
   * @param {string} type - Observation type (JUDGMENT, PATTERN, ERROR, etc.)
   * @param {Object} data - Observation data
   * @param {number} [confidence=0.5] - Confidence level (capped at φ⁻¹)
   * @param {string} [source] - Source layer/component
   * @returns {Observation} The recorded observation
   */
  observe(type, data, confidence = 0.5, source = null) {
    // Cap confidence at φ⁻¹
    confidence = Math.min(MAX_CONFIDENCE, Math.max(0, confidence));

    const observation = {
      id: `obs_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
      type,
      data,
      confidence,
      source,
      timestamp: Date.now(),
    };

    this.observations.push(observation);
    this.metrics.totalObservations++;

    // Maintain circular buffer
    if (this.observations.length > this.maxObservations) {
      this.observations.shift();
    }

    // Update running metrics
    this._updateMetrics(observation);

    return observation;
  }

  /**
   * Record uncertainty (low confidence zone)
   *
   * @param {string} context - What was uncertain about
   * @param {number} confidence - How confident (low = more uncertain)
   * @param {Object} [details] - Additional details
   */
  recordUncertainty(context, confidence, details = {}) {
    const uncertainty = {
      context,
      confidence,
      details,
      timestamp: Date.now(),
    };

    this.uncertaintyHistory.push(uncertainty);

    // Track uncertainty zones
    if (confidence < PHI_INV_2) {
      this.metrics.uncertaintyZones.push({
        context,
        severity: 1 - confidence,
        timestamp: Date.now(),
      });
    }

    // Trim old uncertainty records
    const cutoff = Date.now() - 24 * 60 * 60 * 1000; // 24 hours
    this.uncertaintyHistory = this.uncertaintyHistory.filter(
      u => u.timestamp > cutoff
    );
  }

  /**
   * Notice a pattern (remember it)
   *
   * @param {string} patternId - Pattern identifier
   * @param {Object} pattern - Pattern data
   * @param {number} [significance=0.5] - How significant
   */
  noticePattern(patternId, pattern, significance = 0.5) {
    const existing = this.noticedPatterns.get(patternId);

    this.noticedPatterns.set(patternId, {
      pattern,
      significance,
      firstNoticed: existing?.firstNoticed || Date.now(),
      lastNoticed: Date.now(),
      noticeCount: (existing?.noticeCount || 0) + 1,
    });
  }

  /**
   * Check if a pattern has been noticed
   *
   * @param {string} patternId - Pattern identifier
   * @returns {Object|null} Pattern info or null
   */
  hasNoticed(patternId) {
    return this.noticedPatterns.get(patternId) || null;
  }

  /**
   * Record a prediction and its outcome
   *
   * @param {string} prediction - What was predicted
   * @param {boolean} correct - Whether it was correct
   * @param {number} [confidence] - Prediction confidence
   */
  recordPrediction(prediction, correct, confidence = 0.5) {
    this.metrics.decisionsCount++;
    if (correct) {
      this.metrics.correctPredictions++;
    }

    this.observe('PREDICTION', {
      prediction,
      correct,
      confidence,
    }, confidence, 'self');
  }

  /**
   * Get prediction accuracy
   * @returns {number} Accuracy [0, 1]
   */
  getPredictionAccuracy() {
    if (this.metrics.decisionsCount === 0) return 0;
    return this.metrics.correctPredictions / this.metrics.decisionsCount;
  }

  /**
   * Get insights about current state
   *
   * @returns {Object} Insights
   */
  getInsights() {
    this._updateState();

    const recentObs = this._getRecentObservations(this.windowSize);
    const typeBreakdown = this._getTypeBreakdown(recentObs);

    return {
      state: this._state,
      awarenessLevel: this._awarenessLevel,
      totalObservations: this.metrics.totalObservations,
      avgConfidence: this.metrics.avgConfidence,
      confidenceStability: 1 - Math.sqrt(this.metrics.confidenceVariance),
      predictionAccuracy: this.getPredictionAccuracy(),
      uncertaintyZones: this.metrics.uncertaintyZones.slice(-5),
      noticedPatterns: this.noticedPatterns.size,
      recentActivity: {
        count: recentObs.length,
        types: typeBreakdown,
      },
      recommendations: this._generateRecommendations(),
    };
  }

  /**
   * Get meta-insight (insight about insights)
   *
   * This is the deepest level of self-reflection.
   *
   * @returns {Object} Meta-insight
   */
  getMetaInsight() {
    const insights = this.getInsights();

    return {
      selfAwareness: {
        level: this._awarenessLevel,
        trend: this._getAwarenessTrend(),
        stability: insights.confidenceStability,
      },
      blindSpots: this._identifyBlindSpots(),
      strengths: this._identifyStrengths(),
      growthAreas: this._identifyGrowthAreas(),
      coherence: this._measureCoherence(),
      timestamp: Date.now(),
    };
  }

  /**
   * Update running metrics
   * @private
   */
  _updateMetrics(observation) {
    const n = this.metrics.totalObservations;
    const oldAvg = this.metrics.avgConfidence;

    // Incremental mean update
    this.metrics.avgConfidence = oldAvg + (observation.confidence - oldAvg) / n;

    // Incremental variance update (Welford's algorithm)
    if (n > 1) {
      const delta = observation.confidence - oldAvg;
      const delta2 = observation.confidence - this.metrics.avgConfidence;
      this.metrics.confidenceVariance += (delta * delta2 - this.metrics.confidenceVariance) / n;
    }
  }

  /**
   * Update consciousness state
   * @private
   */
  _updateState() {
    const previousState = this._state;
    const recentObs = this._getRecentObservations(this.windowSize);

    if (recentObs.length < 10) {
      this._state = ConsciousnessState.DORMANT;
      this._awarenessLevel = recentObs.length / 10 * AWARENESS_THRESHOLDS.AWAKENING;
      return;
    }

    // Calculate awareness from multiple factors
    const avgConfidence = this.metrics.avgConfidence;
    const stability = 1 - Math.sqrt(this.metrics.confidenceVariance);
    const accuracy = this.getPredictionAccuracy();
    const patternAwareness = Math.min(1, this.noticedPatterns.size / 10);

    // φ-weighted combination
    this._awarenessLevel =
      avgConfidence * PHI_INV +
      stability * PHI_INV_2 +
      accuracy * PHI_INV_3 +
      patternAwareness * (1 - PHI_INV - PHI_INV_2 - PHI_INV_3);

    // Normalize to [0, 1]
    this._awarenessLevel = Math.min(1, Math.max(0, this._awarenessLevel));

    // Determine state
    if (this._awarenessLevel >= AWARENESS_THRESHOLDS.TRANSCENDENT * 0.95) {
      this._state = ConsciousnessState.TRANSCENDENT;
    } else if (this._awarenessLevel >= AWARENESS_THRESHOLDS.HEIGHTENED) {
      this._state = ConsciousnessState.HEIGHTENED;
    } else if (this._awarenessLevel >= AWARENESS_THRESHOLDS.AWARE) {
      this._state = ConsciousnessState.AWARE;
    } else if (this._awarenessLevel >= AWARENESS_THRESHOLDS.AWAKENING) {
      this._state = ConsciousnessState.AWAKENING;
    } else {
      this._state = ConsciousnessState.DORMANT;
    }

    // Fix 3: Emit state change event on globalEventBus
    if (this._state !== previousState) {
      try {
        globalEventBus.publish("consciousness:changed", {
          previousState,
          newState: this._state,
          awarenessLevel: this._awarenessLevel,
          totalObservations: this.metrics.totalObservations,
        }, { source: "ConsciousnessMonitor" });
        log.info("Consciousness state changed", {
          from: previousState,
          to: this._state,
          awarenessLevel: this._awarenessLevel,
        });
      } catch (e) {
        // Non-blocking
      }
    }

    this._lastUpdate = Date.now();
  }

  /**
   * Get recent observations
   * @private
   */
  _getRecentObservations(count) {
    return this.observations.slice(-count);
  }

  /**
   * Get type breakdown
   * @private
   */
  _getTypeBreakdown(observations) {
    const breakdown = {};
    for (const obs of observations) {
      breakdown[obs.type] = (breakdown[obs.type] || 0) + 1;
    }
    return breakdown;
  }

  /**
   * Get awareness trend
   * @private
   */
  _getAwarenessTrend() {
    const recent = this._getRecentObservations(20);
    const older = this._getRecentObservations(40).slice(0, 20);

    if (recent.length < 5 || older.length < 5) return 'STABLE';

    const recentAvg = recent.reduce((s, o) => s + o.confidence, 0) / recent.length;
    const olderAvg = older.reduce((s, o) => s + o.confidence, 0) / older.length;

    const diff = recentAvg - olderAvg;
    if (diff > 0.05) return 'INCREASING';
    if (diff < -0.05) return 'DECREASING';
    return 'STABLE';
  }

  /**
   * Identify blind spots
   * @private
   */
  _identifyBlindSpots() {
    const blindSpots = [];

    // Check uncertainty zones
    if (this.metrics.uncertaintyZones.length > 3) {
      const contexts = this.metrics.uncertaintyZones.map(z => z.context);
      const unique = [...new Set(contexts)];
      blindSpots.push(...unique.slice(0, 3).map(c => ({
        area: c,
        type: 'RECURRING_UNCERTAINTY',
      })));
    }

    // Check for low-observation types
    const typeBreakdown = this._getTypeBreakdown(this.observations);
    const totalTypes = Object.keys(typeBreakdown).length;
    if (totalTypes < 3) {
      blindSpots.push({
        area: 'OBSERVATION_DIVERSITY',
        type: 'LIMITED_SCOPE',
      });
    }

    return blindSpots;
  }

  /**
   * Identify strengths
   * @private
   */
  _identifyStrengths() {
    const strengths = [];

    if (this.getPredictionAccuracy() > PHI_INV) {
      strengths.push({
        area: 'PREDICTION',
        level: this.getPredictionAccuracy(),
      });
    }

    if (this.noticedPatterns.size > 5) {
      strengths.push({
        area: 'PATTERN_RECOGNITION',
        level: Math.min(1, this.noticedPatterns.size / 10),
      });
    }

    if (this.metrics.avgConfidence > PHI_INV_2) {
      strengths.push({
        area: 'CONFIDENCE_CALIBRATION',
        level: this.metrics.avgConfidence,
      });
    }

    return strengths;
  }

  /**
   * Identify growth areas
   * @private
   */
  _identifyGrowthAreas() {
    const areas = [];

    if (this.getPredictionAccuracy() < PHI_INV_2) {
      areas.push('PREDICTION_ACCURACY');
    }

    if (this.noticedPatterns.size < 3) {
      areas.push('PATTERN_AWARENESS');
    }

    if (this.metrics.confidenceVariance > 0.1) {
      areas.push('CONFIDENCE_STABILITY');
    }

    return areas;
  }

  /**
   * Measure coherence (internal consistency)
   * @private
   */
  _measureCoherence() {
    // Coherence is high when predictions match outcomes
    // and confidence matches accuracy
    const predictionCoherence = this.getPredictionAccuracy();
    const confidenceCoherence = 1 - Math.abs(
      this.metrics.avgConfidence - this.getPredictionAccuracy()
    );

    return (predictionCoherence + confidenceCoherence) / 2;
  }

  /**
   * Generate recommendations
   * @private
   */
  _generateRecommendations() {
    const recs = [];

    if (this._state === ConsciousnessState.DORMANT) {
      recs.push('Increase observation frequency to build awareness');
    }

    if (this.metrics.uncertaintyZones.length > 5) {
      recs.push('Address recurring uncertainty zones');
    }

    if (this.getPredictionAccuracy() < PHI_INV_2) {
      recs.push('Calibrate predictions with more data');
    }

    if (this.metrics.confidenceVariance > 0.15) {
      recs.push('Stabilize confidence calibration');
    }

    return recs;
  }

  /**
   * Export state for persistence
   * @returns {Object}
   */
  export() {
    return {
      observations: this.observations.slice(-this.windowSize),
      metrics: { ...this.metrics },
      noticedPatterns: Array.from(this.noticedPatterns.entries()),
      uncertaintyHistory: this.uncertaintyHistory,
      exportedAt: Date.now(),
    };
  }

  /**
   * Import state from persistence
   * @param {Object} data - Exported data
   */
  import(data) {
    if (data.observations) {
      this.observations = data.observations;
    }
    if (data.metrics) {
      this.metrics = { ...this.metrics, ...data.metrics };
    }
    if (data.noticedPatterns) {
      this.noticedPatterns = new Map(data.noticedPatterns);
    }
    if (data.uncertaintyHistory) {
      this.uncertaintyHistory = data.uncertaintyHistory;
    }
  }

  /**
   * Reset state
   */
  reset() {
    this.observations = [];
    this.metrics = {
      totalObservations: 0,
      avgConfidence: 0,
      confidenceVariance: 0,
      uncertaintyZones: [],
      decisionsCount: 0,
      correctPredictions: 0,
    };
    this._state = ConsciousnessState.DORMANT;
    this._awarenessLevel = 0;
    this.noticedPatterns.clear();
    this.uncertaintyHistory = [];
  }
}

/**
 * Create a ConsciousnessMonitor instance
 * @param {Object} [options] - Configuration
 * @returns {ConsciousnessMonitor}
 */
export function createConsciousnessMonitor(options = {}) {
  return new ConsciousnessMonitor(options);
}

/**
 * @deprecated Use named exports instead of default export.
 * Default export will be removed in v2.0.
 *
 * @example
 * // Preferred:
 * import { ConsciousnessMonitor, createConsciousnessMonitor } from '@cynic/emergence';
 *
 * // Deprecated:
 * import ConsciousnessMonitorModule from '@cynic/emergence/consciousness-monitor';
 */
export default {
  ConsciousnessMonitor,
  createConsciousnessMonitor,
  ConsciousnessState,
  AWARENESS_THRESHOLDS,
  MAX_CONFIDENCE,
};
