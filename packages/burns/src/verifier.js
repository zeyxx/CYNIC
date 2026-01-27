/**
 * @cynic/burns - Burn Verifier
 *
 * Verifies burns via:
 * 1. Direct Solana on-chain verification (preferred)
 * 2. External API fallback (alonisthe.dev/burns)
 *
 * "Onchain is truth - burns must be verified" - κυνικός
 *
 * @module @cynic/burns/verifier
 */

'use strict';

import { PHI_INV } from '@cynic/core';
import { SolanaBurnVerifier } from './solana-verifier.js';

/**
 * Burn verification result
 * @typedef {Object} BurnVerification
 * @property {boolean} verified - Whether burn was verified
 * @property {string} txSignature - Transaction signature
 * @property {number} amount - Burn amount (in lamports or tokens)
 * @property {string} [token] - Token mint address if SPL burn
 * @property {string} burner - Address that performed burn
 * @property {number} timestamp - Burn timestamp (unix)
 * @property {number} slot - Solana slot
 * @property {string} [error] - Error message if verification failed
 * @property {boolean} cached - Whether result was from cache
 */

/**
 * Burn status
 */
export const BurnStatus = {
  /** Burn verified on-chain */
  VERIFIED: 'VERIFIED',
  /** Burn pending verification */
  PENDING: 'PENDING',
  /** Burn not found */
  NOT_FOUND: 'NOT_FOUND',
  /** Verification failed */
  FAILED: 'FAILED',
  /** Invalid transaction (not a burn) */
  INVALID: 'INVALID',
};

/**
 * Default API configuration
 */
export const DEFAULT_CONFIG = {
  apiUrl: 'https://alonisthe.dev/burns',
  timeout: 10000, // 10 seconds
  retries: 3,
  cacheEnabled: true,
  cacheTtl: 24 * 60 * 60 * 1000, // 24 hours
  // Solana on-chain verification (preferred over API)
  solanaCluster: null, // Set to SolanaCluster.MAINNET for on-chain verification
  solanaCommitment: 'confirmed',
};

/**
 * Burn Verifier
 *
 * Verifies that a burn transaction actually occurred on-chain.
 *
 * Two modes:
 * 1. **On-chain** (preferred): Direct Solana RPC verification
 * 2. **API**: External API fallback (alonisthe.dev/burns)
 *
 * Use on-chain mode by setting `solanaCluster` in config.
 */
export class BurnVerifier {
  /**
   * @param {Object} config - Configuration
   * @param {string} [config.solanaCluster] - Solana cluster URL for on-chain verification
   * @param {string} [config.solanaCommitment] - Solana commitment level
   * @param {string} [config.apiUrl] - Burns API URL (fallback)
   * @param {number} [config.timeout] - Request timeout in ms
   * @param {number} [config.retries] - Number of retries
   * @param {boolean} [config.cacheEnabled] - Enable caching
   * @param {number} [config.cacheTtl] - Cache TTL in ms
   * @param {Function} [config.onVerify] - Callback on verification
   */
  constructor(config = {}) {
    this.apiUrl = config.apiUrl || DEFAULT_CONFIG.apiUrl;
    this.timeout = config.timeout || DEFAULT_CONFIG.timeout;
    this.retries = config.retries || DEFAULT_CONFIG.retries;
    this.cacheEnabled = config.cacheEnabled ?? DEFAULT_CONFIG.cacheEnabled;
    this.cacheTtl = config.cacheTtl || DEFAULT_CONFIG.cacheTtl;
    this.onVerify = config.onVerify;

    // ═══════════════════════════════════════════════════════════════════════
    // On-chain verification (preferred)
    // ═══════════════════════════════════════════════════════════════════════
    this.solanaCluster = config.solanaCluster || null;
    this.solanaVerifier = null;

    if (this.solanaCluster) {
      this.solanaVerifier = new SolanaBurnVerifier({
        cluster: this.solanaCluster,
        commitment: config.solanaCommitment || DEFAULT_CONFIG.solanaCommitment,
      });
      console.log(`🔥 Burns: On-chain verification enabled (${this.solanaCluster})`);
    }

    // Simple in-memory cache (used when no solana verifier)
    this.cache = new Map();

    // Stats
    this.stats = {
      totalVerified: 0,
      totalFailed: 0,
      totalCacheHits: 0,
      totalApiCalls: 0,
      totalOnchainCalls: 0,
    };
  }

  /**
   * Verify a burn transaction
   *
   * Uses on-chain verification if solanaCluster is configured,
   * otherwise falls back to external API.
   *
   * @param {string} txSignature - Solana transaction signature
   * @param {Object} [options] - Verification options
   * @param {boolean} [options.skipCache] - Skip cache lookup
   * @param {string} [options.expectedBurner] - Expected burner address
   * @param {number} [options.minAmount] - Minimum burn amount
   * @returns {Promise<BurnVerification>}
   */
  async verify(txSignature, options = {}) {
    // Validate signature format (base58, ~88 chars)
    if (!this._isValidSignature(txSignature)) {
      return {
        verified: false,
        txSignature,
        error: 'Invalid transaction signature format',
        cached: false,
      };
    }

    // ═══════════════════════════════════════════════════════════════════════
    // ON-CHAIN VERIFICATION (preferred)
    // ═══════════════════════════════════════════════════════════════════════
    if (this.solanaVerifier) {
      this.stats.totalOnchainCalls++;

      const result = await this.solanaVerifier.verify(txSignature, options);

      // Update stats
      if (result.verified) {
        this.stats.totalVerified++;
      } else {
        this.stats.totalFailed++;
      }

      // Callback
      if (this.onVerify) {
        this.onVerify(result);
      }

      return result;
    }

    // ═══════════════════════════════════════════════════════════════════════
    // API FALLBACK
    // ═══════════════════════════════════════════════════════════════════════

    // Check cache first
    if (this.cacheEnabled && !options.skipCache) {
      const cached = this._getCached(txSignature);
      if (cached) {
        this.stats.totalCacheHits++;
        return { ...cached, cached: true };
      }
    }

    // Call API
    try {
      const result = await this._callApi(txSignature);
      this.stats.totalApiCalls++;

      // Validate result against expectations
      if (options.expectedBurner && result.burner !== options.expectedBurner) {
        return {
          verified: false,
          txSignature,
          error: `Burner mismatch: expected ${options.expectedBurner}, got ${result.burner}`,
          cached: false,
        };
      }

      if (options.minAmount && result.amount < options.minAmount) {
        return {
          verified: false,
          txSignature,
          amount: result.amount,
          error: `Burn amount ${result.amount} below minimum ${options.minAmount}`,
          cached: false,
        };
      }

      // Cache successful verification
      if (this.cacheEnabled && result.verified) {
        this._setCached(txSignature, result);
      }

      // Update stats
      if (result.verified) {
        this.stats.totalVerified++;
      } else {
        this.stats.totalFailed++;
      }

      // Callback
      if (this.onVerify) {
        this.onVerify(result);
      }

      return { ...result, cached: false };
    } catch (error) {
      this.stats.totalFailed++;
      return {
        verified: false,
        txSignature,
        error: error.message,
        cached: false,
      };
    }
  }

  /**
   * Verify multiple burns in batch
   *
   * @param {string[]} txSignatures - Array of transaction signatures
   * @param {Object} [options] - Verification options
   * @returns {Promise<Map<string, BurnVerification>>}
   */
  async verifyBatch(txSignatures, options = {}) {
    const results = new Map();

    // Process in parallel with concurrency limit
    const batchSize = 5; // Max concurrent requests
    for (let i = 0; i < txSignatures.length; i += batchSize) {
      const batch = txSignatures.slice(i, i + batchSize);
      const batchResults = await Promise.all(
        batch.map((sig) => this.verify(sig, options))
      );

      batch.forEach((sig, idx) => {
        results.set(sig, batchResults[idx]);
      });
    }

    return results;
  }

  /**
   * Check if a burn was verified (cache-first)
   *
   * @param {string} txSignature - Transaction signature
   * @returns {boolean}
   */
  isVerified(txSignature) {
    const cached = this._getCached(txSignature);
    return cached?.verified === true;
  }

  /**
   * Get burn details from cache
   *
   * @param {string} txSignature - Transaction signature
   * @returns {BurnVerification|null}
   */
  getCached(txSignature) {
    return this._getCached(txSignature);
  }

  /**
   * Invalidate cache entry
   *
   * @param {string} txSignature - Transaction signature
   */
  invalidate(txSignature) {
    this.cache.delete(txSignature);
  }

  /**
   * Clear entire cache
   */
  clearCache() {
    this.cache.clear();
  }

  /**
   * Call the burns API
   * @param {string} txSignature - Transaction signature
   * @returns {Promise<BurnVerification>}
   * @private
   */
  async _callApi(txSignature) {
    const url = `${this.apiUrl}/verify/${txSignature}`;

    for (let attempt = 0; attempt < this.retries; attempt++) {
      try {
        const controller = new AbortController();
        const timeoutId = setTimeout(() => controller.abort(), this.timeout);

        const response = await fetch(url, {
          method: 'GET',
          headers: {
            Accept: 'application/json',
            'User-Agent': 'CYNIC/1.0',
          },
          signal: controller.signal,
        });

        clearTimeout(timeoutId);

        if (!response.ok) {
          if (response.status === 404) {
            return {
              verified: false,
              txSignature,
              error: 'Burn not found',
            };
          }
          throw new Error(`API error: ${response.status} ${response.statusText}`);
        }

        const data = await response.json();

        return {
          verified: data.verified === true,
          txSignature,
          amount: data.amount || 0,
          token: data.token || null,
          burner: data.burner || '',
          timestamp: data.timestamp || Date.now(),
          slot: data.slot || 0,
        };
      } catch (error) {
        if (error.name === 'AbortError') {
          error.message = 'Request timeout';
        }

        // Retry on network errors
        if (attempt < this.retries - 1) {
          // Exponential backoff with φ
          const delay = Math.round(PHI_INV * 1000 * Math.pow(2, attempt));
          await new Promise((r) => setTimeout(r, delay));
          continue;
        }

        throw error;
      }
    }
  }

  /**
   * Validate transaction signature format
   * @param {string} sig - Signature to validate
   * @returns {boolean}
   * @private
   */
  _isValidSignature(sig) {
    // Base58 characters: 1-9, A-H, J-N, P-Z, a-k, m-z (no 0, I, O, l)
    const base58Regex = /^[1-9A-HJ-NP-Za-km-z]{64,88}$/;
    return typeof sig === 'string' && base58Regex.test(sig);
  }

  /**
   * Get cached verification
   * @param {string} txSignature - Transaction signature
   * @returns {BurnVerification|null}
   * @private
   */
  _getCached(txSignature) {
    const entry = this.cache.get(txSignature);
    if (!entry) {
      return null;
    }

    // Check TTL
    if (Date.now() - entry.timestamp > this.cacheTtl) {
      this.cache.delete(txSignature);
      return null;
    }

    return entry.data;
  }

  /**
   * Set cached verification
   * @param {string} txSignature - Transaction signature
   * @param {BurnVerification} data - Verification data
   * @private
   */
  _setCached(txSignature, data) {
    this.cache.set(txSignature, {
      data,
      timestamp: Date.now(),
    });
  }

  /**
   * Get verifier statistics
   * @returns {Object}
   */
  getStats() {
    const stats = {
      ...this.stats,
      cacheSize: this.cache.size,
      apiUrl: this.apiUrl,
      cacheEnabled: this.cacheEnabled,
      onchainEnabled: !!this.solanaVerifier,
    };

    // Include Solana verifier stats if available
    if (this.solanaVerifier) {
      stats.solanaCluster = this.solanaCluster;
      stats.solanaCacheSize = this.solanaVerifier.cache.size;
    }

    return stats;
  }

  /**
   * Export verifier state
   * @returns {Object}
   */
  export() {
    return {
      cache: Array.from(this.cache.entries()),
      stats: { ...this.stats },
    };
  }

  /**
   * Import verifier state
   * @param {Object} state - Exported state
   */
  import(state) {
    if (state.cache) {
      this.cache = new Map(state.cache);
    }
    if (state.stats) {
      this.stats = { ...this.stats, ...state.stats };
    }
  }
}

/**
 * Create a burn verifier instance
 * @param {Object} [config] - Configuration
 * @returns {BurnVerifier}
 */
export function createBurnVerifier(config = {}) {
  return new BurnVerifier(config);
}
