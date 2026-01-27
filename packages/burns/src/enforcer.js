/**
 * @cynic/burns - Burn Enforcer
 *
 * Enforces burn requirements before allowing critical operations.
 * "No burn, no judgment" - κυνικός
 *
 * @module @cynic/burns/enforcer
 */

'use strict';

import { PHI_INV, createLogger } from '@cynic/core';
import { createBurnVerifier } from './verifier.js';

const log = createLogger('BurnEnforcer');

/**
 * Burn enforcement error - thrown when burn requirement not met
 */
export class BurnRequiredError extends Error {
  /**
   * @param {string} message - Error message
   * @param {Object} details - Error details
   */
  constructor(message, details = {}) {
    super(message);
    this.name = 'BurnRequiredError';
    this.code = 'BURN_REQUIRED';
    this.details = details;
  }
}

/**
 * Default enforcer configuration
 */
export const DEFAULT_ENFORCER_CONFIG = {
  /** Enable enforcement (if false, all operations pass) */
  enabled: false,
  /** Minimum burn amount in lamports (default: φ⁻¹ SOL = 0.618 SOL) */
  minAmount: Math.floor(PHI_INV * 1_000_000_000),
  /** Burn validity period in ms (default: 24 hours) */
  validityPeriod: 24 * 60 * 60 * 1000,
  /** Grace period for new users (ms, default: 1 hour for testing) */
  gracePeriod: 60 * 60 * 1000,
  /** Operations that require burns */
  protectedOperations: ['judge', 'refine', 'digest'],
  /** Solana cluster for verification */
  solanaCluster: null,
};

/**
 * Burn Enforcer
 *
 * Ensures users have valid burns before critical operations.
 *
 * Usage:
 * ```javascript
 * const enforcer = createBurnEnforcer({
 *   enabled: true,
 *   solanaCluster: SolanaCluster.MAINNET,
 * });
 *
 * // Register a burn for a user
 * await enforcer.registerBurn(userId, 'tx_signature');
 *
 * // Check before operation (throws if no valid burn)
 * await enforcer.requireBurn(userId, 'judge');
 *
 * // Or check without throwing
 * if (enforcer.hasValidBurn(userId)) {
 *   // proceed
 * }
 * ```
 */
export class BurnEnforcer {
  /**
   * @param {Object} config - Configuration
   * @param {boolean} [config.enabled] - Enable enforcement
   * @param {number} [config.minAmount] - Minimum burn amount
   * @param {number} [config.validityPeriod] - Burn validity in ms
   * @param {number} [config.gracePeriod] - Grace period for new users
   * @param {string[]} [config.protectedOperations] - Operations requiring burns
   * @param {string} [config.solanaCluster] - Solana cluster
   * @param {Object} [config.verifier] - Custom BurnVerifier instance
   */
  constructor(config = {}) {
    this.enabled = config.enabled ?? DEFAULT_ENFORCER_CONFIG.enabled;
    this.minAmount = config.minAmount ?? DEFAULT_ENFORCER_CONFIG.minAmount;
    this.validityPeriod = config.validityPeriod ?? DEFAULT_ENFORCER_CONFIG.validityPeriod;
    this.gracePeriod = config.gracePeriod ?? DEFAULT_ENFORCER_CONFIG.gracePeriod;
    this.protectedOperations = config.protectedOperations ?? DEFAULT_ENFORCER_CONFIG.protectedOperations;

    // User burns registry: Map<userId, { txSignature, amount, verifiedAt, expiresAt }>
    this._userBurns = new Map();

    // User first-seen timestamps (for grace period)
    this._userFirstSeen = new Map();

    // Initialize verifier
    this._verifier = config.verifier || createBurnVerifier({
      solanaCluster: config.solanaCluster,
    });

    // Stats
    this.stats = {
      checksPerformed: 0,
      checksPassed: 0,
      checksFailed: 0,
      burnsRegistered: 0,
      gracePeriodUsed: 0,
    };

    if (this.enabled) {
      log.info('Burn enforcement ENABLED', {
        minAmount: this.minAmount,
        validityPeriod: this.validityPeriod,
        protectedOperations: this.protectedOperations,
      });
    } else {
      log.info('Burn enforcement DISABLED (all operations allowed)');
    }
  }

  /**
   * Register and verify a burn for a user
   *
   * @param {string} userId - User identifier
   * @param {string} txSignature - Burn transaction signature
   * @param {Object} [options] - Options
   * @param {string} [options.expectedBurner] - Expected burner address
   * @returns {Promise<Object>} Registration result
   */
  async registerBurn(userId, txSignature, options = {}) {
    if (!userId) {
      throw new Error('userId is required');
    }
    if (!txSignature) {
      throw new Error('txSignature is required');
    }

    // Verify the burn
    const verification = await this._verifier.verify(txSignature, {
      minAmount: this.minAmount,
      expectedBurner: options.expectedBurner,
    });

    if (!verification.verified) {
      log.warn('Burn verification failed', {
        userId,
        txSignature: txSignature.slice(0, 20) + '...',
        error: verification.error,
      });

      return {
        registered: false,
        error: verification.error || 'Burn verification failed',
        verification,
      };
    }

    // Register the burn
    const now = Date.now();
    const burnRecord = {
      txSignature,
      amount: verification.amount,
      burner: verification.burner,
      verifiedAt: now,
      expiresAt: now + this.validityPeriod,
      slot: verification.slot,
    };

    this._userBurns.set(userId, burnRecord);
    this.stats.burnsRegistered++;

    log.info('Burn registered', {
      userId,
      amount: verification.amount,
      expiresAt: new Date(burnRecord.expiresAt).toISOString(),
    });

    return {
      registered: true,
      burn: burnRecord,
      verification,
    };
  }

  /**
   * Check if user has a valid burn (non-throwing)
   *
   * @param {string} userId - User identifier
   * @returns {boolean}
   */
  hasValidBurn(userId) {
    // If enforcement disabled, always pass
    if (!this.enabled) {
      return true;
    }

    const burn = this._userBurns.get(userId);
    if (!burn) {
      return false;
    }

    // Check expiration
    if (Date.now() > burn.expiresAt) {
      this._userBurns.delete(userId);
      return false;
    }

    return true;
  }

  /**
   * Check if user is in grace period
   *
   * @param {string} userId - User identifier
   * @returns {boolean}
   */
  isInGracePeriod(userId) {
    if (!this.gracePeriod) {
      return false;
    }

    let firstSeen = this._userFirstSeen.get(userId);
    if (!firstSeen) {
      firstSeen = Date.now();
      this._userFirstSeen.set(userId, firstSeen);
    }

    return Date.now() - firstSeen < this.gracePeriod;
  }

  /**
   * Require a valid burn for an operation (throws if not met)
   *
   * @param {string} userId - User identifier
   * @param {string} operation - Operation name (e.g., 'judge', 'refine')
   * @throws {BurnRequiredError} If burn requirement not met
   */
  requireBurn(userId, operation = 'unknown') {
    this.stats.checksPerformed++;

    // If enforcement disabled, always pass
    if (!this.enabled) {
      this.stats.checksPassed++;
      return;
    }

    // Check if operation requires burn
    if (!this.protectedOperations.includes(operation)) {
      this.stats.checksPassed++;
      return;
    }

    // Check valid burn
    if (this.hasValidBurn(userId)) {
      this.stats.checksPassed++;
      return;
    }

    // Check grace period
    if (this.isInGracePeriod(userId)) {
      this.stats.checksPassed++;
      this.stats.gracePeriodUsed++;
      log.debug('Grace period used', { userId, operation });
      return;
    }

    // No valid burn
    this.stats.checksFailed++;

    const burn = this._userBurns.get(userId);
    const details = {
      userId,
      operation,
      hasBurn: !!burn,
      burnExpired: burn ? Date.now() > burn.expiresAt : false,
      minAmount: this.minAmount,
      validityPeriod: this.validityPeriod,
    };

    log.warn('Burn requirement not met', details);

    throw new BurnRequiredError(
      `Operation '${operation}' requires a valid burn. ` +
      `Minimum amount: ${this.minAmount / 1_000_000_000} SOL. ` +
      'Register a burn with registerBurn(userId, txSignature).',
      details
    );
  }

  /**
   * Get burn status for a user
   *
   * @param {string} userId - User identifier
   * @returns {Object} Burn status
   */
  getBurnStatus(userId) {
    const burn = this._userBurns.get(userId);
    const inGracePeriod = this.isInGracePeriod(userId);

    if (!this.enabled) {
      return {
        enforcementEnabled: false,
        hasValidBurn: true,
        message: 'Burn enforcement is disabled',
      };
    }

    if (burn && Date.now() <= burn.expiresAt) {
      return {
        enforcementEnabled: true,
        hasValidBurn: true,
        burn: {
          txSignature: burn.txSignature,
          amount: burn.amount,
          verifiedAt: burn.verifiedAt,
          expiresAt: burn.expiresAt,
          remainingMs: burn.expiresAt - Date.now(),
        },
      };
    }

    if (inGracePeriod) {
      const firstSeen = this._userFirstSeen.get(userId);
      return {
        enforcementEnabled: true,
        hasValidBurn: false,
        inGracePeriod: true,
        gracePeriodEndsAt: firstSeen + this.gracePeriod,
        gracePeriodRemainingMs: (firstSeen + this.gracePeriod) - Date.now(),
        message: 'In grace period - please register a burn before it expires',
      };
    }

    return {
      enforcementEnabled: true,
      hasValidBurn: false,
      inGracePeriod: false,
      minAmount: this.minAmount,
      minAmountSol: this.minAmount / 1_000_000_000,
      message: 'Burn required. Please verify a burn transaction to continue.',
    };
  }

  /**
   * Revoke a user's burn (for admin/testing)
   *
   * @param {string} userId - User identifier
   * @returns {boolean} Whether a burn was revoked
   */
  revokeBurn(userId) {
    return this._userBurns.delete(userId);
  }

  /**
   * Clear all burns (for testing)
   */
  clearAll() {
    this._userBurns.clear();
    this._userFirstSeen.clear();
  }

  /**
   * Get enforcer stats
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this.stats,
      enabled: this.enabled,
      activeBurns: this._userBurns.size,
      usersInGracePeriod: Array.from(this._userFirstSeen.entries())
        .filter(([_, firstSeen]) => Date.now() - firstSeen < this.gracePeriod)
        .length,
    };
  }
}

/**
 * Factory function to create BurnEnforcer
 *
 * @param {Object} [config] - Configuration
 * @returns {BurnEnforcer}
 */
export function createBurnEnforcer(config = {}) {
  return new BurnEnforcer(config);
}
