/**
 * Learning Service - RLHF Loop
 *
 * Closes the feedback loop: Human corrections → Learning → Better judgments
 *
 * "CYNIC burns its ego with every correction" - κυνικός
 *
 * ## Architecture
 *
 * 1. Process unapplied feedback from FeedbackRepository
 * 2. Calculate learning deltas (prediction error)
 * 3. Update weight modifiers (bounded by φ⁻²)
 * 4. Track learning patterns for systematic bias detection
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
import { PHI_INV, PHI_INV_2, PHI_INV_3, MIN_PATTERN_SOURCES } from '@cynic/core';
import { getAllDimensions } from './dimensions.js';

/**
 * Learning Service - RLHF feedback loop
 */
export class LearningService extends EventEmitter {
  /**
   * @param {Object} options - Service options
   * @param {Object} [options.persistence] - PersistenceManager with feedback repository
   * @param {number} [options.learningRate] - Learning rate (default: φ⁻³ = 23.6%)
   * @param {number} [options.maxAdjustment] - Max weight adjustment (default: φ⁻² = 38.2%)
   * @param {number} [options.minFeedback] - Min feedback before learning (default: 3)
   * @param {number} [options.decayRate] - How fast old learnings decay (default: 0.95)
   */
  constructor(options = {}) {
    super();

    this.persistence = options.persistence || null;
    this.learningRate = options.learningRate || PHI_INV_3;
    this.maxAdjustment = options.maxAdjustment || PHI_INV_2;
    this.minFeedback = options.minFeedback || MIN_PATTERN_SOURCES;
    this.decayRate = options.decayRate || 0.95;

    // Weight modifiers: dimension -> adjustment multiplier
    // 1.0 = no change, 0.8 = 20% decrease, 1.2 = 20% increase
    this._weightModifiers = new Map();

    // Threshold adjustments: itemType -> { dimension -> delta }
    this._thresholdAdjustments = new Map();

    // Learning patterns: track systematic biases
    this._patterns = {
      byItemType: new Map(), // itemType -> { overscoring: count, underscoring: count }
      byDimension: new Map(), // dimension -> { avgError, feedbackCount }
      bySource: new Map(),    // source -> { count, correctRate, avgDelta }
      overall: {
        totalFeedback: 0,
        correctCount: 0,
        incorrectCount: 0,
        avgScoreError: 0,
        learningIterations: 0,
      },
    };

    // Discovered learnings (persistent insights)
    this._learnings = [];

    // Feedback queue for batch processing
    this._feedbackQueue = [];

    this._initialized = false;
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

    this._initialized = true;
    this.emit('initialized');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // WEIGHT MODIFIERS (for CYNICJudge to use)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get weight modifier for a dimension
   * CYNICJudge calls this when calculating scores
   *
   * @param {string} dimension - Dimension name
   * @returns {number} Modifier (1.0 = no change)
   */
  getWeightModifier(dimension) {
    return this._weightModifiers.get(dimension) || 1.0;
  }

  /**
   * Get all weight modifiers
   * @returns {Object} Dimension -> modifier map
   */
  getAllWeightModifiers() {
    return Object.fromEntries(this._weightModifiers);
  }

  /**
   * Adjust dimension weight (called by SONA for real-time adaptation)
   *
   * @param {string} dimension - Dimension name
   * @param {number} delta - Adjustment amount (bounded by maxAdjustment)
   * @returns {number} New modifier value
   */
  adjustDimensionWeight(dimension, delta) {
    const currentModifier = this._weightModifiers.get(dimension) || 1.0;

    // Bound the delta
    const boundedDelta = Math.max(
      -this.maxAdjustment,
      Math.min(this.maxAdjustment, delta)
    );

    // Calculate new modifier
    let newModifier = currentModifier + boundedDelta;

    // Bound the final modifier: [1 - φ⁻², 1 + φ⁻²] = [0.618, 1.382]
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

  /**
   * Get threshold adjustment for item type and dimension
   *
   * @param {string} itemType - Item type (code, decision, etc.)
   * @param {string} dimension - Dimension name
   * @returns {number} Threshold adjustment (0 = no change)
   */
  getThresholdAdjustment(itemType, dimension) {
    const typeAdjustments = this._thresholdAdjustments.get(itemType);
    if (!typeAdjustments) return 0;
    return typeAdjustments.get(dimension) || 0;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // FEEDBACK PROCESSING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Feedback sources (Ralph-inspired: external validation gates)
   */
  static FEEDBACK_SOURCES = {
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
   * Process a single feedback item
   *
   * @param {Object} feedback - Feedback data
   * @param {string} feedback.outcome - 'correct', 'incorrect', 'partial'
   * @param {number} [feedback.actualScore] - What the score should have been
   * @param {number} feedback.originalScore - What CYNIC scored it
   * @param {string} feedback.itemType - Type of item judged
   * @param {Object} [feedback.dimensionScores] - Original dimension scores
   * @param {string} [feedback.source] - Feedback source (manual, test_result, commit, etc.)
   * @param {Object} [feedback.sourceContext] - Additional context from source
   * @returns {Object} Learning result
   */
  processFeedback(feedback) {
    const {
      outcome,
      actualScore,
      originalScore,
      itemType = 'unknown',
      dimensionScores = {},
      source = LearningService.FEEDBACK_SOURCES.MANUAL,
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

    // Track by source (Ralph-inspired: external validation gates)
    this._trackSourcePattern(source, outcome, scoreDelta);

    // Queue for batch learning
    this._feedbackQueue.push({
      ...feedback,
      source,
      sourceContext,
      scoreDelta,
      processedAt: Date.now(),
    });

    const result = {
      scoreDelta,
      queueSize: this._feedbackQueue.length,
      shouldLearn: this._feedbackQueue.length >= this.minFeedback,
    };

    this.emit('feedback-processed', result);
    return result;
  }

  /**
   * Infer score delta from outcome when actual score not provided
   * @private
   */
  _inferDeltaFromOutcome(outcome, originalScore) {
    switch (outcome) {
      case 'correct':
        return 0; // No change needed
      case 'incorrect':
        // Assume we were significantly wrong
        // If score was high, assume it should be lower; if low, assume higher
        return originalScore > 50 ? -20 : 20;
      case 'partial':
        // Small adjustment
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
    // If we have dimension scores, try to identify which dimensions contributed to error
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
      // are candidates for weight adjustment
      if (Math.abs(scoreDelta) > 10) {
        const contribution = (score - 50) * Math.sign(scoreDelta);
        pattern.avgError = pattern.avgError + (contribution - pattern.avgError) / pattern.feedbackCount;
      }
    }
  }

  /**
   * Track patterns by feedback source (Ralph-inspired: external validation)
   * @private
   * @param {string} source - Feedback source
   * @param {string} outcome - Feedback outcome
   * @param {number} scoreDelta - Score delta
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

    // Update running average of delta
    pattern.avgDelta = pattern.avgDelta + (scoreDelta - pattern.avgDelta) / pattern.count;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // LEARNING ALGORITHM
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Execute learning iteration
   * Processes queued feedback and updates weights
   *
   * @returns {Object} Learning results
   */
  async learn() {
    if (this._feedbackQueue.length < this.minFeedback) {
      return {
        success: false,
        reason: `Insufficient feedback (${this._feedbackQueue.length}/${this.minFeedback})`,
      };
    }

    const startTime = Date.now();
    const feedbackBatch = [...this._feedbackQueue];
    this._feedbackQueue = [];

    // Calculate adjustments from batch
    const adjustments = this._calculateAdjustments(feedbackBatch);

    // Apply adjustments with φ-bounded learning rate
    this._applyAdjustments(adjustments);

    // Apply decay to old learnings
    this._applyDecay();

    // Increment learning iteration
    this._patterns.overall.learningIterations++;

    // Persist state
    if (this.persistence) {
      await this._saveState();

      // Mark feedback as applied in DB
      for (const fb of feedbackBatch) {
        if (fb.feedbackId && this.persistence.feedback) {
          try {
            await this.persistence.feedback.markApplied(fb.feedbackId);
          } catch (e) {
            // Ignore - feedback might be in-memory only
          }
        }
      }
    }

    const result = {
      success: true,
      feedbackProcessed: feedbackBatch.length,
      adjustmentsMade: Object.keys(adjustments.weights).length,
      learningIteration: this._patterns.overall.learningIterations,
      duration: Date.now() - startTime,
    };

    this.emit('learning-complete', result);
    return result;
  }

  /**
   * Calculate weight adjustments from feedback batch
   * @private
   */
  _calculateAdjustments(feedbackBatch) {
    const weightDeltas = new Map();
    const thresholdDeltas = new Map();

    // Group feedback by item type
    const byType = new Map();
    for (const fb of feedbackBatch) {
      const type = fb.itemType || 'unknown';
      if (!byType.has(type)) {
        byType.set(type, []);
      }
      byType.get(type).push(fb);
    }

    // Calculate threshold adjustments per item type
    for (const [itemType, items] of byType) {
      const avgDelta = items.reduce((sum, fb) => sum + fb.scoreDelta, 0) / items.length;

      // Only adjust if consistent bias
      if (Math.abs(avgDelta) > 5 && items.length >= 2) {
        if (!thresholdDeltas.has(itemType)) {
          thresholdDeltas.set(itemType, new Map());
        }

        // Adjust general threshold for this item type
        // Positive delta = we're underscoring, lower threshold
        // Negative delta = we're overscoring, raise threshold
        const adjustment = -avgDelta * this.learningRate;
        thresholdDeltas.get(itemType).set('_general', adjustment);
      }
    }

    // Calculate dimension weight adjustments
    for (const [dimension, pattern] of this._patterns.byDimension) {
      if (pattern.feedbackCount < this.minFeedback) continue;

      // If a dimension consistently contributes to errors, adjust its weight
      // Positive avgError = dimension scores high when we should score low
      // -> reduce weight
      if (Math.abs(pattern.avgError) > 10) {
        const currentModifier = this._weightModifiers.get(dimension) || 1.0;
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

  /**
   * Apply adjustments with φ-bounded limits
   * @private
   */
  _applyAdjustments(adjustments) {
    // Apply weight adjustments
    for (const [dimension, delta] of Object.entries(adjustments.weights)) {
      const current = this._weightModifiers.get(dimension) || 1.0;
      let newModifier = current + delta;

      // Bound to [1 - maxAdjustment, 1 + maxAdjustment]
      const minMod = 1 - this.maxAdjustment;
      const maxMod = 1 + this.maxAdjustment;
      newModifier = Math.max(minMod, Math.min(maxMod, newModifier));

      this._weightModifiers.set(dimension, newModifier);
    }

    // Apply threshold adjustments
    for (const [itemType, dims] of Object.entries(adjustments.thresholds)) {
      if (!this._thresholdAdjustments.has(itemType)) {
        this._thresholdAdjustments.set(itemType, new Map());
      }

      for (const [dim, delta] of Object.entries(dims)) {
        const current = this._thresholdAdjustments.get(itemType).get(dim) || 0;
        let newAdjustment = current + delta;

        // Bound threshold adjustments to ±15 points
        newAdjustment = Math.max(-15, Math.min(15, newAdjustment));

        this._thresholdAdjustments.get(itemType).set(dim, newAdjustment);
      }
    }
  }

  /**
   * Apply decay to old learnings (prevents overfitting)
   * @private
   */
  _applyDecay() {
    // Decay weight modifiers toward 1.0
    for (const [dimension, modifier] of this._weightModifiers) {
      const decayed = 1.0 + (modifier - 1.0) * this.decayRate;
      this._weightModifiers.set(dimension, decayed);
    }

    // Decay threshold adjustments toward 0
    for (const [, dims] of this._thresholdAdjustments) {
      for (const [dim, adjustment] of dims) {
        dims.set(dim, adjustment * this.decayRate);
      }
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // AUTO-LEARNING (Pull from persistence)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Pull and process unapplied feedback from persistence
   *
   * @param {number} [limit=100] - Max feedback to process
   * @returns {Object} Pull results
   */
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
        queueSize: this._feedbackQueue.length,
      };
    } catch (error) {
      return { success: false, reason: error.message };
    }
  }

  /**
   * Run full learning cycle: pull feedback -> learn -> persist
   *
   * @returns {Object} Cycle results
   */
  async runLearningCycle() {
    await this.init();

    // Pull unapplied feedback
    const pullResult = await this.pullFeedback();

    // Execute learning
    const learnResult = await this.learn();

    return {
      pull: pullResult,
      learn: learnResult,
      patterns: this.getPatterns(),
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // EXTERNAL VALIDATION (Ralph-inspired: tests, commits, PRs as feedback)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Process test results as feedback
   * Tests passing = judgment was likely correct
   * Tests failing = judgment may need adjustment
   *
   * @param {Object} params - Test result parameters
   * @param {string} params.judgmentId - Original judgment ID
   * @param {boolean} params.passed - Whether tests passed
   * @param {string} [params.testSuite] - Test suite name
   * @param {number} [params.passCount] - Number of tests passed
   * @param {number} [params.failCount] - Number of tests failed
   * @param {string} [params.itemType] - Type of item (code, config, etc.)
   * @param {number} [params.originalScore] - Original judgment score
   * @returns {Object} Processed feedback result
   */
  processTestResult(params) {
    const {
      judgmentId,
      passed,
      testSuite = 'unknown',
      passCount = 0,
      failCount = 0,
      itemType = 'code',
      originalScore = 50,
    } = params;

    // Determine outcome based on test results
    let outcome = 'partial';
    if (passed && failCount === 0) {
      outcome = 'correct';
    } else if (failCount > passCount) {
      outcome = 'incorrect';
    }

    return this.processFeedback({
      judgmentId,
      outcome,
      originalScore,
      itemType,
      source: LearningService.FEEDBACK_SOURCES.TEST_RESULT,
      sourceContext: {
        testSuite,
        passCount,
        failCount,
        passed,
      },
    });
  }

  /**
   * Process commit result as feedback
   * Successful commit = code was acceptable
   *
   * @param {Object} params - Commit parameters
   * @param {string} params.judgmentId - Original judgment ID
   * @param {boolean} params.success - Whether commit succeeded
   * @param {string} [params.commitHash] - Git commit hash
   * @param {boolean} [params.hooksPassed] - Whether pre-commit hooks passed
   * @param {string} [params.itemType] - Type of item
   * @param {number} [params.originalScore] - Original judgment score
   * @returns {Object} Processed feedback result
   */
  processCommitResult(params) {
    const {
      judgmentId,
      success,
      commitHash = null,
      hooksPassed = true,
      itemType = 'code',
      originalScore = 50,
    } = params;

    // Successful commit with hooks = strong positive signal
    let outcome = 'partial';
    if (success && hooksPassed) {
      outcome = 'correct';
    } else if (!success) {
      outcome = 'incorrect';
    }

    return this.processFeedback({
      judgmentId,
      outcome,
      originalScore,
      itemType,
      source: LearningService.FEEDBACK_SOURCES.COMMIT,
      sourceContext: {
        commitHash,
        hooksPassed,
        success,
      },
    });
  }

  /**
   * Process PR result as feedback
   * Merged PR = strong positive validation
   * Rejected PR = negative validation
   *
   * @param {Object} params - PR parameters
   * @param {string} params.judgmentId - Original judgment ID
   * @param {string} params.status - 'merged', 'rejected', 'open'
   * @param {string} [params.prNumber] - PR number
   * @param {number} [params.reviewScore] - Average review score
   * @param {number} [params.approvalCount] - Number of approvals
   * @param {string} [params.itemType] - Type of item
   * @param {number} [params.originalScore] - Original judgment score
   * @returns {Object} Processed feedback result
   */
  processPRResult(params) {
    const {
      judgmentId,
      status,
      prNumber = null,
      reviewScore = null,
      approvalCount = 0,
      itemType = 'code',
      originalScore = 50,
    } = params;

    let outcome = 'partial';
    let source = LearningService.FEEDBACK_SOURCES.CODE_REVIEW;

    if (status === 'merged') {
      outcome = approvalCount >= 2 ? 'correct' : 'partial';
      source = LearningService.FEEDBACK_SOURCES.PR_MERGED;
    } else if (status === 'rejected') {
      outcome = 'incorrect';
      source = LearningService.FEEDBACK_SOURCES.PR_REJECTED;
    }

    // Use review score if available to calculate actual score
    const actualScore = reviewScore != null ? reviewScore : undefined;

    return this.processFeedback({
      judgmentId,
      outcome,
      actualScore,
      originalScore,
      itemType,
      source,
      sourceContext: {
        prNumber,
        status,
        reviewScore,
        approvalCount,
      },
    });
  }

  /**
   * Process build result as feedback
   *
   * @param {Object} params - Build parameters
   * @param {string} params.judgmentId - Original judgment ID
   * @param {boolean} params.success - Whether build succeeded
   * @param {string} [params.buildId] - Build ID
   * @param {number} [params.duration] - Build duration in ms
   * @param {string} [params.itemType] - Type of item
   * @param {number} [params.originalScore] - Original judgment score
   * @returns {Object} Processed feedback result
   */
  processBuildResult(params) {
    const {
      judgmentId,
      success,
      buildId = null,
      duration = null,
      itemType = 'code',
      originalScore = 50,
    } = params;

    return this.processFeedback({
      judgmentId,
      outcome: success ? 'correct' : 'incorrect',
      originalScore,
      itemType,
      source: LearningService.FEEDBACK_SOURCES.BUILD,
      sourceContext: {
        buildId,
        success,
        duration,
      },
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // ANOMALY SIGNALS (Gap #2 - ResidualDetector → Learning)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Process anomaly signal from ResidualDetector
   *
   * When the judge produces a judgment with high residual (unexplained variance),
   * this signal is used to identify dimensions that may need adjustment.
   *
   * @param {Object} signal - Anomaly signal
   * @param {string} signal.judgmentId - Judgment ID
   * @param {number} signal.residual - Residual value (0-1)
   * @param {number} signal.threshold - Anomaly threshold
   * @param {Object} signal.dimensions - Dimension scores
   * @param {string} signal.verdict - Judgment verdict
   * @param {number} signal.qScore - Q-Score
   */
  processAnomalySignal(signal) {
    if (!signal || !signal.judgmentId) return;

    // Record anomaly pattern
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
    if (!this._patterns.anomalies) {
      this._patterns.anomalies = [];
    }
    this._patterns.anomalies.push(anomalyEntry);
    if (this._patterns.anomalies.length > 100) {
      this._patterns.anomalies.shift();
    }

    // Analyze dimension scores for high/low extremes
    // These might indicate miscalibrated weights
    if (signal.dimensions) {
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
            this.emit('dimension-anomaly', {
              dimension: dim,
              anomalyCount: existing.anomalyCount,
              recommendation: `${dim} may need weight recalibration`,
            });
          }
        }
      }
    }

    // Emit for external listeners
    this.emit('anomaly-processed', {
      judgmentId: signal.judgmentId,
      residual: signal.residual,
      timestamp: Date.now(),
    });

    // Track overall anomaly rate
    this._patterns.overall.anomalyCount =
      (this._patterns.overall.anomalyCount || 0) + 1;

    return anomalyEntry;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // INSIGHTS & STATISTICS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get learned patterns and insights
   *
   * @returns {Object} Patterns and insights
   */
  getPatterns() {
    const insights = [];

    // Check for systematic biases by item type
    for (const [itemType, pattern] of this._patterns.byItemType) {
      if (pattern.feedbackCount >= this.minFeedback) {
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
      if (pattern.feedbackCount >= this.minFeedback && Math.abs(pattern.avgError) > 15) {
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
      learnings: [...this._learnings],
      accuracy: this._patterns.overall.totalFeedback > 0
        ? this._patterns.overall.correctCount / this._patterns.overall.totalFeedback
        : 0,
    };
  }

  /**
   * Add a discovered learning (persistent insight)
   * These are human-validated insights that should persist across sessions
   *
   * @param {Object} learning - Learning to add
   * @param {string} learning.pattern - Pattern description
   * @param {string} learning.insight - What was learned
   * @param {string} [learning.source] - How this was discovered
   * @param {number} [learning.confidence] - Confidence in this learning (0-1)
   */
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

  /**
   * Get all learnings
   * @returns {Object[]} All learnings
   */
  getLearnings() {
    return [...this._learnings];
  }

  /**
   * Export learnings to markdown format (for cynic-learnings.md)
   * @returns {string} Markdown content
   */
  exportToMarkdown() {
    const patterns = this.getPatterns();
    const stats = this.getStats();
    const lines = [];

    lines.push('# CYNIC Learnings');
    lines.push('');
    lines.push('> Auto-generated by CYNIC Learning Service');
    lines.push(`> Last updated: ${new Date().toISOString()}`);
    lines.push('');

    // Overall stats
    lines.push('## Statistics');
    lines.push('');
    lines.push(`- Total feedback: ${stats.totalFeedback}`);
    lines.push(`- Accuracy: ${stats.accuracy}%`);
    lines.push(`- Learning iterations: ${patterns.overall.learningIterations}`);
    lines.push(`- Avg score error: ${stats.avgScoreError}`);
    lines.push('');

    // Feedback sources
    if (this._patterns.bySource.size > 0) {
      lines.push('## Feedback Sources');
      lines.push('');
      lines.push('| Source | Count | Correct Rate | Avg Delta |');
      lines.push('|--------|-------|--------------|-----------|');

      for (const [source, data] of this._patterns.bySource) {
        const correctRate = data.count > 0
          ? Math.round(data.correctCount / data.count * 100)
          : 0;
        lines.push(`| ${source} | ${data.count} | ${correctRate}% | ${data.avgDelta.toFixed(1)} |`);
      }
      lines.push('');
    }

    // Item type biases
    if (patterns.byItemType && Object.keys(patterns.byItemType).length > 0) {
      lines.push('## Patterns by Item Type');
      lines.push('');
      for (const [itemType, data] of Object.entries(patterns.byItemType)) {
        const trend = data.avgDelta > 5 ? 'underscoring' : data.avgDelta < -5 ? 'overscoring' : 'neutral';
        lines.push(`### ${itemType}`);
        lines.push(`- Feedback count: ${data.feedbackCount}`);
        lines.push(`- Avg delta: ${data.avgDelta?.toFixed(1) || 0}`);
        lines.push(`- Trend: ${trend}`);
        lines.push('');
      }
    }

    // Dimension biases
    if (patterns.insights && patterns.insights.length > 0) {
      lines.push('## Insights');
      lines.push('');
      for (const insight of patterns.insights) {
        lines.push(`### ${insight.type}`);
        if (insight.itemType) lines.push(`- Item type: ${insight.itemType}`);
        if (insight.dimension) lines.push(`- Dimension: ${insight.dimension}`);
        if (insight.direction) lines.push(`- Direction: ${insight.direction}`);
        if (insight.avgDelta) lines.push(`- Avg delta: ${insight.avgDelta}`);
        if (insight.avgError) lines.push(`- Avg error: ${insight.avgError}`);
        if (insight.recommendation) lines.push(`- Recommendation: ${insight.recommendation}`);
        lines.push('');
      }
    }

    // Explicit learnings
    if (this._learnings.length > 0) {
      lines.push('## Discovered Learnings');
      lines.push('');
      for (const learning of this._learnings) {
        lines.push(`### ${learning.pattern}`);
        lines.push(`- Insight: ${learning.insight}`);
        lines.push(`- Source: ${learning.source}`);
        lines.push(`- Confidence: ${(learning.confidence * 100).toFixed(0)}%`);
        lines.push(`- Discovered: ${new Date(learning.createdAt).toISOString()}`);
        lines.push('');
      }
    }

    // Weight modifiers
    const modifiers = this.getAllWeightModifiers();
    const changedModifiers = Object.entries(modifiers).filter(([, v]) => Math.abs(v - 1.0) > 0.01);
    if (changedModifiers.length > 0) {
      lines.push('## Weight Adjustments');
      lines.push('');
      lines.push('| Dimension | Modifier | Interpretation |');
      lines.push('|-----------|----------|----------------|');
      for (const [dim, mod] of changedModifiers) {
        const interp = mod > 1 ? 'Increased importance' : 'Decreased importance';
        lines.push(`| ${dim} | ${mod.toFixed(3)} | ${interp} |`);
      }
      lines.push('');
    }

    return lines.join('\n');
  }

  /**
   * Import learnings from markdown format
   * @param {string} markdown - Markdown content
   */
  importFromMarkdown(markdown) {
    // Parse the Discovered Learnings section
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

  /**
   * Get current learning state
   *
   * @returns {Object} Learning state
   */
  getState() {
    return {
      weightModifiers: Object.fromEntries(this._weightModifiers),
      thresholdAdjustments: Object.fromEntries(
        [...this._thresholdAdjustments.entries()].map(([k, v]) => [k, Object.fromEntries(v)])
      ),
      queueSize: this._feedbackQueue.length,
      patterns: this.getPatterns(),
      config: {
        learningRate: this.learningRate,
        maxAdjustment: this.maxAdjustment,
        minFeedback: this.minFeedback,
        decayRate: this.decayRate,
      },
    };
  }

  /**
   * Get statistics summary
   *
   * @returns {Object} Statistics
   */
  getStats() {
    const patterns = this.getPatterns();
    const modifiedDimensions = [...this._weightModifiers.entries()]
      .filter(([, v]) => Math.abs(v - 1.0) > 0.01)
      .length;

    return {
      initialized: this._initialized,
      totalFeedback: patterns.overall.totalFeedback,
      accuracy: Math.round(patterns.accuracy * 1000) / 10, // percentage
      avgScoreError: Math.round(patterns.overall.avgScoreError * 10) / 10,
      learningIterations: patterns.overall.learningIterations,
      modifiedDimensions,
      itemTypesTracked: this._patterns.byItemType.size,
      insightsCount: patterns.insights.length,
      queueSize: this._feedbackQueue.length,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PERSISTENCE
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Load state from persistence
   * @private
   */
  async _loadState() {
    // Try to load from knowledge store if available
    if (this.persistence?.knowledge) {
      try {
        const results = await this.persistence.knowledge.search('learning_state', {
          category: 'system',
          limit: 1,
        });

        if (results.length > 0) {
          const saved = results[0].patterns;
          if (saved) {
            this.import(saved);
            this.emit('state-loaded', { source: 'persistence' });
          }
        }
      } catch (e) {
        // Ignore - start fresh
      }
    }
  }

  /**
   * Save state to persistence
   * @private
   */
  async _saveState() {
    if (this.persistence?.knowledge) {
      try {
        const state = this.export();
        await this.persistence.storeKnowledge({
          sourceType: 'system',
          sourceRef: 'learning_state',
          summary: `Learning state: ${this._patterns.overall.learningIterations} iterations`,
          content: JSON.stringify(state),
          category: 'system',
          patterns: state,
        });
      } catch (e) {
        // Ignore - learning still works in-memory
      }
    }
  }

  /**
   * Export learning state
   *
   * @returns {Object} Exportable state
   */
  export() {
    return {
      weightModifiers: Object.fromEntries(this._weightModifiers),
      thresholdAdjustments: Object.fromEntries(
        [...this._thresholdAdjustments.entries()].map(([k, v]) => [k, Object.fromEntries(v)])
      ),
      patterns: {
        byItemType: Object.fromEntries(this._patterns.byItemType),
        byDimension: Object.fromEntries(this._patterns.byDimension),
        bySource: Object.fromEntries(this._patterns.bySource),
        overall: { ...this._patterns.overall },
      },
      learnings: [...this._learnings],
      exportedAt: Date.now(),
    };
  }

  /**
   * Import learning state
   *
   * @param {Object} state - Saved state
   */
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
      if (state.patterns.byItemType) {
        for (const [type, pattern] of Object.entries(state.patterns.byItemType)) {
          this._patterns.byItemType.set(type, pattern);
        }
      }
      if (state.patterns.byDimension) {
        for (const [dim, pattern] of Object.entries(state.patterns.byDimension)) {
          this._patterns.byDimension.set(dim, pattern);
        }
      }
      if (state.patterns.bySource) {
        for (const [source, pattern] of Object.entries(state.patterns.bySource)) {
          this._patterns.bySource.set(source, pattern);
        }
      }
      if (state.patterns.overall) {
        Object.assign(this._patterns.overall, state.patterns.overall);
      }
    }

    // Import learnings
    if (state.learnings && Array.isArray(state.learnings)) {
      this._learnings = [...state.learnings];
    }
  }

  /**
   * Reset all learning
   * USE WITH CAUTION - erases all learned adjustments
   */
  reset() {
    for (const dim of this._weightModifiers.keys()) {
      this._weightModifiers.set(dim, 1.0);
    }
    this._thresholdAdjustments.clear();
    this._patterns.byItemType.clear();
    this._patterns.byDimension.clear();
    this._patterns.bySource.clear();
    this._patterns.overall = {
      totalFeedback: 0,
      correctCount: 0,
      incorrectCount: 0,
      avgScoreError: 0,
      learningIterations: 0,
    };
    this._feedbackQueue = [];
    this._learnings = [];

    this.emit('reset');
  }
}

export default LearningService;
