/**
 * SocialActor Config — C4.4 (SOCIAL x ACT)
 *
 * Domain-specific configuration for the Social Actor.
 * Template logic lives in create-actor.js.
 *
 * Executes advisory social actions from SocialDecider decisions.
 * Advisory only — no autonomous posting/replying.
 *
 * @module @cynic/node/cycle/configs/social-actor.config
 */

'use strict';

export const SocialActionType = {
  BOOST_ALERT: 'boost_alert',
  ENGAGEMENT_NUDGE: 'engagement_nudge',
  SENTIMENT_ALERT: 'sentiment_alert',
  LOG_INSIGHT: 'log_insight',
  STRATEGY_ADJUST: 'strategy_adjust',
  INVESTIGATE_ALERT: 'investigate_alert',
};

export const socialActorConfig = {
  name: 'SocialActor',
  cell: 'C4.4',
  dimension: 'SOCIAL',
  eventPrefix: 'social',
  actionTypes: SocialActionType,
  maxHistory: 89, // Fib(11)

  cooldowns: {
    [SocialActionType.BOOST_ALERT]: 21 * 60000,         // F8 = 21 min
    [SocialActionType.ENGAGEMENT_NUDGE]: 34 * 60000,    // F9 = 34 min
    [SocialActionType.SENTIMENT_ALERT]: 13 * 60000,     // F7 = 13 min
    [SocialActionType.LOG_INSIGHT]: 8 * 60000,           // F6 = 8 min
    [SocialActionType.STRATEGY_ADJUST]: 55 * 60000,     // F10 = 55 min
    [SocialActionType.INVESTIGATE_ALERT]: 13 * 60000,   // F7 = 13 min
  },

  extraStatFields: ['alertsRaised', 'insightsLogged'],

  mapDecisionToAction(decision) {
    const type = decision.decision || decision.type;
    const map = {
      'amplify': SocialActionType.BOOST_ALERT,
      'engage': SocialActionType.ENGAGEMENT_NUDGE,
      'refocus': SocialActionType.SENTIMENT_ALERT,
      'investigate': SocialActionType.INVESTIGATE_ALERT,
      'maintain': SocialActionType.LOG_INSIGHT,
      'wait': null, // No action on wait
    };
    return map[type] ?? SocialActionType.LOG_INSIGHT;
  },

  assessUrgency(decision) {
    const type = decision.decision || decision.type;
    if (type === 'investigate') return 'critical';
    if (type === 'refocus') return 'high';
    if (type === 'engage') return 'medium';
    return 'low';
  },

  composeMessage(actionType, decision) {
    const reason = decision.reason || '';
    switch (actionType) {
      case SocialActionType.BOOST_ALERT:
        return `*tail wag* Social momentum strong. ${reason}`;
      case SocialActionType.ENGAGEMENT_NUDGE:
        return `*sniff* Engagement is low. ${reason}`;
      case SocialActionType.SENTIMENT_ALERT:
        return `*GROWL* Sentiment shift detected. ${reason}`;
      case SocialActionType.LOG_INSIGHT:
        return `*ears perk* Social insight: ${reason}`;
      case SocialActionType.STRATEGY_ADJUST:
        return `*head tilt* Social strategy adjustment. ${reason}`;
      case SocialActionType.INVESTIGATE_ALERT:
        return `*GROWL* Social health alert — investigation needed. ${reason}`;
      default:
        return `*sniff* Social action: ${actionType}`;
    }
  },

  updateExtraStats(stats, result) {
    if (result.type === SocialActionType.INVESTIGATE_ALERT ||
        result.type === SocialActionType.SENTIMENT_ALERT) {
      stats.alertsRaised++;
    }
    if (result.type === SocialActionType.LOG_INSIGHT) {
      stats.insightsLogged++;
    }
  },

  healthMetric: 'alertsRaised',
  healthStatusBad: 'high_alert_social',
  healthExtraFields: {
    insightsLogged: 'insightsLogged',
  },
};
