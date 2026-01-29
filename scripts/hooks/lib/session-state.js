/**
 * Session State Manager - Shared state between CYNIC hooks
 *
 * Provides in-memory state that persists across hook invocations within a session.
 * Uses temp file with locking for cross-process communication.
 *
 * "Le chien se souvient de tout" - CYNIC remembers everything within the session
 *
 * @module scripts/hooks/lib/session-state
 */

'use strict';

import fs from 'fs';
import path from 'path';
import os from 'os';

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/** Maximum items to keep in history arrays */
const MAX_PROMPT_HISTORY = 5;
const MAX_TOOL_HISTORY = 10;
const MAX_ERRORS = 20;
const MAX_WARNINGS = 10;
const MAX_PATTERNS = 50;

/** Escalation thresholds */
const CONSECUTIVE_ERROR_THRESHOLD_CAUTIOUS = 3;
const CONSECUTIVE_ERROR_THRESHOLD_STRICT = 5;

/** φ-aligned probability for decay (61.8%) */
const PHI_DECAY = 0.618;

// ═══════════════════════════════════════════════════════════════════════════
// SESSION STATE MANAGER
// ═══════════════════════════════════════════════════════════════════════════

/**
 * SessionStateManager - Singleton managing shared state between hooks
 *
 * Implements conceptual interfaces:
 * - ISessionReader: getPromptHistory, getToolHistory, getEscalationLevel, etc.
 * - ISessionWriter: recordPrompt, recordToolCall, setEscalationLevel, etc.
 */
class SessionStateManager {
  constructor() {
    this._initialized = false;
    this._sessionId = null;
    this._stateFilePath = null;
    this._lockFilePath = null;
    this._state = this._getDefaultState();
  }

  /**
   * Get default state structure
   * @private
   */
  _getDefaultState() {
    return {
      // Session identity
      sessionId: null,
      userId: null,
      startTime: null,
      lastActivity: null,

      // Accumulated context
      promptHistory: [],        // Last N prompts
      toolHistory: [],          // Last N tool calls
      warningsIssued: [],       // Warnings from guard
      patternsDetected: [],     // From observe

      // Learning signals
      recentErrors: [],         // Tool errors in this session
      consecutiveErrors: 0,     // For circuit breaker
      dangerBlockedCount: 0,    // How many times guard blocked

      // Intervention state
      pendingQuestions: [],     // From perceive
      activeWarnings: [],       // Unresolved warnings
      escalationLevel: 'normal', // normal → cautious → strict

      // Statistics
      stats: {
        toolCalls: 0,
        errorsEncountered: 0,
        warningsTriggered: 0,
        suggestionsEmitted: 0,
      },
    };
  }

  /**
   * Get temp directory path (cross-platform)
   * @private
   */
  _getTempDir() {
    // Use OS temp directory
    return os.tmpdir();
  }

  /**
   * Get state file path for session
   * @private
   */
  _getStateFilePath(sessionId) {
    const tempDir = this._getTempDir();
    return path.join(tempDir, `cynic-session-${sessionId}.json`);
  }

  /**
   * Get lock file path for session
   * @private
   */
  _getLockFilePath(sessionId) {
    const tempDir = this._getTempDir();
    return path.join(tempDir, `cynic-session-${sessionId}.lock`);
  }

  /**
   * Acquire simple file lock (non-blocking, best effort)
   * @private
   */
  async _acquireLock() {
    if (!this._lockFilePath) return true;

    try {
      // Try to create lock file exclusively
      const lockFd = fs.openSync(this._lockFilePath, 'wx');
      fs.writeSync(lockFd, String(process.pid));
      fs.closeSync(lockFd);
      return true;
    } catch (e) {
      if (e.code === 'EEXIST') {
        // Lock exists - check if stale (>30 seconds old)
        try {
          const stat = fs.statSync(this._lockFilePath);
          const ageMs = Date.now() - stat.mtimeMs;
          if (ageMs > 30000) {
            // Stale lock - remove and retry
            fs.unlinkSync(this._lockFilePath);
            return this._acquireLock();
          }
        } catch (statErr) {
          // Can't stat, try anyway
        }
        // Lock is fresh - wait briefly and continue (best effort)
        await new Promise(r => setTimeout(r, 50));
        return true; // Continue anyway to not block hooks
      }
      return true; // On other errors, continue anyway
    }
  }

  /**
   * Release file lock
   * @private
   */
  _releaseLock() {
    if (!this._lockFilePath) return;

    try {
      fs.unlinkSync(this._lockFilePath);
    } catch (e) {
      // Ignore errors releasing lock
    }
  }

  /**
   * Load state from file
   * @private
   */
  _loadState() {
    if (!this._stateFilePath) return this._getDefaultState();

    try {
      if (fs.existsSync(this._stateFilePath)) {
        const data = fs.readFileSync(this._stateFilePath, 'utf8');
        const parsed = JSON.parse(data);
        // Merge with defaults to ensure all fields exist
        return { ...this._getDefaultState(), ...parsed };
      }
    } catch (e) {
      // On error, return default state
    }

    return this._getDefaultState();
  }

  /**
   * Save state to file (atomic write)
   * @private
   */
  _saveState() {
    if (!this._stateFilePath) return;

    try {
      // Update last activity timestamp
      this._state.lastActivity = Date.now();

      // Write to temp file first, then rename (atomic)
      const tempPath = this._stateFilePath + '.tmp';
      fs.writeFileSync(tempPath, JSON.stringify(this._state, null, 2));
      fs.renameSync(tempPath, this._stateFilePath);
    } catch (e) {
      // Log error but don't throw - state persistence is best-effort
      if (process.env.CYNIC_DEBUG) {
        console.error('[SessionState] Save failed:', e.message);
      }
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PUBLIC API - Initialization
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Initialize session state
   *
   * @param {string} sessionId - Unique session identifier
   * @param {Object} [options] - Initialization options
   * @param {string} [options.userId] - User ID
   * @param {Object} [options.context] - Additional context
   * @returns {Promise<SessionStateManager>} This instance
   */
  async init(sessionId, options = {}) {
    if (this._initialized && this._sessionId === sessionId) {
      // Already initialized for this session - just reload
      await this._acquireLock();
      this._state = this._loadState();
      this._releaseLock();
      return this;
    }

    this._sessionId = sessionId;
    this._stateFilePath = this._getStateFilePath(sessionId);
    this._lockFilePath = this._getLockFilePath(sessionId);

    // Acquire lock and load existing state (if any)
    await this._acquireLock();
    this._state = this._loadState();

    // Set/update session identity
    this._state.sessionId = sessionId;
    if (options.userId) {
      this._state.userId = options.userId;
    }
    if (!this._state.startTime) {
      this._state.startTime = Date.now();
    }

    // Save initial state
    this._saveState();
    this._releaseLock();

    this._initialized = true;
    return this;
  }

  /**
   * Get singleton instance
   * @returns {SessionStateManager}
   */
  static getInstance() {
    if (!SessionStateManager._instance) {
      SessionStateManager._instance = new SessionStateManager();
    }
    return SessionStateManager._instance;
  }

  /**
   * Check if initialized
   * @returns {boolean}
   */
  isInitialized() {
    return this._initialized;
  }

  /**
   * Get current session ID
   * @returns {string|null}
   */
  getSessionId() {
    return this._sessionId;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PUBLIC API - ISessionReader (Read Operations)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get recent prompts
   * @param {number} [limit] - Max prompts to return
   * @returns {Object[]} Prompt history
   */
  getPromptHistory(limit = MAX_PROMPT_HISTORY) {
    return this._state.promptHistory.slice(-limit);
  }

  /**
   * Get recent tool calls
   * @param {number} [limit] - Max tool calls to return
   * @returns {Object[]} Tool history
   */
  getToolHistory(limit = MAX_TOOL_HISTORY) {
    return this._state.toolHistory.slice(-limit);
  }

  /**
   * Get recent errors
   * @param {number} [limit] - Max errors to return
   * @returns {Object[]} Error history
   */
  getRecentErrors(limit = MAX_ERRORS) {
    return this._state.recentErrors.slice(-limit);
  }

  /**
   * Get current escalation level
   * @returns {'normal'|'cautious'|'strict'}
   */
  getEscalationLevel() {
    return this._state.escalationLevel;
  }

  /**
   * Get consecutive error count
   * @returns {number}
   */
  getConsecutiveErrors() {
    return this._state.consecutiveErrors;
  }

  /**
   * Get active warnings
   * @returns {Object[]}
   */
  getActiveWarnings() {
    return this._state.activeWarnings;
  }

  /**
   * Get detected patterns
   * @returns {Object[]}
   */
  getPatternsDetected() {
    return this._state.patternsDetected;
  }

  /**
   * Get session statistics
   * @returns {Object}
   */
  getStats() {
    return { ...this._state.stats };
  }

  /**
   * Get recent context for injection into orchestration
   * @param {number} [limit=5] - Items per category
   * @returns {Object} Combined recent context
   */
  getRecentContext(limit = 5) {
    return {
      prompts: this.getPromptHistory(limit),
      tools: this.getToolHistory(limit),
      errors: this.getRecentErrors(limit),
      warnings: this.getActiveWarnings(),
      escalationLevel: this.getEscalationLevel(),
      consecutiveErrors: this.getConsecutiveErrors(),
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PUBLIC API - ISessionWriter (Write Operations)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record a user prompt
   * @param {Object} prompt - Prompt data
   * @param {string} prompt.content - Prompt text
   * @param {string[]} [prompt.intents] - Detected intents
   */
  recordPrompt(prompt) {
    this._state.promptHistory.push({
      ...prompt,
      timestamp: Date.now(),
    });

    // Trim to max size
    if (this._state.promptHistory.length > MAX_PROMPT_HISTORY) {
      this._state.promptHistory = this._state.promptHistory.slice(-MAX_PROMPT_HISTORY);
    }

    this._saveState();
  }

  /**
   * Record a tool call and its outcome
   * @param {Object} toolCall - Tool call data
   * @param {string} toolCall.tool - Tool name
   * @param {Object} toolCall.input - Tool input
   * @param {*} [toolCall.output] - Tool output
   * @param {boolean} [toolCall.isError] - Whether it was an error
   * @param {string} [toolCall.errorType] - Type of error
   */
  recordToolCall(toolCall) {
    const record = {
      tool: toolCall.tool,
      inputSize: JSON.stringify(toolCall.input || {}).length,
      isError: toolCall.isError || false,
      errorType: toolCall.errorType,
      timestamp: Date.now(),
    };

    this._state.toolHistory.push(record);
    this._state.stats.toolCalls++;

    // Track errors
    if (toolCall.isError) {
      this._state.stats.errorsEncountered++;
      this._state.consecutiveErrors++;
      this._recordError(toolCall);
    } else {
      // Reset consecutive errors on success
      this._state.consecutiveErrors = 0;
    }

    // Trim to max size
    if (this._state.toolHistory.length > MAX_TOOL_HISTORY) {
      this._state.toolHistory = this._state.toolHistory.slice(-MAX_TOOL_HISTORY);
    }

    // Update escalation based on consecutive errors
    this._updateEscalation();

    this._saveState();
  }

  /**
   * Record an error (internal helper)
   * @private
   */
  _recordError(toolCall) {
    const errorRecord = {
      tool: toolCall.tool,
      errorType: toolCall.errorType || 'unknown',
      message: toolCall.errorMessage || '',
      timestamp: Date.now(),
    };

    this._state.recentErrors.push(errorRecord);

    // Trim to max size
    if (this._state.recentErrors.length > MAX_ERRORS) {
      this._state.recentErrors = this._state.recentErrors.slice(-MAX_ERRORS);
    }
  }

  /**
   * Record a warning issued by guard
   * @param {Object} warning - Warning data
   * @param {string} warning.tool - Tool that triggered warning
   * @param {string} warning.message - Warning message
   * @param {string} warning.severity - Warning severity
   * @param {boolean} [warning.blocked] - Whether operation was blocked
   */
  recordWarning(warning) {
    const record = {
      ...warning,
      timestamp: Date.now(),
    };

    this._state.warningsIssued.push(record);
    this._state.stats.warningsTriggered++;

    if (warning.blocked) {
      this._state.dangerBlockedCount++;
    }

    // Add to active warnings (unresolved)
    this._state.activeWarnings.push(record);

    // Trim arrays
    if (this._state.warningsIssued.length > MAX_WARNINGS) {
      this._state.warningsIssued = this._state.warningsIssued.slice(-MAX_WARNINGS);
    }
    if (this._state.activeWarnings.length > MAX_WARNINGS) {
      this._state.activeWarnings = this._state.activeWarnings.slice(-MAX_WARNINGS);
    }

    this._saveState();
  }

  /**
   * Record a detected pattern
   * @param {Object} pattern - Pattern data
   */
  recordPattern(pattern) {
    this._state.patternsDetected.push({
      ...pattern,
      timestamp: Date.now(),
    });

    // Trim to max size
    if (this._state.patternsDetected.length > MAX_PATTERNS) {
      this._state.patternsDetected = this._state.patternsDetected.slice(-MAX_PATTERNS);
    }

    this._saveState();
  }

  /**
   * Record a suggestion that was emitted
   */
  recordSuggestion() {
    this._state.stats.suggestionsEmitted++;
    this._saveState();
  }

  /**
   * Clear active warnings (e.g., after user acknowledges)
   */
  clearActiveWarnings() {
    this._state.activeWarnings = [];
    this._saveState();
  }

  /**
   * Manually set escalation level
   * @param {'normal'|'cautious'|'strict'} level
   */
  setEscalationLevel(level) {
    if (['normal', 'cautious', 'strict'].includes(level)) {
      this._state.escalationLevel = level;
      this._saveState();
    }
  }

  /**
   * Reset consecutive error count
   */
  resetConsecutiveErrors() {
    this._state.consecutiveErrors = 0;
    this._updateEscalation();
    this._saveState();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PUBLIC API - Decision Helpers
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Check if escalation is needed based on current state
   * @returns {boolean}
   */
  shouldEscalate() {
    // Escalate if consecutive errors exceed threshold
    if (this._state.consecutiveErrors >= CONSECUTIVE_ERROR_THRESHOLD_CAUTIOUS) {
      return true;
    }

    // Escalate if many recent errors (>50% of recent tools)
    const recentTools = this._state.toolHistory.slice(-10);
    const recentErrorCount = recentTools.filter(t => t.isError).length;
    if (recentTools.length >= 5 && recentErrorCount / recentTools.length > 0.5) {
      return true;
    }

    return false;
  }

  /**
   * Check if circuit breaker should trip
   * @returns {{shouldTrip: boolean, reason: string}}
   */
  shouldCircuitBreak() {
    // Trip on too many consecutive errors
    if (this._state.consecutiveErrors >= CONSECUTIVE_ERROR_THRESHOLD_STRICT) {
      return {
        shouldTrip: true,
        reason: `${this._state.consecutiveErrors} consecutive errors detected`,
      };
    }

    // Check for repeated same error
    const recentErrors = this._state.recentErrors.slice(-5);
    if (recentErrors.length >= 3) {
      const errorTypes = recentErrors.map(e => e.errorType);
      const sameError = errorTypes.every(t => t === errorTypes[0]);
      if (sameError) {
        return {
          shouldTrip: true,
          reason: `Repeated ${errorTypes[0]} error pattern`,
        };
      }
    }

    return { shouldTrip: false, reason: '' };
  }

  /**
   * Get similar past errors to current error
   * @param {string} errorType - Current error type
   * @returns {Object[]} Similar past errors
   */
  getSimilarErrors(errorType) {
    return this._state.recentErrors.filter(e => e.errorType === errorType);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PRIVATE - Escalation Management
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Update escalation level based on current state
   * @private
   */
  _updateEscalation() {
    const prevLevel = this._state.escalationLevel;

    if (this._state.consecutiveErrors >= CONSECUTIVE_ERROR_THRESHOLD_STRICT) {
      this._state.escalationLevel = 'strict';
    } else if (this._state.consecutiveErrors >= CONSECUTIVE_ERROR_THRESHOLD_CAUTIOUS) {
      this._state.escalationLevel = 'cautious';
    } else if (this._state.consecutiveErrors === 0) {
      // Only de-escalate to normal after clean run
      this._state.escalationLevel = 'normal';
    }

    // Log escalation changes
    if (prevLevel !== this._state.escalationLevel && process.env.CYNIC_DEBUG) {
      console.error(`[SessionState] Escalation: ${prevLevel} → ${this._state.escalationLevel}`);
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PUBLIC API - Cleanup
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Clean up session state (call at session end)
   * Removes temp files and resets state
   */
  cleanup() {
    // Remove state file
    if (this._stateFilePath) {
      try {
        if (fs.existsSync(this._stateFilePath)) {
          fs.unlinkSync(this._stateFilePath);
        }
      } catch (e) {
        // Ignore cleanup errors
      }
    }

    // Remove lock file
    this._releaseLock();

    // Reset state
    this._state = this._getDefaultState();
    this._initialized = false;
    this._sessionId = null;
    this._stateFilePath = null;
    this._lockFilePath = null;
  }

  /**
   * Get full state snapshot (for debugging/persistence)
   * @returns {Object} Complete state
   */
  getSnapshot() {
    return JSON.parse(JSON.stringify(this._state));
  }
}

// Singleton instance
SessionStateManager._instance = null;

// ═══════════════════════════════════════════════════════════════════════════
// EXPORTS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Get SessionStateManager singleton instance
 * @returns {SessionStateManager}
 */
export function getSessionState() {
  return SessionStateManager.getInstance();
}

export { SessionStateManager };
export default SessionStateManager;
