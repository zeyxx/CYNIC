/**
 * SocialDecider Config — C4.3 (SOCIAL x DECIDE)
 *
 * Domain-specific configuration for the Social Decider.
 * Template logic lives in create-decider.js.
 *
 * Decides social actions based on social:judgment events.
 *
 * @module @cynic/node/cycle/configs/social-decider.config
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

export const SocialDecisionType = {
  AMPLIFY: 'amplify',
  MAINTAIN: 'maintain',
  ENGAGE: 'engage',
  INVESTIGATE: 'investigate',
  REFOCUS: 'refocus',
  WAIT: 'wait',
};

const MIN_JUDGMENTS = 3;
const SENTIMENT_THRESHOLD = -0.2; // below this → refocus
const ENGAGEMENT_LOW = 10;        // total engagement below this → engage

export const socialDeciderConfig = {
  name: 'SocialDecider',
  cell: 'C4.3',
  dimension: 'SOCIAL',
  eventPrefix: 'social',
  decisionTypes: SocialDecisionType,
  maxHistory: 89, // Fib(11)
  extraStatFields: ['goalViolations', 'refocusCount'],
  calibrationGroupBy: 'decisionType',
  calibrationClamp: 0.1,

  init(decider) {
    decider._judgmentCount = 0;
  },

  /**
   * Decide based on a social judgment.
   *
   * @param {Object} judgment - From social:judgment event
   * @param {Object} [context] - Additional context
   * @param {Object} decider - The decider instance
   * @returns {Object} Decision result
   */
  decide(judgment, context, decider) {
    const score = judgment.score || 0;
    const verdict = judgment.verdict || 'BARK';
    const sentiment = judgment.avgSentiment ?? 0;
    const engagement = judgment.totalEngagement ?? 0;
    const reach = judgment.totalReach ?? 0;

    decider._judgmentCount++;

    // Wait until we have enough data
    if (decider._judgmentCount < MIN_JUDGMENTS) {
      return decider.recordDecision({
        decision: SocialDecisionType.WAIT,
        reason: `Insufficient social data — ${decider._judgmentCount}/${MIN_JUDGMENTS} judgments.`,
        severity: 'low',
        confidence: PHI_INV_3,
        score,
        verdict,
        sentiment,
      });
    }

    const confidence = calculateConfidence(score, engagement, reach, decider);

    // Negative sentiment → refocus
    if (sentiment < SENTIMENT_THRESHOLD && verdict !== 'HOWL') {
      decider._stats.refocusCount++;
      return decider.recordDecision({
        decision: SocialDecisionType.REFOCUS,
        reason: `Negative sentiment (${sentiment.toFixed(2)}) — consider adjusting messaging.`,
        severity: 'high',
        confidence,
        score,
        verdict,
        sentiment,
      });
    }

    // Very low score → investigate
    if (verdict === 'BARK') {
      decider._stats.goalViolations++;
      return decider.recordDecision({
        decision: SocialDecisionType.INVESTIGATE,
        reason: `Low social health (${score.toFixed(1)}) — investigate engagement drop.`,
        severity: 'high',
        confidence,
        score,
        verdict,
        sentiment,
      });
    }

    // Low engagement → actively engage
    if (engagement < ENGAGEMENT_LOW && verdict !== 'HOWL') {
      return decider.recordDecision({
        decision: SocialDecisionType.ENGAGE,
        reason: `Low engagement (${engagement}) — increase social activity.`,
        severity: 'medium',
        confidence,
        score,
        verdict,
        sentiment,
      });
    }

    // High score → amplify
    if (verdict === 'HOWL') {
      return decider.recordDecision({
        decision: SocialDecisionType.AMPLIFY,
        reason: `Strong social health (${score.toFixed(1)}) — amplify key content.`,
        severity: 'low',
        confidence,
        score,
        verdict,
        sentiment,
      });
    }

    // Default → maintain
    return decider.recordDecision({
      decision: SocialDecisionType.MAINTAIN,
      reason: `Social health moderate (${score.toFixed(1)}) — maintain current strategy.`,
      severity: 'low',
      confidence,
      score,
      verdict,
      sentiment,
    });
  },

  updateExtraStats(stats, result) {
    // goalViolations and refocusCount tracked in decide()
  },

  getHealth(decider) {
    const total = decider._stats.decisionsTotal;
    const investigateRate = total > 0
      ? (decider._stats.byType[SocialDecisionType.INVESTIGATE] || 0) / total
      : 0;

    return {
      status: decider._stats.goalViolations > 3 ? 'stressed'
        : investigateRate > PHI_INV_3 ? 'high_investigation'
          : 'healthy',
      score: Math.min(PHI_INV, 0.5 - (decider._stats.goalViolations || 0) * 0.02),
      totalDecisions: total,
      investigateRate,
      refocusCount: decider._stats.refocusCount || 0,
      goalViolations: decider._stats.goalViolations || 0,
    };
  },
};

// =============================================================================
// Helpers
// =============================================================================

function calculateConfidence(score, engagement, reach, decider) {
  let confidence = PHI_INV;

  // Scale with data volume
  if (engagement < 5) confidence *= 0.5;
  else if (engagement < 20) confidence *= 0.7;
  else if (engagement < 50) confidence *= 0.85;

  // Low reach = low confidence
  if (reach < 100) confidence *= 0.7;

  // Apply calibration
  confidence = decider.applyCalibration('default', confidence);

  return Math.min(PHI_INV, Math.round(confidence * 1000) / 1000);
}
