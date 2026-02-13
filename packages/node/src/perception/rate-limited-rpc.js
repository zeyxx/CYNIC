/**
 * Rate-Limited RPC Client for Solana
 *
 * "φ respects the rate limits" - κυνικός
 *
 * Features:
 * - Client-side rate limiting (5 req/s default)
 * - Multiple RPC endpoint fallback
 * - Exponential backoff on 429 errors
 * - Request queuing
 * - Health monitoring per endpoint
 *
 * @module @cynic/node/perception/rate-limited-rpc
 */

'use strict';

import { Connection } from '@solana/web3.js';
import { createLogger } from '@cynic/core';

const log = createLogger('RateLimitedRPC');

// Rate limiting constants
const DEFAULT_RPS = 5; // Requests per second
const BACKOFF_BASE_MS = 1000; // 1 second base backoff
const MAX_BACKOFF_MS = 32000; // 32 seconds max backoff
const QUEUE_MAX_SIZE = 1000; // Max queued requests

/**
 * Rate-Limited RPC Client
 *
 * Wraps Solana Connection with rate limiting and fallback.
 */
export class RateLimitedRPC {
  constructor(options = {}) {
    this.endpoints = options.endpoints || [];
    this.rps = options.rps || DEFAULT_RPS;
    this.commitment = options.commitment || 'confirmed';

    // Current connection and endpoint
    this.currentEndpoint = 0;
    this.connection = null;

    // Rate limiting
    this.queue = [];
    this.processing = false;
    this.lastRequestTime = 0;
    this.requestDelay = 1000 / this.rps; // ms between requests

    // Backoff per endpoint
    this.backoffs = new Map(); // endpoint -> { count, nextAvailableAt }

    // Stats
    this.stats = {
      requests: 0,
      successes: 0,
      failures: 0,
      rateLimited: 0,
      fallbacks: 0,
      queuedRequests: 0,
    };

    // Health per endpoint
    this.endpointHealth = new Map(); // endpoint -> { requests, failures, lastSuccess }
  }

  /**
   * Initialize connection to first available endpoint
   */
  async initialize() {
    if (this.endpoints.length === 0) {
      throw new Error('No RPC endpoints configured');
    }

    for (let i = 0; i < this.endpoints.length; i++) {
      try {
        await this._connectToEndpoint(i);
        log.info('Connected to RPC', { endpoint: this._maskUrl(this.endpoints[i]) });
        return;
      } catch (error) {
        log.warn('Failed to connect to endpoint', {
          endpoint: this._maskUrl(this.endpoints[i]),
          error: error.message,
        });
      }
    }

    throw new Error('Failed to connect to any RPC endpoint');
  }

  /**
   * Execute RPC request with rate limiting and fallback
   *
   * @param {Function} fn - Function that calls RPC (receives Connection)
   * @returns {Promise<any>} RPC result
   */
  async execute(fn) {
    return new Promise((resolve, reject) => {
      // Check queue size
      if (this.queue.length >= QUEUE_MAX_SIZE) {
        this.stats.failures++;
        return reject(new Error('RPC queue full'));
      }

      // Add to queue
      this.queue.push({ fn, resolve, reject });
      this.stats.queuedRequests++;

      // Start processing if not already
      if (!this.processing) {
        this._processQueue();
      }
    });
  }

  /**
   * Get current endpoint URL (for logging)
   */
  getCurrentEndpoint() {
    return this.endpoints[this.currentEndpoint];
  }

  /**
   * Get stats
   */
  getStats() {
    return {
      ...this.stats,
      queueSize: this.queue.length,
      currentEndpoint: this._maskUrl(this.getCurrentEndpoint()),
      endpointHealth: Array.from(this.endpointHealth.entries()).map(([endpoint, health]) => ({
        endpoint: this._maskUrl(endpoint),
        successRate: health.requests > 0 ? (health.requests - health.failures) / health.requests : 0,
        ...health,
      })),
    };
  }

  // ============================================================================
  // PRIVATE METHODS
  // ============================================================================

  /**
   * Process queued requests with rate limiting
   * @private
   */
  async _processQueue() {
    if (this.processing || this.queue.length === 0) {
      return;
    }

    this.processing = true;

    while (this.queue.length > 0) {
      // Rate limiting: wait if needed
      const now = Date.now();
      const timeSinceLastRequest = now - this.lastRequestTime;
      if (timeSinceLastRequest < this.requestDelay) {
        await new Promise(r => setTimeout(r, this.requestDelay - timeSinceLastRequest));
      }

      // Get next request
      const request = this.queue.shift();
      if (!request) break;

      this.lastRequestTime = Date.now();
      this.stats.requests++;

      try {
        // Execute with current connection
        const result = await this._executeWithFallback(request.fn);
        this.stats.successes++;
        request.resolve(result);
      } catch (error) {
        this.stats.failures++;
        request.reject(error);
      }
    }

    this.processing = false;
  }

  /**
   * Execute request with fallback on failure
   * @private
   */
  async _executeWithFallback(fn, attemptCount = 0) {
    const endpoint = this.endpoints[this.currentEndpoint];

    // Check if endpoint is in backoff
    const backoff = this.backoffs.get(endpoint);
    if (backoff && Date.now() < backoff.nextAvailableAt) {
      // Try next endpoint
      await this._fallbackToNextEndpoint();
      return this._executeWithFallback(fn, attemptCount);
    }

    try {
      const result = await fn(this.connection);

      // Success - record health
      this._recordEndpointHealth(endpoint, true);

      // Reset backoff
      this.backoffs.delete(endpoint);

      return result;
    } catch (error) {
      // Record failure
      this._recordEndpointHealth(endpoint, false);

      // Check if rate limited (429)
      if (error.message && (error.message.includes('429') || error.message.includes('rate limit'))) {
        this.stats.rateLimited++;
        log.warn('Rate limited by RPC', { endpoint: this._maskUrl(endpoint) });

        // Apply exponential backoff
        const currentBackoff = backoff?.count || 0;
        const backoffMs = Math.min(BACKOFF_BASE_MS * Math.pow(2, currentBackoff), MAX_BACKOFF_MS);
        this.backoffs.set(endpoint, {
          count: currentBackoff + 1,
          nextAvailableAt: Date.now() + backoffMs,
        });

        // Try next endpoint
        await this._fallbackToNextEndpoint();
        return this._executeWithFallback(fn, attemptCount + 1);
      }

      // Other error - try fallback if attempts left
      if (attemptCount < this.endpoints.length - 1) {
        log.warn('RPC error, trying fallback', {
          endpoint: this._maskUrl(endpoint),
          error: error.message,
        });

        await this._fallbackToNextEndpoint();
        return this._executeWithFallback(fn, attemptCount + 1);
      }

      // All endpoints failed
      throw error;
    }
  }

  /**
   * Connect to specific endpoint
   * @private
   */
  async _connectToEndpoint(index) {
    const endpoint = this.endpoints[index];

    this.connection = new Connection(endpoint, {
      commitment: this.commitment,
    });

    // Test connection
    await this.connection.getSlot();

    this.currentEndpoint = index;

    // Initialize health tracking
    if (!this.endpointHealth.has(endpoint)) {
      this.endpointHealth.set(endpoint, {
        requests: 0,
        failures: 0,
        lastSuccess: null,
      });
    }
  }

  /**
   * Fallback to next endpoint
   * @private
   */
  async _fallbackToNextEndpoint() {
    const nextIndex = (this.currentEndpoint + 1) % this.endpoints.length;

    if (nextIndex === this.currentEndpoint) {
      // Only one endpoint, can't fallback
      return;
    }

    log.info('Falling back to next endpoint', {
      from: this._maskUrl(this.endpoints[this.currentEndpoint]),
      to: this._maskUrl(this.endpoints[nextIndex]),
    });

    this.stats.fallbacks++;

    try {
      await this._connectToEndpoint(nextIndex);
    } catch (error) {
      log.error('Failed to connect to fallback endpoint', {
        endpoint: this._maskUrl(this.endpoints[nextIndex]),
        error: error.message,
      });
      throw error;
    }
  }

  /**
   * Record endpoint health
   * @private
   */
  _recordEndpointHealth(endpoint, success) {
    const health = this.endpointHealth.get(endpoint) || {
      requests: 0,
      failures: 0,
      lastSuccess: null,
    };

    health.requests++;
    if (success) {
      health.lastSuccess = Date.now();
    } else {
      health.failures++;
    }

    this.endpointHealth.set(endpoint, health);
  }

  /**
   * Mask sensitive parts of URL (API keys)
   * @private
   */
  _maskUrl(url) {
    if (!url) return 'unknown';
    return url.replace(/([?&](api[kK]ey|token)=)[^&]+/g, '$1***');
  }
}

export default RateLimitedRPC;
