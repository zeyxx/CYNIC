/**
 * CosmosActor Config — C7.4 (COSMOS x ACT)
 *
 * Domain-specific configuration for the Cosmos Actor.
 * Template logic lives in create-actor.js.
 *
 * @module @cynic/node/cycle/configs/cosmos-actor.config
 */

'use strict';

export const CosmosActionType = {
  SIGNAL_ALERT: 'signal_alert',
  DOCUMENT_PATTERN: 'document_pattern',
  ADJUST_FOCUS: 'adjust_focus',
  SCHEDULE_REVIEW: 'schedule_review',
  LOG_INSIGHT: 'log_insight',
  RECOMMEND_DIVERSIFY: 'recommend_diversify',
};

export const cosmosActorConfig = {
  name: 'CosmosActor',
  cell: 'C7.4',
  dimension: 'COSMOS',
  eventPrefix: 'cosmos',
  actionTypes: CosmosActionType,
  maxHistory: 144, // Fib(12)

  cooldowns: {
    [CosmosActionType.SIGNAL_ALERT]: 34 * 60000,
    [CosmosActionType.DOCUMENT_PATTERN]: 21 * 60000,
    [CosmosActionType.ADJUST_FOCUS]: 13 * 60000,
    [CosmosActionType.SCHEDULE_REVIEW]: 89 * 60000,
    [CosmosActionType.LOG_INSIGHT]: 8 * 60000,
    [CosmosActionType.RECOMMEND_DIVERSIFY]: 55 * 60000,
  },

  extraStatFields: ['alertsRaised', 'insightsLogged'],

  mapDecisionToAction(decision) {
    const type = decision.decision || decision.type;
    const map = {
      'intervene': CosmosActionType.SIGNAL_ALERT,
      'decelerate': CosmosActionType.SIGNAL_ALERT,
      'focus': CosmosActionType.ADJUST_FOCUS,
      'diversify': CosmosActionType.RECOMMEND_DIVERSIFY,
      'accelerate': CosmosActionType.LOG_INSIGHT,
      'maintain': CosmosActionType.LOG_INSIGHT,
      'wait': null,
    };
    return map[type] ?? CosmosActionType.LOG_INSIGHT;
  },

  assessUrgency(decision) {
    const type = decision.decision || decision.type;
    if (type === 'intervene') return 'critical';
    if (type === 'decelerate' || type === 'focus') return 'high';
    if (type === 'diversify') return 'medium';
    return 'low';
  },

  composeMessage(actionType, decision) {
    const reason = decision.reason || '';
    switch (actionType) {
      case CosmosActionType.SIGNAL_ALERT:
        return `*GROWL* Ecosystem alert. ${reason}`;
      case CosmosActionType.DOCUMENT_PATTERN:
        return `*sniff* Ecosystem pattern documented. ${reason}`;
      case CosmosActionType.ADJUST_FOCUS:
        return `*head tilt* Recommending focus shift. ${reason}`;
      case CosmosActionType.SCHEDULE_REVIEW:
        return `*yawn* Ecosystem review recommended. ${reason}`;
      case CosmosActionType.LOG_INSIGHT:
        return `*ears perk* Ecosystem insight: ${reason}`;
      case CosmosActionType.RECOMMEND_DIVERSIFY:
        return `*tail wag* Diversification opportunity. ${reason}`;
      default:
        return `*sniff* Cosmos action: ${actionType}`;
    }
  },

  updateExtraStats(stats, result) {
    if (result.type === CosmosActionType.SIGNAL_ALERT) stats.alertsRaised++;
    if (result.type === CosmosActionType.LOG_INSIGHT) stats.insightsLogged++;
  },

  healthMetric: 'alertsRaised',
  healthStatusBad: 'high_alert_ecosystem',
  healthExtraFields: {
    insightsLogged: 'insightsLogged',
  },
};
