/**
 * RetryPolicy - Automatic retry logic for transient failures
 *
 * Implements exponential backoff with jitter to handle:
 * - LLM API rate limits and network errors
 * - Database connection timeouts
 * - External API transient failures
 *
 * Integrates with CircuitBreaker for intelligent failure handling.
 *
 * "Persistence is cynical - try again, but don't be stupid about it" - κυνικός
 *
 * @module @cynic/core/retry
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV } from '../axioms/constants.js';
import { CircuitState } from '../circuit-breaker.js';
import { createLogger } from '../logger.js';

const log = createLogger('RetryPolicy');

/**
 * Default retry configuration (φ-derived)
 */
export const DEFAULT_RETRY_CONFIG = Object.freeze({
  maxRetries: 3,              // Maximum retry attempts
  baseDelayMs: 1000,          // Base delay (1 second)
  maxDelayMs: 32000,          // Max delay (32 seconds, 2^5 * 1000)
  multiplier: 2,              // Exponential multiplier
  jitterFactor: 0.2,          // ±20% jitter to prevent thundering herd
  retryableStatusCodes: [408, 429, 500, 502, 503, 504], // HTTP status codes to retry
  retryableErrorCodes: [
    'ECONNRESET',
    'ECONNREFUSED',
    'ETIMEDOUT',
    'ENOTFOUND',
    'EAI_AGAIN',
    'EPIPE',
  ],
});

/**
 * RetryPolicy - Manages retry logic with exponential backoff
 *
 * @extends EventEmitter
 * @fires RetryPolicy#retry - When a retry is attempted
 * @fires RetryPolicy#exhausted - When all retries are exhausted
 * @fires RetryPolicy#success - When operation succeeds after retry
 */
export class RetryPolicy extends EventEmitter {
  constructor(options = {}) {
    super();

    this._config = {
      ...DEFAULT_RETRY_CONFIG,
      ...options,
    };

    this._circuitBreaker = options.circuitBreaker || null;
    this._shouldRetryFn = options.shouldRetry || this._defaultShouldRetry.bind(this);

    this._stats = {
      totalAttempts: 0,
      totalRetries: 0,
      successAfterRetry: 0,
      exhausted: 0,
      circuitBreakerSkips: 0,
    };
  }

  get maxRetries() {
    return this._config.maxRetries;
  }

  get circuitBreaker() {
    return this._circuitBreaker;
  }

  async execute(fn, options = {}) {
    const { context = {}, onRetry, throwOnExhaustion = true } = options;
    let attempt = 0;
    let lastError = null;

    this._stats.totalAttempts++;

    while (attempt <= this._config.maxRetries) {
      try {
        if (this._circuitBreaker && !this._circuitBreaker.canExecute()) {
          this._stats.circuitBreakerSkips++;
          const error = new Error(`Circuit breaker is open`);
          error.code = 'CIRCUIT_OPEN';
          error.circuitState = this._circuitBreaker.state;
          throw error;
        }

        const result = await fn();

        if (attempt > 0) {
          this._stats.successAfterRetry++;
          this.emit('success', { attempt, totalAttempts: attempt + 1, context });
        }

        if (this._circuitBreaker) {
          this._circuitBreaker.recordSuccess();
        }

        return result;

      } catch (error) {
        lastError = error;

        if (this._circuitBreaker) {
          this._circuitBreaker.recordFailure(error);
        }

        const shouldRetry = this._shouldRetryFn(error, attempt);
        const hasMoreAttempts = attempt < this._config.maxRetries;

        if (!shouldRetry || !hasMoreAttempts) {
          if (attempt > 0) {
            this._stats.exhausted++;
            this.emit('exhausted', { error, attempts: attempt + 1, context });
          }

          if (throwOnExhaustion) {
            error.retryAttempts = attempt + 1;
            error.retryExhausted = true;
            throw error;
          }

          return undefined;
        }

        const delay = this._calculateDelay(attempt);

        this._stats.totalRetries++;
        this.emit('retry', { attempt, totalAttempts: attempt + 1, delay, error: error.message, context });

        if (onRetry) {
          await onRetry({ attempt, delay, error });
        }

        await this._sleep(delay);
        attempt++;
      }
    }

    if (throwOnExhaustion && lastError) {
      lastError.retryAttempts = attempt;
      lastError.retryExhausted = true;
      throw lastError;
    }

    return undefined;
  }

  _calculateDelay(attempt) {
    const { baseDelayMs, maxDelayMs, multiplier, jitterFactor } = this._config;
    const exponentialDelay = baseDelayMs * Math.pow(multiplier, attempt);
    const cappedDelay = Math.min(exponentialDelay, maxDelayMs);
    const jitterMultiplier = 1 + (Math.random() * 2 - 1) * jitterFactor;
    return Math.floor(cappedDelay * jitterMultiplier);
  }

  _defaultShouldRetry(error, attempt) {
    if (error.code === 'CIRCUIT_OPEN') return false;
    if (error.code === 'BUDGET_EXCEEDED' || error.code === 'BUDGET_EXHAUSTED') return false;
    if (error.status && this._config.retryableStatusCodes.includes(error.status)) return true;
    if (error.code && this._config.retryableErrorCodes.includes(error.code)) return true;
    if (error.message && error.message.toLowerCase().includes('timeout')) return true;
    if (error.message && error.message.toLowerCase().includes('rate limit')) return true;
    return false;
  }

  _sleep(ms) {
    return new Promise(resolve => setTimeout(resolve, ms));
  }

  getStats() {
    const retryRate = this._stats.totalAttempts > 0 ? this._stats.totalRetries / this._stats.totalAttempts : 0;
    const successRate = this._stats.totalRetries > 0 ? this._stats.successAfterRetry / this._stats.totalRetries : 0;
    return {
      ...this._stats,
      retryRate: Math.round(retryRate * 1000) / 1000,
      successRate: Math.round(successRate * 1000) / 1000,
    };
  }

  resetStats() {
    this._stats = {
      totalAttempts: 0,
      totalRetries: 0,
      successAfterRetry: 0,
      exhausted: 0,
      circuitBreakerSkips: 0,
    };
  }
}

export function createRetryPolicy(options = {}) {
  return new RetryPolicy(options);
}

export async function withRetry(fn, options = {}) {
  const policy = new RetryPolicy(options);
  return await policy.execute(fn, { throwOnExhaustion: true });
}

export class RetryPolicyRegistry {
  constructor() {
    this._policies = new Map();
  }

  get(name, options = {}) {
    if (!this._policies.has(name)) {
      this._policies.set(name, new RetryPolicy(options));
    }
    return this._policies.get(name);
  }

  has(name) {
    return this._policies.has(name);
  }

  remove(name) {
    return this._policies.delete(name);
  }

  getAllStats() {
    const stats = {};
    for (const [name, policy] of this._policies) {
      stats[name] = policy.getStats();
    }
    return stats;
  }

  resetAllStats() {
    for (const policy of this._policies.values()) {
      policy.resetStats();
    }
  }

  get size() {
    return this._policies.size;
  }
}

let _globalRegistry = null;

export function getRetryPolicyRegistry() {
  if (!_globalRegistry) {
    _globalRegistry = new RetryPolicyRegistry();
  }
  return _globalRegistry;
}

export function createRetryPolicyWithCircuitBreaker(options = {}) {
  const { circuitBreaker, maxRetries, ...retryOptions } = options;
  if (!circuitBreaker) throw new Error('circuitBreaker is required');
  return new RetryPolicy({
    ...retryOptions,
    maxRetries: maxRetries !== undefined ? maxRetries : 3,
    circuitBreaker,
  });
}

export default RetryPolicy;
