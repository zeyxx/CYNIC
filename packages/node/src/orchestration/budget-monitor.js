/**
 * BudgetMonitor — A4 Budget Tracking
 *
 * Reads budget state from CostLedger and φ-governor, classifies urgency.
 * Provides real-time recommendations for throttling/escalation.
 *
 * "φ knows when to stop" - κυνικός
 *
 * @module @cynic/node/orchestration/budget-monitor
 */

'use strict';

import { PHI_INV, PHI_INV_2, createLogger } from '@cynic/core';

const log = createLogger('BudgetMonitor');

/**
 * Budget urgency levels (φ-aligned thresholds)
 */
export const BudgetLevel = {
  ABUNDANT: 'ABUNDANT',   // < φ⁻² (38.2%) consumed
  MODERATE: 'MODERATE',   // < φ⁻¹ (61.8%) consumed
  CAUTIOUS: 'CAUTIOUS',   // < 80% consumed
  CRITICAL: 'CRITICAL',   // < 95% consumed
  EXHAUSTED: 'EXHAUSTED', // >= 95% consumed
};

/**
 * Budget recommendations
 */
export const BudgetRecommendation = {
  CONTINUE: 'CONTINUE',   // Proceed normally
  THROTTLE: 'THROTTLE',   // Reduce overhead
  ESCALATE: 'ESCALATE',   // Hand off to FastRouter
  HALT: 'HALT',           // Block operations
};

/**
 * BudgetMonitor
 *
 * Monitors budget state and provides throttling recommendations
 * based on φ-aligned thresholds and φ-governor influence zone.
 */
export class BudgetMonitor {
  /**
   * @param {Object} options
   * @param {Object} options.costLedger - CostLedger instance
   * @param {Object} [options.phiGovernor] - φ-governor instance (optional)
   */
  constructor(options = {}) {
    this.costLedger = options.costLedger;
    this.phiGovernor = options.phiGovernor || null;

    if (!this.costLedger) {
      throw new Error('BudgetMonitor requires costLedger');
    }

    this._stats = {
      assessments: 0,
      lastLevel: null,
      levelTransitions: [],
    };
  }

  /**
   * Assess current budget state and provide recommendation.
   *
   * @returns {Object} Budget assessment
   */
  assess() {
    this._stats.assessments++;

    // Get budget state from CostLedger
    const budget = this.costLedger.getBudgetStatus();

    // Get φ-governor state (if available)
    const phiState = this.phiGovernor?.getState?.() || {
      currentZone: 'balanced',
      ema: 0.5,
    };

    // Classify budget level
    const level = this._classifyLevel(budget.consumedRatio);

    // Determine recommendation
    const recommendation = this._determineRecommendation(level, budget, phiState);

    // Track level transitions
    if (this._stats.lastLevel && this._stats.lastLevel !== level) {
      this._stats.levelTransitions.push({
        from: this._stats.lastLevel,
        to: level,
        timestamp: Date.now(),
      });
      log.info('Budget level transition', {
        from: this._stats.lastLevel,
        to: level,
        consumedRatio: budget.consumedRatio.toFixed(3),
      });
    }
    this._stats.lastLevel = level;

    return {
      level,
      consumedRatio: budget.consumedRatio,
      consumed: budget.consumed,
      remaining: budget.remaining,
      remainingTokens: budget.remaining,
      timeToLimitMinutes: budget.timeToLimitMinutes || null,
      zone: phiState.currentZone,
      recommendation,
      timestamp: Date.now(),
    };
  }

  /**
   * Classify budget level based on consumed ratio.
   *
   * @param {number} consumedRatio - Ratio of budget consumed (0-1)
   * @returns {string} Budget level
   * @private
   */
  _classifyLevel(consumedRatio) {
    if (consumedRatio >= 0.95) {
      return BudgetLevel.EXHAUSTED;
    } else if (consumedRatio >= 0.80) {
      return BudgetLevel.CRITICAL;
    } else if (consumedRatio >= PHI_INV) {
      return BudgetLevel.CAUTIOUS;
    } else if (consumedRatio >= PHI_INV_2) {
      return BudgetLevel.MODERATE;
    } else {
      return BudgetLevel.ABUNDANT;
    }
  }

  /**
   * Determine recommendation based on budget level and φ-governor zone.
   *
   * Decision matrix:
   * - EXHAUSTED → HALT
   * - CRITICAL → ESCALATE
   * - CAUTIOUS → THROTTLE
   * - MODERATE + over zone → THROTTLE
   * - MODERATE + balanced/under → CONTINUE
   * - ABUNDANT → CONTINUE
   *
   * @param {string} level - Budget level
   * @param {Object} budget - Budget state from CostLedger
   * @param {Object} phiState - φ-governor state
   * @returns {string} Recommendation
   * @private
   */
  _determineRecommendation(level, budget, phiState) {
    if (level === BudgetLevel.EXHAUSTED) {
      return BudgetRecommendation.HALT;
    }

    if (level === BudgetLevel.CRITICAL) {
      return BudgetRecommendation.ESCALATE;
    }

    if (level === BudgetLevel.CAUTIOUS) {
      return BudgetRecommendation.THROTTLE;
    }

    if (level === BudgetLevel.MODERATE) {
      // If φ-governor says we're over-influencing, throttle even at MODERATE
      if (phiState.currentZone === 'over') {
        return BudgetRecommendation.THROTTLE;
      }
      return BudgetRecommendation.CONTINUE;
    }

    // ABUNDANT
    return BudgetRecommendation.CONTINUE;
  }

  /**
   * Get monitor statistics.
   *
   * @returns {Object} Stats
   */
  getStats() {
    return {
      assessments: this._stats.assessments,
      lastLevel: this._stats.lastLevel,
      transitionCount: this._stats.levelTransitions.length,
      recentTransitions: this._stats.levelTransitions.slice(-5),
    };
  }

  /**
   * Reset for testing.
   */
  _resetForTesting() {
    this._stats = {
      assessments: 0,
      lastLevel: null,
      levelTransitions: [],
    };
  }
}

export default { BudgetMonitor, BudgetLevel, BudgetRecommendation };
