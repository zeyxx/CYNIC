/**
 * Feedback Processor - Pattern Tracking & Feedback Processing
 *
 * Handles feedback processing, pattern tracking by item type, dimension, and source.
 * Extracted from LearningService to follow BURN axiom (simplicity).
 *
 * @module @cynic/node/judge/feedback-processor
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV_2 } from '@cynic/core';

/**
 * Feedback sources (Ralph-inspired: external validation gates)
 */
export const FEEDBACK_SOURCES = {
  MANUAL: 'manual',           // Human explicit feedback
  TEST_RESULT: 'test_result', // Tests pass/fail
  COMMIT: 'commit',           // Code committed successfully
  PR_MERGED: 'pr_merged',     // PR merged/rejected
  PR_REJECTED: 'pr_rejected', // PR rejected
  CODE_REVIEW: 'code_review', // Code review outcome
  BUILD: 'build',             // Build pass/fail
  DEPLOY: 'deploy',           // Deploy success/failure
};

/**
 * Feedback Processor - Handles feedback processing and pattern tracking
 */
export class FeedbackProcessor extends EventEmitter {
  /**
   * @param {Object} options - Processor options
   * @param {number} [options.immediateRate] - Immediate learning rate (default: 0.146)
   * @param {number} [options.maxAdjustment] - Max weight adjustment (default: phi^-2)
   * @param {boolean} [options.immediateLearn] - Enable immediate learning (default: true)
   */
  constructor(options = {}) {
    super();

    this.immediateRate = options.immediateRate || 0.146; // phi^-4
    this.maxAdjustment = options.maxAdjustment || PHI_INV_2;
    this.immediateLearn = options.immediateLearn !== false;

    // Feedback queue for batch processing
    this._feedbackQueue = [];

    // Learning patterns: track systematic biases
    this._patterns = {
      byItemType: new Map(),  // itemType -> { overscoring, underscoring, feedbackCount, avgDelta }
      byDimension: new Map(), // dimension -> { avgError, feedbackCount, scoreSum }
      bySource: new Map(),    // source -> { count, correctCount, incorrectCount, avgDelta }
      anomalies: [],          // Anomaly history
      overall: {
        totalFeedback: 0,
        correctCount: 0,
        incorrectCount: 0,
        avgScoreError: 0,
        learningIterations: 0,
        anomalyCount: 0,
      },
    };
  }

  /**
   * Get patterns object (for external access)
   * @returns {Object}
   */
  get patterns() {
    return this._patterns;
  }

  /**
   * Get feedback queue
   * @returns {Object[]}
   */
  get feedbackQueue() {
    return this._feedbackQueue;
  }

  /**
   * Set feedback queue (for learning cycle)
   * @param {Object[]} queue
   */
  set feedbackQueue(queue) {
    this._feedbackQueue = queue;
  }

  /**
   * Process a single feedback item
   *
   * @param {Object} feedback - Feedback data
   * @param {string} feedback.outcome - 'correct', 'incorrect', 'partial'
   * @param {number} [feedback.actualScore] - What the score should have been
   * @param {number} feedback.originalScore - What CYNIC scored it
   * @param {string} feedback.itemType - Type of item judged
   * @param {Object} [feedback.dimensionScores] - Original dimension scores
   * @param {string} [feedback.source] - Feedback source
   * @param {Object} [feedback.sourceContext] - Additional context from source
   * @param {Map} [weightModifiers] - Current weight modifiers (for immediate learning)
   * @returns {Object} Processing result
   */
  processFeedback(feedback, weightModifiers = null) {
    const {
      outcome,
      actualScore,
      originalScore,
      itemType = 'unknown',
      dimensionScores = {},
      source = FEEDBACK_SOURCES.MANUAL,
      sourceContext = {},
    } = feedback;

    // Calculate learning delta
    const scoreDelta = actualScore != null
      ? actualScore - originalScore
      : this._inferDeltaFromOutcome(outcome, originalScore);

    // Track overall patterns
    this._patterns.overall.totalFeedback++;
    if (outcome === 'correct') {
      this._patterns.overall.correctCount++;
    } else if (outcome === 'incorrect') {
      this._patterns.overall.incorrectCount++;
    }

    // Update running average of score error
    const prevAvg = this._patterns.overall.avgScoreError;
    const n = this._patterns.overall.totalFeedback;
    this._patterns.overall.avgScoreError = prevAvg + (Math.abs(scoreDelta) - prevAvg) / n;

    // Track by item type
    this._trackItemTypePattern(itemType, scoreDelta);

    // Track by dimension
    this._trackDimensionPatterns(dimensionScores, scoreDelta);

    // Track by source
    this._trackSourcePattern(source, outcome, scoreDelta);

    // Queue for batch learning
    this._feedbackQueue.push({
      ...feedback,
      source,
      sourceContext,
      scoreDelta,
      processedAt: Date.now(),
    });

    // Apply immediate learning if enabled and weight modifiers provided
    let immediateAdjustments = null;
    if (this.immediateLearn && weightModifiers && Object.keys(dimensionScores).length > 0) {
      immediateAdjustments = this._applyImmediateLearning(
        dimensionScores,
        scoreDelta,
        outcome,
        weightModifiers
      );
    }

    const result = {
      scoreDelta,
      queueSize: this._feedbackQueue.length,
      immediateAdjustments,
    };

    this.emit('feedback-processed', result);
    return result;
  }

  /**
   * Apply immediate small adjustments per judgment
   * Uses phi^-4 learning rate (0.146) for micro-adjustments
   * @private
   */
  _applyImmediateLearning(dimensionScores, scoreDelta, outcome, weightModifiers) {
    if (outcome === 'correct' || Math.abs(scoreDelta) < 5) {
      return null; // No adjustment needed for correct judgments
    }

    const adjustments = {};
    const direction = scoreDelta > 0 ? -1 : 1; // If we underscored, increase weights

    for (const [dimension, score] of Object.entries(dimensionScores)) {
      // Higher dimension scores get adjusted more when they contributed to the error
      const contribution = (score - 50) / 100; // -0.5 to +0.5
      const adjustment = direction * contribution * this.immediateRate * (Math.abs(scoreDelta) / 50);

      // Bound the immediate adjustment (smaller than batch adjustments)
      const boundedAdjustment = Math.max(-0.05, Math.min(0.05, adjustment));

      if (Math.abs(boundedAdjustment) > 0.001) {
        const currentModifier = weightModifiers.get(dimension) || 1.0;
        const newModifier = Math.max(0.6, Math.min(1.4, currentModifier + boundedAdjustment));
        weightModifiers.set(dimension, newModifier);
        adjustments[dimension] = { from: currentModifier, to: newModifier, delta: boundedAdjustment };
      }
    }

    if (Object.keys(adjustments).length > 0) {
      this.emit('immediate-learning', {
        scoreDelta,
        outcome,
        adjustments,
        timestamp: Date.now(),
      });
    }

    return adjustments;
  }

  /**
   * Infer score delta from outcome when actual score not provided
   * @private
   */
  _inferDeltaFromOutcome(outcome, originalScore) {
    switch (outcome) {
      case 'correct':
        return 0;
      case 'incorrect':
        return originalScore > 50 ? -20 : 20;
      case 'partial':
        return originalScore > 50 ? -10 : 10;
      default:
        return 0;
    }
  }

  /**
   * Track pattern by item type
   * @private
   */
  _trackItemTypePattern(itemType, scoreDelta) {
    if (!this._patterns.byItemType.has(itemType)) {
      this._patterns.byItemType.set(itemType, {
        overscoring: 0,
        underscoring: 0,
        feedbackCount: 0,
        avgDelta: 0,
      });
    }

    const pattern = this._patterns.byItemType.get(itemType);
    pattern.feedbackCount++;

    if (scoreDelta < -5) {
      pattern.overscoring++;
    } else if (scoreDelta > 5) {
      pattern.underscoring++;
    }

    // Update running average
    pattern.avgDelta = pattern.avgDelta + (scoreDelta - pattern.avgDelta) / pattern.feedbackCount;
  }

  /**
   * Track patterns by dimension
   * @private
   */
  _trackDimensionPatterns(dimensionScores, scoreDelta) {
    for (const [dimension, score] of Object.entries(dimensionScores)) {
      if (!this._patterns.byDimension.has(dimension)) {
        this._patterns.byDimension.set(dimension, {
          avgError: 0,
          feedbackCount: 0,
          scoreSum: 0,
        });
      }

      const pattern = this._patterns.byDimension.get(dimension);
      pattern.feedbackCount++;
      pattern.scoreSum += score;

      // Dimensions with extreme scores that correlate with errors
      if (Math.abs(scoreDelta) > 10) {
        const contribution = (score - 50) * Math.sign(scoreDelta);
        pattern.avgError = pattern.avgError + (contribution - pattern.avgError) / pattern.feedbackCount;
      }
    }
  }

  /**
   * Track patterns by feedback source
   * @private
   */
  _trackSourcePattern(source, outcome, scoreDelta) {
    if (!this._patterns.bySource.has(source)) {
      this._patterns.bySource.set(source, {
        count: 0,
        correctCount: 0,
        incorrectCount: 0,
        avgDelta: 0,
        firstSeen: Date.now(),
        lastSeen: Date.now(),
      });
    }

    const pattern = this._patterns.bySource.get(source);
    pattern.count++;
    pattern.lastSeen = Date.now();

    if (outcome === 'correct') {
      pattern.correctCount++;
    } else if (outcome === 'incorrect') {
      pattern.incorrectCount++;
    }

    pattern.avgDelta = pattern.avgDelta + (scoreDelta - pattern.avgDelta) / pattern.count;
  }

  /**
   * Process anomaly signal from ResidualDetector
   *
   * @param {Object} signal - Anomaly signal
   * @returns {Object} Anomaly entry
   */
  processAnomalySignal(signal) {
    if (!signal || !signal.judgmentId) return null;

    const anomalyEntry = {
      id: `anomaly_${signal.judgmentId}`,
      timestamp: Date.now(),
      residual: signal.residual,
      threshold: signal.threshold,
      dimensions: signal.dimensions,
      verdict: signal.verdict,
      qScore: signal.qScore,
    };

    // Store in history (limited buffer)
    this._patterns.anomalies.push(anomalyEntry);
    if (this._patterns.anomalies.length > 100) {
      this._patterns.anomalies.shift();
    }

    // Track overall anomaly rate
    this._patterns.overall.anomalyCount++;

    // Analyze dimension-level anomalies and emit events for flagged dimensions
    const flagged = this.analyzeDimensionAnomalies(signal);
    for (const flag of flagged) {
      this.emit('dimension-anomaly', {
        dimension: flag.dimension,
        anomalyCount: flag.anomalyCount,
        recommendation: flag.recommendation,
        timestamp: Date.now(),
      });
    }

    this.emit('anomaly-processed', {
      judgmentId: signal.judgmentId,
      residual: signal.residual,
      timestamp: Date.now(),
    });

    return anomalyEntry;
  }

  /**
   * Get dimension anomalies for learning adjustment
   * @param {Object} signal - Anomaly signal with dimension scores
   * @returns {Object[]} Dimensions needing attention
   */
  analyzeDimensionAnomalies(signal) {
    const flagged = [];

    if (signal?.dimensions) {
      for (const [dim, score] of Object.entries(signal.dimensions)) {
        if (typeof score !== 'number') continue;

        // Very high or very low scores in anomalous judgments suggest weight issues
        if (score > 90 || score < 10) {
          const existing = this._patterns.byDimension.get(dim) || {
            feedbackCount: 0,
            anomalyCount: 0,
            avgError: 0,
          };

          existing.anomalyCount = (existing.anomalyCount || 0) + 1;
          this._patterns.byDimension.set(dim, existing);

          // If we see repeated anomalies for this dimension, flag it
          if (existing.anomalyCount >= 3) {
            flagged.push({
              dimension: dim,
              anomalyCount: existing.anomalyCount,
              recommendation: `${dim} may need weight recalibration`,
            });
          }
        }
      }
    }

    return flagged;
  }

  /**
   * Get patterns with insights
   * @param {number} minFeedback - Minimum feedback count for insights
   * @returns {Object}
   */
  getPatterns(minFeedback = 3) {
    const insights = [];

    // Check for systematic biases by item type
    for (const [itemType, pattern] of this._patterns.byItemType) {
      if (pattern.feedbackCount >= minFeedback) {
        if (Math.abs(pattern.avgDelta) > 10) {
          const direction = pattern.avgDelta > 0 ? 'underscoring' : 'overscoring';
          insights.push({
            type: 'item_type_bias',
            itemType,
            direction,
            avgDelta: Math.round(pattern.avgDelta * 10) / 10,
            feedbackCount: pattern.feedbackCount,
            recommendation: `Consider ${direction === 'overscoring' ? 'lowering' : 'raising'} thresholds for ${itemType}`,
          });
        }
      }
    }

    // Check for dimension issues
    for (const [dimension, pattern] of this._patterns.byDimension) {
      if (pattern.feedbackCount >= minFeedback && Math.abs(pattern.avgError) > 15) {
        insights.push({
          type: 'dimension_bias',
          dimension,
          avgError: Math.round(pattern.avgError * 10) / 10,
          feedbackCount: pattern.feedbackCount,
          recommendation: `${dimension} weight may need adjustment`,
        });
      }
    }

    return {
      overall: { ...this._patterns.overall },
      byItemType: Object.fromEntries(this._patterns.byItemType),
      byDimension: Object.fromEntries(this._patterns.byDimension),
      bySource: Object.fromEntries(this._patterns.bySource),
      insights,
      accuracy: this._patterns.overall.totalFeedback > 0
        ? this._patterns.overall.correctCount / this._patterns.overall.totalFeedback
        : 0,
    };
  }

  /**
   * Reset all patterns
   */
  reset() {
    this._patterns.byItemType.clear();
    this._patterns.byDimension.clear();
    this._patterns.bySource.clear();
    this._patterns.anomalies = [];
    this._patterns.overall = {
      totalFeedback: 0,
      correctCount: 0,
      incorrectCount: 0,
      avgScoreError: 0,
      learningIterations: 0,
      anomalyCount: 0,
    };
    this._feedbackQueue = [];
    this.emit('reset');
  }

  /**
   * Export patterns for persistence
   * @returns {Object}
   */
  exportPatterns() {
    return {
      byItemType: Object.fromEntries(this._patterns.byItemType),
      byDimension: Object.fromEntries(this._patterns.byDimension),
      bySource: Object.fromEntries(this._patterns.bySource),
      overall: { ...this._patterns.overall },
    };
  }

  /**
   * Import patterns from persistence
   * @param {Object} patterns
   */
  importPatterns(patterns) {
    if (patterns.byItemType) {
      for (const [type, pattern] of Object.entries(patterns.byItemType)) {
        this._patterns.byItemType.set(type, pattern);
      }
    }
    if (patterns.byDimension) {
      for (const [dim, pattern] of Object.entries(patterns.byDimension)) {
        this._patterns.byDimension.set(dim, pattern);
      }
    }
    if (patterns.bySource) {
      for (const [source, pattern] of Object.entries(patterns.bySource)) {
        this._patterns.bySource.set(source, pattern);
      }
    }
    if (patterns.overall) {
      Object.assign(this._patterns.overall, patterns.overall);
    }
  }
}

export default FeedbackProcessor;
