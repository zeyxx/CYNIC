/**
 * Implicit Feedback Detection - CYNIC learns from user actions
 *
 * "Le chien observe les actions, pas les mots"
 *
 * Most users won't give explicit feedback. This module detects it implicitly:
 * - User follows suggestion within N prompts = positive
 * - User does opposite = negative
 * - User ignores = neutral
 * - Error rate changes after suggestion = validation signal
 *
 * @module scripts/hooks/lib/implicit-feedback
 */

'use strict';

// φ constants for thresholds
const PHI_INV = 0.618033988749895;    // 61.8%
const PHI_INV_2 = 0.381966011250105;  // 38.2%
const PHI_INV_3 = 0.236067977499790;  // 23.6%

// ═══════════════════════════════════════════════════════════════════════════
// CONFIGURATION
// ═══════════════════════════════════════════════════════════════════════════

const CONFIG = Object.freeze({
  // Time window to detect follow-through (prompts, not time)
  FOLLOW_WINDOW_PROMPTS: 3,

  // Time window in ms (5 minutes)
  FOLLOW_WINDOW_MS: 5 * 60 * 1000,

  // Feedback confidence weights
  EXPLICIT_WEIGHT: 1.0,
  IMPLICIT_WEIGHT: 0.5,
  INFERRED_WEIGHT: 0.3,

  // Max suggestions to track
  MAX_TRACKED_SUGGESTIONS: 20,

  // Minimum confidence to count as feedback
  MIN_FEEDBACK_CONFIDENCE: PHI_INV_3, // 23.6%
});

// ═══════════════════════════════════════════════════════════════════════════
// FEEDBACK TYPES
// ═══════════════════════════════════════════════════════════════════════════

const FeedbackType = Object.freeze({
  // Explicit feedback (user said something)
  EXPLICIT_POSITIVE: 'explicit_positive',
  EXPLICIT_NEGATIVE: 'explicit_negative',

  // Implicit feedback (inferred from actions)
  IMPLICIT_FOLLOWED: 'implicit_followed',       // User followed suggestion
  IMPLICIT_OPPOSITE: 'implicit_opposite',       // User did opposite
  IMPLICIT_IGNORED: 'implicit_ignored',         // User ignored suggestion
  IMPLICIT_COMMITTED: 'implicit_committed',     // User committed after review
  IMPLICIT_RETRIED: 'implicit_retried',         // User retried after error

  // Outcome-based feedback
  OUTCOME_ERROR_REDUCED: 'outcome_error_reduced',    // Error rate dropped
  OUTCOME_ERROR_INCREASED: 'outcome_error_increased', // Error rate increased
  OUTCOME_SUCCESS_STREAK: 'outcome_success_streak',   // Multiple successes after suggestion
});

const FeedbackSentiment = Object.freeze({
  POSITIVE: 'positive',
  NEGATIVE: 'negative',
  NEUTRAL: 'neutral',
});

// ═══════════════════════════════════════════════════════════════════════════
// SUGGESTION TRACKING
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Structure for a tracked suggestion
 * @typedef {Object} TrackedSuggestion
 * @property {string} id - Unique suggestion ID
 * @property {string} type - Type of suggestion (skill, agent, tool, etc.)
 * @property {string} content - What was suggested
 * @property {number} timestamp - When suggested
 * @property {number} promptIndex - Prompt count when suggested
 * @property {string|null} expectedTool - Expected tool if followed
 * @property {string|null} expectedAgent - Expected agent if followed
 * @property {string|null} expectedSkill - Expected skill if followed
 * @property {string[]} oppositeActions - Actions that would be "opposite"
 * @property {string} status - pending|followed|ignored|opposed
 * @property {number|null} resolvedAt - When resolved
 * @property {Object|null} feedback - Generated feedback
 */

// ═══════════════════════════════════════════════════════════════════════════
// IMPLICIT FEEDBACK DETECTOR CLASS
// ═══════════════════════════════════════════════════════════════════════════

class ImplicitFeedbackDetector {
  constructor() {
    /** @type {TrackedSuggestion[]} */
    this._trackedSuggestions = [];

    /** @type {number} */
    this._promptCount = 0;

    /** @type {Object[]} */
    this._recentToolCalls = [];

    /** @type {Object[]} */
    this._feedbackHistory = [];

    /** @type {number} */
    this._lastErrorRate = 0;

    /** @type {Object|null} */
    this._sessionState = null;
  }

  /**
   * Set session state reference for error rate tracking
   * @param {Object} sessionState
   */
  setSessionState(sessionState) {
    this._sessionState = sessionState;
  }

  /**
   * Record a new prompt (called from perceive.js)
   */
  recordPrompt() {
    this._promptCount++;

    // Check for timed-out suggestions
    this._expireSuggestions();
  }

  /**
   * Track a suggestion made by CYNIC
   * @param {Object} suggestion
   * @param {string} suggestion.type - skill|agent|tool|warning|pattern
   * @param {string} suggestion.content - What was suggested
   * @param {string} [suggestion.expectedTool] - Tool that would indicate following
   * @param {string} [suggestion.expectedAgent] - Agent that would indicate following
   * @param {string} [suggestion.expectedSkill] - Skill that would indicate following
   * @param {string[]} [suggestion.oppositeActions] - Actions that would be "opposite"
   */
  trackSuggestion(suggestion) {
    const id = `sug_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;

    const tracked = {
      id,
      type: suggestion.type || 'unknown',
      content: suggestion.content || '',
      timestamp: Date.now(),
      promptIndex: this._promptCount,
      expectedTool: suggestion.expectedTool || null,
      expectedAgent: suggestion.expectedAgent || null,
      expectedSkill: suggestion.expectedSkill || null,
      oppositeActions: suggestion.oppositeActions || [],
      status: 'pending',
      resolvedAt: null,
      feedback: null,
    };

    this._trackedSuggestions.push(tracked);

    // Trim to max tracked
    if (this._trackedSuggestions.length > CONFIG.MAX_TRACKED_SUGGESTIONS) {
      // Mark oldest as ignored
      const oldest = this._trackedSuggestions.shift();
      if (oldest.status === 'pending') {
        this._generateFeedback(oldest, FeedbackType.IMPLICIT_IGNORED, PHI_INV_3);
      }
    }

    return id;
  }

  /**
   * Observe a tool call (called from observe.js)
   * @param {string} toolName
   * @param {Object} toolInput
   * @param {boolean} isError
   * @param {Object} toolOutput
   */
  observeToolCall(toolName, toolInput, isError, toolOutput) {
    // Record for pattern analysis
    this._recentToolCalls.push({
      tool: toolName,
      timestamp: Date.now(),
      isError,
      input: toolInput,
    });

    // Keep only recent calls
    if (this._recentToolCalls.length > 50) {
      this._recentToolCalls.shift();
    }

    // Check against pending suggestions
    for (const suggestion of this._trackedSuggestions) {
      if (suggestion.status !== 'pending') continue;

      // Check if this tool call follows the suggestion
      if (this._matchesSuggestion(suggestion, toolName, toolInput)) {
        suggestion.status = 'followed';
        suggestion.resolvedAt = Date.now();
        this._generateFeedback(suggestion, FeedbackType.IMPLICIT_FOLLOWED, PHI_INV);
      }

      // Check if this is an opposite action
      else if (this._isOppositeAction(suggestion, toolName, toolInput)) {
        suggestion.status = 'opposed';
        suggestion.resolvedAt = Date.now();
        this._generateFeedback(suggestion, FeedbackType.IMPLICIT_OPPOSITE, PHI_INV_2);
      }
    }

    // Special detection: commit after review
    if (toolName === 'Bash' && toolInput.command?.startsWith('git commit')) {
      const reviewSuggestion = this._trackedSuggestions.find(
        s => s.status === 'pending' &&
             (s.type === 'review' || s.expectedAgent === 'cynic-reviewer')
      );
      if (reviewSuggestion) {
        reviewSuggestion.status = 'followed';
        reviewSuggestion.resolvedAt = Date.now();
        this._generateFeedback(reviewSuggestion, FeedbackType.IMPLICIT_COMMITTED, PHI_INV);
      }
    }

    // Track error rate changes
    this._checkErrorRateChange(isError);
  }

  /**
   * Detect explicit feedback from user prompt
   * @param {string} prompt - User's prompt text
   * @returns {Object|null} Explicit feedback if detected
   */
  detectExplicitFeedback(prompt) {
    const promptLower = prompt.toLowerCase();

    // Positive signals
    const positivePatterns = [
      /\b(yes|oui|correct|exactly|perfect|great|thanks|merci|bon|good)\b/i,
      /\b(that's right|that works|worked|it works|ça marche)\b/i,
      /\b(helpful|helped|useful)\b/i,
    ];

    // Negative signals
    const negativePatterns = [
      /\b(no|non|wrong|incorrect|not that|pas ça|faux)\b/i,
      /\b(doesn't work|didn't work|ne marche pas|not working)\b/i,
      /\b(try again|again|encore|retry|recommence)\b/i,
      /\b(that's not|ce n'est pas|c'est pas)\b/i,
    ];

    // Check for negative first (more specific)
    for (const pattern of negativePatterns) {
      if (pattern.test(promptLower)) {
        // Find most recent suggestion
        const recent = this._trackedSuggestions
          .filter(s => s.status === 'pending' || s.status === 'followed')
          .pop();

        if (recent) {
          recent.status = 'opposed';
          recent.resolvedAt = Date.now();
          this._generateFeedback(recent, FeedbackType.EXPLICIT_NEGATIVE, 1.0);

          return {
            type: FeedbackType.EXPLICIT_NEGATIVE,
            sentiment: FeedbackSentiment.NEGATIVE,
            confidence: 1.0,
            suggestionId: recent.id,
          };
        }
      }
    }

    // Check for positive
    for (const pattern of positivePatterns) {
      if (pattern.test(promptLower)) {
        const recent = this._trackedSuggestions
          .filter(s => s.status === 'pending' || s.status === 'followed')
          .pop();

        if (recent) {
          recent.status = 'followed';
          recent.resolvedAt = Date.now();
          this._generateFeedback(recent, FeedbackType.EXPLICIT_POSITIVE, 1.0);

          return {
            type: FeedbackType.EXPLICIT_POSITIVE,
            sentiment: FeedbackSentiment.POSITIVE,
            confidence: 1.0,
            suggestionId: recent.id,
          };
        }
      }
    }

    return null;
  }

  /**
   * Check if tool call matches a suggestion
   * @private
   */
  _matchesSuggestion(suggestion, toolName, toolInput) {
    // Check expected tool
    if (suggestion.expectedTool && toolName === suggestion.expectedTool) {
      return true;
    }

    // Check expected agent (Task tool)
    if (suggestion.expectedAgent && toolName === 'Task') {
      const subagentType = toolInput.subagent_type || toolInput.subagentType || '';
      if (subagentType === suggestion.expectedAgent) {
        return true;
      }
    }

    // Check expected skill (Skill tool)
    if (suggestion.expectedSkill && toolName === 'Skill') {
      const skill = toolInput.skill || '';
      if (skill === suggestion.expectedSkill) {
        return true;
      }
    }

    // Check content match (for general suggestions)
    if (suggestion.content) {
      const command = toolInput.command || '';
      if (command.includes(suggestion.content)) {
        return true;
      }
    }

    return false;
  }

  /**
   * Check if tool call is opposite of suggestion
   * @private
   */
  _isOppositeAction(suggestion, toolName, toolInput) {
    // Check explicit opposite actions
    if (suggestion.oppositeActions && suggestion.oppositeActions.length > 0) {
      for (const opposite of suggestion.oppositeActions) {
        if (toolName === opposite) return true;
        const command = toolInput.command || '';
        if (command.includes(opposite)) return true;
      }
    }

    // If we suggested an agent but they used a different one
    if (suggestion.expectedAgent && toolName === 'Task') {
      const subagentType = toolInput.subagent_type || toolInput.subagentType || '';
      if (subagentType && subagentType !== suggestion.expectedAgent) {
        return true;
      }
    }

    return false;
  }

  /**
   * Check for error rate changes after suggestions
   * @private
   */
  _checkErrorRateChange(isError) {
    if (!this._sessionState?.isInitialized()) return;

    const stats = this._sessionState.getStats();
    const currentErrorRate = stats.toolCalls > 0 ?
      stats.errorsEncountered / stats.toolCalls : 0;

    // Check if we have recent suggestions that could be validated by error rate
    const recentSuggestions = this._trackedSuggestions.filter(
      s => s.status === 'followed' &&
           Date.now() - s.resolvedAt < CONFIG.FOLLOW_WINDOW_MS
    );

    if (recentSuggestions.length > 0) {
      // Error rate dropped = positive signal
      if (currentErrorRate < this._lastErrorRate - 0.1) {
        for (const suggestion of recentSuggestions) {
          this._generateFeedback(suggestion, FeedbackType.OUTCOME_ERROR_REDUCED, PHI_INV_2);
        }
      }

      // Error rate increased = negative signal
      if (currentErrorRate > this._lastErrorRate + 0.1) {
        for (const suggestion of recentSuggestions) {
          this._generateFeedback(suggestion, FeedbackType.OUTCOME_ERROR_INCREASED, PHI_INV_2);
        }
      }
    }

    this._lastErrorRate = currentErrorRate;
  }

  /**
   * Expire old pending suggestions
   * @private
   */
  _expireSuggestions() {
    const now = Date.now();
    const currentPrompt = this._promptCount;

    for (const suggestion of this._trackedSuggestions) {
      if (suggestion.status !== 'pending') continue;

      // Check prompt window
      const promptsElapsed = currentPrompt - suggestion.promptIndex;
      if (promptsElapsed > CONFIG.FOLLOW_WINDOW_PROMPTS) {
        suggestion.status = 'ignored';
        suggestion.resolvedAt = now;
        this._generateFeedback(suggestion, FeedbackType.IMPLICIT_IGNORED, PHI_INV_3);
      }

      // Check time window
      const timeElapsed = now - suggestion.timestamp;
      if (timeElapsed > CONFIG.FOLLOW_WINDOW_MS) {
        suggestion.status = 'ignored';
        suggestion.resolvedAt = now;
        this._generateFeedback(suggestion, FeedbackType.IMPLICIT_IGNORED, PHI_INV_3);
      }
    }
  }

  /**
   * Generate feedback event
   * @private
   */
  _generateFeedback(suggestion, type, confidence) {
    // Determine sentiment
    let sentiment = FeedbackSentiment.NEUTRAL;
    if ([
      FeedbackType.EXPLICIT_POSITIVE,
      FeedbackType.IMPLICIT_FOLLOWED,
      FeedbackType.IMPLICIT_COMMITTED,
      FeedbackType.OUTCOME_ERROR_REDUCED,
      FeedbackType.OUTCOME_SUCCESS_STREAK,
    ].includes(type)) {
      sentiment = FeedbackSentiment.POSITIVE;
    } else if ([
      FeedbackType.EXPLICIT_NEGATIVE,
      FeedbackType.IMPLICIT_OPPOSITE,
      FeedbackType.OUTCOME_ERROR_INCREASED,
    ].includes(type)) {
      sentiment = FeedbackSentiment.NEGATIVE;
    }

    // Calculate weighted confidence
    const isExplicit = type.startsWith('explicit_');
    const isOutcome = type.startsWith('outcome_');
    const weight = isExplicit ? CONFIG.EXPLICIT_WEIGHT :
                   isOutcome ? CONFIG.INFERRED_WEIGHT :
                   CONFIG.IMPLICIT_WEIGHT;

    const weightedConfidence = Math.min(confidence * weight, PHI_INV);

    const feedback = {
      id: `fb_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
      type,
      sentiment,
      confidence: weightedConfidence,
      suggestionId: suggestion.id,
      suggestionType: suggestion.type,
      suggestionContent: suggestion.content,
      timestamp: Date.now(),
      promptsElapsed: this._promptCount - suggestion.promptIndex,
      metadata: {
        expectedTool: suggestion.expectedTool,
        expectedAgent: suggestion.expectedAgent,
        expectedSkill: suggestion.expectedSkill,
      },
    };

    suggestion.feedback = feedback;
    this._feedbackHistory.push(feedback);

    // Trim history
    if (this._feedbackHistory.length > 100) {
      this._feedbackHistory.shift();
    }

    return feedback;
  }

  /**
   * Get all feedback for learning system
   * @param {boolean} [onlyNew=false] - Only return unsynced feedback
   * @returns {Object[]}
   */
  getFeedback(onlyNew = false) {
    if (onlyNew) {
      const newFeedback = this._feedbackHistory.filter(f => !f.synced);
      newFeedback.forEach(f => f.synced = true);
      return newFeedback;
    }
    return [...this._feedbackHistory];
  }

  /**
   * Alias for getFeedback - returns full feedback history
   * Used by sleep.js for learning summary
   * @returns {Object[]}
   */
  getHistory() {
    return this.getFeedback(false);
  }

  /**
   * Get feedback statistics
   * @returns {Object}
   */
  getStats() {
    const total = this._feedbackHistory.length;
    const positive = this._feedbackHistory.filter(
      f => f.sentiment === FeedbackSentiment.POSITIVE
    ).length;
    const negative = this._feedbackHistory.filter(
      f => f.sentiment === FeedbackSentiment.NEGATIVE
    ).length;
    const neutral = this._feedbackHistory.filter(
      f => f.sentiment === FeedbackSentiment.NEUTRAL
    ).length;

    const pendingSuggestions = this._trackedSuggestions.filter(
      s => s.status === 'pending'
    ).length;

    const avgConfidence = total > 0 ?
      this._feedbackHistory.reduce((sum, f) => sum + f.confidence, 0) / total : 0;

    return {
      total,
      positive,
      negative,
      neutral,
      positiveRate: total > 0 ? positive / total : 0,
      negativeRate: total > 0 ? negative / total : 0,
      pendingSuggestions,
      avgConfidence,
      promptCount: this._promptCount,
    };
  }

  /**
   * Export state for persistence
   * @returns {Object}
   */
  exportState() {
    return {
      trackedSuggestions: this._trackedSuggestions,
      feedbackHistory: this._feedbackHistory,
      promptCount: this._promptCount,
      lastErrorRate: this._lastErrorRate,
    };
  }

  /**
   * Import state from persistence
   * @param {Object} state
   */
  importState(state) {
    if (state.trackedSuggestions) {
      this._trackedSuggestions = state.trackedSuggestions;
    }
    if (state.feedbackHistory) {
      this._feedbackHistory = state.feedbackHistory;
    }
    if (state.promptCount) {
      this._promptCount = state.promptCount;
    }
    if (state.lastErrorRate !== undefined) {
      this._lastErrorRate = state.lastErrorRate;
    }
  }

  /**
   * Reset state (for testing)
   */
  reset() {
    this._trackedSuggestions = [];
    this._recentToolCalls = [];
    this._feedbackHistory = [];
    this._promptCount = 0;
    this._lastErrorRate = 0;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON INSTANCE
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get singleton ImplicitFeedbackDetector instance
 * @returns {ImplicitFeedbackDetector}
 */
function getImplicitFeedback() {
  if (!_instance) {
    _instance = new ImplicitFeedbackDetector();
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
function resetImplicitFeedback() {
  if (_instance) {
    _instance.reset();
  }
  _instance = null;
}

// ═══════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

export {
  ImplicitFeedbackDetector,
  getImplicitFeedback,
  resetImplicitFeedback,
  FeedbackType,
  FeedbackSentiment,
  CONFIG as IMPLICIT_FEEDBACK_CONFIG,
};

export default ImplicitFeedbackDetector;
