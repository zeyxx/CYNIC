/**
 * @cynic/node - Profile Calculator
 *
 * Fibonacci-weighted profile level calculation.
 * Re-evaluates every 21 interactions with max confidence φ⁻¹.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/profile/calculator
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import {
  OrganicSignals,
  SIGNAL_CONSTANTS,
  SignalType,
  calculateLinguisticSignal,
  calculateBehavioralSignal,
  calculateCodeSignal,
  calculateTemporalSignal,
} from './organic-signals.js';
import { ProfileSignalStore, SessionHistoryStore } from '../privacy/local-store.js';

/**
 * φ-aligned constants for profile calculation
 */
export const PROFILE_CONSTANTS = {
  /** Fibonacci profile levels */
  LEVELS: [1, 2, 3, 5, 8],

  /** Level names */
  LEVEL_NAMES: {
    1: 'Novice',
    2: 'Apprentice',
    3: 'Practitioner',
    5: 'Expert',
    8: 'Master',
  },

  /** Score thresholds for each level */
  THRESHOLDS: {
    1: { min: 0, max: 20 },
    2: { min: 20, max: 40 },
    3: { min: 40, max: 60 },
    5: { min: 60, max: 80 },
    8: { min: 80, max: 100 },
  },

  /** Re-evaluation interval (Fib(8) = 21) */
  REEVALUATION_INTERVAL: 21,

  /** Minimum interactions before level change (Fib(5) = 5) */
  MIN_INTERACTIONS_FOR_CHANGE: 5,

  /** Default starting level (Practitioner) */
  DEFAULT_LEVEL: 3,

  /** Level change smoothing factor (φ⁻¹) */
  SMOOTHING_FACTOR: PHI_INV,

  /** Max confidence (φ⁻¹) */
  MAX_CONFIDENCE: PHI_INV,

  /** Min confidence to consider signal (φ⁻²) */
  MIN_CONFIDENCE: PHI_INV_2,
};

/**
 * Profile level details
 */
export const ProfileLevel = {
  NOVICE: 1,
  APPRENTICE: 2,
  PRACTITIONER: 3,
  EXPERT: 5,
  MASTER: 8,
};

/**
 * Profile state snapshot
 */
export class ProfileState {
  /**
   * @param {number} level - Current profile level
   * @param {number} score - Raw score (0-100)
   * @param {number} confidence - Confidence in assessment (0 to φ⁻¹)
   * @param {number} interactionCount - Total interactions
   */
  constructor(level = PROFILE_CONSTANTS.DEFAULT_LEVEL, score = 50, confidence = 0, interactionCount = 0) {
    this.level = level;
    this.levelName = PROFILE_CONSTANTS.LEVEL_NAMES[level];
    this.score = score;
    this.confidence = confidence;
    this.interactionCount = interactionCount;
    this.lastEvaluatedAt = new Date();
    this.levelHistory = [];
  }

  /**
   * Record a level change
   * @param {number} previousLevel
   * @param {number} newLevel
   * @param {string} reason
   */
  recordChange(previousLevel, newLevel, reason) {
    this.levelHistory.push({
      from: previousLevel,
      to: newLevel,
      reason,
      at: new Date().toISOString(),
    });
  }

  /**
   * Get level description for adaptation
   */
  getAdaptationHints() {
    switch (this.level) {
      case ProfileLevel.NOVICE:
        return {
          explanationDepth: 'detailed',
          terminology: 'simplified',
          examples: 'abundant',
          warnings: 'prominent',
          assumeKnowledge: 'minimal',
        };
      case ProfileLevel.APPRENTICE:
        return {
          explanationDepth: 'moderate',
          terminology: 'introduce_technical',
          examples: 'helpful',
          warnings: 'clear',
          assumeKnowledge: 'basic',
        };
      case ProfileLevel.PRACTITIONER:
        return {
          explanationDepth: 'balanced',
          terminology: 'standard',
          examples: 'when_helpful',
          warnings: 'standard',
          assumeKnowledge: 'intermediate',
        };
      case ProfileLevel.EXPERT:
        return {
          explanationDepth: 'concise',
          terminology: 'advanced',
          examples: 'minimal',
          warnings: 'brief',
          assumeKnowledge: 'substantial',
        };
      case ProfileLevel.MASTER:
        return {
          explanationDepth: 'peer_level',
          terminology: 'expert',
          examples: 'only_if_asked',
          warnings: 'trust_judgment',
          assumeKnowledge: 'comprehensive',
        };
      default:
        return {
          explanationDepth: 'balanced',
          terminology: 'standard',
          examples: 'when_helpful',
          warnings: 'standard',
          assumeKnowledge: 'intermediate',
        };
    }
  }

  /**
   * Serialize for storage
   */
  toJSON() {
    return {
      level: this.level,
      levelName: this.levelName,
      score: this.score,
      confidence: this.confidence,
      interactionCount: this.interactionCount,
      lastEvaluatedAt: this.lastEvaluatedAt.toISOString(),
      levelHistory: this.levelHistory.slice(-10), // Keep last 10 changes
    };
  }

  /**
   * Deserialize from storage
   */
  static fromJSON(data) {
    const state = new ProfileState(
      data.level,
      data.score,
      data.confidence,
      data.interactionCount
    );
    state.lastEvaluatedAt = new Date(data.lastEvaluatedAt);
    state.levelHistory = data.levelHistory || [];
    return state;
  }
}

/**
 * Profile Calculator
 *
 * 100% organic profile detection - no explicit user declaration.
 * Combines signals with Fibonacci weights and maps to levels.
 */
export class ProfileCalculator {
  /**
   * @param {object} [options]
   * @param {ProfileSignalStore} [options.signalStore] - Signal storage
   * @param {SessionHistoryStore} [options.sessionStore] - Session storage
   */
  constructor(options = {}) {
    this.signals = new OrganicSignals();
    this.state = new ProfileState();

    // Stores for persistence (device-local)
    this.signalStore = options.signalStore || new ProfileSignalStore();
    this.sessionStore = options.sessionStore || new SessionHistoryStore();

    // Tracking
    this.messageHistory = [];
    this.toolUsageHistory = [];
    this.errorEvents = [];
    this.codeSnippets = [];

    // Evaluation state
    this.interactionsSinceLastEval = 0;
    this.pendingRecalculation = false;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // SIGNAL COLLECTION (Automatic from user behavior)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Process a user message
   * Extracts linguistic signals automatically.
   *
   * @param {string} message - User message
   */
  processMessage(message) {
    if (!message || typeof message !== 'string') return;

    // Store in history (for self-correction detection)
    this.messageHistory.push(message);
    if (this.messageHistory.length > 21) { // Fib(8)
      this.messageHistory.shift();
    }

    // Calculate linguistic signal
    this.signals.updateLinguistic(message, this.messageHistory);

    // Store signal for temporal analysis
    this.signalStore.recordSignal(
      SignalType.LINGUISTIC,
      this.signals.linguistic.score,
      { breakdown: this.signals.linguistic.breakdown }
    );

    this._incrementInteraction();
  }

  /**
   * Process a tool call
   * Extracts behavioral signals automatically.
   *
   * @param {string} toolName - Tool that was called
   * @param {boolean} success - Whether the call succeeded
   * @param {boolean} [isError=false] - Whether this was an error
   */
  processToolCall(toolName, success, isError = false) {
    const timestamp = Date.now();

    // Track tool usage
    this.toolUsageHistory.push({ tool: toolName, success });
    if (this.toolUsageHistory.length > 55) { // Fib(10)
      this.toolUsageHistory.shift();
    }

    // Track error events
    this.errorEvents.push({ timestamp, isError });
    if (this.errorEvents.length > 34) { // Fib(9)
      this.errorEvents.shift();
    }

    // Calculate behavioral signal
    this.signals.updateBehavioral(
      this.toolUsageHistory,
      this.errorEvents,
      null // Iteration info calculated separately
    );

    // Store signal
    this.signalStore.recordSignal(
      SignalType.BEHAVIORAL,
      this.signals.behavioral.score,
      { breakdown: this.signals.behavioral.breakdown }
    );

    this._incrementInteraction();
  }

  /**
   * Process code snippet
   * Extracts code signals automatically.
   *
   * @param {string} code - Code snippet from user
   */
  processCode(code) {
    if (!code || typeof code !== 'string') return;

    // Store snippet
    this.codeSnippets.push(code);
    if (this.codeSnippets.length > 13) { // Fib(7)
      this.codeSnippets.shift();
    }

    // Calculate code signal
    this.signals.updateCode(code);

    // Store signal
    this.signalStore.recordSignal(
      SignalType.CODE,
      this.signals.code.score,
      { breakdown: this.signals.code.breakdown }
    );

    this._incrementInteraction();
  }

  /**
   * Process session end
   * Updates temporal signals.
   *
   * @param {number} durationMs - Session duration in milliseconds
   */
  processSessionEnd(durationMs) {
    // Record session in session store
    this.sessionStore.endSession();

    // Get signal history for temporal analysis
    const signalHistory = [];
    for (const type of Object.values(SignalType)) {
      const history = this.signalStore.getSignalHistory(type);
      signalHistory.push(...history);
    }

    // Get session history
    const sessions = this.sessionStore.getRecentSessions(13)
      .map(e => ({ durationMs: e.data?.durationMs || 0 }));

    // Calculate temporal signal
    this.signals.updateTemporal(signalHistory, sessions);

    // Store signal
    this.signalStore.recordSignal(
      SignalType.TEMPORAL,
      this.signals.temporal.score,
      { breakdown: this.signals.temporal.breakdown }
    );
  }

  /**
   * Start a new session
   * @param {object} [metadata] - Session metadata
   */
  startSession(metadata = {}) {
    this.sessionStore.startSession(metadata);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PROFILE CALCULATION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Increment interaction counter and maybe recalculate
   * @private
   */
  _incrementInteraction() {
    this.state.interactionCount++;
    this.interactionsSinceLastEval++;

    // Check if we should recalculate
    if (this.interactionsSinceLastEval >= PROFILE_CONSTANTS.REEVALUATION_INTERVAL) {
      this.pendingRecalculation = true;
      this.recalculate();
    }
  }

  /**
   * Recalculate profile level
   *
   * @param {boolean} [force=false] - Force recalculation
   * @returns {{ level: number, changed: boolean, reason: string | null }}
   */
  recalculate(force = false) {
    if (!force && !this.pendingRecalculation) {
      return { level: this.state.level, changed: false, reason: null };
    }

    // Get combined score from signals
    const combined = this.signals.getCombinedScore();
    const newScore = combined.score;
    const confidence = combined.confidence;

    // Apply smoothing to avoid rapid level changes
    const smoothedScore = this._applySmoothing(this.state.score, newScore);

    // Determine new level
    const newLevel = this._scoreToLevel(smoothedScore);
    const previousLevel = this.state.level;

    // Check if level should change
    const shouldChange = this._shouldChangeLevel(previousLevel, newLevel, confidence);

    if (shouldChange) {
      const reason = this._getChangeReason(previousLevel, newLevel, confidence);
      this.state.recordChange(previousLevel, newLevel, reason);
      this.state.level = newLevel;
      this.state.levelName = PROFILE_CONSTANTS.LEVEL_NAMES[newLevel];
    }

    // Update state
    this.state.score = smoothedScore;
    this.state.confidence = confidence;
    this.state.lastEvaluatedAt = new Date();

    // Reset counters
    this.interactionsSinceLastEval = 0;
    this.pendingRecalculation = false;

    return {
      level: this.state.level,
      changed: shouldChange,
      reason: shouldChange ? this._getChangeReason(previousLevel, newLevel, confidence) : null,
    };
  }

  /**
   * Apply smoothing to avoid rapid level changes
   * @private
   */
  _applySmoothing(oldScore, newScore) {
    // Exponential moving average with φ⁻¹ as smoothing factor
    return oldScore * PROFILE_CONSTANTS.SMOOTHING_FACTOR +
           newScore * (1 - PROFILE_CONSTANTS.SMOOTHING_FACTOR);
  }

  /**
   * Map score to Fibonacci level
   * @private
   */
  _scoreToLevel(score) {
    const thresholds = PROFILE_CONSTANTS.THRESHOLDS;

    if (score >= thresholds[8].min) return 8;
    if (score >= thresholds[5].min) return 5;
    if (score >= thresholds[3].min) return 3;
    if (score >= thresholds[2].min) return 2;
    return 1;
  }

  /**
   * Determine if level should change
   * @private
   */
  _shouldChangeLevel(previousLevel, newLevel, confidence) {
    // Don't change if confidence is too low
    if (confidence < PROFILE_CONSTANTS.MIN_CONFIDENCE) {
      return false;
    }

    // Don't change if not enough interactions
    if (this.state.interactionCount < PROFILE_CONSTANTS.MIN_INTERACTIONS_FOR_CHANGE) {
      return false;
    }

    // Level changed
    if (previousLevel !== newLevel) {
      return true;
    }

    return false;
  }

  /**
   * Generate reason for level change
   * @private
   */
  _getChangeReason(previousLevel, newLevel, confidence) {
    const direction = newLevel > previousLevel ? 'increased' : 'decreased';
    const fromName = PROFILE_CONSTANTS.LEVEL_NAMES[previousLevel];
    const toName = PROFILE_CONSTANTS.LEVEL_NAMES[newLevel];

    const signalBreakdown = this.signals.getBreakdown();
    const dominantSignal = this._getDominantSignal(signalBreakdown);

    return `Level ${direction} from ${fromName} to ${toName} ` +
           `(confidence: ${Math.round(confidence * 100)}%, dominant signal: ${dominantSignal})`;
  }

  /**
   * Get the dominant signal type
   * @private
   */
  _getDominantSignal(breakdown) {
    const signals = [
      { type: 'linguistic', score: breakdown.linguistic?.score || 0 },
      { type: 'behavioral', score: breakdown.behavioral?.score || 0 },
      { type: 'code', score: breakdown.code?.score || 0 },
      { type: 'temporal', score: breakdown.temporal?.score || 0 },
    ].filter(s => s.score > 0);

    if (signals.length === 0) return 'none';

    return signals.reduce((a, b) => a.score > b.score ? a : b).type;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PUBLIC API
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get current profile state
   * @returns {ProfileState}
   */
  getState() {
    return this.state;
  }

  /**
   * Get current profile level
   * @returns {number}
   */
  getLevel() {
    return this.state.level;
  }

  /**
   * Get level name
   * @returns {string}
   */
  getLevelName() {
    return this.state.levelName;
  }

  /**
   * Get adaptation hints for agents
   * @returns {object}
   */
  getAdaptationHints() {
    return this.state.getAdaptationHints();
  }

  /**
   * Get full profile breakdown
   * @returns {object}
   */
  getBreakdown() {
    return {
      state: this.state.toJSON(),
      signals: this.signals.getBreakdown(),
      stats: {
        messageCount: this.messageHistory.length,
        toolCallCount: this.toolUsageHistory.length,
        codeSnippetCount: this.codeSnippets.length,
        interactionsSinceEval: this.interactionsSinceLastEval,
        pendingRecalculation: this.pendingRecalculation,
      },
      signalStats: this.signalStore.getSignalStats(),
      sessionStats: this.sessionStore.getSessionStats(),
    };
  }

  /**
   * Get simplified profile object
   * @returns {{ level: number, confidence: number, levelName: string }}
   */
  getProfile() {
    const combined = this.signals.getCombinedScore();

    // Collect all signal histories
    const signalHistory = [];
    for (const type of Object.values(SignalType)) {
      const history = this.signalStore.getSignalHistory(type);
      signalHistory.push(...history.map(h => ({ ...h, type, score: h.value })));
    }

    return {
      level: this.state.level,
      confidence: combined.confidence || this.state.confidence,
      levelName: this.state.levelName,
      score: combined.score,
      signalHistory,
    };
  }

  /**
   * Check if profile needs update (for external triggers)
   * @returns {boolean}
   */
  needsUpdate() {
    return this.pendingRecalculation ||
           this.interactionsSinceLastEval >= PROFILE_CONSTANTS.REEVALUATION_INTERVAL;
  }

  /**
   * Reset profile to defaults
   */
  reset() {
    this.signals = new OrganicSignals();
    this.state = new ProfileState();
    this.messageHistory = [];
    this.toolUsageHistory = [];
    this.errorEvents = [];
    this.codeSnippets = [];
    this.interactionsSinceLastEval = 0;
    this.pendingRecalculation = false;
  }

  /**
   * Export profile for backup (local only)
   * @returns {object}
   */
  exportForBackup() {
    return {
      state: this.state.toJSON(),
      signalStore: this.signalStore.exportForBackup(),
      exportedAt: new Date().toISOString(),
    };
  }

  /**
   * Import profile from backup
   * @param {object} backup
   */
  importFromBackup(backup) {
    if (backup.state) {
      this.state = ProfileState.fromJSON(backup.state);
    }
    if (backup.signalStore) {
      this.signalStore.importFromBackup(backup.signalStore);
    }
  }
}

/**
 * Create a profile calculator with default stores
 *
 * @returns {ProfileCalculator}
 */
export function createProfileCalculator() {
  return new ProfileCalculator({
    signalStore: new ProfileSignalStore(),
    sessionStore: new SessionHistoryStore(),
  });
}

export default {
  PROFILE_CONSTANTS,
  ProfileLevel,
  ProfileState,
  ProfileCalculator,
  createProfileCalculator,
};
