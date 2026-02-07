/**
 * CYNIC Human Actor - C5.4 (HUMAN × ACT)
 *
 * Executes human-facing interventions decided by the HumanAdvisor (C5.3).
 * Bridges decisions into concrete actions: messages, pace changes, celebrations.
 *
 * "Le chien agit — il ne délibère pas éternellement" - κυνικός
 *
 * Actions:
 * - Break reminders (with context-aware timing)
 * - Pace suggestions (slow down / speed up signals)
 * - Celebrations (milestone recognition)
 * - Complexity warnings (cognitive overload alerts)
 * - Session summaries (periodic progress snapshots)
 *
 * @module @cynic/node/symbiosis/human-actor
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('HumanActor');

export const ActionType = {
  BREAK_REMINDER: 'break_reminder',
  PACE_SUGGESTION: 'pace_suggestion',
  CELEBRATION: 'celebration',
  COMPLEXITY_WARNING: 'complexity_warning',
  SESSION_SUMMARY: 'session_summary',
  HYDRATION_REMINDER: 'hydration_reminder',
  FOCUS_NUDGE: 'focus_nudge',
};

export const ActionStatus = {
  QUEUED: 'queued',
  DELIVERED: 'delivered',
  ACKNOWLEDGED: 'acknowledged',
  DISMISSED: 'dismissed',
  EXPIRED: 'expired',
};

// φ-aligned cooldowns (ms)
const COOLDOWNS = {
  [ActionType.BREAK_REMINDER]: 45 * 60000,       // 45 min
  [ActionType.PACE_SUGGESTION]: 15 * 60000,       // 15 min
  [ActionType.CELEBRATION]: 5 * 60000,            // 5 min
  [ActionType.COMPLEXITY_WARNING]: 10 * 60000,    // 10 min
  [ActionType.SESSION_SUMMARY]: 30 * 60000,       // 30 min
  [ActionType.HYDRATION_REMINDER]: 60 * 60000,    // 60 min
  [ActionType.FOCUS_NUDGE]: 20 * 60000,           // 20 min
};

export class HumanActor extends EventEmitter {
  constructor(options = {}) {
    super();

    this._lastAction = new Map(); // type -> timestamp
    this._history = [];
    this._maxHistory = 89; // Fib(11)

    this._stats = {
      actionsTotal: 0,
      byType: {},
      delivered: 0,
      dismissed: 0,
      acknowledged: 0,
      lastAction: null,
    };

    for (const type of Object.values(ActionType)) {
      this._stats.byType[type] = 0;
    }
  }

  /**
   * Execute an intervention
   *
   * @param {Object} intervention - From HumanAdvisor.analyze()
   * @param {Object} [context] - Current human state
   * @returns {Object|null} Action result, or null if cooled down
   */
  act(intervention, context = {}) {
    const actionType = this._mapInterventionToAction(intervention);
    if (!actionType) return null;

    // Check cooldown
    if (this._isOnCooldown(actionType)) {
      log.debug('Action on cooldown', { type: actionType });
      return null;
    }

    const message = this._composeMessage(actionType, intervention, context);

    const result = {
      type: actionType,
      status: ActionStatus.DELIVERED,
      message,
      urgency: intervention.urgency || 'low',
      cell: 'C5.4',
      dimension: 'HUMAN',
      analysis: 'ACT',
      intervention: intervention.type,
      timestamp: Date.now(),
    };

    this._lastAction.set(actionType, Date.now());
    this._updateStats(result);
    this._history.push(result);
    while (this._history.length > this._maxHistory) this._history.shift();

    this.emit('action', result);
    globalEventBus.emit('human:action', result);

    log.debug('Human action delivered', { type: actionType, urgency: result.urgency });

    return result;
  }

  /**
   * Record user response to an action
   */
  recordResponse(actionType, response) {
    const lastIdx = this._history.findLastIndex(a => a.type === actionType);
    if (lastIdx >= 0) {
      this._history[lastIdx].status = response === 'dismiss'
        ? ActionStatus.DISMISSED
        : ActionStatus.ACKNOWLEDGED;

      if (response === 'dismiss') this._stats.dismissed++;
      else this._stats.acknowledged++;
    }
    this.emit('response', { type: actionType, response });
  }

  /**
   * Trigger a celebration
   */
  celebrate(reason, details = {}) {
    return this.act(
      { type: 'CELEBRATE', urgency: 'low', reason, ...details },
      {}
    );
  }

  _mapInterventionToAction(intervention) {
    const map = {
      'BREAK': ActionType.BREAK_REMINDER,
      'PAUSE': ActionType.BREAK_REMINDER,
      'SIMPLIFY': ActionType.COMPLEXITY_WARNING,
      'PACE_DOWN': ActionType.PACE_SUGGESTION,
      'REFOCUS': ActionType.FOCUS_NUDGE,
      'CELEBRATE': ActionType.CELEBRATION,
      'HYDRATE': ActionType.HYDRATION_REMINDER,
      'STRETCH': ActionType.BREAK_REMINDER,
      'CONTEXT_SWITCH': ActionType.FOCUS_NUDGE,
    };
    return map[intervention.type] || ActionType.PACE_SUGGESTION;
  }

  _isOnCooldown(actionType) {
    const last = this._lastAction.get(actionType);
    if (!last) return false;
    const cooldown = COOLDOWNS[actionType] || 10 * 60000;
    return (Date.now() - last) < cooldown;
  }

  _composeMessage(actionType, intervention, context) {
    switch (actionType) {
      case ActionType.BREAK_REMINDER:
        return intervention.urgency === 'high'
          ? '*yawn* Time to step away. Your energy is low — even 5 minutes helps.'
          : '*stretch* Consider a short break when you reach a stopping point.';

      case ActionType.PACE_SUGGESTION:
        return '*sniff* You\'re moving fast. Slower is sometimes faster — fewer bugs, less rework.';

      case ActionType.CELEBRATION:
        return `*tail wag* ${intervention.reason || 'Nice work.'} Keep that momentum.`;

      case ActionType.COMPLEXITY_WARNING:
        return `*head tilt* Cognitive load is high (${context.cognitiveLoad || '?'} items). Consider closing some files or simplifying scope.`;

      case ActionType.HYDRATION_REMINDER:
        return '*sniff* Hydration check. Water helps cognition more than caffeine.';

      case ActionType.FOCUS_NUDGE:
        return '*ears perk* Focus seems scattered. Pick one thing, finish it, then move on.';

      case ActionType.SESSION_SUMMARY: {
        const mins = Math.round(context.sessionMinutes || 0);
        return `*yawn* Session: ${mins}min. Energy: ${((context.energy || 0) * 100).toFixed(0)}%. ${mins > 120 ? 'Consider wrapping up soon.' : 'Pace is sustainable.'}`;
      }

      default:
        return '*sniff* Check in with yourself.';
    }
  }

  _updateStats(result) {
    this._stats.actionsTotal++;
    this._stats.byType[result.type] = (this._stats.byType[result.type] || 0) + 1;
    this._stats.delivered++;
    this._stats.lastAction = Date.now();
  }

  getStats() { return { ...this._stats }; }

  getHealth() {
    const dismissRate = this._stats.delivered > 0
      ? this._stats.dismissed / this._stats.delivered
      : 0;

    return {
      status: dismissRate < PHI_INV_2 ? 'healthy' : 'over_intervening',
      score: Math.min(PHI_INV, 1 - dismissRate),
      actionsTotal: this._stats.actionsTotal,
      dismissRate,
      acknowledgmentRate: this._stats.delivered > 0
        ? this._stats.acknowledged / this._stats.delivered
        : 0,
    };
  }

  getHistory(limit = 21) {
    return this._history.slice(-limit);
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

export function getHumanActor(options = {}) {
  if (!_instance) _instance = new HumanActor(options);
  return _instance;
}

export function resetHumanActor() {
  if (_instance) _instance.removeAllListeners();
  _instance = null;
}

export default { HumanActor, ActionType, ActionStatus, getHumanActor, resetHumanActor };
