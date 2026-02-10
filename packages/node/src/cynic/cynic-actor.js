/**
 * CYNIC Actor - C6.4 (CYNIC × ACT)
 *
 * Self-healing: CYNIC responds to consciousness degradation with graduated actions.
 * Bridges consciousness perception (C6.1) into concrete self-maintenance.
 *
 * "Le chien se lèche les blessures" - κυνικός
 *
 * Actions:
 * - DORMANT/AWAKENING: restrict routing to core dogs (already in router)
 * - Degradation trend: emit warning, pause non-essential intervals
 * - Error spike: rate-limit operations
 * - Recovery: restore capabilities, log healing
 * - Stagnation: trigger pattern refresh
 *
 * @module @cynic/node/cynic/cynic-actor
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('CynicActor');

/** Self-healing action types */
export const SelfHealAction = {
  RESTRICT_ROUTING: 'restrict_routing',
  PAUSE_NONESSENTIAL: 'pause_nonessential',
  RATE_LIMIT: 'rate_limit',
  TRIGGER_REFRESH: 'trigger_refresh',
  RESTORE_CAPABILITIES: 'restore_capabilities',
  LOG_RECOVERY: 'log_recovery',
  EMERGENCY_COMPACT: 'emergency_compact',
};

/** System health states with graduated thresholds */
export const HealthState = {
  OPTIMAL: 'optimal',         // awarenessLevel >= φ⁻¹
  NOMINAL: 'nominal',         // awarenessLevel >= φ⁻²
  DEGRADED: 'degraded',       // awarenessLevel >= φ⁻³
  CRITICAL: 'critical',       // awarenessLevel < φ⁻³
};

function classifyHealth(awarenessLevel) {
  if (awarenessLevel >= PHI_INV) return HealthState.OPTIMAL;
  if (awarenessLevel >= PHI_INV_2) return HealthState.NOMINAL;
  if (awarenessLevel >= PHI_INV_3) return HealthState.DEGRADED;
  return HealthState.CRITICAL;
}

/**
 * CynicActor - C6.4 self-healing actor
 */
export class CynicActor extends EventEmitter {
  constructor(options = {}) {
    super();

    /** @type {import('@cynic/emergence').ConsciousnessMonitor|null} */
    this._consciousnessMonitor = options.consciousnessMonitor || null;

    /** @type {import('../organism/homeostasis.js').HomeostasisTracker|null} */
    this._homeostasis = options.homeostasis || null;

    // State tracking
    this._currentHealthState = HealthState.NOMINAL;
    this._previousHealthState = HealthState.NOMINAL;
    this._degradationStart = null;
    this._lastActionTime = new Map(); // action -> timestamp

    // Rolling awareness history (for trend detection)
    this._awarenessHistory = []; // [{ level, ts }]
    this._maxHistory = 89; // Fib(11)

    // Cooldowns (ms) — prevent action spam
    this._cooldowns = {
      [SelfHealAction.RESTRICT_ROUTING]: 5 * 60000,      // 5 min
      [SelfHealAction.PAUSE_NONESSENTIAL]: 10 * 60000,   // 10 min
      [SelfHealAction.RATE_LIMIT]: 3 * 60000,            // 3 min
      [SelfHealAction.TRIGGER_REFRESH]: 21 * 60000,      // F8=21 min
      [SelfHealAction.RESTORE_CAPABILITIES]: 2 * 60000,  // 2 min
      [SelfHealAction.LOG_RECOVERY]: 60000,               // 1 min
      [SelfHealAction.EMERGENCY_COMPACT]: 34 * 60000,    // F9=34 min
    };

    // Stats
    this._stats = {
      actionsTotal: 0,
      byAction: {},
      stateTransitions: 0,
      degradationEpisodes: 0,
      recoveries: 0,
      lastAction: null,
    };

    for (const action of Object.values(SelfHealAction)) {
      this._stats.byAction[action] = 0;
    }
  }

  /**
   * Process a consciousness state change.
   * Called when ConsciousnessMonitor publishes consciousness:changed.
   *
   * @param {Object} event
   * @param {string} event.newState - DORMANT/AWAKENING/AWARE/HEIGHTENED/TRANSCENDENT
   * @param {number} event.awarenessLevel - 0-1 float
   * @returns {Object[]} Actions taken
   */
  processConsciousnessChange(event) {
    const level = event.awarenessLevel ?? 0.5;
    const state = event.newState || 'AWARE';

    // Record in history
    this._awarenessHistory.push({ level, state, ts: Date.now() });
    while (this._awarenessHistory.length > this._maxHistory) {
      this._awarenessHistory.shift();
    }

    // Classify health
    const newHealthState = classifyHealth(level);
    const actions = [];

    // Detect state transition
    if (newHealthState !== this._currentHealthState) {
      this._stats.stateTransitions++;
      this._previousHealthState = this._currentHealthState;
      this._currentHealthState = newHealthState;

      log.info('Health state transition', {
        from: this._previousHealthState,
        to: newHealthState,
        awarenessLevel: level,
      });

      // Act on transition
      if (this._isDegrading()) {
        actions.push(...this._handleDegradation(newHealthState, level));
      } else if (this._isRecovering()) {
        actions.push(...this._handleRecovery(newHealthState, level));
      }
    }

    // Check for trend-based actions even without state transition
    const trend = this._computeTrend();
    if (trend < -0.1 && newHealthState !== HealthState.OPTIMAL) {
      // Negative trend — take preemptive action
      const preemptive = this._tryAction(SelfHealAction.TRIGGER_REFRESH, {
        reason: 'negative_awareness_trend',
        trend,
        level,
      });
      if (preemptive) actions.push(preemptive);
    }

    return actions;
  }

  /**
   * Process a homeostasis perturbation.
   * Called when HomeostasisTracker detects instability.
   *
   * @param {Object} perturbation
   * @param {string} perturbation.metric - Which metric is perturbed
   * @param {number} perturbation.deviation - Standard deviations from baseline
   * @returns {Object|null} Action taken
   */
  processPerturbation(perturbation) {
    const { metric, deviation } = perturbation;

    // Only act on significant perturbations (> 3σ)
    if (Math.abs(deviation) < 3) return null;

    if (metric === 'errorRate' && deviation > 3) {
      return this._tryAction(SelfHealAction.RATE_LIMIT, {
        reason: 'error_rate_spike',
        metric,
        deviation,
      });
    }

    if (metric === 'latency' && deviation > 3) {
      return this._tryAction(SelfHealAction.PAUSE_NONESSENTIAL, {
        reason: 'latency_spike',
        metric,
        deviation,
      });
    }

    return null;
  }

  // ═══════════════════════════════════════════════════════════════════════════════
  // INTERNAL
  // ═══════════════════════════════════════════════════════════════════════════════

  _isDegrading() {
    const stateOrder = [HealthState.OPTIMAL, HealthState.NOMINAL, HealthState.DEGRADED, HealthState.CRITICAL];
    return stateOrder.indexOf(this._currentHealthState) > stateOrder.indexOf(this._previousHealthState);
  }

  _isRecovering() {
    // Only count as recovery if there was an actual degradation episode
    if (!this._degradationStart) return false;
    const stateOrder = [HealthState.OPTIMAL, HealthState.NOMINAL, HealthState.DEGRADED, HealthState.CRITICAL];
    return stateOrder.indexOf(this._currentHealthState) < stateOrder.indexOf(this._previousHealthState);
  }

  _handleDegradation(healthState, level) {
    const actions = [];

    if (!this._degradationStart) {
      this._degradationStart = Date.now();
      this._stats.degradationEpisodes++;
    }

    if (healthState === HealthState.CRITICAL) {
      // Emergency: restrict + rate limit + pause
      const restrict = this._tryAction(SelfHealAction.RESTRICT_ROUTING, {
        reason: 'critical_degradation',
        level,
      });
      if (restrict) actions.push(restrict);

      const rateLimit = this._tryAction(SelfHealAction.RATE_LIMIT, {
        reason: 'critical_degradation',
        level,
      });
      if (rateLimit) actions.push(rateLimit);

      const pause = this._tryAction(SelfHealAction.PAUSE_NONESSENTIAL, {
        reason: 'critical_degradation',
        level,
      });
      if (pause) actions.push(pause);
    } else if (healthState === HealthState.DEGRADED) {
      // Moderate: restrict routing
      const restrict = this._tryAction(SelfHealAction.RESTRICT_ROUTING, {
        reason: 'moderate_degradation',
        level,
      });
      if (restrict) actions.push(restrict);
    }

    return actions;
  }

  _handleRecovery(healthState, level) {
    const actions = [];
    const recoveryTime = this._degradationStart
      ? Date.now() - this._degradationStart
      : 0;

    this._degradationStart = null;
    this._stats.recoveries++;

    const restore = this._tryAction(SelfHealAction.RESTORE_CAPABILITIES, {
      reason: 'recovery_detected',
      recoveredTo: healthState,
      level,
      recoveryTimeMs: recoveryTime,
    });
    if (restore) actions.push(restore);

    const logAction = this._tryAction(SelfHealAction.LOG_RECOVERY, {
      reason: 'recovery_log',
      from: this._previousHealthState,
      to: healthState,
      recoveryTimeMs: recoveryTime,
    });
    if (logAction) actions.push(logAction);

    return actions;
  }

  /**
   * Try to execute an action (respects cooldown).
   *
   * @param {string} actionType
   * @param {Object} context
   * @returns {Object|null} Action object if executed, null if on cooldown
   */
  _tryAction(actionType, context) {
    const now = Date.now();
    const lastTime = this._lastActionTime.get(actionType) || 0;
    const cooldown = this._cooldowns[actionType] || 60000;

    if (now - lastTime < cooldown) return null;

    this._lastActionTime.set(actionType, now);
    this._stats.actionsTotal++;
    this._stats.byAction[actionType] = (this._stats.byAction[actionType] || 0) + 1;
    this._stats.lastAction = { type: actionType, ts: now };

    const action = {
      cell: 'C6.4',
      dimension: 'CYNIC',
      analysis: 'ACT',
      type: actionType,
      context,
      healthState: this._currentHealthState,
      ts: now,
    };

    this.emit('self_heal', action);
    globalEventBus.emit('cynic:self_heal', action);

    log.info('Self-healing action', {
      action: actionType,
      reason: context.reason,
      healthState: this._currentHealthState,
    });

    return action;
  }

  /**
   * Compute awareness trend over recent history.
   * Returns negative for declining, positive for improving.
   *
   * @returns {number} Trend (-1 to +1)
   */
  _computeTrend() {
    if (this._awarenessHistory.length < 5) return 0;

    const recent = this._awarenessHistory.slice(-13); // Fib(7)
    const mid = Math.floor(recent.length / 2);
    const firstHalf = recent.slice(0, mid);
    const secondHalf = recent.slice(mid);

    const avgFirst = firstHalf.reduce((s, h) => s + h.level, 0) / firstHalf.length;
    const avgSecond = secondHalf.reduce((s, h) => s + h.level, 0) / secondHalf.length;

    return Math.max(-1, Math.min(1, avgSecond - avgFirst));
  }

  // ═══════════════════════════════════════════════════════════════════════════════
  // PUBLIC API
  // ═══════════════════════════════════════════════════════════════════════════════

  getStats() {
    return {
      ...this._stats,
      currentHealthState: this._currentHealthState,
      awarenessHistorySize: this._awarenessHistory.length,
      trend: this._computeTrend(),
      isDegrading: this._degradationStart !== null,
      degradationDurationMs: this._degradationStart
        ? Date.now() - this._degradationStart
        : 0,
    };
  }

  getHealth() {
    const trend = this._computeTrend();
    return {
      status: this._currentHealthState === HealthState.CRITICAL ? 'critical'
        : this._currentHealthState === HealthState.DEGRADED ? 'degraded'
          : trend < -0.1 ? 'declining'
            : 'healthy',
      score: Math.min(PHI_INV, this._awarenessHistory.length > 0
        ? this._awarenessHistory[this._awarenessHistory.length - 1].level
        : 0.5),
      actionsTotal: this._stats.actionsTotal,
      stateTransitions: this._stats.stateTransitions,
      recoveries: this._stats.recoveries,
      trend,
    };
  }
}

// =============================================================================
// SINGLETON
// =============================================================================

let _instance = null;

export function getCynicActor(options = {}) {
  if (!_instance) _instance = new CynicActor(options);
  return _instance;
}

export function resetCynicActor() {
  if (_instance) _instance.removeAllListeners();
  _instance = null;
}

export default { CynicActor, getCynicActor, resetCynicActor, SelfHealAction, HealthState };
