/**
 * φ-Governor — Homeostatic influence controller
 *
 * Maintains CYNIC's influence on the LLM at φ⁻¹ (61.8%) automatically.
 * Like body temperature at 37°C — the governor adjusts injection to
 * converge on the golden ratio of influence.
 *
 * Input: (injectedTokens, totalTokens, previousState)
 * Output: adjustmentFactor for next injection budget
 *
 * The setpoint is φ⁻¹. Above → reduce injection. Below φ⁻² → enrich.
 * The dead zone [φ⁻², φ⁻¹] is where no adjustment is needed.
 *
 * "Le thermostat doré" — κυνικός
 *
 * @module @cynic/core/intelligence/phi-governor
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '../axioms/constants.js';

// =============================================================================
// CONSTANTS
// =============================================================================

/** Target influence ratio — φ⁻¹ */
const SETPOINT = PHI_INV;

/** Lower bound of dead zone — φ⁻² */
const LOWER_BOUND = PHI_INV_2;

/** Maximum adjustment per step — prevents oscillation */
const MAX_STEP = 0.1;

/** Smoothing factor for exponential moving average (α) */
const EMA_ALPHA = 0.3;

/** Maximum history entries */
const MAX_HISTORY = 89; // Fib(11)

// =============================================================================
// GOVERNOR
// =============================================================================

/**
 * Create a φ-governor instance.
 * Stateful — tracks influence history and EMA.
 *
 * @returns {Object} Governor with measure(), adjust(), getState(), reset()
 */
export function createPhiGovernor() {
  let _ema = SETPOINT;  // Start at setpoint (assume ideal)
  let _history = [];
  let _adjustmentFactor = 1.0;  // 1.0 = no change, <1 = reduce, >1 = enrich
  let _consecutiveHigh = 0;
  let _consecutiveLow = 0;

  return {
    /**
     * Measure influence for one prompt cycle.
     *
     * @param {number} injectedTokens - Tokens of CYNIC context injected
     * @param {number} totalTokens - Total tokens in the prompt+response
     * @returns {Object} Measurement: { ratio, zone, ema, adjustment }
     */
    measure(injectedTokens, totalTokens) {
      if (totalTokens <= 0) {
        return { ratio: 0, zone: 'empty', ema: _ema, adjustment: _adjustmentFactor };
      }

      const ratio = Math.min(1, injectedTokens / totalTokens);

      // Update EMA
      _ema = EMA_ALPHA * ratio + (1 - EMA_ALPHA) * _ema;

      // Classify zone
      const zone = classifyZone(ratio);

      // Track consecutive deviations
      if (zone === 'over') {
        _consecutiveHigh++;
        _consecutiveLow = 0;
      } else if (zone === 'under') {
        _consecutiveLow++;
        _consecutiveHigh = 0;
      } else {
        _consecutiveHigh = 0;
        _consecutiveLow = 0;
      }

      // Calculate adjustment
      _adjustmentFactor = calculateAdjustment(ratio, _ema, _consecutiveHigh, _consecutiveLow);

      // Record
      const entry = {
        ratio,
        zone,
        ema: _ema,
        adjustment: _adjustmentFactor,
        injectedTokens,
        totalTokens,
        timestamp: Date.now(),
      };

      _history.push(entry);
      while (_history.length > MAX_HISTORY) _history.shift();

      return entry;
    },

    /**
     * Get the current adjustment factor for the next injection budget.
     *
     * @returns {number} Factor to multiply token budget by (0.5 - 1.5)
     */
    getAdjustment() {
      return _adjustmentFactor;
    },

    /**
     * Apply the governor's adjustment to a token budget.
     *
     * @param {number} baseBudget - Token budget from classifier
     * @returns {number} Adjusted budget
     */
    applyToBudget(baseBudget) {
      return Math.round(baseBudget * _adjustmentFactor);
    },

    /**
     * Get full governor state.
     */
    getState() {
      return {
        ema: _ema,
        adjustmentFactor: _adjustmentFactor,
        currentZone: classifyZone(_ema),
        consecutiveHigh: _consecutiveHigh,
        consecutiveLow: _consecutiveLow,
        historySize: _history.length,
        recentHistory: _history.slice(-5),
      };
    },

    /**
     * Get convergence metrics.
     */
    getConvergence() {
      if (_history.length < 3) {
        return { converged: false, reason: 'insufficient_data', samples: _history.length };
      }

      const recent = _history.slice(-5);
      const avgRatio = recent.reduce((s, h) => s + h.ratio, 0) / recent.length;
      const deviation = Math.abs(avgRatio - SETPOINT);
      const variance = recent.reduce((s, h) => s + Math.pow(h.ratio - avgRatio, 2), 0) / recent.length;

      return {
        converged: deviation < 0.1 && variance < 0.01,
        avgRatio,
        deviation,
        variance,
        samples: _history.length,
        targetReached: deviation < 0.05,
      };
    },

    /**
     * Get history for analysis.
     */
    getHistory(limit = 21) {
      return _history.slice(-limit);
    },

    /**
     * Reset governor to initial state.
     */
    reset() {
      _ema = SETPOINT;
      _history = [];
      _adjustmentFactor = 1.0;
      _consecutiveHigh = 0;
      _consecutiveLow = 0;
    },
  };
}

// =============================================================================
// INTERNAL HELPERS
// =============================================================================

/**
 * Classify influence ratio into zone.
 *
 * @param {number} ratio - Influence ratio (0-1)
 * @returns {string} 'under' | 'balanced' | 'over'
 */
function classifyZone(ratio) {
  if (ratio > SETPOINT) return 'over';
  if (ratio < LOWER_BOUND) return 'under';
  return 'balanced';
}

/**
 * Calculate adjustment factor based on current state.
 *
 * In the dead zone [φ⁻², φ⁻¹]: no adjustment (1.0)
 * Above φ⁻¹: reduce injection (< 1.0)
 * Below φ⁻²: increase injection (> 1.0)
 *
 * Consecutive deviations strengthen the correction (anti-oscillation).
 *
 * @param {number} ratio - Current influence ratio
 * @param {number} ema - Smoothed influence ratio
 * @param {number} consecutiveHigh - Consecutive over-influence readings
 * @param {number} consecutiveLow - Consecutive under-influence readings
 * @returns {number} Adjustment factor (0.5 - 1.5)
 */
function calculateAdjustment(ratio, ema, consecutiveHigh, consecutiveLow) {
  const zone = classifyZone(ema);

  if (zone === 'balanced') return 1.0;

  if (zone === 'over') {
    // How far over?
    const excess = ema - SETPOINT;
    // Base reduction: proportional to excess, capped
    let reduction = Math.min(MAX_STEP, excess * 2);
    // Strengthen if consecutive
    if (consecutiveHigh > 2) reduction = Math.min(MAX_STEP * 2, reduction * 1.5);
    return Math.max(0.5, 1.0 - reduction);
  }

  // zone === 'under'
  const deficit = LOWER_BOUND - ema;
  let boost = Math.min(MAX_STEP, deficit * 2);
  if (consecutiveLow > 2) boost = Math.min(MAX_STEP * 2, boost * 1.5);
  return Math.min(1.5, 1.0 + boost);
}

export {
  SETPOINT,
  LOWER_BOUND,
  MAX_STEP,
  MAX_HISTORY,
  classifyZone,
  calculateAdjustment,
};
