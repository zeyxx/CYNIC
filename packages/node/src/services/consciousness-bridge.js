/**
 * CYNIC Consciousness Bridge
 *
 * Connects system health monitoring to consciousness awareness.
 * Translates system events into consciousness observations.
 *
 * "Le chien est conscient de son corps" - CYNIC knows its state
 *
 * @module @cynic/node/services/consciousness-bridge
 */

'use strict';

import { EventEmitter } from 'events';

/**
 * Map system health to consciousness observation types
 */
export const ObservationType = {
  SYSTEM_HEALTH: 'SYSTEM_HEALTH',
  SLA_VIOLATION: 'SLA_VIOLATION',
  COMPONENT_FAILURE: 'COMPONENT_FAILURE',
  PERFORMANCE_DEGRADATION: 'PERFORMANCE_DEGRADATION',
  RECOVERY: 'RECOVERY',
  // Error types (added Week 4)
  ERROR: 'ERROR',
  ERROR_PATTERN: 'ERROR_PATTERN',
  UNHANDLED_REJECTION: 'UNHANDLED_REJECTION',
  UNCAUGHT_EXCEPTION: 'UNCAUGHT_EXCEPTION',
};

/**
 * Consciousness Bridge - Connects monitoring to awareness
 */
export class ConsciousnessBridge extends EventEmitter {
  /**
   * @param {Object} options
   * @param {ConsciousnessMonitor} options.consciousness - ConsciousnessMonitor instance
   * @param {HeartbeatService} [options.heartbeat] - HeartbeatService instance
   * @param {SLATracker} [options.slaTracker] - SLATracker instance
   * @param {Object} [options.alertManager] - AlertManager for critical alerts
   */
  constructor(options = {}) {
    super();
    this.consciousness = options.consciousness;
    this.heartbeat = options.heartbeat || null;
    this.slaTracker = options.slaTracker || null;
    this.alertManager = options.alertManager || null;

    // State tracking
    this._lastOverallStatus = 'healthy';
    this._failedComponents = new Set();

    // Wire services
    if (this.heartbeat) this._wireHeartbeat();
    if (this.slaTracker) this._wireSLATracker();
  }

  /**
   * Wire HeartbeatService events to consciousness
   * @private
   */
  _wireHeartbeat() {
    // On each heartbeat, observe system health
    this.heartbeat.on('heartbeat', ({ overall, results, metrics }) => {
      // Observe overall health state
      const healthConfidence = overall.ratio; // 0-1 based on healthy components

      this._observe(ObservationType.SYSTEM_HEALTH, {
        status: overall.status,
        healthy: overall.healthy,
        total: overall.total,
        uptime: metrics.systemUptime,
        uptime1h: metrics.systemUptime1h,
      }, healthConfidence);

      // Detect status changes
      if (overall.status !== this._lastOverallStatus) {
        if (overall.status === 'healthy' && this._lastOverallStatus !== 'healthy') {
          // Recovery
          this._observe(ObservationType.RECOVERY, {
            from: this._lastOverallStatus,
            to: overall.status,
            recoveredComponents: [...this._failedComponents],
          }, 0.8);
          this._failedComponents.clear();
        }
        this._lastOverallStatus = overall.status;
      }

      // Track failed components
      for (const [name, result] of Object.entries(results)) {
        if (!result.healthy && !this._failedComponents.has(name)) {
          this._failedComponents.add(name);
          this._observe(ObservationType.COMPONENT_FAILURE, {
            component: name,
            error: result.error,
            latencyMs: result.latencyMs,
          }, 0.9);
        } else if (result.healthy && this._failedComponents.has(name)) {
          this._failedComponents.delete(name);
        }
      }
    });

    // On unhealthy component
    this.heartbeat.on('unhealthy', ({ component, error, latencyMs }) => {
      this._observe(ObservationType.COMPONENT_FAILURE, {
        component,
        error,
        latencyMs,
        timestamp: Date.now(),
      }, 0.85);
    });

    // On degraded performance
    this.heartbeat.on('alert', ({ status, components }) => {
      if (status === 'degraded') {
        this._observe(ObservationType.PERFORMANCE_DEGRADATION, {
          status,
          affectedComponents: components,
        }, 0.7);
      }
    });
  }

  /**
   * Wire SLATracker events to consciousness
   * @private
   */
  _wireSLATracker() {
    this.slaTracker.on('violation', (violation) => {
      // φ-aligned confidence: max 61.8% (φ⁻¹), secondary 38.2% (φ⁻²)
      const confidence = violation.severity === 'critical' ? 0.618 : 0.382;

      this._observe(ObservationType.SLA_VIOLATION, {
        target: violation.target,
        expected: violation.expected,
        actual: violation.actual,
        severity: violation.severity,
        component: violation.component,
      }, confidence);

      // Alert on critical SLA breaches
      if (violation.severity === 'critical' && this.alertManager) {
        this.alertManager.critical?.(`SLA Breach: ${violation.target}`, violation);
      }

      // Also emit for external listeners
      this.emit('sla_breach', violation);
    });
  }

  /**
   * Record observation to consciousness
   * @private
   */
  _observe(type, data, confidence) {
    if (!this.consciousness) return;

    try {
      this.consciousness.observe(type, data, confidence, 'consciousness-bridge');
    } catch (e) {
      // Don't let consciousness errors affect monitoring
    }

    this.emit('observation', { type, data, confidence });
  }

  /**
   * Manually observe a system event
   */
  observe(type, data, confidence = 0.5) {
    this._observe(type, data, confidence);
  }

  /**
   * Observe an error event
   * Maps error severity to consciousness impact
   *
   * @param {Error} error - The error that occurred
   * @param {Object} context - Additional context
   * @param {string} [context.component] - Component that threw
   * @param {string} [context.severity='error'] - 'info'|'warn'|'error'|'critical'
   * @param {boolean} [context.isUnhandled=false] - Was this unhandled?
   */
  observeError(error, context = {}) {
    const {
      component = 'unknown',
      severity = 'error',
      isUnhandled = false,
    } = context;

    // φ-aligned confidence based on severity
    // Critical errors get max attention (φ⁻¹)
    const severityConfidence = {
      info: 0.1,
      warn: 0.382,     // φ⁻²
      error: 0.5,
      critical: 0.618, // φ⁻¹
    };
    const confidence = severityConfidence[severity] || 0.5;

    // Determine observation type
    let type = ObservationType.ERROR;
    if (isUnhandled) {
      type = error.name === 'UnhandledRejection'
        ? ObservationType.UNHANDLED_REJECTION
        : ObservationType.UNCAUGHT_EXCEPTION;
    }

    const errorData = {
      name: error.name,
      message: error.message,
      stack: error.stack?.split('\n').slice(0, 5).join('\n'),
      component,
      severity,
      isUnhandled,
      timestamp: Date.now(),
    };

    this._observe(type, errorData, confidence);

    // Track error patterns
    this._trackErrorPattern(error, component);

    // Alert on critical or unhandled
    if ((severity === 'critical' || isUnhandled) && this.alertManager) {
      this.alertManager.critical?.(
        `${isUnhandled ? 'Unhandled ' : ''}Error in ${component}: ${error.message}`,
        errorData
      );
    }

    this.emit('error_observed', { error, context, confidence });
  }

  /**
   * Track error patterns for learning
   * @private
   */
  _trackErrorPattern(error, component) {
    const patternKey = `${component}:${error.name}`;

    if (!this._errorPatterns) {
      this._errorPatterns = new Map();
    }

    const existing = this._errorPatterns.get(patternKey) || {
      count: 0,
      firstSeen: Date.now(),
      lastSeen: null,
      messages: [],
    };

    existing.count++;
    existing.lastSeen = Date.now();
    existing.messages.push(error.message);
    if (existing.messages.length > 10) {
      existing.messages.shift();
    }

    this._errorPatterns.set(patternKey, existing);

    // If pattern emerges (3+ occurrences), observe it
    if (existing.count >= 3 && existing.count % 3 === 0) {
      this._observe(ObservationType.ERROR_PATTERN, {
        pattern: patternKey,
        component,
        errorName: error.name,
        count: existing.count,
        frequency: existing.count / ((Date.now() - existing.firstSeen) / 60000), // per minute
      }, 0.618);
    }
  }

  /**
   * Get tracked error patterns
   */
  getErrorPatterns() {
    return this._errorPatterns ? Object.fromEntries(this._errorPatterns) : {};
  }

  /**
   * Get current system consciousness state
   */
  getState() {
    const consciousnessState = this.consciousness?.state || 'UNKNOWN';
    const awarenessLevel = this.consciousness?.awarenessLevel || 0;

    return {
      consciousness: consciousnessState,
      awarenessLevel,
      overallHealth: this._lastOverallStatus,
      failedComponents: [...this._failedComponents],
      slaCompliant: this.slaTracker?.isCompliant() ?? null,
    };
  }
}

/**
 * Create a ConsciousnessBridge instance
 */
export function createConsciousnessBridge(options = {}) {
  return new ConsciousnessBridge(options);
}

// Singleton
let _bridge = null;

/**
 * Get the global ConsciousnessBridge instance
 */
export function getConsciousnessBridge(options) {
  if (!_bridge) {
    _bridge = createConsciousnessBridge(options);
  }
  return _bridge;
}

/**
 * Wire all monitoring services to consciousness (convenience function)
 */
export function wireConsciousness(options) {
  const bridge = getConsciousnessBridge(options);

  // Wire additional services if provided after creation
  if (options.heartbeat && !bridge.heartbeat) {
    bridge.heartbeat = options.heartbeat;
    bridge._wireHeartbeat();
  }
  if (options.slaTracker && !bridge.slaTracker) {
    bridge.slaTracker = options.slaTracker;
    bridge._wireSLATracker();
  }

  return bridge;
}

export default {
  ConsciousnessBridge,
  ObservationType,
  createConsciousnessBridge,
  getConsciousnessBridge,
  wireConsciousness,
};
