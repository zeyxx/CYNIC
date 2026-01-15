/**
 * CYNIC Client
 *
 * HTTP client for interacting with CYNIC nodes
 *
 * Usage:
 * ```js
 * import { CYNICClient } from '@cynic/client';
 *
 * const client = new CYNICClient({ endpoint: 'http://localhost:3000' });
 *
 * // Submit K-Score
 * const result = await client.submitKScore(mint, { D: 0.85, O: 0.72, L: 0.91 });
 *
 * // Check health
 * const health = await client.health();
 * ```
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/client
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * CYNIC Client for API interactions
 */
export class CYNICClient {
  /**
   * Create CYNIC client
   * @param {Object} options - Client options
   * @param {string} [options.endpoint] - API endpoint (default: CYNIC_ENDPOINT env or http://localhost:3000)
   * @param {string} [options.apiKey] - API key for authentication
   * @param {number} [options.timeout=30000] - Request timeout in ms
   * @param {number} [options.retries=3] - Number of retries
   * @param {boolean} [options.verbose=false] - Enable verbose logging
   */
  constructor(options = {}) {
    this.endpoint = options.endpoint || process.env.CYNIC_ENDPOINT || 'http://localhost:3000';
    this.apiKey = options.apiKey || process.env.CYNIC_API_KEY || null;
    this.timeout = options.timeout || 30000;
    this.retries = options.retries || 3;
    this.verbose = options.verbose || false;

    // Remove trailing slash from endpoint
    if (this.endpoint.endsWith('/')) {
      this.endpoint = this.endpoint.slice(0, -1);
    }

    // Stats
    this.stats = {
      requests: 0,
      successes: 0,
      failures: 0,
      lastRequest: null,
    };
  }

  /**
   * Make HTTP request
   * @private
   */
  async _request(method, path, body = null) {
    const url = `${this.endpoint}${path}`;
    const headers = {
      'Content-Type': 'application/json',
    };

    if (this.apiKey) {
      headers['X-API-Key'] = this.apiKey;
    }

    const options = {
      method,
      headers,
      signal: AbortSignal.timeout(this.timeout),
    };

    if (body && method !== 'GET') {
      options.body = JSON.stringify(body);
    }

    this.stats.requests++;
    this.stats.lastRequest = Date.now();

    if (this.verbose) {
      console.log(`🐕 CYNIC Client: ${method} ${url}`);
    }

    let lastError;
    for (let attempt = 0; attempt < this.retries; attempt++) {
      try {
        const response = await fetch(url, options);
        const data = await response.json();

        if (!response.ok) {
          throw new Error(data.message || data.error || `HTTP ${response.status}`);
        }

        this.stats.successes++;
        return data;
      } catch (err) {
        lastError = err;
        if (this.verbose) {
          console.warn(`⚠️ CYNIC Client: Attempt ${attempt + 1} failed: ${err.message}`);
        }

        // Don't retry on client errors
        if (err.message.includes('400') || err.message.includes('401')) {
          break;
        }

        // Wait before retry (exponential backoff with φ)
        if (attempt < this.retries - 1) {
          await new Promise(r => setTimeout(r, 1000 * Math.pow(PHI_INV, this.retries - attempt)));
        }
      }
    }

    this.stats.failures++;
    throw lastError;
  }

  /**
   * Get node health
   * @returns {Promise<Object>} Health status
   */
  async health() {
    return this._request('GET', '/health');
  }

  /**
   * Get node info
   * @returns {Promise<Object>} Node info
   */
  async info() {
    return this._request('GET', '/info');
  }

  /**
   * Get consensus status
   * @returns {Promise<Object>} Consensus status
   */
  async consensusStatus() {
    return this._request('GET', '/consensus/status');
  }

  /**
   * Submit a general judgment
   * @param {Object} item - Item to judge
   * @param {Object} [context] - Judgment context
   * @returns {Promise<Object>} Judgment result
   */
  async judge(item, context = {}) {
    return this._request('POST', '/judge', {
      item,
      context,
    });
  }

  /**
   * Submit K-Score for consensus
   *
   * This is the main method HolDex will use to submit K-Scores
   *
   * @param {string} mint - Token mint address
   * @param {Object} components - K-Score components
   * @param {number} components.D - Diamond Hands (0-1)
   * @param {number} components.O - Organic Growth (0-1)
   * @param {number} components.L - Longevity (0-1)
   * @returns {Promise<Object>} K-Score result with consensus proof
   *
   * @example
   * ```js
   * const result = await client.submitKScore('So11...mint', {
   *   D: 0.85,  // 85% diamond hands
   *   O: 0.72,  // 72% organic growth
   *   L: 0.91   // 91% longevity
   * });
   *
   * console.log(result.kScore);      // 82.47
   * console.log(result.tier);        // "PLATINUM"
   * console.log(result.merkleProof); // Proof of inclusion
   * ```
   */
  async submitKScore(mint, components) {
    if (!mint) {
      throw new Error('Missing required parameter: mint');
    }

    if (!components || typeof components.D !== 'number' || typeof components.O !== 'number' || typeof components.L !== 'number') {
      throw new Error('Invalid components: {D, O, L} required as numbers');
    }

    // Validate ranges
    const { D, O, L } = components;
    if (D < 0 || D > 1 || O < 0 || O > 1 || L < 0 || L > 1) {
      throw new Error('Component values must be in range [0, 1]');
    }

    return this._request('POST', '/judge/kscore', {
      mint,
      components: {
        D: parseFloat(D.toFixed(6)),
        O: parseFloat(O.toFixed(6)),
        L: parseFloat(L.toFixed(6)),
      },
    });
  }

  /**
   * Get Merkle proof for a hash
   * @param {string} hash - Hash to get proof for
   * @returns {Promise<Object>} Merkle proof
   */
  async getMerkleProof(hash) {
    return this._request('GET', `/merkle/proof/${hash}`);
  }

  /**
   * Wait for K-Score finalization
   *
   * Polls the node until the K-Score is finalized
   *
   * @param {string} requestId - Request ID from submitKScore
   * @param {Object} [options] - Options
   * @param {number} [options.timeout=60000] - Max wait time in ms
   * @param {number} [options.pollInterval=1000] - Poll interval in ms
   * @returns {Promise<Object>} Finalized result
   */
  async waitForFinality(requestId, options = {}) {
    const timeout = options.timeout || 60000;
    const pollInterval = options.pollInterval || 1000;
    const startTime = Date.now();

    while (Date.now() - startTime < timeout) {
      try {
        const status = await this.consensusStatus();

        // If height has advanced, the request is likely finalized
        // In a full implementation, we'd track the specific request
        if (status.height > 0) {
          return {
            requestId,
            status: 'finalized',
            height: status.height,
            timestamp: Date.now(),
          };
        }
      } catch (err) {
        if (this.verbose) {
          console.warn(`⚠️ Finality poll failed: ${err.message}`);
        }
      }

      await new Promise(r => setTimeout(r, pollInterval));
    }

    throw new Error(`Timeout waiting for finality of ${requestId}`);
  }

  /**
   * Get client statistics
   * @returns {Object} Request statistics
   */
  getStats() {
    return {
      ...this.stats,
      successRate: this.stats.requests > 0 ? this.stats.successes / this.stats.requests : 0,
      endpoint: this.endpoint,
    };
  }

  /**
   * Create multiple clients for redundancy
   * @param {string[]} endpoints - Array of endpoints
   * @param {Object} [options] - Client options
   * @returns {CYNICClient[]} Array of clients
   */
  static createPool(endpoints, options = {}) {
    return endpoints.map(endpoint => new CYNICClient({ ...options, endpoint }));
  }

  /**
   * Submit K-Score to multiple nodes for redundancy
   *
   * Submits to all nodes and returns the first successful response
   *
   * @param {CYNICClient[]} clients - Array of clients
   * @param {string} mint - Token mint address
   * @param {Object} components - K-Score components
   * @returns {Promise<Object>} First successful result
   */
  static async submitKScoreRedundant(clients, mint, components) {
    const results = await Promise.allSettled(
      clients.map(client => client.submitKScore(mint, components))
    );

    // Return first success
    for (const result of results) {
      if (result.status === 'fulfilled') {
        return result.value;
      }
    }

    // All failed
    const errors = results.map(r => r.reason?.message).filter(Boolean);
    throw new Error(`All ${clients.length} nodes failed: ${errors.join(', ')}`);
  }
}

/**
 * Create a preconfigured client for HolDex
 * @param {Object} [options] - Options
 * @returns {CYNICClient} Configured client
 */
export function createHolDexClient(options = {}) {
  return new CYNICClient({
    endpoint: options.endpoint || process.env.CYNIC_ENDPOINT || 'http://localhost:3000',
    apiKey: options.apiKey || process.env.CYNIC_API_KEY,
    timeout: 30000,
    retries: 3,
    verbose: options.verbose || process.env.CYNIC_VERBOSE === 'true',
  });
}

export default CYNICClient;
