/**
 * Learning Service - RLHF Loop (Refactored)
 *
 * Closes the feedback loop: Human corrections → Learning → Better judgments
 *
 * "CYNIC burns its ego with every correction" - κυνικός
 *
 * ## Architecture (BURN: Composition over Duplication)
 *
 * This module now COMPOSES extracted sub-modules:
 * - FeedbackProcessor: Pattern tracking, feedback processing
 * - ExternalValidation: Tests, commits, PRs, builds as feedback
 * - RCAIntegration: Root cause analysis for test failures
 *
 * ## φ-Bounded Learning
 *
 * - Max weight adjustment: φ⁻² = 38.2%
 * - Learning rate: φ⁻³ = 23.6%
 * - Min feedback for learning: 3 (MIN_PATTERN_SOURCES)
 *
 * @module @cynic/node/judge/learning-service
 */

'use strict';

import { EventEmitter } from 'events';
import {
  PHI_INV, PHI_INV_2, PHI_INV_3, MIN_PATTERN_SOURCES,
  globalEventBus,
} from '@cynic/core';
import { getAllDimensions } from './dimensions.js';
import { FeedbackProcessor, FEEDBACK_SOURCES } from './feedback-processor.js';
import { ExternalValidation } from './external-validation.js';
import { RCAIntegration, Hypothesis, OracleType } from './rca-integration.js';

// Math modules for intelligent learning
import { BetaDistribution } from '../inference/bayes.js';
import { createMarkovChain } from '../inference/markov.js';
import { computeStats, zScore, confidenceInterval } from '../inference/gaussian.js';
import { entropyConfidence, normalizedEntropy } from '../inference/entropy.js';

// Re-export for backward compatibility
export { FEEDBACK_SOURCES, Hypothesis, OracleType };

/**
 * Learning Service - RLHF feedback loop
 *
 * Facade that composes FeedbackProcessor, ExternalValidation, and RCAIntegration.
 */
export class LearningService extends EventEmitter {
  /**
   * @param {Object} options - Service options
   * @param {Object} [options.persistence] - PersistenceManager with feedback repository
   * @param {number} [options.learningRate] - Learning rate (default: φ⁻³ = 23.6%)
   * @param {number} [options.maxAdjustment] - Max weight adjustment (default: φ⁻² = 38.2%)
   * @param {number} [options.minFeedback] - Min feedback before learning (default: 3)
   * @param {number} [options.decayRate] - How fast old learnings decay (default: 0.95)
   * @param {boolean} [options.immediateLearn] - Enable per-judgment immediate learning (default: true)
   * @param {number} [options.immediateRate] - Immediate learning rate (default: φ⁻⁴ = 0.146)
   * @param {boolean} [options.useRootCauseAnalysis] - Enable RCA (default: true)
   * @param {Object} [options.qLearningService] - QLearningService instance for routing learning
   */
  constructor(options = {}) {
    super();

    this.persistence = options.persistence || null;
    this.learningRate = options.learningRate || PHI_INV_3;
    this.maxAdjustment = options.maxAdjustment || PHI_INV_2;
    this.minFeedback = options.minFeedback || MIN_PATTERN_SOURCES;
    this.decayRate = options.decayRate || 0.95;
    this.immediateLearn = options.immediateLearn !== false;
    this.immediateRate = options.immediateRate || 0.146;
    this._useRootCauseAnalysis = options.useRootCauseAnalysis !== false;

    // FIX P4: Q-Learning bridge for routing optimization
    this._qLearningService = options.qLearningService || null;

    // Weight modifiers: dimension -> adjustment multiplier
    this._weightModifiers = new Map();

    // Threshold adjustments: itemType -> { dimension -> delta }
    this._thresholdAdjustments = new Map();

    // Discovered learnings (persistent insights)
    this._learnings = [];

    // ═══════════════════════════════════════════════════════════════════════
    // MATH MODULE INTEGRATION
    // ═══════════════════════════════════════════════════════════════════════

    // Bayesian confidence per dimension (Beta distribution)
    // α = correct feedback, β = incorrect feedback
    // Tracks which dimensions are more reliable
    this._dimensionConfidence = new Map();

    // Markov chain for feedback outcome prediction
    // States: correct, partially_correct, incorrect
    this._feedbackChain = createMarkovChain(['correct', 'partially_correct', 'incorrect']);

    // History of adjustments for Gaussian confidence intervals
    this._adjustmentHistory = [];

    // Feedback delta history for anomaly detection
    this._deltaHistory = [];

    // Compose sub-modules
    this._feedbackProcessor = new FeedbackProcessor({
      immediateRate: this.immediateRate,
      maxAdjustment: this.maxAdjustment,
      immediateLearn: this.immediateLearn,
    });

    this._rcaIntegration = new RCAIntegration({
      processFeedback: (fb) => this.processFeedback(fb),
      sourcePatterns: this._feedbackProcessor.patterns.bySource,
    });

    this._externalValidation = new ExternalValidation({
      processFeedback: (fb) => this.processFeedback(fb),
      analyzeTestFailure: (params) => this._rcaIntegration.analyzeTestFailure(params),
    });

    // Wire up events from sub-modules
    this._wireSubModuleEvents();

    this._initialized = false;
  }

  /**
   * Set Q-Learning service for routing optimization bridge
   * @param {Object} qLearningService - QLearningService instance
   */
  setQLearningService(qLearningService) {
    this._qLearningService = qLearningService;
  }

  /**
   * Wire up events from sub-modules to this service
   * @private
   */
  _wireSubModuleEvents() {
    // Forward feedback processor events
    this._feedbackProcessor.on('feedback-processed', (data) => {
      this.emit('feedback-processed', data);
    });
    this._feedbackProcessor.on('immediate-learning', (data) => {
      this.emit('immediate-learning', data);
    });
    this._feedbackProcessor.on('anomaly-processed', (data) => {
      this.emit('anomaly-processed', data);
    });

    // Forward RCA events
    this._rcaIntegration.on('root-cause:analyzed', (data) => {
      this.emit('root-cause:analyzed', data);
    });
    this._rcaIntegration.on('root-cause:learned', (data) => {
      this.emit('root-cause:learned', data);
    });
    this._rcaIntegration.on('root-cause:feedback', (data) => {
      this.emit('root-cause:feedback', data);
    });
    this._rcaIntegration.on('root-cause:processed', (data) => {
      this.emit('root-cause:processed', data);
    });
    this._rcaIntegration.on('spec-gap-detected', (data) => {
      this.emit('spec-gap-detected', data);
    });

    // Forward external validation events
    this._externalValidation.on('test-result-processed', (data) => {
      this.emit('test-result-processed', data);
    });
    this._externalValidation.on('test-bug-detected', (data) => {
      this.emit('test-bug-detected', data);
    });
    this._externalValidation.on('flaky-test-detected', (data) => {
      this.emit('flaky-test-detected', data);
    });
    this._externalValidation.on('spec-gap-detected', (data) => {
      this.emit('spec-gap-detected', data);
    });
    this._externalValidation.on('commit-result-processed', (data) => {
      this.emit('commit-result-processed', data);
    });
    this._externalValidation.on('pr-result-processed', (data) => {
      this.emit('pr-result-processed', data);
    });
    this._externalValidation.on('build-result-processed', (data) => {
      this.emit('build-result-processed', data);
    });
    this._externalValidation.on('deploy-result-processed', (data) => {
      this.emit('deploy-result-processed', data);
    });
  }

  /**
   * Initialize the service
   */
  async init() {
    if (this._initialized) return;

    // Initialize all dimension modifiers to 1.0 (no change)
    const allDimensions = getAllDimensions();
    for (const name of Object.keys(allDimensions)) {
      this._weightModifiers.set(name, 1.0);
    }

    // Load persisted learning state if available
    if (this.persistence) {
      await this._loadState();
    }

    // Subscribe to circuit breaker events for learning
    this._subscribeToCircuitBreakerEvents();

    // Initialize RCA integration
    if (this._useRootCauseAnalysis) {
      this._rcaIntegration.init();
    }

    this._initialized = true;
    this.emit('initialized');
  }

  /**
   * Subscribe to circuit breaker events for reliability learning
   * @private
   */
  _subscribeToCircuitBreakerEvents() {
    globalEventBus.on('CIRCUIT_OPENED', (event) => {
      const { service, feedback } = event.payload || {};
      if (feedback?.dimensionScores) {
        for (const [dim, score] of Object.entries(feedback.dimensionScores)) {
          if (this._weightModifiers.has(dim) || dim === 'reliability' || dim === 'availability') {
            const delta = -this.immediateRate * (1 - score / 100);
            this.adjustDimensionWeight(dim, delta);
          }
        }
      }
      this.emit('circuit:learned', { service, type: 'opened', feedback });
    });

    globalEventBus.on('CIRCUIT_RECOVERED', (event) => {
      const { service, feedback } = event.payload || {};
      if (feedback?.dimensionScores) {
        for (const [dim, score] of Object.entries(feedback.dimensionScores)) {
          if (this._weightModifiers.has(dim) || dim === 'reliability' || dim === 'availability') {
            const delta = this.immediateRate * 0.5 * (score / 100);
            this.adjustDimensionWeight(dim, delta);
          }
        }
      }
      this.emit('circuit:learned', { service, type: 'recovered', feedback });
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // WEIGHT MODIFIERS
  // ═══════════════════════════════════════════════════════════════════════════

  getWeightModifier(dimension) {
    return this._weightModifiers.get(dimension) || 1.0;
  }

  getAllWeightModifiers() {
    return Object.fromEntries(this._weightModifiers);
  }

  adjustDimensionWeight(dimension, delta) {
    const currentModifier = this._weightModifiers.get(dimension) || 1.0;
    const boundedDelta = Math.max(-this.maxAdjustment, Math.min(this.maxAdjustment, delta));
    let newModifier = currentModifier + boundedDelta;
    newModifier = Math.max(1 - this.maxAdjustment, Math.min(1 + this.maxAdjustment, newModifier));
    this._weightModifiers.set(dimension, newModifier);

    this.emit('weight:adjusted', {
      dimension,
      oldModifier: currentModifier,
      newModifier,
      delta: boundedDelta,
      source: 'sona',
    });

    return newModifier;
  }

  getThresholdAdjustment(itemType, dimension) {
    const typeAdjustments = this._thresholdAdjustments.get(itemType);
    if (!typeAdjustments) return 0;
    return typeAdjustments.get(dimension) || 0;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // MATH MODULE METHODS (Bayes, Markov, Gaussian, Entropy)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get or create dimension confidence tracker
   * @private
   * @param {string} dimension - Dimension name
   * @returns {BetaDistribution}
   */
  _getDimensionConfidence(dimension) {
    if (!this._dimensionConfidence.has(dimension)) {
      // Prior: slight optimism α=2, β=1
      this._dimensionConfidence.set(dimension, new BetaDistribution(2, 1));
    }
    return this._dimensionConfidence.get(dimension);
  }

  /**
   * Update dimension confidence based on feedback outcome
   * @private
   * @param {string} dimension - Dimension name
   * @param {boolean} wasCorrect - Whether feedback was correct
   */
  _updateDimensionConfidence(dimension, wasCorrect) {
    const tracker = this._getDimensionConfidence(dimension);
    if (wasCorrect) {
      tracker.recordSuccess();
    } else {
      tracker.recordFailure();
    }
  }

  /**
   * Get Bayesian confidence for a dimension
   * @param {string} dimension - Dimension name
   * @returns {Object} {probability, confidence, α, β}
   */
  getDimensionConfidenceEstimate(dimension) {
    const tracker = this._getDimensionConfidence(dimension);
    const mean = tracker.getMean();
    const strength = tracker.getStrength();

    // Confidence in our estimate grows with observations, φ-bounded
    const confidence = Math.min(PHI_INV, strength / 30); // 30 feedback = max confidence

    return {
      probability: mean,
      confidence,
      alpha: tracker.alpha,
      beta: tracker.beta,
      strength,
    };
  }

  /**
   * Predict next feedback outcome using Markov chain
   * @private
   * @param {string} lastOutcome - Last feedback outcome
   * @returns {Object} Prediction
   */
  _predictFeedbackOutcome(lastOutcome) {
    const prediction = this._feedbackChain.predict(lastOutcome || 'correct');
    return {
      predicted: prediction.state || 'correct',
      probability: prediction.probability,
      confidence: prediction.confidence,
    };
  }

  /**
   * Record feedback outcome for Markov learning
   * @private
   * @param {string} outcome - Feedback outcome
   */
  _recordFeedbackOutcome(outcome) {
    // Normalize outcome to our states
    let state = 'correct';
    if (outcome === 'incorrect' || outcome === 'fail' || outcome === 'rejected') {
      state = 'incorrect';
    } else if (outcome === 'partially_correct' || outcome === 'partial') {
      state = 'partially_correct';
    }

    // Record transition from last outcome
    if (this._lastFeedbackOutcome) {
      this._feedbackChain.observe(this._lastFeedbackOutcome, state);
    }
    this._lastFeedbackOutcome = state;
  }

  /**
   * Record adjustment for confidence interval calculation
   * @private
   * @param {number} delta - Adjustment delta
   */
  _recordAdjustment(delta) {
    this._adjustmentHistory.push(delta);
    // Keep bounded at Fib(10) = 55
    while (this._adjustmentHistory.length > 55) {
      this._adjustmentHistory.shift();
    }
  }

  /**
   * Get confidence interval for typical adjustments
   * Uses Gaussian statistics on historical adjustments
   * @returns {Object} {mean, stdDev, interval, confidence}
   */
  getAdjustmentConfidenceInterval() {
    if (this._adjustmentHistory.length < 5) {
      return { mean: 0, stdDev: 0, interval: [0, 0], confidence: 0 };
    }

    const stats = computeStats(this._adjustmentHistory);
    // 95% confidence interval (≈1.96 std devs, but φ-bounded)
    const ci = confidenceInterval(stats.mean, stats.stdDev, this._adjustmentHistory.length, 0.95);

    return {
      mean: stats.mean,
      stdDev: stats.stdDev,
      interval: ci,
      confidence: Math.min(PHI_INV, this._adjustmentHistory.length / 30),
    };
  }

  /**
   * Detect if a feedback delta is anomalous
   * @private
   * @param {number} delta - Score delta
   * @returns {Object} {isAnomaly, zScore, severity}
   */
  _detectFeedbackAnomaly(delta) {
    this._deltaHistory.push(delta);
    // Keep bounded at Fib(8) = 21
    while (this._deltaHistory.length > 21) {
      this._deltaHistory.shift();
    }

    if (this._deltaHistory.length < 5) {
      return { isAnomaly: false, zScore: 0, severity: 'none' };
    }

    const history = this._deltaHistory.slice(0, -1);
    const stats = computeStats(history);
    const z = zScore(delta, stats.mean, stats.stdDev);

    let severity = 'none';
    let isAnomaly = false;

    if (Math.abs(z) > 3) {
      severity = 'critical';
      isAnomaly = true;
    } else if (Math.abs(z) > 2) {
      severity = 'significant';
      isAnomaly = true;
    } else if (Math.abs(z) > 1.5) {
      severity = 'minor';
    }

    return {
      isAnomaly,
      zScore: Math.round(z * 100) / 100,
      severity,
      mean: stats.mean,
      stdDev: stats.stdDev,
    };
  }

  /**
   * Calculate learning entropy (uncertainty in feedback patterns)
   * @returns {Object} {entropy, normalized, state}
   */
  getLearningEntropy() {
    const patterns = this._feedbackProcessor.patterns;
    const counts = [
      patterns.overall.correctCount || 0,
      patterns.overall.totalFeedback - (patterns.overall.correctCount || 0),
    ].filter(c => c > 0);

    if (counts.length <= 1) {
      return { entropy: 0, normalized: 0, state: 'STABLE', confidence: PHI_INV };
    }

    const analysis = entropyConfidence(counts);

    // Classify learning state based on entropy
    let state = 'STABLE';
    if (analysis.normalized > PHI_INV) {
      state = 'HIGH_UNCERTAINTY';
    } else if (analysis.normalized > PHI_INV_2) {
      state = 'MODERATE_UNCERTAINTY';
    } else if (analysis.normalized > PHI_INV_3) {
      state = 'LOW_UNCERTAINTY';
    }

    return {
      entropy: analysis.entropy,
      normalized: analysis.normalized,
      state,
      confidence: analysis.confidence,
    };
  }

  /**
   * Get dimension reliability ranking using Bayesian estimates
   * @returns {Array} Dimensions sorted by reliability
   */
  getDimensionReliabilityRanking() {
    const rankings = [];
    for (const [dimension, tracker] of this._dimensionConfidence) {
      const mean = tracker.getMean();
      const strength = tracker.getStrength();
      const confidence = Math.min(PHI_INV, strength / 30);

      rankings.push({
        dimension,
        reliability: mean,
        confidence,
        strength,
        alpha: tracker.alpha,
        beta: tracker.beta,
      });
    }

    // Sort by reliability (descending)
    rankings.sort((a, b) => b.reliability - a.reliability);
    return rankings;
  }

  /**
   * Get inference statistics
   * @returns {Object} Math module stats
   */
  getInferenceStats() {
    const entropy = this.getLearningEntropy();
    const adjustmentCI = this.getAdjustmentConfidenceInterval();
    const prediction = this._predictFeedbackOutcome(this._lastFeedbackOutcome);

    return {
      entropy,
      adjustmentConfidenceInterval: adjustmentCI,
      nextFeedbackPrediction: prediction,
      dimensionReliability: this.getDimensionReliabilityRanking().slice(0, 5), // Top 5
      feedbackChainSize: this._feedbackChain.getMatrix ? Object.keys(this._feedbackChain.getMatrix()).length : 0,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // FEEDBACK PROCESSING (Delegates to FeedbackProcessor)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * For backward compatibility
   */
  static FEEDBACK_SOURCES = FEEDBACK_SOURCES;

  processFeedback(feedback) {
    // Record for Markov learning
    this._recordFeedbackOutcome(feedback.outcome);

    // Detect anomaly in score delta
    const anomaly = this._detectFeedbackAnomaly(feedback.scoreDelta || 0);
    if (anomaly.isAnomaly) {
      this.emit('feedback-anomaly', {
        feedback,
        anomaly,
      });
    }

    // Update dimension confidence if dimension info available
    if (feedback.dimensions) {
      const isCorrect = feedback.outcome === 'correct' || feedback.outcome === 'pass';
      for (const dim of Object.keys(feedback.dimensions)) {
        this._updateDimensionConfidence(dim, isCorrect);
      }
    }

    // Delegate to FeedbackProcessor
    const result = this._feedbackProcessor.processFeedback(feedback, this._weightModifiers);

    // Enrich result with inference data
    result.inference = {
      anomaly: anomaly.isAnomaly ? anomaly : null,
      entropy: this.getLearningEntropy(),
      prediction: this._predictFeedbackOutcome(feedback.outcome),
    };

    return result;
  }

  processAnomalySignal(signal) {
    return this._feedbackProcessor.processAnomalySignal(signal);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // EXTERNAL VALIDATION (Delegates to ExternalValidation)
  // ═══════════════════════════════════════════════════════════════════════════

  processTestResult(params) {
    return this._externalValidation.processTestResult({
      ...params,
      useRootCause: params.useRootCause ?? this._useRootCauseAnalysis,
    });
  }

  processCommitResult(params) {
    return this._externalValidation.processCommitResult(params);
  }

  processPRResult(params) {
    return this._externalValidation.processPRResult(params);
  }

  processBuildResult(params) {
    return this._externalValidation.processBuildResult(params);
  }

  processDeployResult(params) {
    return this._externalValidation.processDeployResult(params);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // ROOT CAUSE ANALYSIS (Delegates to RCAIntegration)
  // ═══════════════════════════════════════════════════════════════════════════

  getRootCauseAnalyzer() {
    return this._rcaIntegration.getRootCauseAnalyzer();
  }

  analyzeTestFailure(params) {
    return this._rcaIntegration.analyzeTestFailure(params);
  }

  feedbackOnRootCause(actualCause, wasCorrect) {
    return this._rcaIntegration.feedbackOnRootCause(actualCause, wasCorrect);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // LEARNING ALGORITHM
  // ═══════════════════════════════════════════════════════════════════════════

  async learn() {
    const feedbackQueue = this._feedbackProcessor.feedbackQueue;
    if (feedbackQueue.length < this.minFeedback) {
      return {
        success: false,
        reason: `Insufficient feedback (${feedbackQueue.length}/${this.minFeedback})`,
      };
    }

    const startTime = Date.now();
    const feedbackBatch = [...feedbackQueue];
    this._feedbackProcessor.feedbackQueue = [];

    const adjustments = this._calculateAdjustments(feedbackBatch);
    this._applyAdjustments(adjustments);
    this._applyDecay();

    this._feedbackProcessor.patterns.overall.learningIterations++;

    if (this.persistence) {
      await this._saveState();
      await this._syncPatternsToRepository();

      for (const fb of feedbackBatch) {
        if (fb.feedbackId && this.persistence.feedback) {
          try {
            await this.persistence.feedback.markApplied(fb.feedbackId);
          } catch (_) {}
        }
      }
    }

    // FIX P4: Bridge feedback to Q-Learning for routing optimization
    // "Le chien apprend de ses erreurs" - κυνικός
    if (this._qLearningService && feedbackBatch.length > 0) {
      for (const fb of feedbackBatch) {
        try {
          const reward = this._feedbackToReward(fb);
          const state = fb.context?.taskType || fb.itemType || 'general';
          const action = fb.context?.dogUsed || fb.context?.agent || 'default';

          // Record as Q-Learning episode feedback
          this._qLearningService.recordFeedback?.({
            state,
            action,
            reward,
            source: fb.source,
            outcome: fb.outcome,
          });
        } catch (_) {
          // Q-Learning bridge is best-effort
        }
      }
    }

    const result = {
      success: true,
      feedbackProcessed: feedbackBatch.length,
      adjustmentsMade: Object.keys(adjustments.weights).length,
      learningIteration: this._feedbackProcessor.patterns.overall.learningIterations,
      duration: Date.now() - startTime,
      qLearningBridged: this._qLearningService ? feedbackBatch.length : 0,
    };

    this.emit('learning-complete', result);
    return result;
  }

  /**
   * Convert feedback to Q-Learning reward signal
   * @private
   */
  _feedbackToReward(feedback) {
    // Map feedback outcomes to rewards
    const rewardMap = {
      correct: 1.0,
      partially_correct: 0.5,
      incorrect: -1.0,
      pass: 1.0,
      fail: -1.0,
      merged: 1.0,
      rejected: -0.5,
      build_success: 0.8,
      build_failure: -0.8,
    };

    return rewardMap[feedback.outcome] ?? 0;
  }

  _calculateAdjustments(feedbackBatch) {
    const weightDeltas = new Map();
    const thresholdDeltas = new Map();
    const patterns = this._feedbackProcessor.patterns;

    const byType = new Map();
    for (const fb of feedbackBatch) {
      const type = fb.itemType || 'unknown';
      if (!byType.has(type)) byType.set(type, []);
      byType.get(type).push(fb);
    }

    for (const [itemType, items] of byType) {
      const avgDelta = items.reduce((sum, fb) => sum + fb.scoreDelta, 0) / items.length;
      if (Math.abs(avgDelta) > 5 && items.length >= 2) {
        if (!thresholdDeltas.has(itemType)) thresholdDeltas.set(itemType, new Map());
        const adjustment = -avgDelta * this.learningRate;
        thresholdDeltas.get(itemType).set('_general', adjustment);
      }
    }

    for (const [dimension, pattern] of patterns.byDimension) {
      if (pattern.feedbackCount < this.minFeedback) continue;
      if (Math.abs(pattern.avgError) > 10) {
        const delta = -pattern.avgError * this.learningRate * 0.01;
        weightDeltas.set(dimension, delta);
      }
    }

    return {
      weights: Object.fromEntries(weightDeltas),
      thresholds: Object.fromEntries(
        [...thresholdDeltas.entries()].map(([k, v]) => [k, Object.fromEntries(v)])
      ),
    };
  }

  _applyAdjustments(adjustments) {
    for (const [dimension, delta] of Object.entries(adjustments.weights)) {
      const current = this._weightModifiers.get(dimension) || 1.0;
      let newModifier = current + delta;
      const minMod = 1 - this.maxAdjustment;
      const maxMod = 1 + this.maxAdjustment;
      newModifier = Math.max(minMod, Math.min(maxMod, newModifier));
      this._weightModifiers.set(dimension, newModifier);

      // Record adjustment for Gaussian confidence intervals
      this._recordAdjustment(delta);
    }

    for (const [itemType, dims] of Object.entries(adjustments.thresholds)) {
      if (!this._thresholdAdjustments.has(itemType)) {
        this._thresholdAdjustments.set(itemType, new Map());
      }
      for (const [dim, delta] of Object.entries(dims)) {
        const current = this._thresholdAdjustments.get(itemType).get(dim) || 0;
        let newAdjustment = current + delta;
        newAdjustment = Math.max(-15, Math.min(15, newAdjustment));
        this._thresholdAdjustments.get(itemType).set(dim, newAdjustment);
      }
    }
  }

  _applyDecay() {
    for (const [dimension, modifier] of this._weightModifiers) {
      const decayed = 1.0 + (modifier - 1.0) * this.decayRate;
      this._weightModifiers.set(dimension, decayed);
    }

    for (const [, dims] of this._thresholdAdjustments) {
      for (const [dim, adjustment] of dims) {
        dims.set(dim, adjustment * this.decayRate);
      }
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // AUTO-LEARNING
  // ═══════════════════════════════════════════════════════════════════════════

  async pullFeedback(limit = 100) {
    if (!this.persistence?.feedback) {
      return { success: false, reason: 'No feedback repository' };
    }

    try {
      const unapplied = await this.persistence.feedback.findUnapplied(limit);
      for (const fb of unapplied) {
        this.processFeedback({
          feedbackId: fb.id,
          outcome: fb.outcome,
          actualScore: fb.actual_score,
          originalScore: fb.q_score,
          itemType: fb.item_type,
          reason: fb.reason,
        });
      }
      return {
        success: true,
        pulled: unapplied.length,
        queueSize: this._feedbackProcessor.feedbackQueue.length,
      };
    } catch (error) {
      return { success: false, reason: error.message };
    }
  }

  async runLearningCycle() {
    await this.init();
    const pullResult = await this.pullFeedback();
    const learnResult = await this.learn();
    return {
      pull: pullResult,
      learn: learnResult,
      patterns: this.getPatterns(),
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // INSIGHTS & STATISTICS
  // ═══════════════════════════════════════════════════════════════════════════

  getPatterns() {
    const processed = this._feedbackProcessor.getPatterns(this.minFeedback);
    return {
      ...processed,
      learnings: [...this._learnings],
    };
  }

  addLearning(learning) {
    const entry = {
      id: `learn_${Date.now().toString(36)}`,
      pattern: learning.pattern,
      insight: learning.insight,
      source: learning.source || 'manual',
      confidence: learning.confidence || 0.5,
      createdAt: Date.now(),
      appliedCount: 0,
    };
    this._learnings.push(entry);
    this.emit('learning-added', entry);
    return entry;
  }

  getLearnings() {
    return [...this._learnings];
  }

  getState() {
    return {
      weightModifiers: Object.fromEntries(this._weightModifiers),
      thresholdAdjustments: Object.fromEntries(
        [...this._thresholdAdjustments.entries()].map(([k, v]) => [k, Object.fromEntries(v)])
      ),
      queueSize: this._feedbackProcessor.feedbackQueue.length,
      patterns: this.getPatterns(),
      config: {
        learningRate: this.learningRate,
        maxAdjustment: this.maxAdjustment,
        minFeedback: this.minFeedback,
        decayRate: this.decayRate,
      },
    };
  }

  getStats() {
    const patterns = this.getPatterns();
    const modifiedDimensions = [...this._weightModifiers.entries()]
      .filter(([, v]) => Math.abs(v - 1.0) > 0.01)
      .length;

    return {
      initialized: this._initialized,
      totalFeedback: patterns.overall.totalFeedback,
      accuracy: Math.round(patterns.accuracy * 1000) / 10,
      avgScoreError: Math.round(patterns.overall.avgScoreError * 10) / 10,
      learningIterations: patterns.overall.learningIterations,
      modifiedDimensions,
      itemTypesTracked: Object.keys(patterns.byItemType).length,
      insightsCount: patterns.insights.length,
      queueSize: this._feedbackProcessor.feedbackQueue.length,
      // New: Inference enrichments
      inference: this.getInferenceStats(),
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PERSISTENCE
  // ═══════════════════════════════════════════════════════════════════════════

  async _loadState() {
    await this._loadPatternsFromRepository();

    if (this.persistence?.knowledge) {
      try {
        const results = await this.persistence.knowledge.search('learning_state', {
          category: 'system',
          limit: 1,
        });
        if (results.length > 0 && results[0].patterns) {
          this.import(results[0].patterns);
          this.emit('state-loaded', { source: 'persistence' });
        }
      } catch (_) {}
    }
  }

  async _saveState() {
    if (this.persistence?.knowledge) {
      try {
        const state = this.export();
        await this.persistence.storeKnowledge({
          sourceType: 'system',
          sourceRef: 'learning_state',
          summary: `Learning state: ${this._feedbackProcessor.patterns.overall.learningIterations} iterations`,
          content: JSON.stringify(state),
          category: 'system',
          patterns: state,
        });
      } catch (_) {}
    }
  }

  async _syncPatternsToRepository() {
    if (!this.persistence?.patterns) return;

    try {
      const patterns = this._feedbackProcessor.patterns;

      for (const [dimension, pattern] of patterns.byDimension) {
        await this.persistence.patterns.upsert({
          category: 'learning:dimension',
          name: dimension,
          description: `Learning pattern for dimension: ${dimension}`,
          confidence: Math.min(pattern.avgError ? (1 - pattern.avgError / 100) : 0.5, 0.618),
          frequency: pattern.feedbackCount || 1,
          tags: ['learning', 'dimension', dimension],
          data: {
            avgError: pattern.avgError,
            feedbackCount: pattern.feedbackCount,
            weightModifier: this._weightModifiers.get(dimension) || 1.0,
          },
        });
      }

      for (const [itemType, pattern] of patterns.byItemType) {
        await this.persistence.patterns.upsert({
          category: 'learning:itemType',
          name: itemType,
          description: `Learning pattern for item type: ${itemType}`,
          confidence: Math.min(0.618, 0.5 + (pattern.overscoring + pattern.underscoring > 10 ? 0.1 : 0)),
          frequency: pattern.overscoring + pattern.underscoring,
          tags: ['learning', 'itemType', itemType],
          data: {
            overscoring: pattern.overscoring,
            underscoring: pattern.underscoring,
            bias: pattern.overscoring - pattern.underscoring,
          },
        });
      }

      for (const [source, pattern] of patterns.bySource) {
        await this.persistence.patterns.upsert({
          category: 'learning:source',
          name: source,
          description: `Learning pattern for source: ${source}`,
          confidence: Math.min(pattern.correctRate || 0.5, 0.618),
          frequency: pattern.count || 1,
          tags: ['learning', 'source', source.split(':')[0]],
          data: {
            count: pattern.count,
            correctRate: pattern.correctRate,
            avgDelta: pattern.avgDelta,
          },
        });
      }

      await this.persistence.patterns.upsert({
        patternId: 'pat_learning_overall',
        category: 'learning:meta',
        name: 'overall_statistics',
        description: 'Overall learning statistics across all feedback',
        confidence: patterns.overall.totalFeedback > 0
          ? Math.min(patterns.overall.correctCount / patterns.overall.totalFeedback, 0.618)
          : 0.5,
        frequency: patterns.overall.learningIterations,
        tags: ['learning', 'meta', 'statistics'],
        data: {
          ...patterns.overall,
          accuracy: patterns.overall.totalFeedback > 0
            ? patterns.overall.correctCount / patterns.overall.totalFeedback
            : 0,
        },
      });

      this.emit('patterns:synced', {
        dimensions: patterns.byDimension.size,
        itemTypes: patterns.byItemType.size,
        sources: patterns.bySource.size,
      });
    } catch (e) {
      this.emit('patterns:sync-error', { error: e.message });
    }
  }

  async _loadPatternsFromRepository() {
    if (!this.persistence?.patterns) return;

    try {
      const dimensionPatterns = await this.persistence.patterns.findByCategory('learning:dimension');
      for (const pat of dimensionPatterns) {
        if (pat.data) {
          this._feedbackProcessor.patterns.byDimension.set(pat.name, {
            avgError: pat.data.avgError || 0,
            feedbackCount: pat.data.feedbackCount || 0,
            scoreSum: 0,
          });
          if (pat.data.weightModifier) {
            this._weightModifiers.set(pat.name, pat.data.weightModifier);
          }
        }
      }

      const itemTypePatterns = await this.persistence.patterns.findByCategory('learning:itemType');
      for (const pat of itemTypePatterns) {
        if (pat.data) {
          this._feedbackProcessor.patterns.byItemType.set(pat.name, {
            overscoring: pat.data.overscoring || 0,
            underscoring: pat.data.underscoring || 0,
            feedbackCount: 0,
            avgDelta: 0,
          });
        }
      }

      const sourcePatterns = await this.persistence.patterns.findByCategory('learning:source');
      for (const pat of sourcePatterns) {
        if (pat.data) {
          this._feedbackProcessor.patterns.bySource.set(pat.name, {
            count: pat.data.count || 0,
            correctCount: 0,
            incorrectCount: 0,
            correctRate: pat.data.correctRate || 0,
            avgDelta: pat.data.avgDelta || 0,
            firstSeen: Date.now(),
            lastSeen: Date.now(),
          });
        }
      }

      const overall = await this.persistence.patterns.findById('pat_learning_overall');
      if (overall?.data) {
        Object.assign(this._feedbackProcessor.patterns.overall, overall.data);
      }

      if (this.persistence?.patternEvolution) {
        try {
          const evolved = await this.persistence.patternEvolution.findByType('dimension');
          for (const row of evolved) {
            if (row.pattern_key && row.weight_modifier != null) {
              if (!this._weightModifiers.has(row.pattern_key)) {
                this._weightModifiers.set(row.pattern_key, parseFloat(row.weight_modifier));
              }
            }
          }
        } catch (_) {}
      }

      this.emit('patterns:loaded', {
        dimensions: this._feedbackProcessor.patterns.byDimension.size,
        itemTypes: this._feedbackProcessor.patterns.byItemType.size,
        sources: this._feedbackProcessor.patterns.bySource.size,
        evolvedWeights: this._weightModifiers.size,
      });
    } catch (_) {}
  }

  export() {
    return {
      weightModifiers: Object.fromEntries(this._weightModifiers),
      thresholdAdjustments: Object.fromEntries(
        [...this._thresholdAdjustments.entries()].map(([k, v]) => [k, Object.fromEntries(v)])
      ),
      patterns: this._feedbackProcessor.exportPatterns(),
      learnings: [...this._learnings],
      exportedAt: Date.now(),
    };
  }

  import(state) {
    if (state.weightModifiers) {
      for (const [dim, mod] of Object.entries(state.weightModifiers)) {
        this._weightModifiers.set(dim, mod);
      }
    }

    if (state.thresholdAdjustments) {
      for (const [itemType, dims] of Object.entries(state.thresholdAdjustments)) {
        this._thresholdAdjustments.set(itemType, new Map(Object.entries(dims)));
      }
    }

    if (state.patterns) {
      this._feedbackProcessor.importPatterns(state.patterns);
    }

    if (state.learnings && Array.isArray(state.learnings)) {
      this._learnings = [...state.learnings];
    }
  }

  reset() {
    for (const dim of this._weightModifiers.keys()) {
      this._weightModifiers.set(dim, 1.0);
    }
    this._thresholdAdjustments.clear();
    this._feedbackProcessor.reset();
    this._learnings = [];

    // Reset math module state
    this._dimensionConfidence.clear();
    this._feedbackChain = createMarkovChain(['correct', 'partially_correct', 'incorrect']);
    this._adjustmentHistory = [];
    this._deltaHistory = [];
    this._lastFeedbackOutcome = null;

    this.emit('reset');
  }

  // Backward compatibility: exportToMarkdown and importFromMarkdown
  exportToMarkdown() {
    const patterns = this.getPatterns();
    const stats = this.getStats();
    const lines = [];

    lines.push('# CYNIC Learnings');
    lines.push('');
    lines.push(`> Last updated: ${new Date().toISOString()}`);
    lines.push('');
    lines.push('## Statistics');
    lines.push('');
    lines.push(`- Total feedback: ${stats.totalFeedback}`);
    lines.push(`- Accuracy: ${stats.accuracy}%`);
    lines.push(`- Learning iterations: ${patterns.overall.learningIterations}`);
    lines.push('');

    if (this._learnings.length > 0) {
      lines.push('## Discovered Learnings');
      lines.push('');
      for (const learning of this._learnings) {
        lines.push(`### ${learning.pattern}`);
        lines.push(`- Insight: ${learning.insight}`);
        lines.push(`- Source: ${learning.source}`);
        lines.push(`- Confidence: ${(learning.confidence * 100).toFixed(0)}%`);
        lines.push('');
      }
    }

    return lines.join('\n');
  }

  importFromMarkdown(markdown) {
    const learningsMatch = markdown.match(/## Discovered Learnings\n\n([\s\S]*?)(?=\n## |$)/);
    if (learningsMatch) {
      const learningsSection = learningsMatch[1];
      const learningBlocks = learningsSection.split(/\n### /).filter(Boolean);

      for (const block of learningBlocks) {
        const lines = block.split('\n');
        const pattern = lines[0]?.trim();
        if (!pattern) continue;

        const learning = { pattern };
        for (const line of lines.slice(1)) {
          const match = line.match(/^- (\w+): (.+)$/);
          if (match) {
            const [, key, value] = match;
            if (key === 'Insight') learning.insight = value;
            if (key === 'Source') learning.source = value;
            if (key === 'Confidence') learning.confidence = parseFloat(value) / 100;
          }
        }

        if (learning.insight) {
          this._learnings.push({
            id: `learn_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`,
            ...learning,
            createdAt: Date.now(),
            appliedCount: 0,
          });
        }
      }
    }
  }
}

export default LearningService;
