/**
 * Feedback Collector - Error tracking and learning signal collection
 *
 * Records tool outcomes, tracks consecutive errors, detects anti-patterns,
 * and signals when escalation is needed.
 *
 * "Le chien observe et apprend" - CYNIC watches and learns
 *
 * @module scripts/hooks/lib/feedback-collector
 */

'use strict';

import { getSessionState } from './session-state.js';

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/** Anti-pattern detection thresholds */
const ANTI_PATTERN_THRESHOLD = 3; // Same approach failed N times
const SIMILAR_ERROR_THRESHOLD = 2; // Same error type N times

/** φ-aligned confidence threshold */
const PHI_CONFIDENCE = 0.618;

// ═══════════════════════════════════════════════════════════════════════════
// ANTI-PATTERN DEFINITIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Known anti-patterns that indicate the user might be stuck
 */
const ANTI_PATTERNS = [
  {
    id: 'repeated_file_not_found',
    description: 'Repeatedly trying to access non-existent files',
    detect: (errors) => {
      const fnfErrors = errors.filter(e => e.errorType === 'file_not_found');
      return fnfErrors.length >= ANTI_PATTERN_THRESHOLD;
    },
    suggestion: 'The file might not exist. Try listing directory contents with `ls` or check the path.',
  },
  {
    id: 'repeated_syntax_error',
    description: 'Multiple syntax errors in quick succession',
    detect: (errors) => {
      const syntaxErrors = errors.filter(e => e.errorType === 'syntax');
      return syntaxErrors.length >= ANTI_PATTERN_THRESHOLD;
    },
    suggestion: 'Multiple syntax errors detected. Consider reading the file first to understand its structure.',
  },
  {
    id: 'repeated_permission_denied',
    description: 'Repeatedly encountering permission issues',
    detect: (errors) => {
      const permErrors = errors.filter(e => e.errorType === 'permission');
      return permErrors.length >= SIMILAR_ERROR_THRESHOLD;
    },
    suggestion: 'Permission denied errors are repeating. Check if elevated permissions are needed.',
  },
  {
    id: 'repeated_connection_refused',
    description: 'Service connection failures',
    detect: (errors) => {
      const connErrors = errors.filter(e => e.errorType === 'connection');
      return connErrors.length >= SIMILAR_ERROR_THRESHOLD;
    },
    suggestion: 'Connection refused - the service might not be running. Check if it needs to be started.',
  },
  {
    id: 'repeated_test_failure',
    description: 'Tests failing repeatedly without change',
    detect: (errors, tools) => {
      const testFailures = errors.filter(e => e.errorType === 'test_failure');
      if (testFailures.length < SIMILAR_ERROR_THRESHOLD) return false;

      // Check if we're running tests without making code changes
      const recentTools = tools.slice(-5);
      const codeChanges = recentTools.filter(t => t.tool === 'Write' || t.tool === 'Edit');
      return codeChanges.length === 0;
    },
    suggestion: 'Tests are failing repeatedly. Consider analyzing the test output or making code changes.',
  },
  {
    id: 'same_tool_same_error',
    description: 'Same tool producing same error repeatedly',
    detect: (errors, tools) => {
      if (errors.length < ANTI_PATTERN_THRESHOLD) return false;

      const recentErrors = errors.slice(-ANTI_PATTERN_THRESHOLD);
      const sameTool = recentErrors.every(e => e.tool === recentErrors[0].tool);
      const sameError = recentErrors.every(e => e.errorType === recentErrors[0].errorType);

      return sameTool && sameError;
    },
    suggestion: 'Same operation keeps failing. Try a different approach.',
  },
];

// ═══════════════════════════════════════════════════════════════════════════
// FEEDBACK COLLECTOR
// ═══════════════════════════════════════════════════════════════════════════

/**
 * FeedbackCollector - Tracks tool outcomes and detects patterns
 */
class FeedbackCollector {
  constructor() {
    this._lastDetectedPattern = null;
    this._detectedPatterns = new Set();
  }

  /**
   * Record a tool outcome
   *
   * @param {string} toolName - Name of the tool
   * @param {Object} outcome - Outcome details
   * @param {boolean} outcome.success - Whether tool succeeded
   * @param {string} [outcome.errorType] - Type of error if failed
   * @param {string} [outcome.errorMessage] - Error message
   * @param {Object} [outcome.input] - Tool input (for pattern detection)
   * @returns {Object} Recording result with escalation info
   */
  record(toolName, outcome) {
    const sessionState = getSessionState();

    // Record to SessionState
    sessionState.recordToolCall({
      tool: toolName,
      input: outcome.input || {},
      isError: !outcome.success,
      errorType: outcome.errorType,
      errorMessage: outcome.errorMessage,
    });

    // Check for anti-patterns
    const antiPattern = this._detectAntiPattern(sessionState);

    // Check for escalation
    const shouldEscalate = sessionState.shouldEscalate();
    const circuitBreak = sessionState.shouldCircuitBreak();

    return {
      recorded: true,
      escalationLevel: sessionState.getEscalationLevel(),
      consecutiveErrors: sessionState.getConsecutiveErrors(),
      shouldEscalate,
      circuitBreak,
      antiPattern,
    };
  }

  /**
   * Get recent error patterns
   * @returns {Object[]} Error patterns with counts
   */
  getErrorPatterns() {
    const sessionState = getSessionState();
    const errors = sessionState.getRecentErrors();

    // Group errors by type
    const patterns = {};
    for (const error of errors) {
      const key = error.errorType || 'unknown';
      if (!patterns[key]) {
        patterns[key] = {
          type: key,
          count: 0,
          tools: new Set(),
          lastSeen: null,
        };
      }
      patterns[key].count++;
      patterns[key].tools.add(error.tool);
      patterns[key].lastSeen = error.timestamp;
    }

    // Convert to array and add tools as array
    return Object.values(patterns).map(p => ({
      ...p,
      tools: Array.from(p.tools),
    })).sort((a, b) => b.count - a.count);
  }

  /**
   * Check if escalation is recommended
   * @returns {{shouldEscalate: boolean, reason: string}}
   */
  shouldEscalate() {
    const sessionState = getSessionState();

    if (sessionState.shouldEscalate()) {
      return {
        shouldEscalate: true,
        reason: this._getEscalationReason(sessionState),
      };
    }

    return { shouldEscalate: false, reason: '' };
  }

  /**
   * Get suggestion for current state
   * @returns {Object|null} Suggestion if applicable
   */
  getSuggestion() {
    if (this._lastDetectedPattern) {
      return {
        patternId: this._lastDetectedPattern.id,
        description: this._lastDetectedPattern.description,
        suggestion: this._lastDetectedPattern.suggestion,
      };
    }
    return null;
  }

  /**
   * Check if same error occurred recently
   * @param {string} errorType - Error type to check
   * @returns {Object[]} Similar recent errors
   */
  getSimilarRecentErrors(errorType) {
    const sessionState = getSessionState();
    return sessionState.getSimilarErrors(errorType);
  }

  /**
   * Reset pattern detection (call after suggestion shown)
   */
  resetPatternDetection() {
    this._lastDetectedPattern = null;
    this._detectedPatterns.clear();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PRIVATE HELPERS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Detect anti-patterns in recent history
   * @private
   */
  _detectAntiPattern(sessionState) {
    const errors = sessionState.getRecentErrors();
    const tools = sessionState.getToolHistory();

    // Check each anti-pattern
    for (const pattern of ANTI_PATTERNS) {
      // Skip if already detected this session (avoid spam)
      if (this._detectedPatterns.has(pattern.id)) {
        continue;
      }

      if (pattern.detect(errors, tools)) {
        this._lastDetectedPattern = pattern;
        this._detectedPatterns.add(pattern.id);
        return pattern;
      }
    }

    return null;
  }

  /**
   * Get human-readable escalation reason
   * @private
   */
  _getEscalationReason(sessionState) {
    const consecutive = sessionState.getConsecutiveErrors();
    const recentErrors = sessionState.getRecentErrors(10);
    const errorRate = recentErrors.length / sessionState.getToolHistory(10).length;

    if (consecutive >= 5) {
      return `${consecutive} consecutive errors`;
    }
    if (errorRate > 0.5) {
      return `High error rate (${Math.round(errorRate * 100)}%)`;
    }
    if (this._lastDetectedPattern) {
      return `Anti-pattern: ${this._lastDetectedPattern.description}`;
    }
    return 'Multiple issues detected';
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON & EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

/** Singleton instance */
let _instance = null;

/**
 * Get FeedbackCollector singleton instance
 * @returns {FeedbackCollector}
 */
export function getFeedbackCollector() {
  if (!_instance) {
    _instance = new FeedbackCollector();
  }
  return _instance;
}

export { FeedbackCollector, ANTI_PATTERNS };
export default FeedbackCollector;
