/**
 * External Validation - Tests, Commits, PRs, Builds as Feedback
 *
 * Ralph-inspired: External validation gates provide implicit feedback signals.
 * Extracted from LearningService to follow BURN axiom (simplicity).
 *
 * @module @cynic/node/judge/external-validation
 */

'use strict';

import { EventEmitter } from 'events';
import { FEEDBACK_SOURCES } from './feedback-processor.js';

/**
 * External Validation Processor
 *
 * Converts external signals (test results, commits, PRs, builds)
 * into structured feedback for the learning system.
 */
export class ExternalValidation extends EventEmitter {
  /**
   * @param {Object} options
   * @param {Function} options.processFeedback - Callback to process feedback
   * @param {Function} [options.analyzeTestFailure] - Callback for RCA analysis
   */
  constructor(options = {}) {
    super();
    this._processFeedback = options.processFeedback || (() => ({}));
    this._analyzeTestFailure = options.analyzeTestFailure || null;
  }

  /**
   * Set the feedback processor callback
   * @param {Function} fn
   */
  setFeedbackProcessor(fn) {
    this._processFeedback = fn;
  }

  /**
   * Set the test failure analyzer callback (RCA)
   * @param {Function} fn
   */
  setTestFailureAnalyzer(fn) {
    this._analyzeTestFailure = fn;
  }

  /**
   * Process test results as feedback
   * Tests passing = judgment was likely correct
   * Tests failing = judgment may need adjustment
   *
   * @param {Object} params - Test result parameters
   * @param {string} params.judgmentId - Original judgment ID
   * @param {boolean} params.passed - Whether tests passed
   * @param {string} [params.testSuite] - Test suite name
   * @param {string} [params.testName] - Specific test name
   * @param {string} [params.errorMessage] - Error message if failed
   * @param {number} [params.passCount] - Number of tests passed
   * @param {number} [params.failCount] - Number of tests failed
   * @param {string} [params.itemType] - Type of item
   * @param {number} [params.originalScore] - Original judgment score
   * @param {Object} [params.dimensionScores] - Original dimension scores
   * @param {boolean} [params.recentCodeChange] - Was code recently changed?
   * @param {boolean} [params.recentTestChange] - Was test recently changed?
   * @param {boolean} [params.flakeHistory] - Does this test have flake history?
   * @param {boolean} [params.useRootCause] - Enable RCA for this call
   * @returns {Object} Processed feedback result
   */
  processTestResult(params) {
    const {
      judgmentId,
      passed,
      testSuite = 'unknown',
      testName = null,
      errorMessage = null,
      passCount = 0,
      failCount = 0,
      itemType = 'code',
      originalScore = 50,
      dimensionScores = {},
      recentCodeChange = false,
      recentTestChange = false,
      flakeHistory = false,
      useRootCause = false,
    } = params;

    // If tests pass, no need for root cause analysis
    if (passed && failCount === 0) {
      return this._processFeedback({
        judgmentId,
        outcome: 'correct',
        originalScore,
        itemType,
        source: FEEDBACK_SOURCES.TEST_RESULT,
        sourceContext: {
          testSuite,
          testName,
          passCount,
          failCount,
          passed,
        },
      });
    }

    // Tests failed - should we analyze root cause?
    let rootCauseDecision = null;
    if (useRootCause && this._analyzeTestFailure && failCount > 0) {
      rootCauseDecision = this._analyzeTestFailure({
        testName: testName || testSuite,
        errorMessage,
        recentCodeChange,
        recentTestChange,
        flakeHistory,
        dimensionScores,
        originalScore,
      });

      // Based on root cause, adjust how we process the feedback
      if (rootCauseDecision?.hypothesis === 'TEST_BUG') {
        // Don't penalize the judgment - the test is wrong, not the code
        this.emit('test-bug-detected', { testName, rootCauseDecision });
        return {
          outcome: 'partial',
          scoreDelta: 0,
          rootCause: rootCauseDecision,
          action: 'FLAG_TEST_FOR_REVIEW',
          message: 'Test failure attributed to test bug - no weight adjustment',
        };
      }

      if (rootCauseDecision?.hypothesis === 'FLAKY') {
        this.emit('flaky-test-detected', { testName, rootCauseDecision });
        return {
          outcome: 'partial',
          scoreDelta: 0,
          rootCause: rootCauseDecision,
          action: 'RETRY_AND_TRACK_FLAKINESS',
          message: 'Test failure attributed to flakiness - no weight adjustment',
        };
      }

      if (rootCauseDecision?.hypothesis === 'SPEC_GAP') {
        this.emit('spec-gap-detected', { testName, rootCauseDecision });
        return this._processFeedback({
          judgmentId,
          outcome: 'partial',
          originalScore,
          itemType,
          source: FEEDBACK_SOURCES.TEST_RESULT,
          sourceContext: {
            testSuite,
            testName,
            passCount,
            failCount,
            passed,
            rootCause: rootCauseDecision.hypothesis,
          },
        });
      }
    }

    // Determine outcome based on test results
    let outcome = 'partial';
    if (failCount > passCount) {
      outcome = 'incorrect';
    }

    const result = this._processFeedback({
      judgmentId,
      outcome,
      originalScore,
      itemType,
      dimensionScores,
      source: FEEDBACK_SOURCES.TEST_RESULT,
      sourceContext: {
        testSuite,
        testName,
        passCount,
        failCount,
        passed,
        rootCause: rootCauseDecision?.hypothesis || null,
      },
    });

    if (rootCauseDecision) {
      result.rootCause = rootCauseDecision;
    }

    this.emit('test-result-processed', { judgmentId, outcome, rootCause: rootCauseDecision });
    return result;
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

    const result = this._processFeedback({
      judgmentId,
      outcome,
      originalScore,
      itemType,
      source: FEEDBACK_SOURCES.COMMIT,
      sourceContext: {
        commitHash,
        hooksPassed,
        success,
      },
    });

    this.emit('commit-result-processed', { judgmentId, outcome, commitHash });
    return result;
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
    let source = FEEDBACK_SOURCES.CODE_REVIEW;

    if (status === 'merged') {
      outcome = approvalCount >= 2 ? 'correct' : 'partial';
      source = FEEDBACK_SOURCES.PR_MERGED;
    } else if (status === 'rejected') {
      outcome = 'incorrect';
      source = FEEDBACK_SOURCES.PR_REJECTED;
    }

    // Use review score if available to calculate actual score
    const actualScore = reviewScore != null ? reviewScore : undefined;

    const result = this._processFeedback({
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

    this.emit('pr-result-processed', { judgmentId, outcome, status, prNumber });
    return result;
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

    const result = this._processFeedback({
      judgmentId,
      outcome: success ? 'correct' : 'incorrect',
      originalScore,
      itemType,
      source: FEEDBACK_SOURCES.BUILD,
      sourceContext: {
        buildId,
        success,
        duration,
      },
    });

    this.emit('build-result-processed', { judgmentId, outcome: success ? 'correct' : 'incorrect', buildId });
    return result;
  }

  /**
   * Process deploy result as feedback
   *
   * @param {Object} params - Deploy parameters
   * @param {string} params.judgmentId - Original judgment ID
   * @param {boolean} params.success - Whether deploy succeeded
   * @param {string} [params.environment] - Deploy environment
   * @param {string} [params.version] - Deployed version
   * @param {string} [params.itemType] - Type of item
   * @param {number} [params.originalScore] - Original judgment score
   * @returns {Object} Processed feedback result
   */
  processDeployResult(params) {
    const {
      judgmentId,
      success,
      environment = 'unknown',
      version = null,
      itemType = 'code',
      originalScore = 50,
    } = params;

    const result = this._processFeedback({
      judgmentId,
      outcome: success ? 'correct' : 'incorrect',
      originalScore,
      itemType,
      source: FEEDBACK_SOURCES.DEPLOY,
      sourceContext: {
        environment,
        version,
        success,
      },
    });

    this.emit('deploy-result-processed', { judgmentId, outcome: success ? 'correct' : 'incorrect', environment });
    return result;
  }
}

export default ExternalValidation;
