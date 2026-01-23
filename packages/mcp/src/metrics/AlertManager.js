/**
 * Alert Manager
 *
 * Threshold-based alert system for CYNIC metrics.
 * Emits events when alerts are triggered or cleared.
 *
 * @module @cynic/mcp/metrics/AlertManager
 */

'use strict';

import { EventEmitter } from 'events';

/**
 * Alert severity levels
 */
export const ALERT_LEVELS = {
  INFO: 'info',
  WARNING: 'warning',
  CRITICAL: 'critical',
};

/**
 * Default thresholds for alerts
 */
export const DEFAULT_THRESHOLDS = {
  avgQScoreMin: 30,           // Alert if avg Q-Score drops below this
  chainInvalid: true,         // Alert if chain integrity fails
  cacheHitRateMin: 0.5,       // Alert if cache hit rate below 50%
  driftsCritical: 1,          // Alert if any critical drifts
  sessionIdleHours: 24,       // Alert if session idle for 24h
};

/**
 * AlertManager
 *
 * Manages alerts based on metric thresholds.
 * Extends EventEmitter to notify listeners of alert changes.
 *
 * Events:
 *   - 'alert' (alert) - New alert triggered
 *   - 'alert_cleared' (alert) - Alert acknowledged/cleared
 */
export class AlertManager extends EventEmitter {
  /**
   * @param {Object} [options={}] - Configuration
   * @param {Object} [options.thresholds] - Custom thresholds
   * @param {Object} [options.pojChainManager] - For chain integrity checks
   */
  constructor(options = {}) {
    super();

    this.thresholds = { ...DEFAULT_THRESHOLDS, ...options.thresholds };
    this.pojChainManager = options.pojChainManager || null;

    /** @type {Array<Object>} Active alerts */
    this._alerts = [];

    /** @type {Object} Stats */
    this._stats = {
      alertsTriggered: 0,
      alertsCleared: 0,
    };
  }

  /**
   * Check metrics against thresholds and update alerts
   * @param {Object} metrics - Raw metrics object
   * @returns {Promise<Object[]>} Current active alerts
   */
  async checkAlerts(metrics) {
    const newAlerts = [];

    // ═══════════════════════════════════════════════════════════════════════
    // Check average Q-Score
    // ═══════════════════════════════════════════════════════════════════════

    if (metrics.judgments?.total > 10) {
      const avgScore = metrics.judgments.avgQScore || 0;
      if (avgScore < this.thresholds.avgQScoreMin) {
        newAlerts.push({
          level: ALERT_LEVELS.WARNING,
          type: 'low_q_score',
          message: `Average Q-Score ${avgScore.toFixed(1)} below threshold ${this.thresholds.avgQScoreMin}`,
          value: avgScore,
          threshold: this.thresholds.avgQScoreMin,
          timestamp: Date.now(),
        });
      }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Check cache hit rate
    // ═══════════════════════════════════════════════════════════════════════

    if (metrics.cache?.library) {
      const { hits, misses } = metrics.cache.library;
      if (hits + misses > 20) {
        const hitRate = metrics.cache.library.hitRate;
        if (hitRate < this.thresholds.cacheHitRateMin) {
          newAlerts.push({
            level: ALERT_LEVELS.INFO,
            type: 'low_cache_hit_rate',
            message: `Cache hit rate ${(hitRate * 100).toFixed(1)}% below threshold ${this.thresholds.cacheHitRateMin * 100}%`,
            value: hitRate,
            threshold: this.thresholds.cacheHitRateMin,
            timestamp: Date.now(),
          });
        }
      }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Check critical drifts
    // ═══════════════════════════════════════════════════════════════════════

    if (metrics.integrator?.criticalDrifts >= this.thresholds.driftsCritical) {
      newAlerts.push({
        level: ALERT_LEVELS.CRITICAL,
        type: 'critical_drifts',
        message: `${metrics.integrator.criticalDrifts} critical module drift(s) detected`,
        value: metrics.integrator.criticalDrifts,
        threshold: this.thresholds.driftsCritical,
        timestamp: Date.now(),
      });
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Check chain integrity (async)
    // ═══════════════════════════════════════════════════════════════════════

    if (this.thresholds.chainInvalid && this.pojChainManager) {
      try {
        const verification = await this.pojChainManager.verifyIntegrity();
        if (!verification.valid) {
          newAlerts.push({
            level: ALERT_LEVELS.CRITICAL,
            type: 'chain_invalid',
            message: `PoJ chain integrity failure: ${verification.errors.length} errors`,
            errors: verification.errors.slice(0, 3),
            timestamp: Date.now(),
          });
        }
      } catch (e) {
        // Ignore verification errors
      }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // Update alerts and emit events
    // ═══════════════════════════════════════════════════════════════════════

    const oldAlertTypes = new Set(this._alerts.map(a => a.type));

    for (const alert of newAlerts) {
      if (!oldAlertTypes.has(alert.type)) {
        this._stats.alertsTriggered++;
        this.emit('alert', alert);
      }
    }

    // Check for cleared alerts
    const newAlertTypes = new Set(newAlerts.map(a => a.type));
    for (const oldAlert of this._alerts) {
      if (!newAlertTypes.has(oldAlert.type)) {
        this._stats.alertsCleared++;
        this.emit('alert_cleared', oldAlert);
      }
    }

    this._alerts = newAlerts;
    return this._alerts;
  }

  /**
   * Get all active alerts
   * @returns {Object[]} Active alerts
   */
  getAlerts() {
    return [...this._alerts];
  }

  /**
   * Get specific alert by type
   * @param {string} type - Alert type
   * @returns {Object|null} Alert or null
   */
  getAlert(type) {
    return this._alerts.find(a => a.type === type) || null;
  }

  /**
   * Manually clear an alert (acknowledge)
   * @param {string} type - Alert type to clear
   * @returns {boolean} Whether alert was found and cleared
   */
  clearAlert(type) {
    const idx = this._alerts.findIndex(a => a.type === type);
    if (idx >= 0) {
      const alert = this._alerts.splice(idx, 1)[0];
      this._stats.alertsCleared++;
      this.emit('alert_cleared', alert);
      return true;
    }
    return false;
  }

  /**
   * Get alert count
   * @returns {number} Number of active alerts
   */
  get count() {
    return this._alerts.length;
  }

  /**
   * Get alerts by level
   * @param {string} level - Alert level (info, warning, critical)
   * @returns {Object[]} Alerts matching level
   */
  getByLevel(level) {
    return this._alerts.filter(a => a.level === level);
  }

  /**
   * Check if there are critical alerts
   * @returns {boolean}
   */
  hasCritical() {
    return this._alerts.some(a => a.level === ALERT_LEVELS.CRITICAL);
  }

  /**
   * Get stats
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this._stats,
      active: this._alerts.length,
      critical: this.getByLevel(ALERT_LEVELS.CRITICAL).length,
      warning: this.getByLevel(ALERT_LEVELS.WARNING).length,
      info: this.getByLevel(ALERT_LEVELS.INFO).length,
    };
  }

  /**
   * Update thresholds
   * @param {Object} thresholds - New thresholds to merge
   */
  setThresholds(thresholds) {
    this.thresholds = { ...this.thresholds, ...thresholds };
  }

  /**
   * Set PoJ chain manager for integrity checks
   * @param {Object} pojChainManager
   */
  setPojChainManager(pojChainManager) {
    this.pojChainManager = pojChainManager;
  }
}

export default AlertManager;
