/**
 * Suggestion Engine - Proactive suggestion generation for CYNIC
 *
 * Generates contextual suggestions based on session state, detected patterns,
 * and accumulated errors. Makes observe hook proactive.
 *
 * "Le chien parle quand il faut" - CYNIC speaks when necessary
 *
 * @module scripts/hooks/lib/suggestion-engine
 */

'use strict';

import { getSessionState } from './session-state.js';
import { getFeedbackCollector } from './feedback-collector.js';

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/** Minimum errors before suggesting */
const MIN_ERRORS_FOR_SUGGESTION = 2;

/** Cooldown between suggestions (ms) - avoid spamming */
const SUGGESTION_COOLDOWN_MS = 30000; // 30 seconds

/** φ-aligned probability for non-critical suggestions */
const PHI_PROBABILITY = 0.618;

// ═══════════════════════════════════════════════════════════════════════════
// SUGGESTION TEMPLATES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Suggestion templates in CYNIC voice
 */
const SUGGESTION_TEMPLATES = {
  // Error-based suggestions
  consecutive_errors: [
    '*sniff* {count} errors in a row. Maybe step back and reconsider the approach?',
    '*ears perk* Pattern detected: {count} consecutive failures. Different strategy needed?',
    '*head tilt* Same thing, different result expected? {count} tries say otherwise.',
  ],

  // Anti-pattern suggestions (placeholders, actual text from FeedbackCollector)
  anti_pattern: [
    '*sniff* {description}\n   \u2192 {suggestion}',
  ],

  // Escalation suggestions
  escalation_cautious: [
    '*ears perk* Entering cautious mode. Will be more careful with the next operations.',
    '*sniff* Multiple issues detected. Tightening safety checks.',
  ],

  escalation_strict: [
    '*GROWL* Strict mode activated. Too many problems. Reviewing all operations carefully.',
    '*ears perk* Something is wrong. Taking extra precautions now.',
  ],

  // General guidance
  file_not_found: [
    '*sniff* Can\'t find that file. Try `ls` to see what\'s there?',
  ],

  permission_denied: [
    '*head tilt* Permission denied. Check file ownership or try elevated permissions?',
  ],

  connection_refused: [
    '*ears perk* Service not responding. Is it running? Try starting it first.',
  ],

  syntax_error: [
    '*sniff* Syntax error detected. Read the file first to understand its structure?',
  ],

  type_error: [
    '*head tilt* Type mismatch somewhere. Check variable types?',
  ],

  test_failure: [
    '*ears perk* Tests failing. Read the error output carefully?',
  ],

  // Recovery suggestions
  recovery: [
    '*tail wag* Things are looking better now.',
    '*sniff* Back on track. Good recovery.',
  ],
};

// ═══════════════════════════════════════════════════════════════════════════
// SUGGESTION ENGINE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * SuggestionEngine - Generates contextual suggestions for observe hook
 */
class SuggestionEngine {
  constructor() {
    this._lastSuggestionTime = 0;
    this._lastSuggestionType = null;
    this._previousEscalationLevel = 'normal';
  }

  /**
   * Check if a suggestion should be emitted
   *
   * @param {Object} [options] - Check options
   * @param {boolean} [options.forceTrigger=false] - Force suggestion regardless of cooldown
   * @returns {{shouldSuggest: boolean, reason: string}}
   */
  shouldSuggest(options = {}) {
    const { forceTrigger = false } = options;

    // Check cooldown
    if (!forceTrigger && Date.now() - this._lastSuggestionTime < SUGGESTION_COOLDOWN_MS) {
      return { shouldSuggest: false, reason: 'cooldown' };
    }

    const sessionState = getSessionState();
    const feedbackCollector = getFeedbackCollector();

    // Check for anti-pattern
    const suggestion = feedbackCollector.getSuggestion();
    if (suggestion) {
      return { shouldSuggest: true, reason: 'anti_pattern' };
    }

    // Check for consecutive errors
    const consecutive = sessionState.getConsecutiveErrors();
    if (consecutive >= MIN_ERRORS_FOR_SUGGESTION) {
      return { shouldSuggest: true, reason: 'consecutive_errors' };
    }

    // Check for escalation level change
    const currentLevel = sessionState.getEscalationLevel();
    if (currentLevel !== this._previousEscalationLevel) {
      return { shouldSuggest: true, reason: 'escalation_change' };
    }

    // Check for similar recent errors
    const recentErrors = sessionState.getRecentErrors(5);
    if (recentErrors.length >= MIN_ERRORS_FOR_SUGGESTION) {
      const errorTypes = recentErrors.map(e => e.errorType);
      const sameType = errorTypes.every(t => t === errorTypes[0]);
      if (sameType) {
        return { shouldSuggest: true, reason: 'same_error_type' };
      }
    }

    return { shouldSuggest: false, reason: '' };
  }

  /**
   * Generate a contextual suggestion
   *
   * @returns {Object|null} Suggestion object or null
   */
  generateSuggestion() {
    const sessionState = getSessionState();
    const feedbackCollector = getFeedbackCollector();
    const check = this.shouldSuggest();

    if (!check.shouldSuggest) {
      return null;
    }

    let message = null;
    let type = check.reason;

    switch (check.reason) {
      case 'anti_pattern': {
        const suggestion = feedbackCollector.getSuggestion();
        if (suggestion) {
          message = this._formatAntiPatternSuggestion(suggestion);
          type = suggestion.patternId;
        }
        break;
      }

      case 'consecutive_errors': {
        const count = sessionState.getConsecutiveErrors();
        message = this._formatConsecutiveErrorSuggestion(count);
        break;
      }

      case 'escalation_change': {
        const level = sessionState.getEscalationLevel();
        message = this._formatEscalationSuggestion(level);
        this._previousEscalationLevel = level;
        break;
      }

      case 'same_error_type': {
        const recentErrors = sessionState.getRecentErrors(3);
        if (recentErrors.length > 0) {
          const errorType = recentErrors[0].errorType;
          message = this._formatErrorTypeSuggestion(errorType);
        }
        break;
      }
    }

    if (message) {
      // Update tracking
      this._lastSuggestionTime = Date.now();
      this._lastSuggestionType = type;

      // Record in session state
      sessionState.recordSuggestion();

      // Reset pattern detection to avoid repeating
      if (check.reason === 'anti_pattern') {
        feedbackCollector.resetPatternDetection();
      }

      return {
        type,
        message: this._formatMessage(message),
        timestamp: Date.now(),
      };
    }

    return null;
  }

  /**
   * Format suggestion for observe hook output
   *
   * @param {Object} suggestion - Suggestion object from generateSuggestion()
   * @returns {string} Formatted message for hook output
   */
  formatForOutput(suggestion) {
    if (!suggestion) return '';

    return `\n\u2500\u2500 CYNIC SUGGESTION \u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\u2500\n   ${suggestion.message}\n`;
  }

  /**
   * Get recovery message when things improve
   * @returns {string|null}
   */
  getRecoveryMessage() {
    const sessionState = getSessionState();
    const currentLevel = sessionState.getEscalationLevel();

    // Check if we've recovered from escalation
    if (this._previousEscalationLevel !== 'normal' && currentLevel === 'normal') {
      this._previousEscalationLevel = 'normal';
      return this._pickRandom(SUGGESTION_TEMPLATES.recovery);
    }

    return null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PRIVATE FORMATTERS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Format anti-pattern suggestion
   * @private
   */
  _formatAntiPatternSuggestion(suggestion) {
    const template = this._pickRandom(SUGGESTION_TEMPLATES.anti_pattern);
    return template
      .replace('{description}', suggestion.description)
      .replace('{suggestion}', suggestion.suggestion);
  }

  /**
   * Format consecutive error suggestion
   * @private
   */
  _formatConsecutiveErrorSuggestion(count) {
    const template = this._pickRandom(SUGGESTION_TEMPLATES.consecutive_errors);
    return template.replace('{count}', count);
  }

  /**
   * Format escalation level change suggestion
   * @private
   */
  _formatEscalationSuggestion(level) {
    const templates = level === 'strict'
      ? SUGGESTION_TEMPLATES.escalation_strict
      : SUGGESTION_TEMPLATES.escalation_cautious;
    return this._pickRandom(templates);
  }

  /**
   * Format error type specific suggestion
   * @private
   */
  _formatErrorTypeSuggestion(errorType) {
    const templates = SUGGESTION_TEMPLATES[errorType];
    if (templates && templates.length > 0) {
      return this._pickRandom(templates);
    }
    // Fallback to generic
    return `*sniff* Multiple ${errorType} errors. Try a different approach?`;
  }

  /**
   * Final message formatting
   * @private
   */
  _formatMessage(message) {
    // Ensure single-line doesn't have trailing newlines
    return message.trim();
  }

  /**
   * Pick random template from array
   * @private
   */
  _pickRandom(templates) {
    if (!templates || templates.length === 0) {
      return '';
    }
    const index = Math.floor(Math.random() * templates.length);
    return templates[index];
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON & EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

/** Singleton instance */
let _instance = null;

/**
 * Get SuggestionEngine singleton instance
 * @returns {SuggestionEngine}
 */
export function getSuggestionEngine() {
  if (!_instance) {
    _instance = new SuggestionEngine();
  }
  return _instance;
}

export { SuggestionEngine };
export default SuggestionEngine;
