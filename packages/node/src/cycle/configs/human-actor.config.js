/**
 * HumanActor Config — C5.4 (HUMAN x ACT)
 *
 * Domain-specific configuration for the Human Actor.
 * Template logic lives in create-actor.js.
 *
 * @module @cynic/node/cycle/configs/human-actor.config
 */

'use strict';

export const HumanActionType = {
  BREAK_REMINDER: 'break_reminder',
  PACE_SUGGESTION: 'pace_suggestion',
  CELEBRATION: 'celebration',
  COMPLEXITY_WARNING: 'complexity_warning',
  SESSION_SUMMARY: 'session_summary',
  HYDRATION_REMINDER: 'hydration_reminder',
  FOCUS_NUDGE: 'focus_nudge',
};

export const humanActorConfig = {
  name: 'HumanActor',
  cell: 'C5.4',
  dimension: 'HUMAN',
  eventPrefix: 'human',
  actionTypes: HumanActionType,
  maxHistory: 89, // Fib(11)

  cooldowns: {
    [HumanActionType.BREAK_REMINDER]: 45 * 60000,
    [HumanActionType.PACE_SUGGESTION]: 15 * 60000,
    [HumanActionType.CELEBRATION]: 5 * 60000,
    [HumanActionType.COMPLEXITY_WARNING]: 10 * 60000,
    [HumanActionType.SESSION_SUMMARY]: 30 * 60000,
    [HumanActionType.HYDRATION_REMINDER]: 60 * 60000,
    [HumanActionType.FOCUS_NUDGE]: 20 * 60000,
  },

  extraStatFields: ['dismissed', 'acknowledged'],

  mapDecisionToAction(decision) {
    const map = {
      'BREAK': HumanActionType.BREAK_REMINDER,
      'PAUSE': HumanActionType.BREAK_REMINDER,
      'SIMPLIFY': HumanActionType.COMPLEXITY_WARNING,
      'PACE_DOWN': HumanActionType.PACE_SUGGESTION,
      'REFOCUS': HumanActionType.FOCUS_NUDGE,
      'CELEBRATE': HumanActionType.CELEBRATION,
      'HYDRATE': HumanActionType.HYDRATION_REMINDER,
      'STRETCH': HumanActionType.BREAK_REMINDER,
      'CONTEXT_SWITCH': HumanActionType.FOCUS_NUDGE,
    };
    return map[decision.type] || HumanActionType.PACE_SUGGESTION;
  },

  assessUrgency(decision) {
    return decision.urgency || 'low';
  },

  composeMessage(actionType, decision, context) {
    switch (actionType) {
      case HumanActionType.BREAK_REMINDER:
        return decision.urgency === 'high'
          ? '*yawn* Time to step away. Your energy is low — even 5 minutes helps.'
          : '*stretch* Consider a short break when you reach a stopping point.';
      case HumanActionType.PACE_SUGGESTION:
        return '*sniff* You\'re moving fast. Slower is sometimes faster — fewer bugs, less rework.';
      case HumanActionType.CELEBRATION:
        return `*tail wag* ${decision.reason || decision.reasoning || 'Nice work.'} Keep that momentum.`;
      case HumanActionType.COMPLEXITY_WARNING:
        return `*head tilt* Cognitive load is high (${context.cognitiveLoad || '?'} items). Consider closing some files or simplifying scope.`;
      case HumanActionType.HYDRATION_REMINDER:
        return '*sniff* Hydration check. Water helps cognition more than caffeine.';
      case HumanActionType.FOCUS_NUDGE:
        return '*ears perk* Focus seems scattered. Pick one thing, finish it, then move on.';
      case HumanActionType.SESSION_SUMMARY: {
        const mins = Math.round(context.sessionMinutes || 0);
        return `*yawn* Session: ${mins}min. Energy: ${((context.energy || 0) * 100).toFixed(0)}%. ${mins > 120 ? 'Consider wrapping up soon.' : 'Pace is sustainable.'}`;
      }
      default:
        return '*sniff* Check in with yourself.';
    }
  },

  updateExtraStats(stats, result) {
    // dismissed/acknowledged tracked via recordResponse override
  },

  healthMetric: 'dismissed',
  healthStatusBad: 'over_intervening',
};
