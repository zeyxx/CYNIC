/**
 * CYNIC Learning Loop
 *
 * "φ distrusts φ" - Even CYNIC learns from mistakes
 *
 * Learning system that improves judgments based on feedback:
 * 1. Feedback Analysis - Extract patterns from corrections
 * 2. Weight Calibration - Adjust axiom weights over time
 * 3. Bias Detection - Identify systematic errors
 * 4. Confidence Calibration - Improve confidence accuracy
 *
 * @module @cynic/core/learning
 * @philosophy The dog that learns from every hunt
 */

'use strict';

import { PHI_INV, PHI_INV_2, THRESHOLDS } from '../axioms/constants.js';

// =============================================================================
// CONSTANTS
// =============================================================================

export const LEARNING_CONSTANTS = {
  /** Minimum samples for reliable learning (Fib(8) = 21) */
  MIN_SAMPLES: 21,

  /** Learning rate (φ⁻² = 0.382) */
  LEARNING_RATE: PHI_INV_2,

  /** Weight decay factor (prevents overfitting) */
  WEIGHT_DECAY: 0.99,

  /** Max weight deviation from 1.0 (±0.382) */
  MAX_WEIGHT_DEVIATION: PHI_INV_2,

  /** Bias detection threshold */
  BIAS_THRESHOLD: 0.15,

  /** Confidence calibration window (Fib(7) = 13) */
  CALIBRATION_WINDOW: 13,

  /** Max learnings to store (Fib(10) = 55) */
  MAX_LEARNINGS: 55,
};

/**
 * Feedback outcomes
 */
export const FeedbackOutcome = {
  CORRECT: 'correct',
  INCORRECT: 'incorrect',
  PARTIAL: 'partial',
};

/**
 * Bias types
 */
export const BiasType = {
  OVERCONFIDENT: 'overconfident',     // Consistently too confident
  UNDERCONFIDENT: 'underconfident',   // Consistently too cautious
  AXIOM_SKEW: 'axiom_skew',           // One axiom consistently off
  VERDICT_BIAS: 'verdict_bias',       // Tendency toward certain verdicts
  SOURCE_BIAS: 'source_bias',         // Bias based on source type
};

// =============================================================================
// FEEDBACK ANALYZER
// =============================================================================

/**
 * Analyze feedback to extract learning signals
 */
export class FeedbackAnalyzer {
  constructor() {
    /** @type {Array<Object>} Recent feedback samples */
    this.samples = [];

    /** Aggregated statistics */
    this.stats = {
      total: 0,
      correct: 0,
      incorrect: 0,
      partial: 0,
      avgScoreError: 0,
      axiomErrors: { PHI: 0, VERIFY: 0, CULTURE: 0, BURN: 0 },
    };
  }

  /**
   * Add feedback sample
   * @param {Object} feedback - Feedback data
   * @param {Object} feedback.judgment - Original judgment
   * @param {string} feedback.outcome - 'correct', 'incorrect', 'partial'
   * @param {number} [feedback.actualScore] - What score should have been
   * @param {string} [feedback.reason] - Explanation
   */
  addSample(feedback) {
    const sample = {
      ...feedback,
      timestamp: Date.now(),
      scoreError: feedback.actualScore
        ? feedback.actualScore - (feedback.judgment?.Q || feedback.judgment?.qScore || 0)
        : 0,
    };

    this.samples.push(sample);
    this._updateStats(sample);

    // Trim old samples
    while (this.samples.length > LEARNING_CONSTANTS.MAX_LEARNINGS) {
      this.samples.shift();
    }

    return sample;
  }

  /**
   * Update aggregate statistics
   * @private
   */
  _updateStats(sample) {
    this.stats.total++;

    switch (sample.outcome) {
      case FeedbackOutcome.CORRECT:
        this.stats.correct++;
        break;
      case FeedbackOutcome.INCORRECT:
        this.stats.incorrect++;
        break;
      case FeedbackOutcome.PARTIAL:
        this.stats.partial++;
        break;
    }

    // Update average score error
    if (sample.scoreError !== 0) {
      const n = this.stats.total;
      this.stats.avgScoreError =
        ((n - 1) * this.stats.avgScoreError + sample.scoreError) / n;
    }
  }

  /**
   * Calculate accuracy rate
   */
  getAccuracy() {
    if (this.stats.total === 0) return 0;
    return (this.stats.correct + this.stats.partial * 0.5) / this.stats.total;
  }

  /**
   * Get recent samples
   */
  getRecentSamples(count = 10) {
    return this.samples.slice(-count);
  }

  /**
   * Get statistics
   */
  getStats() {
    return {
      ...this.stats,
      accuracy: this.getAccuracy(),
      sampleCount: this.samples.length,
    };
  }

  /**
   * Export state
   */
  export() {
    return {
      samples: this.samples,
      stats: this.stats,
    };
  }

  /**
   * Import state
   */
  import(state) {
    this.samples = state.samples || [];
    this.stats = state.stats || this.stats;
    return this;
  }
}

// =============================================================================
// WEIGHT CALIBRATOR
// =============================================================================

/**
 * Calibrate axiom weights based on feedback
 */
export class WeightCalibrator {
  constructor() {
    /** Current axiom weights (start at 1.0 = neutral) */
    this.weights = {
      PHI: 1.0,
      VERIFY: 1.0,
      CULTURE: 1.0,
      BURN: 1.0,
    };

    /** Weight history for trend analysis */
    this.history = [];

    /** Calibration iterations */
    this.iterations = 0;
  }

  /**
   * Update weights based on feedback batch
   * @param {Array<Object>} feedbackBatch - Array of feedback with judgments
   * @returns {Object} Weight adjustments made
   */
  calibrate(feedbackBatch) {
    if (feedbackBatch.length < LEARNING_CONSTANTS.MIN_SAMPLES) {
      return {
        updated: false,
        reason: `Need ${LEARNING_CONSTANTS.MIN_SAMPLES} samples, have ${feedbackBatch.length}`,
      };
    }

    // Calculate error gradients for each axiom
    const gradients = { PHI: 0, VERIFY: 0, CULTURE: 0, BURN: 0 };
    let validSamples = 0;

    for (const fb of feedbackBatch) {
      if (!fb.judgment?.breakdown && !fb.judgment?.axiomScores) continue;
      if (fb.actualScore === undefined) continue;

      const breakdown = fb.judgment.breakdown || fb.judgment.axiomScores;
      const predictedQ = fb.judgment.Q || fb.judgment.qScore;
      const actualQ = fb.actualScore;
      const error = actualQ - predictedQ;

      // Distribute error proportionally to axiom contributions
      const total = Object.values(breakdown).reduce((a, b) => a + b, 0);
      if (total === 0) continue;

      for (const axiom of Object.keys(gradients)) {
        if (breakdown[axiom] !== undefined) {
          const contribution = breakdown[axiom] / total;
          gradients[axiom] += error * contribution;
        }
      }
      validSamples++;
    }

    if (validSamples === 0) {
      return { updated: false, reason: 'No valid samples with breakdown' };
    }

    // Average gradients
    for (const axiom of Object.keys(gradients)) {
      gradients[axiom] /= validSamples;
    }

    // Apply weight updates with learning rate and decay
    const adjustments = {};
    const lr = LEARNING_CONSTANTS.LEARNING_RATE;
    const decay = LEARNING_CONSTANTS.WEIGHT_DECAY;
    const maxDev = LEARNING_CONSTANTS.MAX_WEIGHT_DEVIATION;

    for (const axiom of Object.keys(this.weights)) {
      // Apply decay toward 1.0
      this.weights[axiom] = 1.0 + (this.weights[axiom] - 1.0) * decay;

      // Apply gradient
      const adjustment = lr * gradients[axiom] / 100; // Normalize
      this.weights[axiom] += adjustment;

      // Clamp to bounds
      this.weights[axiom] = Math.max(1 - maxDev, Math.min(1 + maxDev, this.weights[axiom]));

      adjustments[axiom] = adjustment;
    }

    // Record history
    this.history.push({
      timestamp: Date.now(),
      weights: { ...this.weights },
      gradients,
      samples: validSamples,
    });

    // Trim history
    while (this.history.length > 100) {
      this.history.shift();
    }

    this.iterations++;

    return {
      updated: true,
      weights: { ...this.weights },
      adjustments,
      gradients,
      samplesUsed: validSamples,
      iteration: this.iterations,
    };
  }

  /**
   * Apply weights to axiom scores
   * @param {Object} breakdown - Original axiom scores
   * @returns {Object} Weighted scores
   */
  applyWeights(breakdown) {
    const weighted = {};
    for (const [axiom, score] of Object.entries(breakdown)) {
      weighted[axiom] = score * (this.weights[axiom] || 1.0);
    }
    return weighted;
  }

  /**
   * Get current weights
   */
  getWeights() {
    return { ...this.weights };
  }

  /**
   * Reset weights to neutral
   */
  reset() {
    this.weights = { PHI: 1.0, VERIFY: 1.0, CULTURE: 1.0, BURN: 1.0 };
    this.history = [];
    this.iterations = 0;
  }

  /**
   * Export state
   */
  export() {
    return {
      weights: this.weights,
      history: this.history,
      iterations: this.iterations,
    };
  }

  /**
   * Import state
   */
  import(state) {
    this.weights = state.weights || this.weights;
    this.history = state.history || [];
    this.iterations = state.iterations || 0;
    return this;
  }
}

// =============================================================================
// BIAS DETECTOR
// =============================================================================

/**
 * Detect systematic biases in judgments
 */
export class BiasDetector {
  constructor() {
    /** Detected biases */
    this.biases = [];

    /** Tracking data */
    this.tracking = {
      confidenceErrors: [],
      verdictDistribution: { HOWL: 0, WAG: 0, GROWL: 0, BARK: 0 },
      axiomErrors: { PHI: [], VERIFY: [], CULTURE: [], BURN: [] },
      sourcePerformance: {},
    };
  }

  /**
   * Analyze for biases
   * @param {Array<Object>} feedbackBatch
   * @returns {Object} Detected biases
   */
  analyze(feedbackBatch) {
    this.biases = [];

    // Update tracking
    for (const fb of feedbackBatch) {
      this._trackSample(fb);
    }

    // Check for confidence bias
    this._checkConfidenceBias();

    // Check for verdict bias
    this._checkVerdictBias(feedbackBatch);

    // Check for axiom skew
    this._checkAxiomSkew();

    // Check for source bias
    this._checkSourceBias();

    return {
      biases: this.biases,
      severity: this._calculateSeverity(),
      recommendations: this._generateRecommendations(),
    };
  }

  /**
   * Track a feedback sample
   * @private
   */
  _trackSample(fb) {
    const judgment = fb.judgment;
    if (!judgment) return;

    // Track confidence error
    if (fb.outcome !== undefined && judgment.confidence !== undefined) {
      const wasCorrect = fb.outcome === FeedbackOutcome.CORRECT;
      this.tracking.confidenceErrors.push({
        confidence: judgment.confidence,
        correct: wasCorrect,
      });
    }

    // Track verdict
    const verdict = typeof judgment.verdict === 'string'
      ? judgment.verdict
      : judgment.verdict?.verdict;
    if (verdict && this.tracking.verdictDistribution[verdict] !== undefined) {
      this.tracking.verdictDistribution[verdict]++;
    }

    // Track axiom errors
    if (fb.actualScore !== undefined) {
      const breakdown = judgment.breakdown || judgment.axiomScores;
      if (breakdown) {
        const predictedQ = judgment.Q || judgment.qScore;
        const error = fb.actualScore - predictedQ;
        for (const axiom of Object.keys(this.tracking.axiomErrors)) {
          if (breakdown[axiom] !== undefined) {
            this.tracking.axiomErrors[axiom].push(error);
          }
        }
      }
    }

    // Track source performance
    const source = fb.context?.source || judgment.source || 'unknown';
    if (!this.tracking.sourcePerformance[source]) {
      this.tracking.sourcePerformance[source] = { correct: 0, total: 0 };
    }
    this.tracking.sourcePerformance[source].total++;
    if (fb.outcome === FeedbackOutcome.CORRECT) {
      this.tracking.sourcePerformance[source].correct++;
    }
  }

  /**
   * Check for confidence bias
   * @private
   */
  _checkConfidenceBias() {
    const errors = this.tracking.confidenceErrors;
    if (errors.length < LEARNING_CONSTANTS.CALIBRATION_WINDOW) return;

    const recent = errors.slice(-LEARNING_CONSTANTS.CALIBRATION_WINDOW);
    const avgConfidence = recent.reduce((s, e) => s + e.confidence, 0) / recent.length;
    const accuracy = recent.filter(e => e.correct).length / recent.length;

    // Overconfident if high confidence but low accuracy
    if (avgConfidence > 0.5 && accuracy < avgConfidence - LEARNING_CONSTANTS.BIAS_THRESHOLD) {
      this.biases.push({
        type: BiasType.OVERCONFIDENT,
        severity: avgConfidence - accuracy,
        details: { avgConfidence, accuracy },
        suggestion: 'Lower confidence estimates',
      });
    }

    // Underconfident if low confidence but high accuracy
    if (avgConfidence < 0.4 && accuracy > avgConfidence + LEARNING_CONSTANTS.BIAS_THRESHOLD) {
      this.biases.push({
        type: BiasType.UNDERCONFIDENT,
        severity: accuracy - avgConfidence,
        details: { avgConfidence, accuracy },
        suggestion: 'Increase confidence estimates',
      });
    }
  }

  /**
   * Check for verdict bias
   * @private
   */
  _checkVerdictBias(feedbackBatch) {
    const dist = this.tracking.verdictDistribution;
    const total = Object.values(dist).reduce((a, b) => a + b, 0);
    if (total < LEARNING_CONSTANTS.MIN_SAMPLES) return;

    // Check for extreme skew
    for (const [verdict, count] of Object.entries(dist)) {
      const ratio = count / total;
      // Warning if any verdict is > 61.8% (φ⁻¹)
      if (ratio > PHI_INV) {
        this.biases.push({
          type: BiasType.VERDICT_BIAS,
          severity: ratio - 0.25, // Expected ~25% each
          details: { verdict, ratio, distribution: dist },
          suggestion: `Review ${verdict} criteria - may be too broad`,
        });
      }
    }
  }

  /**
   * Check for axiom skew
   * @private
   */
  _checkAxiomSkew() {
    for (const [axiom, errors] of Object.entries(this.tracking.axiomErrors)) {
      if (errors.length < LEARNING_CONSTANTS.MIN_SAMPLES) continue;

      const avgError = errors.reduce((a, b) => a + b, 0) / errors.length;
      if (Math.abs(avgError) > LEARNING_CONSTANTS.BIAS_THRESHOLD * 100) {
        this.biases.push({
          type: BiasType.AXIOM_SKEW,
          severity: Math.abs(avgError) / 100,
          details: { axiom, avgError, samples: errors.length },
          suggestion: avgError > 0
            ? `${axiom} scores tend too low - consider increasing`
            : `${axiom} scores tend too high - consider decreasing`,
        });
      }
    }
  }

  /**
   * Check for source bias
   * @private
   */
  _checkSourceBias() {
    for (const [source, perf] of Object.entries(this.tracking.sourcePerformance)) {
      if (perf.total < 5) continue;

      const accuracy = perf.correct / perf.total;
      // Flag sources with very different accuracy
      if (accuracy < 0.3 || accuracy > 0.9) {
        this.biases.push({
          type: BiasType.SOURCE_BIAS,
          severity: Math.abs(accuracy - 0.6),
          details: { source, accuracy, samples: perf.total },
          suggestion: accuracy < 0.3
            ? `Source "${source}" consistently misjudged - review criteria`
            : `Source "${source}" may be too easy to judge`,
        });
      }
    }
  }

  /**
   * Calculate overall severity
   * @private
   */
  _calculateSeverity() {
    if (this.biases.length === 0) return 'none';
    const maxSeverity = Math.max(...this.biases.map(b => b.severity));
    if (maxSeverity > 0.3) return 'high';
    if (maxSeverity > 0.15) return 'medium';
    return 'low';
  }

  /**
   * Generate recommendations
   * @private
   */
  _generateRecommendations() {
    return this.biases.map(b => b.suggestion);
  }

  /**
   * Clear tracking data
   */
  reset() {
    this.biases = [];
    this.tracking = {
      confidenceErrors: [],
      verdictDistribution: { HOWL: 0, WAG: 0, GROWL: 0, BARK: 0 },
      axiomErrors: { PHI: [], VERIFY: [], CULTURE: [], BURN: [] },
      sourcePerformance: {},
    };
  }

  /**
   * Export state
   */
  export() {
    return {
      biases: this.biases,
      tracking: this.tracking,
    };
  }

  /**
   * Import state
   */
  import(state) {
    this.biases = state.biases || [];
    this.tracking = state.tracking || this.tracking;
    return this;
  }
}

// =============================================================================
// LEARNING LOOP (Main orchestrator)
// =============================================================================

/**
 * Complete Learning Loop
 *
 * Coordinates feedback analysis, weight calibration, and bias detection.
 */
export class LearningLoop {
  constructor(options = {}) {
    this.analyzer = options.analyzer || new FeedbackAnalyzer();
    this.calibrator = options.calibrator || new WeightCalibrator();
    this.biasDetector = options.biasDetector || new BiasDetector();

    /** Pending feedback for batch processing */
    this.pendingFeedback = [];

    /** Learning history */
    this.learningHistory = [];

    /** Configuration */
    this.config = {
      autoCalibrate: options.autoCalibrate !== false,
      calibrateThreshold: options.calibrateThreshold || LEARNING_CONSTANTS.MIN_SAMPLES,
      detectBiases: options.detectBiases !== false,
    };
  }

  /**
   * Process feedback
   * @param {Object} feedback
   * @returns {Object} Processing result
   */
  processFeedback(feedback) {
    // Add to analyzer
    const sample = this.analyzer.addSample(feedback);

    // Add to pending batch
    this.pendingFeedback.push(feedback);

    // Check if we should auto-calibrate
    let calibrationResult = null;
    let biasResult = null;

    if (this.config.autoCalibrate &&
        this.pendingFeedback.length >= this.config.calibrateThreshold) {
      calibrationResult = this.calibrate();
      biasResult = this.detectBiases();
      this.pendingFeedback = []; // Clear batch
    }

    return {
      sample,
      stats: this.analyzer.getStats(),
      calibration: calibrationResult,
      biases: biasResult,
    };
  }

  /**
   * Manually trigger calibration
   */
  calibrate() {
    const batch = this.pendingFeedback.length > 0
      ? this.pendingFeedback
      : this.analyzer.samples;

    const result = this.calibrator.calibrate(batch);

    if (result.updated) {
      this.learningHistory.push({
        type: 'calibration',
        timestamp: Date.now(),
        result,
      });
    }

    return result;
  }

  /**
   * Detect biases in recent feedback
   */
  detectBiases() {
    const batch = this.pendingFeedback.length > 0
      ? this.pendingFeedback
      : this.analyzer.samples;

    const result = this.biasDetector.analyze(batch);

    if (result.biases.length > 0) {
      this.learningHistory.push({
        type: 'bias_detection',
        timestamp: Date.now(),
        result,
      });
    }

    return result;
  }

  /**
   * Apply learned weights to axiom scores
   * @param {Object} breakdown - Raw axiom scores
   * @returns {Object} Calibrated scores
   */
  applyLearning(breakdown) {
    return this.calibrator.applyWeights(breakdown);
  }

  /**
   * Get current learning state
   */
  getState() {
    return {
      analyzer: this.analyzer.getStats(),
      weights: this.calibrator.getWeights(),
      biases: this.biasDetector.biases,
      pendingFeedback: this.pendingFeedback.length,
      learningHistory: this.learningHistory.slice(-10),
    };
  }

  /**
   * Get learning summary
   */
  getSummary() {
    const stats = this.analyzer.getStats();
    const weights = this.calibrator.getWeights();

    return {
      totalFeedback: stats.total,
      accuracy: stats.accuracy,
      calibrationIterations: this.calibrator.iterations,
      activeWeights: weights,
      detectedBiases: this.biasDetector.biases.length,
      recommendations: this.biasDetector.biases.map(b => b.suggestion),
    };
  }

  /**
   * Reset all learning
   */
  reset() {
    this.analyzer = new FeedbackAnalyzer();
    this.calibrator.reset();
    this.biasDetector.reset();
    this.pendingFeedback = [];
    this.learningHistory = [];
  }

  /**
   * Export state for persistence
   */
  export() {
    return {
      analyzer: this.analyzer.export(),
      calibrator: this.calibrator.export(),
      biasDetector: this.biasDetector.export(),
      learningHistory: this.learningHistory,
      config: this.config,
    };
  }

  /**
   * Import state from persistence
   */
  import(state) {
    if (state.analyzer) this.analyzer.import(state.analyzer);
    if (state.calibrator) this.calibrator.import(state.calibrator);
    if (state.biasDetector) this.biasDetector.import(state.biasDetector);
    this.learningHistory = state.learningHistory || [];
    this.config = { ...this.config, ...state.config };
    return this;
  }
}

// =============================================================================
// EXPORTS
// =============================================================================

export default {
  // Constants
  LEARNING_CONSTANTS,
  FeedbackOutcome,
  BiasType,

  // Classes
  FeedbackAnalyzer,
  WeightCalibrator,
  BiasDetector,
  LearningLoop,
};
