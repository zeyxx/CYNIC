/**
 * CodeActor Config — C1.4 (CODE x ACT)
 *
 * Domain-specific configuration for the Code Actor.
 * Template logic lives in create-actor.js.
 *
 * @module @cynic/node/cycle/configs/code-actor.config
 */

'use strict';

export const CodeActionType = {
  APPROVE_COMMIT: 'approve_commit',
  FLAG_REVIEW: 'flag_review',
  SUGGEST_TESTS: 'suggest_tests',
  SUGGEST_REFACTOR: 'suggest_refactor',
  BLOCK_ALERT: 'block_alert',
  LOG_DEBT: 'log_debt',
};

export const codeActorConfig = {
  name: 'CodeActor',
  cell: 'C1.4',
  dimension: 'CODE',
  eventPrefix: 'code',
  actionTypes: CodeActionType,
  maxHistory: 233, // Fib(13)

  cooldowns: {
    [CodeActionType.APPROVE_COMMIT]: 0,
    [CodeActionType.FLAG_REVIEW]: 5 * 60000,
    [CodeActionType.SUGGEST_TESTS]: 13 * 60000,
    [CodeActionType.SUGGEST_REFACTOR]: 21 * 60000,
    [CodeActionType.BLOCK_ALERT]: 0,
    [CodeActionType.LOG_DEBT]: 8 * 60000,
  },

  extraStatFields: ['blocksRaised', 'reviewsFlagged', 'debtsLogged'],

  // Domain init: debt tracking
  init(actor) {
    actor._debtLog = [];
    actor._maxDebt = 89; // Fib(11)
  },

  mapDecisionToAction(decision) {
    const type = decision.type || decision.decision;
    const map = {
      'approve': CodeActionType.APPROVE_COMMIT,
      'approve_commit': CodeActionType.APPROVE_COMMIT,
      'queue_review': CodeActionType.FLAG_REVIEW,
      'require_tests': CodeActionType.SUGGEST_TESTS,
      'require_refactor': CodeActionType.SUGGEST_REFACTOR,
      'block': CodeActionType.BLOCK_ALERT,
      'defer': CodeActionType.LOG_DEBT,
    };
    return map[type] || CodeActionType.LOG_DEBT;
  },

  assessUrgency(decision) {
    const type = decision.type || decision.decision;
    if (type === 'block') return 'critical';
    if (type === 'require_tests' || type === 'require_refactor') return 'high';
    if (type === 'queue_review') return 'medium';
    return 'low';
  },

  composeMessage(actionType, decision, context) {
    const qScore = context.qScore ? ` (Q:${context.qScore})` : '';
    switch (actionType) {
      case CodeActionType.APPROVE_COMMIT:
        return `*tail wag* Code approved${qScore}. ${decision.reasoning || 'φ permits.'}`;
      case CodeActionType.FLAG_REVIEW:
        return `*sniff* Code needs review${qScore}. ${decision.reasoning || 'Confidence below φ⁻¹.'}`;
      case CodeActionType.SUGGEST_TESTS:
        return `*ears perk* Tests needed${qScore}. ${decision.reasoning || 'Non-trivial change without coverage.'}`;
      case CodeActionType.SUGGEST_REFACTOR:
        return `*head tilt* Consider refactoring${qScore}. ${decision.reasoning || 'Complexity warrants simplification.'}`;
      case CodeActionType.BLOCK_ALERT:
        return `*GROWL* Code BLOCKED${qScore}. ${decision.reasoning || 'Risk exceeds φ threshold.'}`;
      case CodeActionType.LOG_DEBT:
        return `*sniff* Technical debt noted${qScore}. ${decision.reasoning || 'Deferred for now.'}`;
      default:
        return `*sniff* Code action: ${actionType}${qScore}`;
    }
  },

  updateExtraStats(stats, result) {
    if (result.type === CodeActionType.BLOCK_ALERT) stats.blocksRaised++;
    if (result.type === CodeActionType.FLAG_REVIEW) stats.reviewsFlagged++;
  },

  postAct(result, decision, context, actor) {
    // Track technical debt
    if ([CodeActionType.LOG_DEBT, CodeActionType.SUGGEST_REFACTOR, CodeActionType.SUGGEST_TESTS].includes(result.type)) {
      actor._debtLog.push({
        type: result.type,
        reasoning: decision.reasoning || decision.reason,
        risk: decision.risk,
        judgmentId: context.judgmentId,
        timestamp: Date.now(),
      });
      while (actor._debtLog.length > actor._maxDebt) actor._debtLog.shift();
      actor._stats.debtsLogged++;
    }
  },

  healthMetric: 'blocksRaised',
  healthThreshold: undefined, // defaults to PHI_INV_2
  healthStatusBad: 'high_risk_codebase',
  healthExtraFields: {
    debtCount: null, // handled by custom getHealth override
    reviewsFlagged: 'reviewsFlagged',
  },
};
