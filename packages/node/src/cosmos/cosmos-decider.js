/**
 * Cosmos Decider - C7.3 (COSMOS × DECIDE)
 *
 * Decides ecosystem-level actions based on CosmosJudge judgments.
 * Advisory: recommends focus shifts, resource rebalancing, interventions.
 *
 * Pattern-aware: different judgment types trigger different strategies.
 * Stability-checked: avoids flip-flopping without new data.
 * Calibrated: outcome tracking adjusts confidence over time.
 *
 * "Le chien décide pour les étoiles" - κυνικός
 *
 * @module @cynic/node/cosmos/cosmos-decider
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('CosmosDecider');

export const CosmosDecisionType = {
  ACCELERATE: 'accelerate',     // Increase activity/resourcing
  MAINTAIN: 'maintain',         // Hold current pace
  DECELERATE: 'decelerate',     // Reduce activity
  FOCUS: 'focus',               // Concentrate on key areas
  DIVERSIFY: 'diversify',       // Expand to underserved repos
  INTERVENE: 'intervene',       // Signal alert, need human attention
  WAIT: 'wait',                 // Hold decision pending more data
};

const DECISION_THRESHOLDS = {
  accelerate: 75,  // HOWL territory — ecosystem thriving
  maintain: 55,    // WAG territory — healthy baseline
  decelerate: 40,  // Above GROWL — needs slowing
  intervene: 30,   // Below GROWL — needs attention
};

/**
 * Per-judgment-type decision strategies.
 * Different ecosystem dimensions require different observation thresholds.
 */
const PATTERN_STRATEGIES = {
  ecosystem_coherence:  { minObservations: 5,  weight: { coherence: 0.5, utility: 0.2, sustainability: 0.3 } },
  ecosystem_utility:    { minObservations: 3,  weight: { coherence: 0.2, utility: 0.5, sustainability: 0.3 } },
  ecosystem_security:   { minObservations: 2,  weight: { coherence: 0.1, utility: 0.2, sustainability: 0.7 } },
  repo_distribution:    { minObservations: 5,  weight: { coherence: 0.3, utility: 0.4, sustainability: 0.3 } },
  cross_project_health: { minObservations: 8,  weight: { coherence: 0.4, utility: 0.3, sustainability: 0.3 } },
  default:              { minObservations: 3,  weight: { coherence: 0.34, utility: 0.33, sustainability: 0.33 } },
};

/** Minimum new observations before allowing decision reversal */
const STABILITY_THRESHOLD = 3;

/** Cooldown (ms) between INTERVENE decisions — F8=21 min */
const INTERVENE_COOLDOWN_MS = 21 * 60000;

export class CosmosDecider extends EventEmitter {
  constructor(options = {}) {
    super();

    this._history = [];
    this._maxHistory = 89; // Fib(11)
    this._outcomes = [];   // Outcome calibration

    this._lastInterveneTime = 0;

    this._stats = {
      totalDecisions: 0,
      byType: {},
      outcomesRecorded: 0,
      stabilityHolds: 0,
      goalViolations: 0,
      lastDecision: null,
    };

    for (const type of Object.values(CosmosDecisionType)) {
      this._stats.byType[type] = 0;
    }
  }

  /**
   * Decide based on a cosmos judgment
   *
   * @param {Object} judgment - From CosmosJudge.judge()
   * @param {Object} [context] - Additional context
   * @returns {Object} Decision result
   */
  decide(judgment, context = {}) {
    const score = judgment.score || 0;
    const verdict = judgment.verdict || 'BARK';
    const type = judgment.type || 'unknown';

    // Get pattern-specific strategy
    const strategy = PATTERN_STRATEGIES[type] || PATTERN_STRATEGIES.default;

    // Extract factors with weighted scoring
    const factors = this._extractFactors(judgment, context, strategy);

    // Calculate confidence (pattern-aware)
    const confidence = this._calculateConfidence(factors, strategy);

    // Make decision (stability-checked)
    const decision = this._makeDecision(score, verdict, factors, type, strategy);

    const result = {
      decision: decision.type,
      reason: decision.reason,
      severity: decision.severity || 'medium',
      confidence,
      score,
      verdict,
      factors,
      judgmentType: type,
      cell: 'C7.3',
      dimension: 'COSMOS',
      analysis: 'DECIDE',
      timestamp: Date.now(),
    };

    this._updateStats(result);
    this._history.push(result);
    while (this._history.length > this._maxHistory) this._history.shift();

    this.emit('decision', result);
    globalEventBus.publish('cosmos:decision', {
      decision: result,
      judgment,
    }, { source: 'CosmosDecider' });

    log.debug('Cosmos decision', { decision: decision.type, score, confidence, judgmentType: type });

    return result;
  }

  /**
   * Record the outcome of a previous decision for calibration.
   *
   * @param {Object} outcome
   * @param {string} outcome.decisionType - Type of decision
   * @param {string} outcome.result - 'success', 'partial', or 'failure'
   * @param {string} [outcome.reason] - Why it succeeded/failed
   */
  recordOutcome(outcome) {
    const entry = {
      decisionType: outcome.decisionType || 'unknown',
      result: outcome.result || 'unknown',
      reason: outcome.reason || null,
      ts: Date.now(),
    };

    this._outcomes.push(entry);
    while (this._outcomes.length > this._maxHistory) this._outcomes.shift();
    this._stats.outcomesRecorded++;

    this.emit('outcome_recorded', entry);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Factor extraction
  // ═══════════════════════════════════════════════════════════════════════════

  _extractFactors(judgment, context, strategy) {
    const scores = judgment.scores || {};
    const w = strategy.weight;

    const coherence = scores.coherence || 50;
    const utility = scores.utility || 50;
    const sustainability = scores.sustainability || 50;

    // Weighted composite score
    const weightedScore = coherence * w.coherence + utility * w.utility + sustainability * w.sustainability;

    return {
      coherence,
      utility,
      sustainability,
      weightedScore,
      observationCount: context.observationCount || 0,
      recentTrend: context.trend || 'stable',
      consensusLevel: context.consensus || PHI_INV_2,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Confidence calculation
  // ═══════════════════════════════════════════════════════════════════════════

  _calculateConfidence(factors, strategy) {
    let confidence = PHI_INV; // Start at max

    const minObs = strategy.minObservations || 3;

    // Low observation count → lower confidence (pattern-aware threshold)
    if (factors.observationCount < minObs) {
      confidence *= 0.5 + 0.5 * (factors.observationCount / minObs);
    } else if (factors.observationCount < minObs * 2) {
      confidence *= 0.8;
    }

    // Low consensus → lower confidence
    if (factors.consensusLevel < PHI_INV_3) confidence *= 0.7;

    // Falling trend → slightly lower confidence
    if (factors.recentTrend === 'falling') confidence *= 0.9;

    // Apply calibration from past outcomes
    confidence = this._applyCalibration(confidence);

    return Math.min(PHI_INV, Math.round(confidence * 1000) / 1000);
  }

  /**
   * Adjust confidence based on past outcome accuracy.
   */
  _applyCalibration(baseConfidence) {
    if (this._outcomes.length < 5) return baseConfidence;

    const successRate = this._outcomes.filter(o => o.result === 'success').length / this._outcomes.length;
    const gap = successRate - baseConfidence;

    // Clamp adjustment to [-0.1, +0.1], scale by φ⁻¹
    const adjustment = Math.max(-0.1, Math.min(0.1, gap * PHI_INV));
    return Math.max(0, Math.min(PHI_INV, baseConfidence + adjustment));
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Decision logic (pattern-aware + stability-checked)
  // ═══════════════════════════════════════════════════════════════════════════

  _makeDecision(score, verdict, factors, judgmentType, strategy) {
    const minObs = strategy.minObservations || 3;

    // Not enough data — wait
    if (factors.observationCount < minObs) {
      return {
        type: CosmosDecisionType.WAIT,
        reason: `Insufficient data for ${judgmentType} — need ${minObs}, have ${factors.observationCount}.`,
        severity: 'low',
      };
    }

    // Use weighted score for decisions instead of raw score
    const effectiveScore = factors.weightedScore;

    // Very low with falling trend — intervene (cooldown-protected)
    if (effectiveScore < DECISION_THRESHOLDS.intervene && factors.recentTrend === 'falling') {
      const now = Date.now();
      if (now - this._lastInterveneTime >= INTERVENE_COOLDOWN_MS) {
        this._lastInterveneTime = now;
        this._stats.goalViolations++;
        return {
          type: CosmosDecisionType.INTERVENE,
          reason: `Ecosystem ${judgmentType} weighted score ${effectiveScore.toFixed(1)} with falling trend.`,
          severity: 'critical',
        };
      }
      // On cooldown — decelerate instead
      return {
        type: CosmosDecisionType.DECELERATE,
        reason: `Intervention cooldown active. Decelerating for ${judgmentType} (${effectiveScore.toFixed(1)}).`,
        severity: 'high',
      };
    }

    // BARK
    if (verdict === 'BARK') {
      this._stats.goalViolations++;
      return {
        type: CosmosDecisionType.DECELERATE,
        reason: `Low ecosystem ${judgmentType} (${score}). Reduce activity and stabilize.`,
        severity: 'high',
      };
    }

    // GROWL — domain-aware: pick FOCUS vs DIVERSIFY based on weakest factor
    if (verdict === 'GROWL') {
      const weakest = this._findWeakestFactor(factors);
      if (weakest === 'sustainability') {
        return this._stabilityCheck(CosmosDecisionType.FOCUS, judgmentType,
          `Sustainability (${factors.sustainability}) weakest factor. Focus on key repos.`, 'medium');
      }
      return this._stabilityCheck(CosmosDecisionType.DIVERSIFY, judgmentType,
        `Utility (${factors.utility}) weakest factor. Expand to underserved areas.`, 'medium');
    }

    // WAG — maintain
    if (verdict === 'WAG') {
      return this._stabilityCheck(CosmosDecisionType.MAINTAIN, judgmentType,
        `Ecosystem ${judgmentType} healthy (${score}). Maintain current pace.`, 'low');
    }

    // HOWL — accelerate
    return this._stabilityCheck(CosmosDecisionType.ACCELERATE, judgmentType,
      `Ecosystem ${judgmentType} thriving (${score}). Conditions favorable for acceleration.`, 'low');
  }

  /**
   * Find the weakest of the three domain factors.
   */
  _findWeakestFactor(factors) {
    const { coherence, utility, sustainability } = factors;
    if (sustainability <= utility && sustainability <= coherence) return 'sustainability';
    if (utility <= coherence) return 'utility';
    return 'coherence';
  }

  /**
   * Stability check: avoid flip-flopping without sufficient new data.
   * If we'd reverse a previous decision for the same judgmentType,
   * require STABILITY_THRESHOLD new observations before allowing it.
   */
  _stabilityCheck(proposedType, judgmentType, reason, severity) {
    const lastSameType = this._findLastDecisionFor(judgmentType);

    if (lastSameType && lastSameType.decision !== proposedType) {
      // We'd reverse. Check if we have enough new observations.
      const observationsSince = this._history.length - this._history.indexOf(lastSameType);
      if (observationsSince < STABILITY_THRESHOLD) {
        this._stats.stabilityHolds++;
        return {
          type: lastSameType.decision,
          reason: `Holding ${lastSameType.decision} — only ${observationsSince} observations since last decision. Need ${STABILITY_THRESHOLD}.`,
          severity: lastSameType.severity || severity,
        };
      }
    }

    return { type: proposedType, reason, severity };
  }

  /**
   * Find the most recent decision for a given judgmentType.
   */
  _findLastDecisionFor(judgmentType) {
    for (let i = this._history.length - 1; i >= 0; i--) {
      if (this._history[i].judgmentType === judgmentType) return this._history[i];
    }
    return null;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Stats
  // ═══════════════════════════════════════════════════════════════════════════

  _updateStats(result) {
    this._stats.totalDecisions++;
    this._stats.byType[result.decision] = (this._stats.byType[result.decision] || 0) + 1;
    this._stats.lastDecision = Date.now();
  }

  getStats() {
    return {
      ...this._stats,
      historySize: this._history.length,
      outcomesSize: this._outcomes.length,
      calibration: this._getCalibrationSummary(),
    };
  }

  getHistory(limit = 21) {
    return this._history.slice(-limit);
  }

  getHealth() {
    const total = this._stats.totalDecisions;
    const interventionRate = total > 0
      ? (this._stats.byType[CosmosDecisionType.INTERVENE] || 0) / total
      : 0;

    return {
      status: this._stats.goalViolations > 3 ? 'stressed'
        : interventionRate > PHI_INV_3 ? 'high_intervention'
          : 'healthy',
      score: Math.min(PHI_INV, 0.5 - this._stats.goalViolations * 0.02),
      totalDecisions: total,
      interventionRate,
      stabilityHolds: this._stats.stabilityHolds,
      goalViolations: this._stats.goalViolations,
    };
  }

  _getCalibrationSummary() {
    const summary = {};
    for (const type of Object.values(CosmosDecisionType)) {
      const relevant = this._outcomes.filter(o => o.decisionType === type);
      if (relevant.length === 0) continue;
      summary[type] = {
        outcomes: relevant.length,
        successRate: relevant.filter(o => o.result === 'success').length / relevant.length,
      };
    }
    return summary;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

export function getCosmosDecider(options = {}) {
  if (!_instance) _instance = new CosmosDecider(options);
  return _instance;
}

export function resetCosmosDecider() {
  if (_instance) _instance.removeAllListeners();
  _instance = null;
}

export default { CosmosDecider, CosmosDecisionType, getCosmosDecider, resetCosmosDecider };
