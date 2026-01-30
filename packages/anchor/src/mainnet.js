/**
 * @cynic/anchor - Mainnet Configuration
 *
 * v1.1: Production-ready mainnet support with rate limiting,
 * failover, and priority fees.
 *
 * "Onchain is truth - but truth has costs" - κυνικός
 *
 * @module @cynic/anchor/mainnet
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import { SolanaCluster, ANCHOR_CONSTANTS } from './constants.js';

// =============================================================================
// MAINNET CONSTANTS
// =============================================================================

/**
 * Mainnet RPC providers (priority order)
 */
export const MAINNET_RPCS = Object.freeze({
  // Helius (best performance, rate-limited by API key)
  HELIUS: process.env.HELIUS_API_KEY
    ? `https://mainnet.helius-rpc.com/?api-key=${process.env.HELIUS_API_KEY}`
    : null,

  // QuickNode (if configured)
  QUICKNODE: process.env.QUICKNODE_RPC_URL || null,

  // Triton (if configured)
  TRITON: process.env.TRITON_RPC_URL || null,

  // Public RPC (fallback, heavily rate-limited)
  PUBLIC: SolanaCluster.MAINNET,
});

/**
 * Rate limiting constants (φ-aligned)
 */
export const RATE_LIMITS = Object.freeze({
  // Max requests per second for public RPC
  PUBLIC_RPS: 2,

  // Max requests per second for premium RPC
  PREMIUM_RPS: Math.floor(PHI_INV * 100), // 61 RPS

  // Cooldown after rate limit hit (ms)
  COOLDOWN_MS: Math.round(PHI_INV * 5000), // 3.09s

  // Max retries on rate limit
  MAX_RETRIES: Math.round(PHI_INV_2 * 10), // 4

  // Retry delay multiplier (exponential backoff)
  RETRY_MULTIPLIER: 1 + PHI_INV, // 1.618x
});

/**
 * Priority fee tiers (in micro-lamports per CU)
 */
export const PRIORITY_FEES = Object.freeze({
  // No priority (may take longer)
  NONE: 0,

  // Low priority (normal times)
  LOW: 1000,

  // Medium priority (moderate congestion)
  MEDIUM: 5000,

  // High priority (high congestion)
  HIGH: 25000,

  // Maximum priority (time-critical)
  MAX: 100000,

  // φ-aligned default (medium-low)
  DEFAULT: Math.round(PHI_INV * 5000), // ~3090 micro-lamports
});

/**
 * Compute budget for CYNIC anchor transactions
 */
export const COMPUTE_BUDGET = Object.freeze({
  // Max compute units for anchor transaction
  MAX_UNITS: 200000,

  // Default compute units (smaller = cheaper)
  DEFAULT_UNITS: 50000,
});

// =============================================================================
// RATE LIMITER
// =============================================================================

/**
 * Simple token bucket rate limiter
 */
export class RateLimiter {
  /**
   * @param {number} tokensPerSecond - Tokens to add per second
   * @param {number} maxTokens - Maximum tokens in bucket
   */
  constructor(tokensPerSecond, maxTokens = tokensPerSecond * 2) {
    this._tokensPerSecond = tokensPerSecond;
    this._maxTokens = maxTokens;
    this._tokens = maxTokens;
    this._lastRefill = Date.now();
  }

  /**
   * Refill tokens based on time elapsed
   * @private
   */
  _refill() {
    const now = Date.now();
    const elapsed = (now - this._lastRefill) / 1000;
    this._tokens = Math.min(
      this._maxTokens,
      this._tokens + elapsed * this._tokensPerSecond,
    );
    this._lastRefill = now;
  }

  /**
   * Try to consume a token
   * @returns {boolean} True if token consumed, false if rate limited
   */
  tryConsume() {
    this._refill();
    if (this._tokens >= 1) {
      this._tokens -= 1;
      return true;
    }
    return false;
  }

  /**
   * Wait until a token is available
   * @returns {Promise<void>}
   */
  async waitForToken() {
    while (!this.tryConsume()) {
      // Calculate wait time
      const waitMs = ((1 - this._tokens) / this._tokensPerSecond) * 1000;
      await new Promise(r => setTimeout(r, Math.max(10, Math.ceil(waitMs))));
    }
  }

  /**
   * Get current token count
   * @returns {number}
   */
  getTokens() {
    this._refill();
    return this._tokens;
  }
}

// =============================================================================
// RPC FAILOVER
// =============================================================================

/**
 * RPC endpoint with health tracking
 */
class RpcEndpoint {
  /**
   * @param {string} url - RPC URL
   * @param {boolean} isPremium - Whether this is a premium endpoint
   */
  constructor(url, isPremium = false) {
    this.url = url;
    this.isPremium = isPremium;
    this.failures = 0;
    this.lastFailure = null;
    this.rateLimiter = new RateLimiter(
      isPremium ? RATE_LIMITS.PREMIUM_RPS : RATE_LIMITS.PUBLIC_RPS,
    );
  }

  /**
   * Check if endpoint is healthy
   * @returns {boolean}
   */
  isHealthy() {
    // Endpoints become healthy again after cooldown
    if (this.failures > 0 && this.lastFailure) {
      const elapsed = Date.now() - this.lastFailure;
      const cooldown = RATE_LIMITS.COOLDOWN_MS * Math.pow(RATE_LIMITS.RETRY_MULTIPLIER, this.failures - 1);
      if (elapsed > cooldown) {
        // Reset after cooldown
        this.failures = Math.max(0, this.failures - 1);
        return this.failures < RATE_LIMITS.MAX_RETRIES;
      }
    }
    return this.failures < RATE_LIMITS.MAX_RETRIES;
  }

  /**
   * Mark a successful request
   */
  success() {
    this.failures = Math.max(0, this.failures - 1);
  }

  /**
   * Mark a failed request
   */
  failure() {
    this.failures++;
    this.lastFailure = Date.now();
  }
}

/**
 * RPC failover manager
 */
export class RpcFailover {
  /**
   * @param {Object} [options] - Options
   */
  constructor(options = {}) {
    this._endpoints = [];
    this._currentIndex = 0;

    // Add configured endpoints in priority order
    if (MAINNET_RPCS.HELIUS) {
      this._endpoints.push(new RpcEndpoint(MAINNET_RPCS.HELIUS, true));
    }
    if (MAINNET_RPCS.QUICKNODE) {
      this._endpoints.push(new RpcEndpoint(MAINNET_RPCS.QUICKNODE, true));
    }
    if (MAINNET_RPCS.TRITON) {
      this._endpoints.push(new RpcEndpoint(MAINNET_RPCS.TRITON, true));
    }
    if (options.includePublic !== false) {
      this._endpoints.push(new RpcEndpoint(MAINNET_RPCS.PUBLIC, false));
    }

    // Add custom endpoints if provided
    if (options.customRpcs) {
      for (const rpc of options.customRpcs) {
        this._endpoints.push(new RpcEndpoint(rpc.url, rpc.isPremium ?? false));
      }
    }

    if (this._endpoints.length === 0) {
      throw new Error('No RPC endpoints configured. Set HELIUS_API_KEY or another RPC env var.');
    }
  }

  /**
   * Get the next healthy endpoint
   * @returns {RpcEndpoint|null}
   */
  getEndpoint() {
    // Try from current position
    for (let i = 0; i < this._endpoints.length; i++) {
      const index = (this._currentIndex + i) % this._endpoints.length;
      const endpoint = this._endpoints[index];
      if (endpoint.isHealthy()) {
        this._currentIndex = index;
        return endpoint;
      }
    }
    return null;
  }

  /**
   * Execute a function with failover
   * @param {Function} fn - Function that takes RPC URL and returns promise
   * @returns {Promise<any>}
   */
  async execute(fn) {
    let lastError = null;
    const tried = new Set();

    for (let attempt = 0; attempt < this._endpoints.length * 2; attempt++) {
      const endpoint = this.getEndpoint();

      if (!endpoint) {
        throw new Error(`All RPC endpoints exhausted. Last error: ${lastError?.message || 'unknown'}`);
      }

      // Skip if we've already tried this endpoint in this execution
      if (tried.has(endpoint.url)) {
        // Move to next endpoint
        this._currentIndex = (this._currentIndex + 1) % this._endpoints.length;
        continue;
      }
      tried.add(endpoint.url);

      try {
        // Wait for rate limit
        await endpoint.rateLimiter.waitForToken();

        // Execute
        const result = await fn(endpoint.url);
        endpoint.success();
        return result;
      } catch (error) {
        lastError = error;
        endpoint.failure();

        // Move to next endpoint
        this._currentIndex = (this._currentIndex + 1) % this._endpoints.length;

        // Check if it's a rate limit error
        const isRateLimit = error.message?.includes('429') ||
                           error.message?.includes('rate limit') ||
                           error.message?.includes('Too Many Requests');

        if (isRateLimit) {
          // Longer cooldown for rate limits
          await new Promise(r => setTimeout(r, RATE_LIMITS.COOLDOWN_MS));
        }
      }
    }

    throw new Error(`All RPC endpoints tried. Last error: ${lastError?.message || 'unknown'}`);
  }

  /**
   * Get all endpoints status
   * @returns {Object[]}
   */
  getStatus() {
    return this._endpoints.map(e => ({
      url: e.url.replace(/api-key=[^&]+/, 'api-key=***'),
      isPremium: e.isPremium,
      failures: e.failures,
      healthy: e.isHealthy(),
      tokens: e.rateLimiter.getTokens(),
    }));
  }
}

// =============================================================================
// MAINNET CONFIG
// =============================================================================

/**
 * Mainnet-specific configuration
 */
export const MAINNET_CONFIG = Object.freeze({
  // Use mainnet cluster
  cluster: MAINNET_RPCS.HELIUS || MAINNET_RPCS.PUBLIC,

  // Safety: require explicit confirmation for mainnet
  requireConfirmation: true,

  // Use priority fees
  priorityFee: PRIORITY_FEES.DEFAULT,

  // Compute budget
  computeUnits: COMPUTE_BUDGET.DEFAULT_UNITS,

  // Longer anchor interval for mainnet (save costs)
  intervalMs: ANCHOR_CONSTANTS.ANCHOR_INTERVAL_MS * 2, // ~2 minutes

  // Larger batches for efficiency
  batchSize: ANCHOR_CONSTANTS.ANCHOR_BATCH_SIZE * 2, // 76 items

  // Wait for finalized confirmations
  commitment: 'finalized',

  // Skip preflight for speed (we validate ourselves)
  skipPreflight: false,
});

/**
 * Create mainnet anchorer configuration
 * @param {Object} [overrides] - Configuration overrides
 * @returns {Object} Mainnet config
 */
export function createMainnetConfig(overrides = {}) {
  return {
    ...MAINNET_CONFIG,
    ...overrides,
    // Ensure mainnet RPC is used
    cluster: overrides.cluster || MAINNET_CONFIG.cluster,
  };
}

/**
 * Check if running on mainnet
 * @param {string} cluster - Cluster URL
 * @returns {boolean}
 */
export function isMainnet(cluster) {
  if (!cluster) return false;
  return cluster.includes('mainnet') ||
         cluster.includes('mainnet-beta') ||
         (cluster.includes('helius') && !cluster.includes('devnet'));
}

/**
 * Validate mainnet wallet has sufficient SOL
 * @param {Object} connection - Solana connection
 * @param {string} walletAddress - Wallet public key
 * @param {number} [minSol=0.1] - Minimum SOL balance
 * @returns {Promise<{valid: boolean, balance?: number, error?: string}>}
 */
export async function validateMainnetWallet(connection, walletAddress, minSol = 0.1) {
  try {
    const { PublicKey, LAMPORTS_PER_SOL } = await import('@solana/web3.js');
    const pubkey = new PublicKey(walletAddress);
    const balance = await connection.getBalance(pubkey);
    const solBalance = balance / LAMPORTS_PER_SOL;

    if (solBalance < minSol) {
      return {
        valid: false,
        balance: solBalance,
        error: `Insufficient balance: ${solBalance.toFixed(4)} SOL (minimum: ${minSol} SOL)`,
      };
    }

    return {
      valid: true,
      balance: solBalance,
    };
  } catch (error) {
    return {
      valid: false,
      error: error.message,
    };
  }
}

// =============================================================================
// PRIORITY FEE ESTIMATION
// =============================================================================

/**
 * Estimate optimal priority fee based on recent fees
 * @param {Object} connection - Solana connection
 * @returns {Promise<number>} Priority fee in micro-lamports
 */
export async function estimatePriorityFee(connection) {
  try {
    // Get recent priority fees
    const recentFees = await connection.getRecentPrioritizationFees();

    if (!recentFees || recentFees.length === 0) {
      return PRIORITY_FEES.DEFAULT;
    }

    // Calculate median fee
    const fees = recentFees
      .map(f => f.prioritizationFee)
      .filter(f => f > 0)
      .sort((a, b) => a - b);

    if (fees.length === 0) {
      return PRIORITY_FEES.DEFAULT;
    }

    const medianFee = fees[Math.floor(fees.length / 2)];

    // Add φ-weighted margin for reliability
    const targetFee = Math.ceil(medianFee * (1 + PHI_INV));

    // Clamp to reasonable bounds
    return Math.max(PRIORITY_FEES.LOW, Math.min(PRIORITY_FEES.MAX, targetFee));
  } catch (error) {
    // Fallback to default
    return PRIORITY_FEES.DEFAULT;
  }
}

// =============================================================================
// EXPORTS
// =============================================================================

export default {
  MAINNET_RPCS,
  RATE_LIMITS,
  PRIORITY_FEES,
  COMPUTE_BUDGET,
  RateLimiter,
  RpcFailover,
  MAINNET_CONFIG,
  createMainnetConfig,
  isMainnet,
  validateMainnetWallet,
  estimatePriorityFee,
};
