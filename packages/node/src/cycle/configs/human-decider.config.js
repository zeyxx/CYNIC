/**
 * HumanDecider Config — C5.3 (HUMAN × DECIDE)
 *
 * Domain-specific configuration for the Human Decider.
 * Template logic lives in create-decider.js.
 *
 * Converts HumanJudge verdicts into intervention decisions.
 * Incorporates circadian awareness, φ-aligned thresholds, cooldown protection.
 *
 * "Le chien décide pour le bien du maître" - κυνικός
 *
 * @module @cynic/node/cycle/configs/human-decider.config
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

export const HumanDecisionType = {
  INTERVENE: 'intervene',      // High urgency intervention (break, pause)
  WARN: 'warn',                // Medium urgency warning (pace down, complexity)
  NUDGE: 'nudge',              // Low urgency nudge (refocus, hydrate)
  CELEBRATE: 'celebrate',      // Positive reinforcement
  HOLD: 'hold',                // No action needed
};

/**
 * Circadian phases (affects energy expectations)
 */
const CircadianPhase = {
  MORNING_PEAK: 'morning_peak',         // 9-11 AM - high energy expected
  MIDDAY_DIP: 'midday_dip',             // 1-3 PM - lower energy normal
  AFTERNOON_RECOVERY: 'afternoon_recovery', // 3-5 PM - second wind
  EVENING_DECLINE: 'evening_decline',   // 5-7 PM - natural decline
  NIGHT_LOW: 'night_low',               // 10 PM - 6 AM - rest period
};

/**
 * Intervention cooldowns (milliseconds) - φ-aligned
 */
const COOLDOWNS = {
  [HumanDecisionType.INTERVENE]: 45 * 60000,   // 45 min
  [HumanDecisionType.WARN]: 20 * 60000,        // 20 min
  [HumanDecisionType.NUDGE]: 15 * 60000,       // 15 min
  [HumanDecisionType.CELEBRATE]: 30 * 60000,   // 30 min
};

/**
 * Thresholds (φ-aligned, on 0-100 scale)
 */
const THRESHOLDS = {
  // Verdict-based
  verdictCritical: 23.6,    // CRITICAL verdict (< PHI_INV_3 * 100)
  verdictStrained: 38.2,    // STRAINED verdict (< PHI_INV_2 * 100)
  verdictSteady: 61.8,      // STEADY verdict (< PHI_INV * 100)

  // Factor-based
  burnoutRiskHigh: 0.5,     // 50% burnout risk
  burnoutRiskMedium: 0.3,   // 30% burnout risk
  frustrationHigh: 0.6,     // 60% frustration
  cognitiveOverload: 7,     // Miller's Law upper bound
  sessionLongHours: 2,      // 2 hours session
  sessionVeryLongHours: 4,  // 4 hours session
};

export const humanDeciderConfig = {
  name: 'HumanDecider',
  cell: 'C5.3',
  dimension: 'HUMAN',
  eventPrefix: 'human',
  decisionTypes: HumanDecisionType,
  maxHistory: 89, // Fib(11)
  extraStatFields: ['interventionsByUrgency', 'cooldownHits'],
  calibrationGroupBy: 'decisionType',
  calibrationClamp: 0.1,

  init(decider) {
    decider._lastInterventionTimes = new Map();
    decider._interventionsByUrgency = { critical: 0, high: 0, medium: 0, low: 0 };
    decider._cooldownHits = 0;
  },

  /**
   * Decide based on a human judgment.
   *
   * @param {Object} judgment - From HumanJudge.judge()
   * @param {Object} [context] - Additional context
   * @param {Object} decider - The decider instance
   * @returns {Object} Decision result
   */
  decide(judgment, context, decider) {
    const score = judgment.score || 0;
    const verdict = judgment.verdict || 'STEADY';
    const scores = judgment.scores || {};

    // Get circadian context
    const circadian = getCircadianPhase();
    const adjustedThresholds = adjustForCircadian(circadian, THRESHOLDS);

    // Extract factors
    const factors = extractFactors(judgment, scores, context, adjustedThresholds);

    // Calculate confidence
    const confidence = calculateConfidence(factors, judgment, decider);

    // Make decision
    const decision = makeDecision(verdict, factors, adjustedThresholds, circadian, decider);

    const result = {
      decision: decision.type,
      reason: decision.reason,
      urgency: decision.urgency || 'low',
      severity: decision.severity || 'medium',
      confidence,
      score,
      verdict,
      factors,
      interventionType: decision.interventionType || null,
      circadian,
    };

    return decider.recordDecision(result);
  },

  updateExtraStats(stats, result) {
    // Track interventions by urgency
    const urgency = result.urgency || 'low';
    if (stats.interventionsByUrgency) {
      stats.interventionsByUrgency[urgency] = (stats.interventionsByUrgency[urgency] || 0) + 1;
    }
  },

  getHealth(decider) {
    const total = decider._stats.decisionsTotal;
    const interventionRate = total > 0
      ? ((decider._stats.byType[HumanDecisionType.INTERVENE] || 0) +
         (decider._stats.byType[HumanDecisionType.WARN] || 0)) / total
      : 0;

    const cooldownHitRate = total > 0
      ? (decider._cooldownHits || 0) / total
      : 0;

    return {
      status: interventionRate > 0.5 ? 'over_intervening'
        : interventionRate > PHI_INV_3 ? 'active'
          : 'healthy',
      score: Math.min(PHI_INV, 0.6 - interventionRate * 0.3),
      totalDecisions: total,
      interventionRate,
      cooldownHitRate,
      interventionsByUrgency: decider._interventionsByUrgency || {},
    };
  },
};

// =============================================================================
// Decision logic (extracted from HumanAdvisor, adapted for factory pattern)
// =============================================================================

function getCircadianPhase() {
  const hour = new Date().getHours();

  if (hour >= 9 && hour < 11) return CircadianPhase.MORNING_PEAK;
  if (hour >= 13 && hour < 15) return CircadianPhase.MIDDAY_DIP;
  if (hour >= 15 && hour < 17) return CircadianPhase.AFTERNOON_RECOVERY;
  if (hour >= 17 && hour < 19) return CircadianPhase.EVENING_DECLINE;
  if (hour >= 22 || hour < 6) return CircadianPhase.NIGHT_LOW;

  return CircadianPhase.MORNING_PEAK; // Default
}

function adjustForCircadian(phase, thresholds) {
  const adjusted = { ...thresholds };

  switch (phase) {
    case CircadianPhase.MIDDAY_DIP:
      // Lower energy is expected, be more tolerant
      adjusted.verdictStrained *= 0.9;
      adjusted.verdictCritical *= 0.9;
      break;
    case CircadianPhase.NIGHT_LOW:
      // Much lower energy expected, but also warn more about long sessions
      adjusted.verdictStrained *= 0.8;
      adjusted.sessionLongHours *= 0.5; // 1 hour at night = 2 hours during day
      break;
    case CircadianPhase.MORNING_PEAK:
      // Higher expectations during peak
      adjusted.verdictSteady *= 1.1;
      break;
  }

  return adjusted;
}

function extractFactors(judgment, scores, context, thresholds) {
  const burnoutRisk = scores.burnoutRisk || 0;
  const wellbeing = scores.wellbeing / 100 || 0;
  const productivity = scores.productivity / 100 || 0;
  const engagement = scores.engagement / 100 || 0;

  // Extract from original perception data (if available)
  const energy = context.energy || wellbeing;
  const frustration = context.frustration || (1 - wellbeing);
  const cognitiveLoad = context.cognitiveLoad || 5;
  const sessionMinutes = context.sessionMinutes || 0;

  return {
    burnoutRisk,
    wellbeing,
    productivity,
    engagement,
    energy,
    frustration,
    cognitiveLoad,
    sessionHours: sessionMinutes / 60,
    isHighBurnoutRisk: burnoutRisk >= thresholds.burnoutRiskHigh,
    isMediumBurnoutRisk: burnoutRisk >= thresholds.burnoutRiskMedium,
    isHighFrustration: frustration >= thresholds.frustrationHigh,
    isCognitiveOverload: cognitiveLoad >= thresholds.cognitiveOverload,
    isLongSession: sessionMinutes / 60 >= thresholds.sessionLongHours,
    isVeryLongSession: sessionMinutes / 60 >= thresholds.sessionVeryLongHours,
  };
}

function calculateConfidence(factors, judgment, decider) {
  let confidence = PHI_INV;

  // Reduce confidence if factors are borderline
  if (factors.burnoutRisk > 0.2 && factors.burnoutRisk < 0.4) {
    confidence *= 0.9; // Medium risk = less certain intervention needed
  }

  // Reduce confidence for long sessions (fatigue affects judgment)
  if (factors.sessionHours > 3) {
    confidence *= 0.85;
  }

  // Apply calibration
  confidence = decider.applyCalibration('default', confidence);

  return Math.min(PHI_INV, Math.round(confidence * 1000) / 1000);
}

function makeDecision(verdict, factors, thresholds, circadian, decider) {
  // Priority order: critical conditions → high risk → warnings → nudges → hold

  // CRITICAL: CRITICAL verdict or very high burnout risk
  if (verdict === 'CRITICAL' || factors.isHighBurnoutRisk) {
    return checkCooldown(
      HumanDecisionType.INTERVENE,
      'BREAK',
      `${verdict === 'CRITICAL' ? 'Critical state' : 'High burnout risk'} (${(factors.burnoutRisk * 100).toFixed(0)}%). Immediate break recommended.`,
      'critical',
      decider
    );
  }

  // HIGH: STRAINED verdict + very long session
  if (verdict === 'STRAINED' && factors.isVeryLongSession) {
    return checkCooldown(
      HumanDecisionType.INTERVENE,
      'BREAK',
      `Strained state after ${factors.sessionHours.toFixed(1)}h session. Time to rest.`,
      'high',
      decider
    );
  }

  // HIGH: High frustration
  if (factors.isHighFrustration) {
    return checkCooldown(
      HumanDecisionType.WARN,
      'PAUSE',
      `Frustration at ${(factors.frustration * 100).toFixed(0)}%. Step away briefly.`,
      'high',
      decider
    );
  }

  // MEDIUM: Cognitive overload
  if (factors.isCognitiveOverload) {
    return checkCooldown(
      HumanDecisionType.WARN,
      'SIMPLIFY',
      `Cognitive load at ${factors.cognitiveLoad}/9. Simplify scope.`,
      'medium',
      decider
    );
  }

  // MEDIUM: STRAINED verdict + medium burnout risk
  if (verdict === 'STRAINED' && factors.isMediumBurnoutRisk) {
    return checkCooldown(
      HumanDecisionType.WARN,
      'PACE_DOWN',
      `Strained with ${(factors.burnoutRisk * 100).toFixed(0)}% burnout risk. Reduce pace.`,
      'medium',
      decider
    );
  }

  // LOW: Long session but not critical
  if (factors.isLongSession && !factors.isVeryLongSession) {
    return checkCooldown(
      HumanDecisionType.NUDGE,
      'BREAK',
      `Session ${factors.sessionHours.toFixed(1)}h. Consider a break soon.`,
      'low',
      decider
    );
  }

  // POSITIVE: THRIVING verdict
  if (verdict === 'THRIVING' && factors.productivity > PHI_INV) {
    return checkCooldown(
      HumanDecisionType.CELEBRATE,
      'CELEBRATE',
      `Thriving state. Keep this sustainable pace.`,
      'low',
      decider
    );
  }

  // DEFAULT: No action needed
  return {
    type: HumanDecisionType.HOLD,
    reason: `State ${verdict}. No intervention needed.`,
    urgency: 'none',
    severity: 'none',
    interventionType: null,
  };
}

function checkCooldown(decisionType, interventionType, reason, urgency, decider) {
  const cooldownMs = COOLDOWNS[decisionType] || 0;
  const lastTime = decider._lastInterventionTimes.get(decisionType) || 0;
  const now = Date.now();

  if (cooldownMs > 0 && now - lastTime < cooldownMs) {
    // Still on cooldown
    decider._cooldownHits++;
    const remainingMin = Math.ceil((cooldownMs - (now - lastTime)) / 60000);
    return {
      type: HumanDecisionType.HOLD,
      reason: `${interventionType} on cooldown (${remainingMin}min remaining). Holding.`,
      urgency: 'none',
      severity: 'none',
      interventionType: null,
      cooldownActive: true,
    };
  }

  // Record intervention time
  decider._lastInterventionTimes.set(decisionType, now);

  // Track by urgency
  if (decider._interventionsByUrgency) {
    decider._interventionsByUrgency[urgency] = (decider._interventionsByUrgency[urgency] || 0) + 1;
  }

  return {
    type: decisionType,
    reason,
    urgency,
    severity: urgency, // Map urgency to severity for consistency
    interventionType,
  };
}
