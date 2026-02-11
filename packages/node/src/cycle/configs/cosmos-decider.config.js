/**
 * CosmosDecider Config — C7.3 (COSMOS x DECIDE)
 *
 * Domain-specific configuration for the Cosmos Decider.
 * Template logic lives in create-decider.js.
 *
 * @module @cynic/node/cycle/configs/cosmos-decider.config
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

export const CosmosDecisionType = {
  ACCELERATE: 'accelerate',
  MAINTAIN: 'maintain',
  DECELERATE: 'decelerate',
  FOCUS: 'focus',
  DIVERSIFY: 'diversify',
  INTERVENE: 'intervene',
  WAIT: 'wait',
};

const DECISION_THRESHOLDS = {
  accelerate: 75,
  maintain: 55,
  decelerate: 40,
  intervene: 30,
};

const PATTERN_STRATEGIES = {
  ecosystem_coherence:  { minObservations: 5,  weight: { coherence: 0.5, utility: 0.2, sustainability: 0.3 } },
  ecosystem_utility:    { minObservations: 3,  weight: { coherence: 0.2, utility: 0.5, sustainability: 0.3 } },
  ecosystem_security:   { minObservations: 2,  weight: { coherence: 0.1, utility: 0.2, sustainability: 0.7 } },
  repo_distribution:    { minObservations: 5,  weight: { coherence: 0.3, utility: 0.4, sustainability: 0.3 } },
  cross_project_health: { minObservations: 8,  weight: { coherence: 0.4, utility: 0.3, sustainability: 0.3 } },
  default:              { minObservations: 3,  weight: { coherence: 0.34, utility: 0.33, sustainability: 0.33 } },
};

const STABILITY_THRESHOLD = 3;
const INTERVENE_COOLDOWN_MS = 21 * 60000; // F8 = 21 min

export const cosmosDeciderConfig = {
  name: 'CosmosDecider',
  cell: 'C7.3',
  dimension: 'COSMOS',
  eventPrefix: 'cosmos',
  decisionTypes: CosmosDecisionType,
  maxHistory: 89, // Fib(11)
  extraStatFields: ['stabilityHolds', 'goalViolations'],
  calibrationGroupBy: 'decisionType',
  calibrationClamp: 0.1,

  init(decider) {
    decider._lastInterveneTime = 0;
  },

  /**
   * Decide based on a cosmos judgment.
   *
   * @param {Object} judgment - From CosmosJudge.judge()
   * @param {Object} [context] - Additional context
   * @param {Object} decider - The decider instance
   * @returns {Object} Decision result
   */
  decide(judgment, context, decider) {
    const score = judgment.score || 0;
    const verdict = judgment.verdict || 'BARK';
    const type = judgment.type || 'unknown';

    const strategy = PATTERN_STRATEGIES[type] || PATTERN_STRATEGIES.default;
    const factors = extractFactors(judgment, context, strategy);
    const confidence = calculateConfidence(factors, strategy, decider);
    const decision = makeDecision(score, verdict, factors, type, strategy, decider);

    const result = {
      decision: decision.type,
      reason: decision.reason,
      severity: decision.severity || 'medium',
      confidence,
      score,
      verdict,
      factors,
      judgmentType: type,
    };

    return decider.recordDecision(result);
  },

  updateExtraStats(stats, result) {
    if (result.decision === CosmosDecisionType.INTERVENE) stats.goalViolations++;
  },

  getHealth(decider) {
    const total = decider._stats.decisionsTotal;
    const interventionRate = total > 0
      ? (decider._stats.byType[CosmosDecisionType.INTERVENE] || 0) / total
      : 0;

    return {
      status: decider._stats.goalViolations > 3 ? 'stressed'
        : interventionRate > PHI_INV_3 ? 'high_intervention'
          : 'healthy',
      score: Math.min(PHI_INV, 0.5 - (decider._stats.goalViolations || 0) * 0.02),
      totalDecisions: total,
      interventionRate,
      stabilityHolds: decider._stats.stabilityHolds || 0,
      goalViolations: decider._stats.goalViolations || 0,
    };
  },
};

// =============================================================================
// Domain-specific helpers
// =============================================================================

function extractFactors(judgment, context, strategy) {
  const scores = judgment.scores || {};
  const w = strategy.weight;

  const coherence = scores.coherence || 50;
  const utility = scores.utility || 50;
  const sustainability = scores.sustainability || 50;

  return {
    coherence,
    utility,
    sustainability,
    weightedScore: coherence * w.coherence + utility * w.utility + sustainability * w.sustainability,
    observationCount: context.observationCount || 0,
    recentTrend: context.trend || 'stable',
    consensusLevel: context.consensus || PHI_INV_2,
  };
}

function calculateConfidence(factors, strategy, decider) {
  let confidence = PHI_INV;
  const minObs = strategy.minObservations || 3;

  if (factors.observationCount < minObs) {
    confidence *= 0.5 + 0.5 * (factors.observationCount / minObs);
  } else if (factors.observationCount < minObs * 2) {
    confidence *= 0.8;
  }

  if (factors.consensusLevel < PHI_INV_3) confidence *= 0.7;
  if (factors.recentTrend === 'falling') confidence *= 0.9;

  confidence = decider.applyCalibration('default', confidence);

  return Math.min(PHI_INV, Math.round(confidence * 1000) / 1000);
}

function makeDecision(score, verdict, factors, judgmentType, strategy, decider) {
  const minObs = strategy.minObservations || 3;

  if (factors.observationCount < minObs) {
    return {
      type: CosmosDecisionType.WAIT,
      reason: `Insufficient data for ${judgmentType} — need ${minObs}, have ${factors.observationCount}.`,
      severity: 'low',
    };
  }

  const effectiveScore = factors.weightedScore;

  // Very low + falling → intervene (cooldown-protected)
  if (effectiveScore < DECISION_THRESHOLDS.intervene && factors.recentTrend === 'falling') {
    const now = Date.now();
    if (now - decider._lastInterveneTime >= INTERVENE_COOLDOWN_MS) {
      decider._lastInterveneTime = now;
      return {
        type: CosmosDecisionType.INTERVENE,
        reason: `Ecosystem ${judgmentType} weighted score ${effectiveScore.toFixed(1)} with falling trend.`,
        severity: 'critical',
      };
    }
    return {
      type: CosmosDecisionType.DECELERATE,
      reason: `Intervention cooldown active. Decelerating for ${judgmentType} (${effectiveScore.toFixed(1)}).`,
      severity: 'high',
    };
  }

  if (verdict === 'BARK') {
    return {
      type: CosmosDecisionType.DECELERATE,
      reason: `Low ecosystem ${judgmentType} (${score}). Reduce activity and stabilize.`,
      severity: 'high',
    };
  }

  if (verdict === 'GROWL') {
    const weakest = findWeakestFactor(factors);
    if (weakest === 'sustainability') {
      return stabilityCheck(CosmosDecisionType.FOCUS, judgmentType,
        `Sustainability (${factors.sustainability}) weakest factor. Focus on key repos.`, 'medium', decider);
    }
    return stabilityCheck(CosmosDecisionType.DIVERSIFY, judgmentType,
      `Utility (${factors.utility}) weakest factor. Expand to underserved areas.`, 'medium', decider);
  }

  if (verdict === 'WAG') {
    return stabilityCheck(CosmosDecisionType.MAINTAIN, judgmentType,
      `Ecosystem ${judgmentType} healthy (${score}). Maintain current pace.`, 'low', decider);
  }

  return stabilityCheck(CosmosDecisionType.ACCELERATE, judgmentType,
    `Ecosystem ${judgmentType} thriving (${score}). Conditions favorable for acceleration.`, 'low', decider);
}

function findWeakestFactor(factors) {
  const { coherence, utility, sustainability } = factors;
  if (sustainability <= utility && sustainability <= coherence) return 'sustainability';
  if (utility <= coherence) return 'utility';
  return 'coherence';
}

function stabilityCheck(proposedType, judgmentType, reason, severity, decider) {
  const history = decider._history;
  const lastSameType = findLastDecisionFor(history, judgmentType);

  if (lastSameType && lastSameType.decision !== proposedType) {
    const observationsSince = history.length - history.indexOf(lastSameType);
    if (observationsSince < STABILITY_THRESHOLD) {
      decider._stats.stabilityHolds++;
      return {
        type: lastSameType.decision,
        reason: `Holding ${lastSameType.decision} — only ${observationsSince} observations since last decision. Need ${STABILITY_THRESHOLD}.`,
        severity: lastSameType.severity || severity,
      };
    }
  }

  return { type: proposedType, reason, severity };
}

function findLastDecisionFor(history, judgmentType) {
  for (let i = history.length - 1; i >= 0; i--) {
    if (history[i].judgmentType === judgmentType) return history[i];
  }
  return null;
}
