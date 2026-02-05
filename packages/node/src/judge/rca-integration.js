/**
 * Root Cause Analysis Integration
 *
 * Integrates RootCauseAnalyzer with LearningService.
 * Handles test failure analysis, hypothesis tracking, and feedback on RCA decisions.
 * Extracted from LearningService to follow BURN axiom (simplicity).
 *
 * @module @cynic/node/judge/rca-integration
 */

'use strict';

import { EventEmitter } from 'events';
import {
  getRootCauseAnalyzer,
  OracleType,
  Hypothesis,
} from './root-cause-analyzer.js';

export { Hypothesis, OracleType };

/**
 * RCA Integration - Connects RootCauseAnalyzer with learning system
 */
export class RCAIntegration extends EventEmitter {
  /**
   * @param {Object} options
   * @param {Function} [options.processFeedback] - Callback to process feedback
   * @param {Map} [options.sourcePatterns] - Source patterns map for tracking
   */
  constructor(options = {}) {
    super();
    this._processFeedback = options.processFeedback || (() => ({}));
    this._sourcePatterns = options.sourcePatterns || new Map();
    this._rootCauseAnalyzer = null;
    this._lastRootCauseDecision = null;
    this._initialized = false;
  }

  /**
   * Initialize RCA integration
   */
  init() {
    if (this._initialized) return;

    this._rootCauseAnalyzer = getRootCauseAnalyzer();

    // When RCA makes a decision, process it
    this._rootCauseAnalyzer.on('decision', (decision) => {
      this._processRootCauseDecision(decision);
    });

    // When RCA learns from feedback, sync with our learning
    this._rootCauseAnalyzer.on('learned', (learning) => {
      this.emit('root-cause:learned', learning);
    });

    this._initialized = true;
  }

  /**
   * Set the feedback processor callback
   * @param {Function} fn
   */
  setFeedbackProcessor(fn) {
    this._processFeedback = fn;
  }

  /**
   * Set source patterns map (for tracking)
   * @param {Map} patterns
   */
  setSourcePatterns(patterns) {
    this._sourcePatterns = patterns;
  }

  /**
   * Get the RootCauseAnalyzer instance
   * @returns {RootCauseAnalyzer|null}
   */
  getRootCauseAnalyzer() {
    return this._rootCauseAnalyzer;
  }

  /**
   * Get the last RCA decision
   * @returns {Object|null}
   */
  getLastDecision() {
    return this._lastRootCauseDecision;
  }

  /**
   * Analyze test failure to determine root cause
   *
   * @param {Object} params - Test failure parameters
   * @param {string} params.testName - Name of the failing test
   * @param {string} params.errorMessage - Error message
   * @param {boolean} [params.recentCodeChange] - Was code recently changed?
   * @param {boolean} [params.recentTestChange] - Was test recently changed?
   * @param {boolean} [params.flakeHistory] - Does this test have flake history?
   * @param {Object} [params.dimensionScores] - Original dimension scores
   * @param {number} [params.originalScore] - Original judgment score
   * @returns {Object} Root cause analysis result
   */
  analyzeTestFailure(params) {
    if (!this._rootCauseAnalyzer) {
      return { hypothesis: Hypothesis.UNKNOWN, reason: 'RCA not initialized' };
    }

    const {
      testName,
      errorMessage,
      recentCodeChange = false,
      recentTestChange = false,
      flakeHistory = false,
      dimensionScores = {},
      originalScore = 50,
    } = params;

    // Clear previous evidence
    this._rootCauseAnalyzer.clearEvidence();

    // Add test result evidence
    this._rootCauseAnalyzer.addEvidence({
      type: OracleType.TEST_RESULT,
      verdict: 'fail',
      confidence: 0.9,
      data: { testName, errorMessage },
    });

    // Add context-based evidence
    if (recentCodeChange) {
      this._rootCauseAnalyzer.addEvidence({
        type: OracleType.COMMIT_HISTORY,
        verdict: 'fail',
        confidence: 0.7,
        data: { recentCodeChange: true },
      });
    }

    if (recentTestChange) {
      this._rootCauseAnalyzer.addEvidence({
        type: OracleType.COMMIT_HISTORY,
        verdict: 'pass', // Test change might mean test is wrong
        confidence: 0.6,
        data: { recentTestChange: true, testRecentlyChanged: true },
      });
    }

    if (flakeHistory) {
      this._rootCauseAnalyzer.addEvidence({
        type: OracleType.RUNTIME_BEHAVIOR,
        verdict: 'flaky',
        confidence: 0.8,
        data: { flakeHistory: true },
      });
    }

    // Analyze with context
    const decision = this._rootCauseAnalyzer.analyze({
      recentCodeChange,
      recentTestChange,
      flakeHistory,
    });

    // Store for potential feedback later
    this._lastRootCauseDecision = {
      decision,
      params,
      timestamp: Date.now(),
    };

    this.emit('root-cause:analyzed', {
      testName,
      hypothesis: decision.hypothesis,
      confidence: decision.confidence,
      action: decision.action,
    });

    return decision;
  }

  /**
   * Process RootCauseAnalyzer decision for learning adjustments
   * @private
   */
  _processRootCauseDecision(decision) {
    // Don't learn from UNKNOWN decisions
    if (decision.hypothesis === Hypothesis.UNKNOWN) return;

    // Track root cause patterns
    const key = `rootCause:${decision.hypothesis}`;
    const pattern = this._sourcePatterns.get(key) || {
      count: 0,
      correctRate: 0,
      avgDelta: 0,
    };
    pattern.count++;
    this._sourcePatterns.set(key, pattern);

    // If hypothesis is SPEC_GAP, flag for dimension discovery
    if (decision.hypothesis === Hypothesis.SPEC_GAP) {
      this.emit('spec-gap-detected', {
        decisionId: decision.id,
        residual: decision.residual,
        reasoning: decision.reasoning,
      });
    }

    this.emit('root-cause:processed', {
      hypothesis: decision.hypothesis,
      confidence: decision.confidence,
      action: decision.action,
    });
  }

  /**
   * Provide feedback on the last root cause analysis
   *
   * @param {string} actualCause - What was the actual root cause?
   * @param {boolean} wasCorrect - Was the analysis correct?
   * @returns {Object} Feedback result
   */
  feedbackOnRootCause(actualCause, wasCorrect) {
    if (!this._rootCauseAnalyzer || !this._lastRootCauseDecision) {
      return { success: false, reason: 'No recent root cause analysis' };
    }

    const { decision, params } = this._lastRootCauseDecision;

    // Provide feedback to RootCauseAnalyzer (updates oracle trust)
    this._rootCauseAnalyzer.feedback(decision.id, actualCause, wasCorrect);

    // Determine score delta based on what actually happened
    let scoreDelta = 0;
    if (!wasCorrect) {
      // We misjudged - need to understand the impact
      if (actualCause === Hypothesis.CODE_BUG && decision.hypothesis === Hypothesis.TEST_BUG) {
        // We thought test was wrong but code was wrong
        scoreDelta = -15; // We were too lenient
      } else if (actualCause === Hypothesis.TEST_BUG && decision.hypothesis === Hypothesis.CODE_BUG) {
        // We thought code was wrong but test was wrong
        scoreDelta = 15; // We were too harsh
      }
    }

    // Process as standard feedback
    const outcome = wasCorrect ? 'correct' : 'incorrect';
    this._processFeedback({
      outcome,
      originalScore: params.originalScore || 50,
      actualScore: (params.originalScore || 50) + scoreDelta,
      itemType: 'root_cause_analysis',
      dimensionScores: params.dimensionScores || {},
      source: `root_cause:${actualCause}`,
      sourceContext: {
        predictedCause: decision.hypothesis,
        actualCause,
        wasCorrect,
      },
    });

    // Update source pattern accuracy
    const key = `rootCause:${decision.hypothesis}`;
    const pattern = this._sourcePatterns.get(key);
    if (pattern) {
      pattern.correctRate = (pattern.correctRate * (pattern.count - 1) + (wasCorrect ? 1 : 0)) / pattern.count;
    }

    this.emit('root-cause:feedback', {
      decisionId: decision.id,
      predictedCause: decision.hypothesis,
      actualCause,
      wasCorrect,
    });

    return { success: true, scoreDelta };
  }
}

export default RCAIntegration;
