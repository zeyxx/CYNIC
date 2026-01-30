/**
 * CircuitBreaker - Resilience Pattern for Service Protection
 *
 * Prevents cascade failures by stopping requests to failing services.
 * φ-derived thresholds for graceful degradation.
 *
 * States:
 * - CLOSED: Normal operation, requests pass through
 * - OPEN: Service failing, requests immediately rejected
 * - HALF_OPEN: Testing if service recovered
 *
 * "Don't trust, verify - but know when to stop trying" - κυνικός
 *
 * @module @cynic/core/circuit-breaker
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV } from './axioms/constants.js';

/**
 * Circuit breaker states
 */
export const CircuitState = Object.freeze({
  CLOSED: 'CLOSED',       // Normal operation
  OPEN: 'OPEN',           // Failing, reject requests
  HALF_OPEN: 'HALF_OPEN', // Testing if recovered
});

/**
 * Default circuit breaker configuration (φ-derived)
 */
export const DEFAULT_CIRCUIT_CONFIG = Object.freeze({
  failureThreshold: 5,            // Open after 5 consecutive failures
  resetTimeoutMs: 61800,          // φ⁻¹ × 100000 - time before trying again
  halfOpenMaxRequests: 1,         // Allow 1 test request in half-open
  successThreshold: 2,            // Successes needed to close from half-open
  // v1.1: Exponential backoff
  maxResetTimeoutMs: 300000,      // Max 5 minutes (Fibonacci × 1000)
  backoffMultiplier: PHI_INV + 1, // ~1.618 (φ) - golden ratio backoff
  jitterFactor: 0.1,              // ±10% jitter to prevent thundering herd
  // v1.1: Health probe
  healthProbeIntervalMs: 5000,    // Probe health every 5s in half-open
});

/**
 * Circuit Breaker - Protects services from cascade failures
 *
 * @extends EventEmitter
 * @fires CircuitBreaker#stateChange - When state transitions
 * @fires CircuitBreaker#failure - When a failure is recorded
 * @fires CircuitBreaker#success - When a success is recorded
 *
 * @example
 * const breaker = new CircuitBreaker({ name: 'postgres' });
 *
 * // Before making request
 * if (!breaker.canExecute()) {
 *   throw new Error('Circuit is open');
 * }
 *
 * try {
 *   const result = await makeRequest();
 *   breaker.recordSuccess();
 *   return result;
 * } catch (err) {
 *   breaker.recordFailure();
 *   throw err;
 * }
 */
export class CircuitBreaker extends EventEmitter {
  /**
   * Create a circuit breaker
   *
   * @param {Object} [options] - Configuration options
   * @param {string} [options.name='default'] - Breaker name for logging
   * @param {number} [options.failureThreshold=5] - Failures before opening
   * @param {number} [options.resetTimeoutMs=61800] - Time before half-open
   * @param {number} [options.halfOpenMaxRequests=1] - Test requests in half-open
   * @param {number} [options.successThreshold=2] - Successes to close from half-open
   */
  constructor(options = {}) {
    super();

    this._name = options.name || 'default';
    this._config = {
      ...DEFAULT_CIRCUIT_CONFIG,
      ...options,
    };

    // State
    this._state = CircuitState.CLOSED;
    this._failureCount = 0;
    this._successCount = 0;
    this._lastFailureTime = 0;
    this._halfOpenRequests = 0;

    // v1.1: Exponential backoff tracking
    this._consecutiveOpenings = 0;        // Times circuit has opened consecutively
    this._currentResetTimeoutMs = this._config.resetTimeoutMs;
    this._lastProbeTime = 0;

    // v1.1: Health probe callback
    this._healthProbe = options.healthProbe || null;

    // Stats
    this._stats = {
      totalRequests: 0,
      totalFailures: 0,
      totalSuccesses: 0,
      stateChanges: 0,
      lastStateChange: null,
      consecutiveOpenings: 0,
      currentBackoffMs: this._config.resetTimeoutMs,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // State Access
  // ═══════════════════════════════════════════════════════════════════════════

  /** @returns {string} Current circuit state */
  get state() {
    return this._state;
  }

  /** @returns {string} Breaker name */
  get name() {
    return this._name;
  }

  /** @returns {boolean} Whether circuit is closed (normal operation) */
  get isClosed() {
    return this._state === CircuitState.CLOSED;
  }

  /** @returns {boolean} Whether circuit is open (rejecting requests) */
  get isOpen() {
    return this._state === CircuitState.OPEN;
  }

  /** @returns {boolean} Whether circuit is half-open (testing) */
  get isHalfOpen() {
    return this._state === CircuitState.HALF_OPEN;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Request Gate
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Check if a request can be executed
   *
   * Call this before making a request. If false, the circuit is open
   * and the request should be rejected immediately.
   *
   * @returns {boolean} Whether request can proceed
   */
  canExecute() {
    this._stats.totalRequests++;

    switch (this._state) {
      case CircuitState.CLOSED:
        return true;

      case CircuitState.OPEN: {
        // v1.1: Use current backoff timeout with jitter
        const timeoutWithJitter = this._getTimeoutWithJitter();
        if (Date.now() - this._lastFailureTime >= timeoutWithJitter) {
          this._transitionTo(CircuitState.HALF_OPEN);
          this._halfOpenRequests = 1;
          return true;
        }
        return false;
      }

      case CircuitState.HALF_OPEN:
        // Allow limited test requests
        if (this._halfOpenRequests < this._config.halfOpenMaxRequests) {
          this._halfOpenRequests++;
          return true;
        }
        return false;

      default:
        return false;
    }
  }

  /**
   * Check if ready for next probe (health check in half-open)
   * v1.1: Use periodic probes instead of request-based
   * @returns {boolean}
   */
  canProbe() {
    if (this._state !== CircuitState.HALF_OPEN) return false;
    const now = Date.now();
    if (now - this._lastProbeTime >= this._config.healthProbeIntervalMs) {
      this._lastProbeTime = now;
      return true;
    }
    return false;
  }

  /**
   * Execute health probe if available
   * v1.1: Run lightweight health check before allowing traffic
   * @returns {Promise<boolean>} Whether service is healthy
   */
  async probeHealth() {
    if (!this._healthProbe) return true;
    try {
      const result = await this._healthProbe();
      if (result) {
        this.recordSuccess();
      } else {
        this.recordFailure(new Error('Health probe returned false'));
      }
      return result;
    } catch (error) {
      this.recordFailure(error);
      return false;
    }
  }

  /**
   * Get reset timeout with jitter
   * v1.1: Add ±jitterFactor randomness to prevent thundering herd
   * @private
   * @returns {number} Timeout with jitter applied
   */
  _getTimeoutWithJitter() {
    const jitter = this._config.jitterFactor;
    const randomFactor = 1 + (Math.random() * 2 - 1) * jitter; // 1 ± jitter
    return Math.floor(this._currentResetTimeoutMs * randomFactor);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Result Recording
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record a successful request
   *
   * Call this after a request completes successfully.
   */
  recordSuccess() {
    this._stats.totalSuccesses++;

    switch (this._state) {
      case CircuitState.CLOSED:
        // Reset failure count on success
        this._failureCount = 0;
        break;

      case CircuitState.HALF_OPEN:
        this._successCount++;
        // Check if enough successes to close
        if (this._successCount >= this._config.successThreshold) {
          this._transitionTo(CircuitState.CLOSED);
        }
        break;

      case CircuitState.OPEN:
        // Shouldn't happen, but handle gracefully
        break;
    }

    this.emit('success', { name: this._name, state: this._state });
  }

  /**
   * Record a failed request
   *
   * Call this after a request fails.
   *
   * @param {Error} [error] - The error that occurred
   */
  recordFailure(error) {
    this._stats.totalFailures++;
    this._lastFailureTime = Date.now();
    this._failureCount++;

    switch (this._state) {
      case CircuitState.CLOSED:
        // Check if threshold reached
        if (this._failureCount >= this._config.failureThreshold) {
          this._transitionTo(CircuitState.OPEN);
        }
        break;

      case CircuitState.HALF_OPEN:
        // Failed during test - back to open
        this._transitionTo(CircuitState.OPEN);
        break;

      case CircuitState.OPEN:
        // Already open, just update failure time
        break;
    }

    this.emit('failure', {
      name: this._name,
      state: this._state,
      failureCount: this._failureCount,
      error: error?.message,
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // State Transitions
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Transition to a new state
   * @private
   */
  _transitionTo(newState) {
    const oldState = this._state;
    this._state = newState;
    this._stats.stateChanges++;
    this._stats.lastStateChange = Date.now();

    // Reset counters on state change
    if (newState === CircuitState.CLOSED) {
      this._failureCount = 0;
      this._successCount = 0;
      // v1.1: Reset backoff on successful recovery
      this._consecutiveOpenings = 0;
      this._currentResetTimeoutMs = this._config.resetTimeoutMs;
      this._stats.consecutiveOpenings = 0;
      this._stats.currentBackoffMs = this._config.resetTimeoutMs;
    } else if (newState === CircuitState.HALF_OPEN) {
      this._successCount = 0;
      this._halfOpenRequests = 0;
      // v1.1: Allow immediate first probe when entering half-open
      this._lastProbeTime = 0;
    } else if (newState === CircuitState.OPEN) {
      // v1.1: Exponential backoff - increase timeout each time circuit opens
      if (oldState === CircuitState.HALF_OPEN) {
        // Failed recovery - increase backoff
        this._consecutiveOpenings++;
        this._currentResetTimeoutMs = Math.min(
          this._config.resetTimeoutMs * Math.pow(this._config.backoffMultiplier, this._consecutiveOpenings),
          this._config.maxResetTimeoutMs,
        );
        this._stats.consecutiveOpenings = this._consecutiveOpenings;
        this._stats.currentBackoffMs = Math.floor(this._currentResetTimeoutMs);
      }
    }

    this.emit('stateChange', {
      name: this._name,
      from: oldState,
      to: newState,
      timestamp: Date.now(),
      backoffMs: this._currentResetTimeoutMs,
    });
  }

  /**
   * Manually reset the circuit breaker to CLOSED state
   *
   * Use with caution - only for manual recovery scenarios.
   */
  reset() {
    this._transitionTo(CircuitState.CLOSED);
    this._failureCount = 0;
    this._successCount = 0;
    this._lastFailureTime = 0;
    this._halfOpenRequests = 0;
    // v1.1: Reset backoff
    this._consecutiveOpenings = 0;
    this._currentResetTimeoutMs = this._config.resetTimeoutMs;
    this._stats.consecutiveOpenings = 0;
    this._stats.currentBackoffMs = this._config.resetTimeoutMs;
  }

  /**
   * Force the circuit to OPEN state
   *
   * Useful for maintenance or manual intervention.
   */
  trip() {
    this._transitionTo(CircuitState.OPEN);
    this._lastFailureTime = Date.now();
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Stats & Info
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get circuit breaker state for monitoring
   * @returns {Object} Current state info
   */
  getState() {
    const now = Date.now();
    const timeInState = this._stats.lastStateChange ? now - this._stats.lastStateChange : 0;
    const timeUntilProbe = this._state === CircuitState.OPEN
      ? Math.max(0, this._getTimeoutWithJitter() - (now - this._lastFailureTime))
      : 0;

    return {
      name: this._name,
      state: this._state,
      failureCount: this._failureCount,
      successCount: this._successCount,
      lastFailureTime: this._lastFailureTime,
      // v1.1: Backoff info
      consecutiveOpenings: this._consecutiveOpenings,
      currentBackoffMs: Math.floor(this._currentResetTimeoutMs),
      timeInStateMs: timeInState,
      timeUntilProbeMs: Math.floor(timeUntilProbe),
      config: { ...this._config },
    };
  }

  /**
   * Get circuit breaker statistics
   * @returns {Object} Stats
   */
  getStats() {
    const failureRate = this._stats.totalRequests > 0
      ? this._stats.totalFailures / this._stats.totalRequests
      : 0;

    return {
      ...this._stats,
      failureRate: Math.round(failureRate * 1000) / 1000,
      currentState: this._state,
    };
  }
}

/**
 * Execute a function with circuit breaker protection
 *
 * @param {CircuitBreaker} breaker - Circuit breaker instance
 * @param {Function} fn - Async function to execute
 * @param {Object} [options] - Options
 * @param {*} [options.fallback] - Fallback value if circuit is open
 * @param {boolean} [options.throwOnOpen=true] - Throw error if circuit is open
 * @param {boolean} [options.probeFirst=false] - v1.1: Run health probe before execution in half-open
 * @returns {Promise<*>} Result or fallback
 * @throws {Error} If circuit is open and throwOnOpen is true
 *
 * @example
 * const result = await withCircuitBreaker(breaker, async () => {
 *   return await database.query('SELECT * FROM users');
 * }, { fallback: [] });
 */
export async function withCircuitBreaker(breaker, fn, options = {}) {
  const { fallback, throwOnOpen = true, probeFirst = false } = options;

  if (!breaker.canExecute()) {
    if (throwOnOpen) {
      const error = new Error(`Circuit breaker "${breaker.name}" is open`);
      error.code = 'CIRCUIT_OPEN';
      error.state = breaker.getState();
      throw error;
    }
    return fallback;
  }

  // v1.1: Optional health probe before execution in half-open
  if (probeFirst && breaker.isHalfOpen && breaker.canProbe()) {
    const healthy = await breaker.probeHealth();
    if (!healthy) {
      if (throwOnOpen) {
        const error = new Error(`Circuit breaker "${breaker.name}" health probe failed`);
        error.code = 'CIRCUIT_PROBE_FAILED';
        error.state = breaker.getState();
        throw error;
      }
      return fallback;
    }
  }

  try {
    const result = await fn();
    breaker.recordSuccess();
    return result;
  } catch (error) {
    breaker.recordFailure(error);
    throw error;
  }
}

export default CircuitBreaker;
