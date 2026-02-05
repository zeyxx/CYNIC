/**
 * Resilience - Recovery and Fault Tolerance
 *
 * "Le chien tombe sept fois, se relève huit" - κυνικός
 *
 * Measures how well CYNIC recovers from failures:
 * - Recovery time after errors
 * - Bounce-back from perturbations
 * - Fault tolerance
 * - Self-healing capability
 *
 * High resilience = quick recovery, robust
 * Low resilience = slow recovery, fragile
 *
 * @module @cynic/node/organism/resilience
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import { computeStats, GaussianDistribution } from '../inference/gaussian.js';
import { detectAnomaly } from '../inference/poisson.js';

// ═══════════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════════

export const RESILIENCE_CONFIG = {
  // Recovery time thresholds (ms)
  RECOVERY_FAST: 1000,        // < 1s = fast recovery
  RECOVERY_NORMAL: 5000,      // < 5s = normal recovery
  RECOVERY_SLOW: 30000,       // < 30s = slow recovery
  RECOVERY_CRITICAL: 60000,   // > 60s = critical (system may be stuck)

  // Failure types
  FAILURE_TYPES: {
    ERROR: 'error',
    TIMEOUT: 'timeout',
    REJECTION: 'rejection',
    CRASH: 'crash',
    OVERLOAD: 'overload',
  },

  // Recovery success thresholds
  SUCCESS_RATE_HEALTHY: PHI_INV,    // > 61.8% recovery success = healthy
  SUCCESS_RATE_WARNING: PHI_INV_2,  // > 38.2% = warning
  SUCCESS_RATE_CRITICAL: PHI_INV_3, // < 23.6% = critical

  // Maximum history
  MAX_INCIDENTS: 500,

  // Circuit breaker thresholds
  CIRCUIT_BREAKER_THRESHOLD: 5,     // 5 failures = trip
  CIRCUIT_BREAKER_TIMEOUT: 30000,   // 30s cooldown
};

// ═══════════════════════════════════════════════════════════════════════════════
// INCIDENT
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Failure incident with recovery tracking
 */
export class Incident {
  /**
   * @param {string} type - Failure type
   * @param {Object} [data] - Incident data
   */
  constructor(type, data = {}) {
    this.id = `incident_${Date.now()}_${Math.random().toString(36).substr(2, 9)}`;
    this.timestamp = Date.now();
    this.type = type;
    this.severity = data.severity || 1;
    this.source = data.source || 'unknown';
    this.message = data.message || '';
    this.metadata = data.metadata || {};

    // Recovery tracking
    this.recoveredAt = null;
    this.recoveryTimeMs = null;
    this.recoverySuccess = null;
    this.recoveryAttempts = 0;
  }

  /**
   * Mark incident as recovered
   * @param {boolean} [success=true]
   */
  markRecovered(success = true) {
    if (this.recoveredAt) return; // Already recovered

    this.recoveredAt = Date.now();
    this.recoveryTimeMs = this.recoveredAt - this.timestamp;
    this.recoverySuccess = success;
  }

  /**
   * Increment recovery attempt
   */
  attemptRecovery() {
    this.recoveryAttempts++;
  }

  /**
   * Check if recovered
   */
  get isRecovered() {
    return this.recoveredAt !== null;
  }

  /**
   * Check if recovery was successful
   */
  get isSuccessful() {
    return this.recoverySuccess === true;
  }

  /**
   * Get time since incident (ms)
   */
  get age() {
    return Date.now() - this.timestamp;
  }

  toJSON() {
    return {
      id: this.id,
      timestamp: this.timestamp,
      type: this.type,
      severity: this.severity,
      source: this.source,
      message: this.message,
      recoveredAt: this.recoveredAt,
      recoveryTimeMs: this.recoveryTimeMs,
      recoverySuccess: this.recoverySuccess,
      recoveryAttempts: this.recoveryAttempts,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// RESILIENCE TRACKER
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Tracks system resilience
 */
export class ResilienceTracker {
  constructor(options = {}) {
    /** @type {Incident[]} */
    this.incidents = [];
    this.maxIncidents = options.maxIncidents || RESILIENCE_CONFIG.MAX_INCIDENTS;

    // Active (unrecovered) incidents
    /** @type {Map<string, Incident>} */
    this.activeIncidents = new Map();

    // Circuit breaker state
    this.circuitBreaker = {
      failures: 0,
      lastFailure: null,
      tripped: false,
      trippedAt: null,
    };

    // Statistics
    this.totalIncidents = 0;
    this.totalRecoveries = 0;
    this.totalFailedRecoveries = 0;

    // Session
    this.sessionStart = Date.now();
  }

  /**
   * Record a new incident
   * @param {string} type - Failure type
   * @param {Object} [data] - Incident data
   * @returns {Incident}
   */
  recordIncident(type, data = {}) {
    const incident = new Incident(type, data);

    this.incidents.push(incident);
    this.activeIncidents.set(incident.id, incident);
    this.totalIncidents++;

    // Update circuit breaker
    this.circuitBreaker.failures++;
    this.circuitBreaker.lastFailure = Date.now();

    // Check if should trip
    if (this.circuitBreaker.failures >= RESILIENCE_CONFIG.CIRCUIT_BREAKER_THRESHOLD) {
      this.circuitBreaker.tripped = true;
      this.circuitBreaker.trippedAt = Date.now();
    }

    // Prune old incidents
    while (this.incidents.length > this.maxIncidents) {
      const old = this.incidents.shift();
      this.activeIncidents.delete(old.id);
    }

    return incident;
  }

  // Convenience methods
  recordError(data = {}) {
    return this.recordIncident(RESILIENCE_CONFIG.FAILURE_TYPES.ERROR, data);
  }

  recordTimeout(data = {}) {
    return this.recordIncident(RESILIENCE_CONFIG.FAILURE_TYPES.TIMEOUT, data);
  }

  recordCrash(data = {}) {
    return this.recordIncident(RESILIENCE_CONFIG.FAILURE_TYPES.CRASH, { ...data, severity: 3 });
  }

  /**
   * Mark an incident as recovered
   * @param {string} incidentId
   * @param {boolean} [success=true]
   */
  markRecovered(incidentId, success = true) {
    const incident = this.activeIncidents.get(incidentId);
    if (!incident) return;

    incident.markRecovered(success);
    this.activeIncidents.delete(incidentId);

    if (success) {
      this.totalRecoveries++;
      // Reduce circuit breaker failures on success
      this.circuitBreaker.failures = Math.max(0, this.circuitBreaker.failures - 1);
    } else {
      this.totalFailedRecoveries++;
    }

    // Reset circuit breaker if no failures
    if (this.circuitBreaker.failures === 0) {
      this.circuitBreaker.tripped = false;
      this.circuitBreaker.trippedAt = null;
    }
  }

  /**
   * Mark most recent incident as recovered
   * @param {boolean} [success=true]
   */
  markLastRecovered(success = true) {
    const lastActive = Array.from(this.activeIncidents.values()).pop();
    if (lastActive) {
      this.markRecovered(lastActive.id, success);
    }
  }

  /**
   * Get active (unrecovered) incidents
   * @returns {Incident[]}
   */
  getActiveIncidents() {
    return Array.from(this.activeIncidents.values());
  }

  /**
   * Get recovered incidents
   * @param {number} [limit=10]
   * @returns {Incident[]}
   */
  getRecoveredIncidents(limit = 10) {
    return this.incidents
      .filter(i => i.isRecovered)
      .slice(-limit);
  }

  /**
   * Calculate mean time to recovery (MTTR)
   * @returns {number} Average recovery time in ms
   */
  getMTTR() {
    const recovered = this.incidents.filter(
      i => i.isRecovered && i.recoverySuccess
    );

    if (recovered.length === 0) return 0;

    const totalTime = recovered.reduce((sum, i) => sum + i.recoveryTimeMs, 0);
    return totalTime / recovered.length;
  }

  /**
   * Calculate recovery success rate
   * @returns {number} 0 to 1
   */
  getRecoveryRate() {
    const recovered = this.incidents.filter(i => i.isRecovered);
    if (recovered.length === 0) return 1; // No incidents = perfect

    const successful = recovered.filter(i => i.recoverySuccess).length;
    return successful / recovered.length;
  }

  /**
   * Calculate resilience score
   * Combines recovery rate and MTTR
   * @returns {number} 0 to φ⁻¹
   */
  getResilienceScore() {
    const recoveryRate = this.getRecoveryRate();
    const mttr = this.getMTTR();

    // MTTR factor: faster recovery = higher score
    // Normalize to 0-1 scale (30s = optimal)
    const mttrFactor = mttr > 0
      ? Math.min(1, RESILIENCE_CONFIG.RECOVERY_NORMAL / mttr)
      : 1;

    // Combine
    const raw = recoveryRate * mttrFactor;
    return Math.min(PHI_INV, raw);
  }

  /**
   * Get resilience status
   * @returns {string}
   */
  getStatus() {
    if (this.circuitBreaker.tripped) return 'tripped';

    const score = this.getResilienceScore();
    const activeCount = this.activeIncidents.size;

    if (activeCount > 3) return 'overwhelmed';
    if (score >= RESILIENCE_CONFIG.SUCCESS_RATE_HEALTHY) return 'robust';
    if (score >= RESILIENCE_CONFIG.SUCCESS_RATE_WARNING) return 'degraded';
    if (score >= RESILIENCE_CONFIG.SUCCESS_RATE_CRITICAL) return 'fragile';
    return 'critical';
  }

  /**
   * Check if circuit breaker is tripped
   * @returns {boolean}
   */
  isCircuitBreakerTripped() {
    // Auto-reset after timeout
    if (this.circuitBreaker.tripped && this.circuitBreaker.trippedAt) {
      const elapsed = Date.now() - this.circuitBreaker.trippedAt;
      if (elapsed >= RESILIENCE_CONFIG.CIRCUIT_BREAKER_TIMEOUT) {
        this.circuitBreaker.tripped = false;
        this.circuitBreaker.failures = 0;
      }
    }

    return this.circuitBreaker.tripped;
  }

  /**
   * Get health assessment
   * @returns {Object}
   */
  getHealth() {
    const score = this.getResilienceScore();
    const status = this.getStatus();
    const mttr = this.getMTTR();
    const recoveryRate = this.getRecoveryRate();

    return {
      score,
      status,
      details: {
        mttr,
        recoveryRate,
        activeIncidents: this.activeIncidents.size,
        totalIncidents: this.totalIncidents,
        totalRecoveries: this.totalRecoveries,
        totalFailedRecoveries: this.totalFailedRecoveries,
        circuitBreaker: {
          tripped: this.circuitBreaker.tripped,
          failures: this.circuitBreaker.failures,
        },
      },
    };
  }

  /**
   * Get statistics
   * @returns {Object}
   */
  getStats() {
    const byType = {};
    for (const incident of this.incidents) {
      byType[incident.type] = (byType[incident.type] || 0) + 1;
    }

    return {
      score: this.getResilienceScore(),
      status: this.getStatus(),
      mttr: this.getMTTR(),
      recoveryRate: this.getRecoveryRate(),
      incidents: {
        total: this.totalIncidents,
        active: this.activeIncidents.size,
        byType,
      },
      recoveries: {
        successful: this.totalRecoveries,
        failed: this.totalFailedRecoveries,
      },
      circuitBreaker: { ...this.circuitBreaker },
      session: {
        durationMs: Date.now() - this.sessionStart,
      },
    };
  }

  /**
   * Reset circuit breaker manually
   */
  resetCircuitBreaker() {
    this.circuitBreaker = {
      failures: 0,
      lastFailure: null,
      tripped: false,
      trippedAt: null,
    };
  }

  /**
   * Reset tracker
   */
  reset() {
    this.incidents = [];
    this.activeIncidents.clear();
    this.totalIncidents = 0;
    this.totalRecoveries = 0;
    this.totalFailedRecoveries = 0;
    this.resetCircuitBreaker();
    this.sessionStart = Date.now();
  }

  /**
   * Serialize
   */
  toJSON() {
    return {
      incidents: this.incidents.map(i => i.toJSON()),
      circuitBreaker: this.circuitBreaker,
      totalIncidents: this.totalIncidents,
      totalRecoveries: this.totalRecoveries,
      totalFailedRecoveries: this.totalFailedRecoveries,
      sessionStart: this.sessionStart,
    };
  }

  /**
   * Restore
   */
  static fromJSON(data) {
    const tracker = new ResilienceTracker();

    tracker.incidents = (data.incidents || []).map(i => {
      const incident = new Incident(i.type, i);
      incident.id = i.id;
      incident.timestamp = i.timestamp;
      incident.recoveredAt = i.recoveredAt;
      incident.recoveryTimeMs = i.recoveryTimeMs;
      incident.recoverySuccess = i.recoverySuccess;
      incident.recoveryAttempts = i.recoveryAttempts;

      if (!incident.isRecovered) {
        tracker.activeIncidents.set(incident.id, incident);
      }

      return incident;
    });

    tracker.circuitBreaker = data.circuitBreaker || tracker.circuitBreaker;
    tracker.totalIncidents = data.totalIncidents || 0;
    tracker.totalRecoveries = data.totalRecoveries || 0;
    tracker.totalFailedRecoveries = data.totalFailedRecoveries || 0;
    tracker.sessionStart = data.sessionStart || Date.now();

    return tracker;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// FACTORY & SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

export function createResilienceTracker(options = {}) {
  return new ResilienceTracker(options);
}

let _resilienceTracker = null;

export function getResilienceTracker() {
  if (!_resilienceTracker) {
    _resilienceTracker = new ResilienceTracker();
  }
  return _resilienceTracker;
}

export function resetResilienceTracker() {
  _resilienceTracker = null;
}

export default {
  Incident,
  ResilienceTracker,
  createResilienceTracker,
  getResilienceTracker,
  resetResilienceTracker,
  RESILIENCE_CONFIG,
};
