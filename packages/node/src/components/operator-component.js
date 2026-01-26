/**
 * OperatorComponent - Node Operator Domain
 *
 * Encapsulates operator identity, E-Score management, and burn verification.
 * Part of CYNICNode decomposition (Phase 2, #27).
 *
 * "The operator is the soul of the node" - κυνικός
 *
 * @module @cynic/node/components/operator-component
 */

'use strict';

import { PHI_INV, createLogger } from '@cynic/core';
import { Operator } from '../operator/operator.js';

const log = createLogger('OperatorComponent');
import { createBurnVerifier, BurnStatus } from '@cynic/burns';
import { EScoreHistoryRepository, getPool } from '@cynic/persistence';

/**
 * Operator Component - manages operator identity and scoring
 *
 * Single Responsibility: Operator lifecycle, E-Score, and burn verification
 */
export class OperatorComponent {
  /**
   * Create operator component
   *
   * @param {Object} options - Component options
   * @param {string} [options.name] - Operator name
   * @param {Object} [options.identity] - Existing identity to import
   * @param {Object} [options.burns] - Burns verification config
   * @param {boolean} [options.burns.enabled=false] - Enable burn verification
   * @param {number} [options.burns.minAmount] - Min burn amount (lamports)
   * @param {string} [options.burns.cluster] - Solana cluster
   * @param {Object} [options.persistence] - Persistence config
   * @param {boolean} [options.persistence.enabled=true] - Enable persistence
   */
  constructor(options = {}) {
    // Initialize operator
    this._operator = new Operator({
      name: options.name,
      identity: options.identity,
    });

    // Burns configuration
    this._burnsConfig = {
      enabled: options.burns?.enabled || false,
      minAmount: options.burns?.minAmount || Math.floor(PHI_INV * 1_000_000_000),
      cluster: options.burns?.cluster || 'mainnet-beta',
    };

    // Initialize burn verifier
    this._burnVerifier = createBurnVerifier({
      solanaCluster: this._burnsConfig.cluster,
      onVerify: async (result) => {
        if (result.verified && result.amount > 0) {
          // Automatically record verified burns
          this._operator.recordBurn(result.amount, result.burnType || 'verified_onchain');
          await this._recordEScoreSnapshot(`burn_${result.burnType || 'verified'}`);
        }
      },
    });

    // Persistence layer
    this._persistence = null;
    if (options.persistence?.enabled !== false) {
      try {
        const pool = getPool();
        this._persistence = {
          escoreHistory: new EScoreHistoryRepository(pool),
        };
      } catch (e) {
        // Database not available
        log.warn('Persistence unavailable - running in-memory');
      }
    }

    // Epoch counter for periodic snapshots
    this._epochCount = 0;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Identity Accessors
  // ═══════════════════════════════════════════════════════════════════════════

  /** @returns {string} Operator ID */
  get id() {
    return this._operator.id;
  }

  /** @returns {string} Public key */
  get publicKey() {
    return this._operator.publicKey;
  }

  /** @returns {string} Private key */
  get privateKey() {
    return this._operator.privateKey;
  }

  /** @returns {Object} Full identity object */
  get identity() {
    return this._operator.identity;
  }

  /** @returns {string} Operator name */
  get name() {
    return this._operator.identity.name;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // E-Score Methods
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get current E-Score
   * @returns {number} Composite E-Score
   */
  getEScore() {
    return this._operator.getEScore();
  }

  /**
   * Get E-Score breakdown by dimension
   * @returns {Object} E-Score breakdown
   */
  getEScoreBreakdown() {
    return this._operator.getEScoreBreakdown();
  }

  /**
   * Get vote weight for consensus
   * @returns {number} Vote weight
   */
  getVoteWeight() {
    return this._operator.getVoteWeight();
  }

  /**
   * Update uptime tracking
   * @param {number} uptime - Uptime in ms
   */
  updateUptime(uptime) {
    this._operator.updateUptime(uptime);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Burn Verification
  // ═══════════════════════════════════════════════════════════════════════════

  /** @returns {boolean} Whether burns verification is enabled */
  get burnsEnabled() {
    return this._burnsConfig.enabled;
  }

  /** @returns {number} Minimum burn amount */
  get minBurnAmount() {
    return this._burnsConfig.minAmount;
  }

  /**
   * Verify a burn transaction
   *
   * @param {string} burnTx - Burn transaction signature
   * @param {Object} [options] - Verification options
   * @param {string} [options.expectedBurner] - Expected burner address
   * @returns {Promise<Object>} Verification result
   */
  async verifyBurn(burnTx, options = {}) {
    return this._burnVerifier.verify(burnTx, {
      minAmount: this._burnsConfig.minAmount,
      expectedBurner: options.expectedBurner,
    });
  }

  /**
   * Check if a burn has been verified
   * @param {string} burnTx - Burn transaction signature
   * @returns {boolean} Whether burn is verified
   */
  isBurnVerified(burnTx) {
    return this._burnVerifier.isVerified(burnTx);
  }

  /**
   * Record a manual burn (for testing or manual tracking)
   *
   * @param {number} amount - Amount burned
   * @param {string} [reason] - Burn reason
   * @returns {Object} Burn record
   */
  recordBurn(amount, reason = 'manual') {
    return this._operator.recordBurn(amount, reason);
  }

  /** @returns {number} Total burned amount */
  get totalBurned() {
    return this._operator.burn?.totalBurned || 0;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Stats & Recording
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record a judgment made
   */
  recordJudgment() {
    this._operator.recordJudgment();
  }

  /**
   * Record a block produced
   */
  recordBlock() {
    this._operator.recordBlock();
  }

  /**
   * Record a pattern contributed
   */
  recordPattern() {
    this._operator.recordPattern();
  }

  /**
   * Get public operator info (safe to share)
   * @returns {Object} Public info
   */
  getPublicInfo() {
    return this._operator.getPublicInfo();
  }

  /**
   * Get operator stats
   * @returns {Object} Stats
   */
  getStats() {
    return this._operator.stats;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Persistence
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record E-Score snapshot to persistence
   *
   * @param {string} [trigger='manual'] - What triggered the snapshot
   * @returns {Promise<boolean>} Success
   */
  async _recordEScoreSnapshot(trigger = 'manual') {
    if (!this._persistence?.escoreHistory) return false;

    try {
      const breakdown = this.getEScoreBreakdown();
      await this._persistence.escoreHistory.recordSnapshot(
        this.id,
        this.getEScore(),
        {
          hold: breakdown.dimensions?.find(d => d.dimension === 'HOLD')?.score || 0,
          burn: breakdown.dimensions?.find(d => d.dimension === 'BURN')?.score || 0,
          use: breakdown.dimensions?.find(d => d.dimension === 'USE')?.score || 0,
          build: breakdown.dimensions?.find(d => d.dimension === 'BUILD')?.score || 0,
          run: breakdown.dimensions?.find(d => d.dimension === 'RUN')?.score || 0,
          refer: breakdown.dimensions?.find(d => d.dimension === 'REFER')?.score || 0,
          time: breakdown.dimensions?.find(d => d.dimension === 'TIME')?.score || 0,
        },
        trigger
      );
      return true;
    } catch (e) {
      log.warn('E-Score snapshot failed', { error: e.message });
      return false;
    }
  }

  /**
   * Called on each epoch - handles periodic E-Score snapshots
   */
  onEpoch() {
    this._epochCount++;

    // Periodic snapshot every 36 epochs (~1 hour at 100s scaled)
    if (this._persistence?.escoreHistory && this._epochCount % 36 === 0) {
      this._recordEScoreSnapshot('epoch_periodic').catch(() => {});
    }
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Export/Import
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Export component state
   * @returns {Object} Exportable state
   */
  export() {
    return {
      operator: this._operator.export(),
      burnsConfig: this._burnsConfig,
      burnsState: this._burnVerifier.export(),
    };
  }

  /**
   * Import state into component
   *
   * @param {Object} state - Saved state
   */
  import(state) {
    if (state.operator) {
      this._operator = Operator.import(state.operator);
    }
    if (state.burnsState) {
      this._burnVerifier.import(state.burnsState);
    }
  }

  /**
   * Get burn verifier stats
   * @returns {Object} Stats
   */
  getBurnVerifierStats() {
    return this._burnVerifier.getStats();
  }

  /**
   * Get underlying operator (for backward compatibility)
   * @returns {Operator} Raw operator
   * @deprecated Use component methods instead
   */
  get operator() {
    return this._operator;
  }
}

export default OperatorComponent;
