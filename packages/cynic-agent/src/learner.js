/**
 * @cynic/agent - Learner Module
 *
 * Learns from outcomes via UnifiedSignal pipeline.
 * "Le chien se souvient" - κυνικός
 *
 * @module @cynic/agent/learner
 */

'use strict';

import { EventEmitter } from 'eventemitter3';
import { PHI_INV, PHI_INV_2, createLogger, globalEventBus, EventType } from '@cynic/core';

const log = createLogger('Learner');

// Lazy-load UnifiedSignalStore to avoid circular deps
let _signalStore = null;
async function getSignalStore() {
  if (_signalStore) return _signalStore;
  try {
    const { getUnifiedSignalStore } = await import('@cynic/node');
    _signalStore = getUnifiedSignalStore();
    return _signalStore;
  } catch (e) {
    log.debug('UnifiedSignalStore not available', { error: e.message });
    return null;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════════

const DEFAULT_CONFIG = {
  // Outcome evaluation delay
  evaluationDelay: 30000, // 30s

  // Learning rate (how fast we adapt)
  learningRate: PHI_INV_2, // 38.2%

  // Minimum samples before adjusting
  minSamples: 5,

  // Persistence: always persist — CYNIC remembers everything
  persistSignals: process.env.PERSIST_SIGNALS !== 'false',
};

// ═══════════════════════════════════════════════════════════════════════════════
// Outcome Types
// ═══════════════════════════════════════════════════════════════════════════════

export const OutcomeType = {
  PROFITABLE: 'profitable',     // Made money
  BREAKEVEN: 'breakeven',       // ~0% return
  LOSS: 'loss',                 // Lost money
  MISSED_OPPORTUNITY: 'missed', // Should have acted but didn't
  AVOIDED_LOSS: 'avoided_loss', // Correctly didn't act
};

// ═══════════════════════════════════════════════════════════════════════════════
// Learner Class
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Learner - Learns from trading outcomes
 *
 * Integrates with CYNIC's UnifiedSignal system for:
 * - RLHF-style feedback on judgments
 * - DPO pairing (correct vs incorrect decisions)
 * - Q-learning episodes for strategy refinement
 */
export class Learner extends EventEmitter {
  constructor(options = {}) {
    super();

    this.config = { ...DEFAULT_CONFIG, ...options };

    // Action records pending evaluation
    this.pendingActions = new Map();

    // Learning state
    this.lessons = [];
    this.maxLessons = 200;

    // Dimension adjustments from learning
    this.dimensionAdjustments = {};

    // Pattern recognition
    this.patterns = new Map();

    // Metrics
    this.metrics = {
      actionsRecorded: 0,
      outcomesEvaluated: 0,
      lessonsLearned: 0,
      patternsDetected: 0,
      totalPnL: 0,
      winRate: 0,
      wins: 0,
      losses: 0,
    };
  }

  /**
   * Record an action for later evaluation
   *
   * @param {Object} actionData - Action data including opportunity, judgment, decision, result
   */
  recordAction(actionData) {
    const { opportunity, judgment, decision, result } = actionData;

    const record = {
      id: `rec_${Date.now()}_${Math.random().toString(36).slice(2, 6)}`,
      timestamp: Date.now(),
      opportunity,
      judgment,
      decision,
      result,
      outcome: null, // To be filled by evaluateOutcome
    };

    this.pendingActions.set(record.id, record);
    this.metrics.actionsRecorded++;

    log.debug('Action recorded', {
      id: record.id,
      action: decision.action,
      qScore: judgment.qScore,
    });

    return record;
  }

  /**
   * Evaluate the outcome of an action
   *
   * @param {Object} actionResult - The action result to evaluate
   * @returns {Object} Outcome evaluation
   */
  async evaluateOutcome(actionResult) {
    this.metrics.outcomesEvaluated++;

    // Find pending action record
    let record = null;
    for (const [id, rec] of this.pendingActions) {
      if (rec.result?.id === actionResult.id) {
        record = rec;
        this.pendingActions.delete(id);
        break;
      }
    }

    if (!record) {
      // Create minimal record for standalone evaluation
      record = {
        id: `eval_${Date.now()}`,
        timestamp: Date.now(),
        result: actionResult,
      };
    }

    // Calculate P&L (simplified for simulation)
    let pnl = 0;
    let outcomeType = OutcomeType.BREAKEVEN;

    if (actionResult.simulated) {
      // Use simulated P&L
      pnl = actionResult.simulatedPnL || 0;
    } else {
      // In production, would check actual token price change
      pnl = 0; // Would fetch from chain
    }

    // Determine outcome type
    if (pnl > 0.01) {
      outcomeType = OutcomeType.PROFITABLE;
      this.metrics.wins++;
    } else if (pnl < -0.01) {
      outcomeType = OutcomeType.LOSS;
      this.metrics.losses++;
    } else {
      outcomeType = OutcomeType.BREAKEVEN;
    }

    this.metrics.totalPnL += pnl;
    this.metrics.winRate = this.metrics.wins / (this.metrics.wins + this.metrics.losses) || 0;

    // Create outcome
    const outcome = {
      id: `out_${Date.now()}`,
      recordId: record.id,
      timestamp: Date.now(),
      outcomeType,
      pnl,
      pnlPercent: pnl * 100,
      success: actionResult.success,
    };

    record.outcome = outcome;

    // Extract lesson
    const lesson = await this._extractLesson(record, outcome);
    if (lesson) {
      this._recordLesson(lesson);
      this.emit('lesson', lesson);
    }

    // Create UnifiedSignal for learning pipeline
    const signal = this._createUnifiedSignal(record, outcome);

    // Persist if enabled
    if (this.config.persistSignals) {
      await this._persistSignal(signal);
    }

    // Emit to globalEventBus for collective learning
    globalEventBus.emit(EventType.USER_FEEDBACK, {
      id: outcome.id,
      payload: {
        source: 'cynic-agent',
        outcomeType,
        pnl,
        success: actionResult.success,
        winRate: this.metrics.winRate,
      },
    });

    log.info('Outcome evaluated', {
      outcomeType,
      pnl: (pnl * 100).toFixed(2) + '%',
      winRate: (this.metrics.winRate * 100).toFixed(1) + '%',
    });

    return outcome;
  }

  /**
   * Extract a lesson from the record and outcome
   * @private
   */
  async _extractLesson(record, outcome) {
    if (!record.judgment) return null;

    const { judgment, decision, result } = record;
    const { outcomeType, pnl } = outcome;

    // Determine if we should learn from this
    const isSignificant = Math.abs(pnl) > 0.02 || !result.success;

    if (!isSignificant) return null;

    // Find which dimensions contributed to the outcome
    const contributingDimensions = this._findContributingDimensions(judgment.scores, outcomeType);

    const lesson = {
      id: `les_${Date.now()}`,
      timestamp: Date.now(),
      outcomeType,
      pnl,
      qScore: judgment.qScore,
      confidence: judgment.confidence,
      verdict: judgment.verdict,
      action: decision.action,

      // What we learned
      contributingDimensions,
      recommendation: this._generateRecommendation(outcomeType, contributingDimensions),

      // For DPO pairing
      isPositive: outcomeType === OutcomeType.PROFITABLE,
      isNegative: outcomeType === OutcomeType.LOSS,
    };

    return lesson;
  }

  /**
   * Find dimensions that contributed to outcome
   * @private
   */
  _findContributingDimensions(scores, outcomeType) {
    const contributing = [];

    for (const [dim, score] of Object.entries(scores)) {
      // High scores on losses = dimension failed us
      if (outcomeType === OutcomeType.LOSS && score > 0.6) {
        contributing.push({ dimension: dim, score, contribution: 'false_positive' });
      }

      // Low scores on wins = dimension was overcautious
      if (outcomeType === OutcomeType.PROFITABLE && score < 0.4) {
        contributing.push({ dimension: dim, score, contribution: 'false_negative' });
      }
    }

    return contributing.slice(0, 5); // Top 5 contributors
  }

  /**
   * Generate recommendation from lesson
   * @private
   */
  _generateRecommendation(outcomeType, contributingDimensions) {
    if (contributingDimensions.length === 0) {
      return 'No clear pattern detected';
    }

    const dim = contributingDimensions[0];

    if (dim.contribution === 'false_positive') {
      return `Reduce weight on "${dim.dimension}" - scored ${(dim.score * 100).toFixed(0)}% but led to loss`;
    }

    if (dim.contribution === 'false_negative') {
      return `Increase weight on "${dim.dimension}" - scored ${(dim.score * 100).toFixed(0)}% but outcome was positive`;
    }

    return 'Continue monitoring pattern';
  }

  /**
   * Create UnifiedSignal for learning pipeline
   * @private
   */
  _createUnifiedSignal(record, outcome) {
    return {
      id: `sig_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`,
      timestamp: Date.now(),
      source: 'AGENT_EXECUTION',
      sessionId: record.id,

      // Input (what we saw)
      input: {
        itemType: 'trading_opportunity',
        itemHash: record.opportunity?.id,
        tool: 'cynic-agent',
        dog: 'Oracle', // Primary dog for trading
        taskType: 'trade_evaluation',
      },

      // Judgment (what we decided)
      judgment: record.judgment ? {
        qScore: record.judgment.qScore,
        confidence: record.judgment.confidence,
        verdict: record.judgment.verdict,
        judgmentId: record.judgment.id,
      } : null,

      // Outcome (ground truth)
      outcome: {
        status: outcome.success ? 'CORRECT' : 'INCORRECT',
        actualScore: outcome.pnl > 0 ? 75 : outcome.pnl < 0 ? 25 : 50,
        reason: outcome.outcomeType,
      },

      // Learning
      learning: {
        reward: outcome.pnl,
        scoreDelta: record.judgment
          ? (outcome.pnl > 0 ? 75 : 25) - record.judgment.qScore
          : 0,
        feedbackType: outcome.outcomeType === OutcomeType.PROFITABLE
          ? 'POSITIVE'
          : outcome.outcomeType === OutcomeType.LOSS
            ? 'NEGATIVE'
            : 'NEUTRAL',
        canPair: outcome.outcomeType !== OutcomeType.BREAKEVEN,
        isChosen: outcome.outcomeType === OutcomeType.PROFITABLE,
      },
    };
  }

  /**
   * Persist signal to UnifiedSignalStore
   * @private
   */
  async _persistSignal(signal) {
    try {
      const store = await getSignalStore();
      if (!store) {
        log.debug('Signal store not available, skipping persistence', { id: signal.id });
        return;
      }

      await store.record(signal);
      log.debug('Signal persisted to UnifiedSignalStore', { id: signal.id });
    } catch (e) {
      log.warn('Failed to persist signal', { id: signal.id, error: e.message });
    }
  }

  /**
   * Record lesson to history
   * @private
   */
  _recordLesson(lesson) {
    this.lessons.push(lesson);
    this.metrics.lessonsLearned++;

    while (this.lessons.length > this.maxLessons) {
      this.lessons.shift();
    }

    // Update dimension adjustments
    for (const contrib of lesson.contributingDimensions) {
      const current = this.dimensionAdjustments[contrib.dimension] || 0;

      if (contrib.contribution === 'false_positive') {
        // Reduce weight
        this.dimensionAdjustments[contrib.dimension] = current - this.config.learningRate * 0.1;
      } else if (contrib.contribution === 'false_negative') {
        // Increase weight
        this.dimensionAdjustments[contrib.dimension] = current + this.config.learningRate * 0.1;
      }
    }

    log.info('Lesson learned', {
      outcomeType: lesson.outcomeType,
      recommendation: lesson.recommendation,
    });
  }

  /**
   * Get dimension weight adjustments
   *
   * @returns {Object} Map of dimension → adjustment
   */
  getDimensionAdjustments() {
    return { ...this.dimensionAdjustments };
  }

  /**
   * Get recent lessons
   */
  getLessons(limit = 20) {
    return this.lessons.slice(-limit);
  }

  /**
   * Get status
   */
  getStatus() {
    return {
      metrics: { ...this.metrics },
      pendingActions: this.pendingActions.size,
      lessonsCount: this.lessons.length,
      dimensionAdjustments: Object.keys(this.dimensionAdjustments).length,
      winRate: this.metrics.winRate,
      totalPnL: this.metrics.totalPnL,
    };
  }
}

export default Learner;
