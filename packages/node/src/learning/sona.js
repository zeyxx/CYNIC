/**
 * SONA - Self-Optimizing Neural Adaptation
 *
 * Real-time learning system that adapts pattern weights based on
 * observed outcomes. Inspired by Claude Flow's SONA but with φ-alignment.
 *
 * Key Features:
 * - Sub-millisecond adaptation (<0.05ms target)
 * - Pattern-dimension correlation tracking
 * - Continuous weight optimization
 * - Integration with EWC++ (respects locked patterns)
 *
 * "The dog learns by watching, not by being told" - κυνικός
 *
 * @module @cynic/node/learning/sona
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV } from '@cynic/core';

/**
 * φ-aligned SONA constants
 */
const PHI_INV_2 = 0.381966011250105; // φ⁻²
const PHI_INV_3 = 0.236067977499790; // φ⁻³

const SONA_CONFIG = Object.freeze({
  // Adaptation rate (φ-aligned)
  ADAPTATION_RATE: PHI_INV_3,          // 0.236 - How fast to adapt
  MAX_ADAPTATION: PHI_INV_2,           // 0.382 - Max single adaptation
  DECAY_RATE: 0.999,                   // Slow decay toward baseline

  // Pattern correlation thresholds
  MIN_OBSERVATIONS: 3,                 // Minimum observations before adapting
  CORRELATION_THRESHOLD: PHI_INV_2,    // 0.382 - Min correlation to act on
  CONFIDENCE_THRESHOLD: PHI_INV_3,     // 0.236 - Min confidence to trust

  // Time windows (Fibonacci-aligned in ms)
  RECENT_WINDOW_MS: 21000,             // F(8) seconds - "recent" observations
  ADAPTATION_INTERVAL_MS: 55,          // F(10) ms - batch adaptations
  STATS_WINDOW_MS: 89000,              // F(11) seconds - stats window

  // Limits (Fibonacci)
  MAX_TRACKED_PATTERNS: 144,           // F(12) - Max patterns to track
  MAX_OBSERVATIONS_PER_PATTERN: 89,    // F(11) - Rolling window
  BATCH_SIZE: 13,                      // F(7) - Batch size for updates
});

/**
 * Observation record for pattern-outcome correlation
 */
class PatternObservation {
  constructor(patternId, dimensionScores, outcome, timestamp = Date.now()) {
    this.patternId = patternId;
    this.dimensionScores = dimensionScores;
    this.outcome = outcome; // 1 = correct, 0 = incorrect, 0.5 = partial
    this.timestamp = timestamp;
  }

  get age() {
    return Date.now() - this.timestamp;
  }

  isRecent(windowMs = SONA_CONFIG.RECENT_WINDOW_MS) {
    return this.age < windowMs;
  }
}

/**
 * SONA - Self-Optimizing Neural Adaptation
 *
 * Tracks pattern usage during judgments and correlates with outcomes
 * to optimize dimension weights in real-time.
 */
export class SONA extends EventEmitter {
  constructor(options = {}) {
    super();

    this.config = { ...SONA_CONFIG, ...options.config };

    // Pattern observation store: patternId → PatternObservation[]
    this._observations = new Map();

    // Dimension-pattern correlation: dimension → patternId → correlation
    this._correlations = new Map();

    // Pending weight adjustments (batched for performance)
    this._pendingAdjustments = new Map();

    // Adaptation queue for <0.05ms processing
    this._adaptationQueue = [];

    // Statistics
    this.stats = {
      totalObservations: 0,
      adaptationsMade: 0,
      avgAdaptationTime: 0,
      lastAdaptation: null,
      patternsTracked: 0,
      correlationsFound: 0,
    };

    // Batch adaptation timer
    this._adaptationTimer = null;

    // External services (injected)
    this._ewcService = options.ewcService || null;
    this._learningService = options.learningService || null;
  }

  /**
   * Start the adaptation loop
   */
  start() {
    if (this._adaptationTimer) return;

    this._adaptationTimer = setInterval(
      () => this._processAdaptationBatch(),
      this.config.ADAPTATION_INTERVAL_MS
    );

    this.emit('sona:started');
  }

  /**
   * Stop the adaptation loop
   */
  stop() {
    if (this._adaptationTimer) {
      clearInterval(this._adaptationTimer);
      this._adaptationTimer = null;
      this.emit('sona:stopped');
    }
  }

  /**
   * Record a pattern observation during judgment
   *
   * @param {Object} context - Judgment context
   * @param {string} context.patternId - Pattern that was used
   * @param {Object} context.dimensionScores - Dimension scores at judgment time
   * @param {string} context.judgmentId - Judgment ID for linking
   */
  observe(context) {
    const { patternId, dimensionScores, judgmentId } = context;

    if (!patternId || !dimensionScores) return;

    // Create pending observation (outcome unknown yet)
    const observation = new PatternObservation(
      patternId,
      { ...dimensionScores },
      null // Outcome will be set when feedback arrives
    );
    observation.judgmentId = judgmentId;

    // Queue for linking with feedback
    this._adaptationQueue.push(observation);

    // Track pattern
    if (!this._observations.has(patternId)) {
      this._observations.set(patternId, []);
    }

    this.stats.totalObservations++;
    this.emit('sona:observed', { patternId, judgmentId });
  }

  /**
   * Process feedback and link to observations
   * This is where learning happens.
   *
   * @param {Object} feedback - Feedback record
   * @param {string} feedback.judgmentId - Linked judgment
   * @param {string} feedback.outcome - 'correct', 'incorrect', 'partial'
   * @param {number} feedback.actualScore - Ground truth score
   * @param {number} feedback.originalScore - Original judgment score
   */
  processFeedback(feedback) {
    const startTime = performance.now();

    const { judgmentId, outcome, actualScore, originalScore } = feedback;

    // Find pending observations for this judgment
    const pendingIdx = this._adaptationQueue.findIndex(
      obs => obs.judgmentId === judgmentId
    );

    if (pendingIdx === -1) return;

    const observation = this._adaptationQueue.splice(pendingIdx, 1)[0];

    // Convert outcome to numeric
    observation.outcome = outcome === 'correct' ? 1.0 :
                         outcome === 'partial' ? 0.5 : 0.0;
    observation.scoreDelta = actualScore - originalScore;

    // Add to pattern's observation history
    const observations = this._observations.get(observation.patternId);
    if (observations) {
      observations.push(observation);

      // Prune old observations (rolling window)
      while (observations.length > this.config.MAX_OBSERVATIONS_PER_PATTERN) {
        observations.shift();
      }

      // Trigger correlation update
      this._updateCorrelations(observation);

      // Queue adaptation if enough observations
      if (observations.length >= this.config.MIN_OBSERVATIONS) {
        this._queueAdaptation(observation.patternId);
      }
    }

    const elapsed = performance.now() - startTime;
    this._updateAdaptationStats(elapsed);

    this.emit('sona:feedback', { patternId: observation.patternId, outcome, elapsed });
  }

  /**
   * Update dimension-pattern correlations
   * @private
   */
  _updateCorrelations(observation) {
    const { patternId, dimensionScores, outcome, scoreDelta } = observation;

    for (const [dimension, score] of Object.entries(dimensionScores)) {
      if (!this._correlations.has(dimension)) {
        this._correlations.set(dimension, new Map());
      }

      const dimCorrelations = this._correlations.get(dimension);

      if (!dimCorrelations.has(patternId)) {
        dimCorrelations.set(patternId, {
          sumProduct: 0,
          sumScore: 0,
          sumOutcome: 0,
          sumScoreSq: 0,
          sumOutcomeSq: 0,
          count: 0,
        });
      }

      const corr = dimCorrelations.get(patternId);

      // Update running correlation stats (Welford-style for numerical stability)
      const normalizedScore = score / 100; // Normalize to 0-1
      corr.sumProduct += normalizedScore * outcome;
      corr.sumScore += normalizedScore;
      corr.sumOutcome += outcome;
      corr.sumScoreSq += normalizedScore * normalizedScore;
      corr.sumOutcomeSq += outcome * outcome;
      corr.count++;

      // Calculate Pearson correlation coefficient
      if (corr.count >= this.config.MIN_OBSERVATIONS) {
        const n = corr.count;
        const numerator = n * corr.sumProduct - corr.sumScore * corr.sumOutcome;
        const denominator = Math.sqrt(
          (n * corr.sumScoreSq - corr.sumScore ** 2) *
          (n * corr.sumOutcomeSq - corr.sumOutcome ** 2)
        );

        corr.correlation = denominator > 0 ? numerator / denominator : 0;
        corr.strength = Math.abs(corr.correlation);
      }
    }

    this.stats.correlationsFound = this._countSignificantCorrelations();
  }

  /**
   * Count correlations above threshold
   * @private
   */
  _countSignificantCorrelations() {
    let count = 0;
    for (const dimCorrelations of this._correlations.values()) {
      for (const corr of dimCorrelations.values()) {
        if (corr.strength >= this.config.CORRELATION_THRESHOLD) {
          count++;
        }
      }
    }
    return count;
  }

  /**
   * Queue a pattern for adaptation
   * @private
   */
  _queueAdaptation(patternId) {
    if (!this._pendingAdjustments.has(patternId)) {
      this._pendingAdjustments.set(patternId, { queued: Date.now() });
    }
  }

  /**
   * Process adaptation batch (called on interval)
   * Target: <0.05ms per batch
   * @private
   */
  async _processAdaptationBatch() {
    const startTime = performance.now();

    const batch = Array.from(this._pendingAdjustments.keys())
      .slice(0, this.config.BATCH_SIZE);

    if (batch.length === 0) return;

    for (const patternId of batch) {
      await this._adaptPattern(patternId);
      this._pendingAdjustments.delete(patternId);
    }

    const elapsed = performance.now() - startTime;
    this._updateAdaptationStats(elapsed);

    if (batch.length > 0) {
      this.emit('sona:batch', { count: batch.length, elapsed });
    }
  }

  /**
   * Adapt a single pattern's influence on dimensions
   * @private
   */
  async _adaptPattern(patternId) {
    const observations = this._observations.get(patternId);
    if (!observations || observations.length < this.config.MIN_OBSERVATIONS) {
      return;
    }

    // Check EWC lock
    if (this._ewcService) {
      const canModify = await this._ewcService.canModifyPattern?.(patternId);
      if (canModify === false) {
        // Pattern is EWC-locked, skip adaptation
        return;
      }
    }

    // Calculate recent performance
    const recentObs = observations.filter(o => o.isRecent());
    if (recentObs.length < this.config.MIN_OBSERVATIONS) return;

    const avgOutcome = recentObs.reduce((sum, o) => sum + o.outcome, 0) / recentObs.length;
    const avgDelta = recentObs.reduce((sum, o) => sum + (o.scoreDelta || 0), 0) / recentObs.length;

    // Determine adaptation direction
    // High outcome + positive delta = pattern is helping
    // Low outcome + negative delta = pattern is hurting
    const confidence = Math.min(recentObs.length / 10, PHI_INV);

    if (confidence < this.config.CONFIDENCE_THRESHOLD) return;

    // Calculate weight adjustment
    const performanceSignal = avgOutcome - 0.5; // Center around neutral
    const adjustment = performanceSignal * this.config.ADAPTATION_RATE * confidence;
    const boundedAdjustment = Math.max(
      -this.config.MAX_ADAPTATION,
      Math.min(this.config.MAX_ADAPTATION, adjustment)
    );

    // Apply to learning service if available
    if (this._learningService && Math.abs(boundedAdjustment) > 0.001) {
      // Find which dimensions this pattern most correlates with
      const topCorrelations = this._getTopCorrelationsForPattern(patternId, 3);

      for (const { dimension, correlation } of topCorrelations) {
        const dimAdjustment = boundedAdjustment * correlation;

        // Queue adjustment to learning service
        this._learningService.adjustDimensionWeight?.(dimension, dimAdjustment);
      }

      this.stats.adaptationsMade++;
      this.stats.lastAdaptation = Date.now();

      this.emit('sona:adapted', {
        patternId,
        avgOutcome,
        adjustment: boundedAdjustment,
        dimensions: topCorrelations.map(c => c.dimension),
      });
    }
  }

  /**
   * Get top dimension correlations for a pattern
   * @private
   */
  _getTopCorrelationsForPattern(patternId, limit = 3) {
    const results = [];

    for (const [dimension, dimCorrelations] of this._correlations.entries()) {
      const corr = dimCorrelations.get(patternId);
      if (corr && corr.strength >= this.config.CORRELATION_THRESHOLD) {
        results.push({
          dimension,
          correlation: corr.correlation,
          strength: corr.strength,
        });
      }
    }

    return results
      .sort((a, b) => b.strength - a.strength)
      .slice(0, limit);
  }

  /**
   * Update adaptation timing statistics
   * @private
   */
  _updateAdaptationStats(elapsed) {
    // Exponential moving average
    const alpha = 0.1;
    this.stats.avgAdaptationTime =
      this.stats.avgAdaptationTime * (1 - alpha) + elapsed * alpha;
  }

  /**
   * Get pattern performance summary
   *
   * @param {string} patternId - Pattern to summarize
   * @returns {Object|null} Performance summary
   */
  getPatternPerformance(patternId) {
    const observations = this._observations.get(patternId);
    if (!observations || observations.length === 0) return null;

    const recent = observations.filter(o => o.isRecent());
    const all = observations;

    const calcStats = (obs) => ({
      count: obs.length,
      avgOutcome: obs.reduce((s, o) => s + o.outcome, 0) / obs.length,
      avgDelta: obs.reduce((s, o) => s + (o.scoreDelta || 0), 0) / obs.length,
      successRate: obs.filter(o => o.outcome >= 0.5).length / obs.length,
    });

    return {
      patternId,
      recent: calcStats(recent),
      overall: calcStats(all),
      topCorrelations: this._getTopCorrelationsForPattern(patternId, 5),
    };
  }

  /**
   * Get SONA statistics
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this.stats,
      patternsTracked: this._observations.size,
      pendingAdaptations: this._pendingAdjustments.size,
      queuedObservations: this._adaptationQueue.length,
      config: this.config,
    };
  }

  /**
   * Get dimension insights from correlations
   * @returns {Object} Dimension insights
   */
  getDimensionInsights() {
    const insights = {};

    for (const [dimension, dimCorrelations] of this._correlations.entries()) {
      const patterns = [];

      for (const [patternId, corr] of dimCorrelations.entries()) {
        if (corr.strength >= this.config.CORRELATION_THRESHOLD) {
          patterns.push({
            patternId,
            correlation: corr.correlation,
            strength: corr.strength,
            observations: corr.count,
          });
        }
      }

      if (patterns.length > 0) {
        insights[dimension] = {
          patterns: patterns.sort((a, b) => b.strength - a.strength),
          avgCorrelation: patterns.reduce((s, p) => s + p.correlation, 0) / patterns.length,
          strongestPattern: patterns[0]?.patternId,
        };
      }
    }

    return insights;
  }

  /**
   * Inject external services
   */
  setEWCService(ewcService) {
    this._ewcService = ewcService;
  }

  setLearningService(learningService) {
    this._learningService = learningService;
  }

  /**
   * Reset SONA state (for testing)
   */
  reset() {
    this._observations.clear();
    this._correlations.clear();
    this._pendingAdjustments.clear();
    this._adaptationQueue = [];

    this.stats = {
      totalObservations: 0,
      adaptationsMade: 0,
      avgAdaptationTime: 0,
      lastAdaptation: null,
      patternsTracked: 0,
      correlationsFound: 0,
    };
  }
}

/**
 * Create SONA instance
 *
 * @param {Object} [options] - Options
 * @returns {SONA}
 */
export function createSONA(options = {}) {
  return new SONA(options);
}

export { SONA_CONFIG };

export default {
  SONA,
  createSONA,
  SONA_CONFIG,
};
