/**
 * Actor Factory — creates domain-specific Actor classes from config
 *
 * Template code (65% of each Actor) lives here ONCE.
 * Domain logic (35%) lives in config objects.
 *
 * Usage:
 *   const { Class, getInstance, resetInstance } = createActor(codeActorConfig);
 *
 * "Le code génère le code" — one factory, N domains
 *
 * @module @cynic/node/cycle/create-actor
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, createLogger, globalEventBus } from '@cynic/core';
import { pushHistory } from '@cynic/core';
import { ActionStatus } from './shared-enums.js';

/**
 * Create an Actor class from a domain config.
 *
 * @param {Object} config - Domain-specific configuration
 * @param {string} config.name - Actor name (e.g. 'CodeActor')
 * @param {string} config.cell - Matrix cell (e.g. 'C1.4')
 * @param {string} config.dimension - Domain name (e.g. 'CODE')
 * @param {string} config.eventPrefix - Event name prefix (e.g. 'code')
 * @param {Object} config.actionTypes - Enum of action types
 * @param {Object} config.cooldowns - Map of actionType → cooldown ms
 * @param {number} [config.maxHistory=233] - Max history size (Fibonacci)
 * @param {Function} config.mapDecisionToAction - (decision) → actionType string
 * @param {Function} config.assessUrgency - (decision) → 'critical'|'high'|'medium'|'low'
 * @param {Function} config.composeMessage - (actionType, decision, context) → string
 * @param {string[]} [config.extraStatFields=[]] - Additional stat counter fields
 * @param {Function} [config.updateExtraStats] - (stats, result) → void
 * @param {Function} [config.postAct] - (result, decision, context, actor) → void
 * @param {string} config.healthMetric - Stat field name for health calculation
 * @param {number} [config.healthThreshold=PHI_INV_2] - Below this ratio = healthy
 * @param {string} [config.healthStatusBad='unhealthy'] - Status when unhealthy
 * @param {Object} [config.healthExtraFields] - Extra fields for getHealth()
 * @returns {{ Class, ActionStatus, getInstance, resetInstance }}
 */
export function createActor(config) {
  const log = createLogger(config.name);
  const defaultCooldown = 5 * 60000;

  class DomainActor extends EventEmitter {
    constructor(options = {}) {
      super();

      this._lastAction = new Map();
      this._history = [];
      this._maxHistory = config.maxHistory || 233;

      // Stats: shared fields + domain extras
      this._stats = {
        actionsTotal: 0,
        byType: {},
        delivered: 0,
        lastAction: null,
      };

      for (const type of Object.values(config.actionTypes)) {
        this._stats.byType[type] = 0;
      }

      for (const field of (config.extraStatFields || [])) {
        this._stats[field] = 0;
      }

      // Domain-specific init
      if (config.init) config.init(this, options);
    }

    /**
     * Execute an action based on a decision.
     *
     * @param {Object} decision - From the domain Decider
     * @param {Object} [context] - Additional context
     * @returns {Object|null} Action result, or null if on cooldown
     */
    act(decision, context = {}) {
      const actionType = config.mapDecisionToAction(decision);
      if (!actionType) return null;

      if (this._isOnCooldown(actionType)) {
        log.debug(`${config.name} action on cooldown`, { type: actionType });
        return null;
      }

      const message = config.composeMessage(actionType, decision, context);

      const result = {
        type: actionType,
        status: ActionStatus.DELIVERED,
        message,
        urgency: config.assessUrgency(decision),
        cell: config.cell,
        dimension: config.dimension,
        analysis: 'ACT',
        decision: decision.type || decision.decision,
        reasoning: decision.reasoning || decision.reason,
        risk: decision.risk,
        confidence: decision.confidence,
        judgmentId: context.judgmentId,
        timestamp: Date.now(),
      };

      this._lastAction.set(actionType, Date.now());
      this._updateStats(result);
      pushHistory(this._history, result, this._maxHistory);

      // Domain-specific post-processing
      if (config.postAct) config.postAct(result, decision, context, this);

      this.emit('action', result);

      // Publish to global bus (handle both emit and publish APIs)
      if (typeof globalEventBus.publish === 'function') {
        globalEventBus.publish(`${config.eventPrefix}:action`, result, { source: config.name });
      } else {
        globalEventBus.emit(`${config.eventPrefix}:action`, result);
      }

      log.debug(`${config.name} action delivered`, { type: actionType, urgency: result.urgency });

      return result;
    }

    /**
     * Record that an action was acted upon.
     */
    recordResponse(actionType, response) {
      const lastIdx = this._history.findLastIndex(a => a.type === actionType);
      if (lastIdx >= 0) {
        this._history[lastIdx].status = response === 'dismiss'
          ? ActionStatus.DISMISSED
          : ActionStatus.ACTED_ON;
      }
      this.emit('response', { type: actionType, response });
    }

    _isOnCooldown(actionType) {
      const last = this._lastAction.get(actionType);
      if (!last) return false;
      const cooldown = config.cooldowns[actionType] ?? defaultCooldown;
      return (Date.now() - last) < cooldown;
    }

    _updateStats(result) {
      this._stats.actionsTotal++;
      this._stats.byType[result.type] = (this._stats.byType[result.type] || 0) + 1;
      this._stats.delivered++;
      this._stats.lastAction = Date.now();

      if (config.updateExtraStats) config.updateExtraStats(this._stats, result);
    }

    getStats() { return { ...this._stats }; }

    getHistory(limit = 21) {
      return this._history.slice(-limit);
    }

    getHealth() {
      const dangerCount = this._stats[config.healthMetric] || 0;
      const rate = this._stats.actionsTotal > 0
        ? dangerCount / this._stats.actionsTotal
        : 0;

      const health = {
        status: rate < (config.healthThreshold || PHI_INV_2) ? 'healthy' : (config.healthStatusBad || 'unhealthy'),
        score: Math.min(PHI_INV, 1 - rate),
        actionsTotal: this._stats.actionsTotal,
        [config.healthMetric + 'Rate']: rate,
      };

      // Add domain-specific health fields
      if (config.healthExtraFields) {
        for (const [key, statField] of Object.entries(config.healthExtraFields)) {
          health[key] = this._stats[statField] || 0;
        }
      }

      return health;
    }
  }

  // Singleton management
  let _instance = null;

  function getInstance(options = {}) {
    if (!_instance) _instance = new DomainActor(options);
    return _instance;
  }

  function resetInstance() {
    if (_instance) _instance.removeAllListeners();
    _instance = null;
  }

  return {
    Class: DomainActor,
    ActionStatus,
    getInstance,
    resetInstance,
  };
}
