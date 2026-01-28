/**
 * CircuitBreaker - Resilience Pattern for Orchestration
 *
 * Prevents cascading failures by:
 * - Tracking success/failure rates
 * - Opening circuit when failures exceed threshold
 * - Allowing recovery after timeout
 *
 * φ-aligned thresholds:
 * - Failure threshold: 61.8% (φ⁻¹)
 * - Recovery timeout: 5s (Fibonacci)
 * - Sample window: 13 calls (Fibonacci)
 *
 * "φ protects the whole" - κυνικός
 *
 * @module @cynic/node/orchestration/circuit-breaker
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('CircuitBreaker');

/**
 * Circuit states
 * @enum {string}
 */
export const CircuitState = {
  CLOSED: 'closed',     // Normal operation
  OPEN: 'open',         // Failing, reject calls
  HALF_OPEN: 'half_open', // Testing recovery
};

/**
 * CircuitBreaker - Manages circuit state for a service
 */
export class CircuitBreaker {
  /**
   * Create a circuit breaker
   *
   * @param {Object} options - Configuration
   * @param {string} [options.name] - Circuit name for logging
   * @param {number} [options.failureThreshold] - Failure rate to open (default: 0.618)
   * @param {number} [options.successThreshold] - Success rate to close from half-open (default: 0.618)
   * @param {number} [options.timeout] - Time in ms before half-open (default: 5000)
   * @param {number} [options.sampleWindow] - Calls to consider (default: 13)
   * @param {number} [options.minSamples] - Min samples before checking (default: 5)
   * @param {Function} [options.onStateChange] - Callback on state change
   */
  constructor(options = {}) {
    this.name = options.name || 'default';
    this.failureThreshold = options.failureThreshold || PHI_INV; // 61.8%
    this.successThreshold = options.successThreshold || PHI_INV;
    this.timeout = options.timeout || 5000; // 5s Fibonacci
    this.sampleWindow = options.sampleWindow || 13; // Fibonacci
    this.minSamples = options.minSamples || 5; // Fibonacci

    this.onStateChange = options.onStateChange || null;

    // State
    this._state = CircuitState.CLOSED;
    this._lastStateChange = Date.now();
    this._openedAt = null;

    // Sliding window of recent results (true = success, false = failure)
    this._results = [];

    // Statistics
    this.stats = {
      totalCalls: 0,
      successes: 0,
      failures: 0,
      rejectedCalls: 0,
      stateChanges: 0,
      lastFailure: null,
      lastSuccess: null,
    };

    log.debug('CircuitBreaker created', {
      name: this.name,
      failureThreshold: this.failureThreshold,
      timeout: this.timeout,
    });
  }

  /**
   * Get current state
   * @returns {CircuitState}
   */
  get state() {
    // Check if we should transition from OPEN to HALF_OPEN
    if (this._state === CircuitState.OPEN) {
      const elapsed = Date.now() - this._openedAt;
      if (elapsed >= this.timeout) {
        this._transition(CircuitState.HALF_OPEN);
      }
    }
    return this._state;
  }

  /**
   * Check if circuit allows calls
   * @returns {boolean}
   */
  isAllowed() {
    const currentState = this.state; // triggers timeout check

    if (currentState === CircuitState.CLOSED) {
      return true;
    }

    if (currentState === CircuitState.HALF_OPEN) {
      // Allow a single test call in half-open
      return true;
    }

    // OPEN - reject
    this.stats.rejectedCalls++;
    return false;
  }

  /**
   * Execute a function with circuit breaker protection
   *
   * @param {Function} fn - Async function to execute
   * @param {*} fallback - Value to return if circuit is open
   * @returns {Promise<*>} Result or fallback
   */
  async execute(fn, fallback = null) {
    if (!this.isAllowed()) {
      log.warn('Circuit breaker rejecting call', { name: this.name, state: this._state });
      return fallback;
    }

    try {
      const result = await fn();
      this.recordSuccess();
      return result;
    } catch (error) {
      this.recordFailure(error);
      throw error;
    }
  }

  /**
   * Record a successful call
   */
  recordSuccess() {
    this.stats.totalCalls++;
    this.stats.successes++;
    this.stats.lastSuccess = Date.now();

    this._results.push(true);
    this._trimResults();

    // If half-open and success, might close
    if (this._state === CircuitState.HALF_OPEN) {
      const successRate = this._getSuccessRate();
      if (successRate >= this.successThreshold) {
        this._transition(CircuitState.CLOSED);
      }
    }

    log.trace('Success recorded', { name: this.name, state: this._state });
  }

  /**
   * Record a failed call
   *
   * @param {Error} [error] - The error that occurred
   */
  recordFailure(error = null) {
    this.stats.totalCalls++;
    this.stats.failures++;
    this.stats.lastFailure = Date.now();

    this._results.push(false);
    this._trimResults();

    // Check if we should open the circuit
    if (this._state === CircuitState.CLOSED) {
      const failureRate = this._getFailureRate();
      if (this._results.length >= this.minSamples && failureRate >= this.failureThreshold) {
        this._transition(CircuitState.OPEN);
        log.warn('Circuit breaker opened', {
          name: this.name,
          failureRate,
          error: error?.message,
        });
      }
    }

    // If half-open and failure, reopen
    if (this._state === CircuitState.HALF_OPEN) {
      this._transition(CircuitState.OPEN);
      log.warn('Circuit breaker re-opened after half-open failure', {
        name: this.name,
        error: error?.message,
      });
    }

    log.trace('Failure recorded', { name: this.name, state: this._state });
  }

  /**
   * Manually reset the circuit breaker
   */
  reset() {
    this._results = [];
    this._transition(CircuitState.CLOSED);
    log.debug('Circuit breaker reset', { name: this.name });
  }

  /**
   * Force the circuit open (for testing or manual override)
   */
  forceOpen() {
    this._transition(CircuitState.OPEN);
    log.warn('Circuit breaker force-opened', { name: this.name });
  }

  /**
   * Get current statistics
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.stats,
      state: this._state,
      failureRate: this._getFailureRate(),
      successRate: this._getSuccessRate(),
      resultsInWindow: this._results.length,
      isAllowed: this.isAllowed(),
      timeSinceOpen: this._openedAt ? Date.now() - this._openedAt : null,
    };
  }

  /**
   * Get health status
   * @returns {Object}
   */
  getHealth() {
    const failureRate = this._getFailureRate();
    const isHealthy = this._state === CircuitState.CLOSED && failureRate < this.failureThreshold * 0.5;

    return {
      name: this.name,
      state: this._state,
      healthy: isHealthy,
      failureRate,
      message: this._state === CircuitState.OPEN
        ? `Circuit open (failure rate: ${(failureRate * 100).toFixed(1)}%)`
        : this._state === CircuitState.HALF_OPEN
          ? 'Circuit testing recovery'
          : `Circuit healthy (failure rate: ${(failureRate * 100).toFixed(1)}%)`,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PRIVATE METHODS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Transition to new state
   * @private
   */
  _transition(newState) {
    if (this._state === newState) return;

    const oldState = this._state;
    this._state = newState;
    this._lastStateChange = Date.now();
    this.stats.stateChanges++;

    if (newState === CircuitState.OPEN) {
      this._openedAt = Date.now();
    }

    if (newState === CircuitState.HALF_OPEN) {
      // Clear results when entering half-open to test recovery cleanly
      this._results = [];
    }

    if (newState === CircuitState.CLOSED) {
      // Clear results on close to start fresh
      this._results = [];
    }

    log.debug('Circuit state changed', {
      name: this.name,
      from: oldState,
      to: newState,
    });

    if (this.onStateChange) {
      try {
        this.onStateChange(oldState, newState, this);
      } catch (err) {
        log.warn('State change callback error', { error: err.message });
      }
    }
  }

  /**
   * Trim results to sample window
   * @private
   */
  _trimResults() {
    while (this._results.length > this.sampleWindow) {
      this._results.shift();
    }
  }

  /**
   * Get failure rate from recent results
   * @private
   * @returns {number} Failure rate (0-1)
   */
  _getFailureRate() {
    if (this._results.length === 0) return 0;
    const failures = this._results.filter(r => !r).length;
    return failures / this._results.length;
  }

  /**
   * Get success rate from recent results
   * @private
   * @returns {number} Success rate (0-1)
   */
  _getSuccessRate() {
    if (this._results.length === 0) return 1;
    const successes = this._results.filter(r => r).length;
    return successes / this._results.length;
  }
}

/**
 * Create a circuit breaker
 *
 * @param {Object} options - Configuration
 * @returns {CircuitBreaker}
 */
export function createCircuitBreaker(options) {
  return new CircuitBreaker(options);
}

/**
 * CircuitBreakerRegistry - Manages multiple circuit breakers
 */
export class CircuitBreakerRegistry {
  constructor() {
    this._breakers = new Map();
  }

  /**
   * Get or create a circuit breaker
   *
   * @param {string} name - Circuit name
   * @param {Object} [options] - Options for new breaker
   * @returns {CircuitBreaker}
   */
  get(name, options = {}) {
    if (!this._breakers.has(name)) {
      this._breakers.set(name, new CircuitBreaker({ ...options, name }));
    }
    return this._breakers.get(name);
  }

  /**
   * Get all circuit breaker statuses
   * @returns {Object[]}
   */
  getAllHealth() {
    return Array.from(this._breakers.values()).map(cb => cb.getHealth());
  }

  /**
   * Get all statistics
   * @returns {Object}
   */
  getAllStats() {
    const stats = {};
    for (const [name, breaker] of this._breakers) {
      stats[name] = breaker.getStats();
    }
    return stats;
  }

  /**
   * Reset all circuit breakers
   */
  resetAll() {
    for (const breaker of this._breakers.values()) {
      breaker.reset();
    }
    log.debug('All circuit breakers reset');
  }
}

// Global registry singleton
let _globalRegistry = null;

/**
 * Get the global circuit breaker registry
 * @returns {CircuitBreakerRegistry}
 */
export function getCircuitBreakerRegistry() {
  if (!_globalRegistry) {
    _globalRegistry = new CircuitBreakerRegistry();
  }
  return _globalRegistry;
}

export default CircuitBreaker;
