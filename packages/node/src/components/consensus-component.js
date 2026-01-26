/**
 * ConsensusComponent - φ-BFT Consensus Domain
 *
 * Encapsulates φ-BFT consensus engine, block production, and validator management.
 * Part of CYNICNode decomposition (Phase 2, #31).
 *
 * "61.8% supermajority" - κυνικός
 *
 * @module @cynic/node/components/consensus-component
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger } from '@cynic/core';
import {
  ConsensusEngine,
  ConsensusState,
  BlockStatus,
  ConsensusGossip,
  createJudgmentBlock,
} from '@cynic/protocol';

const log = createLogger('ConsensusComponent');

/**
 * Consensus Component - manages φ-BFT consensus
 *
 * Single Responsibility: Consensus protocol, block finalization, validator set
 */
export class ConsensusComponent extends EventEmitter {
  /**
   * Create consensus component
   *
   * @param {Object} options - Component options
   * @param {boolean} [options.enabled=true] - Enable consensus
   * @param {number} [options.confirmations=32] - Confirmations for finality
   * @param {string} options.publicKey - Node public key
   * @param {string} options.privateKey - Node private key
   * @param {number} options.eScore - Node E-Score
   * @param {number} [options.burned=0] - Burned amount
   * @param {Object} options.gossip - Gossip protocol instance
   */
  constructor(options = {}) {
    super();

    this._config = {
      enabled: options.enabled ?? true,
      confirmationsForFinality: options.confirmations || 32,
    };

    this._publicKey = options.publicKey;
    this._privateKey = options.privateKey;

    // Initialize consensus engine
    this._consensus = new ConsensusEngine({
      publicKey: options.publicKey,
      privateKey: options.privateKey,
      eScore: options.eScore,
      burned: options.burned || 0,
      confirmationsForFinality: this._config.confirmationsForFinality,
    });

    // Initialize consensus-gossip bridge (if gossip provided)
    this._gossip = options.gossip;
    this._consensusGossip = null;

    if (options.gossip) {
      this._consensusGossip = new ConsensusGossip({
        consensus: this._consensus,
        gossip: options.gossip,
        autoSync: true,
      });
    }

    // External handlers
    this._handlers = {};
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Lifecycle
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Start consensus
   * @param {Object} selfValidator - Self validator info
   * @param {number} selfValidator.eScore - E-Score
   * @param {number} [selfValidator.burned=0] - Burned amount
   */
  start(selfValidator = {}) {
    if (!this._config.enabled) return;

    // Register self as validator
    this._consensus.registerValidator({
      publicKey: this._publicKey,
      eScore: selfValidator.eScore || 50,
      burned: selfValidator.burned || 0,
      uptime: 1.0,
    });

    // Start engine
    this._consensus.start();

    // Start gossip bridge
    if (this._consensusGossip) {
      this._consensusGossip.start();
    }

    log.info('Started', { supermajority: '61.8%', confirmations: this._config.confirmationsForFinality });
  }

  /**
   * Stop consensus
   */
  stop() {
    if (!this._config.enabled) return;

    if (this._consensusGossip) {
      this._consensusGossip.stop();
    }
    this._consensus.stop();

    log.info('Stopped');
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Event Wiring
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Wire consensus events to external handlers
   *
   * @param {Object} handlers - Event handlers
   * @param {Function} [handlers.onBlockFinalized] - (blockHash, slot, block) => Promise<void>
   * @param {Function} [handlers.onBlockConfirmed] - (slot, ratio) => void
   * @param {Function} [handlers.onConsensusStarted] - (slot) => void
   * @param {Function} [handlers.onSlotChange] - (currentSlot, previousSlot) => void
   */
  wireEvents(handlers = {}) {
    this._handlers = handlers;

    // Block finalized
    this._consensus.on('block:finalized', async (event) => {
      const { blockHash, slot, block } = event;
      log.info('Block finalized', { slot, hash: blockHash.slice(0, 16) });

      this.emit('block:finalized', event);
      await handlers.onBlockFinalized?.(blockHash, slot, block);
    });

    // Block confirmed (not yet finalized)
    this._consensus.on('block:confirmed', (event) => {
      log.debug('Block confirmed', { slot: event.slot, ratio: (event.ratio * 100).toFixed(1) });

      this.emit('block:confirmed', event);
      handlers.onBlockConfirmed?.(event.slot, event.ratio);
    });

    // Consensus started
    this._consensus.on('consensus:started', (event) => {
      log.info('Consensus started', { slot: event.slot });

      this.emit('consensus:started', event);
      handlers.onConsensusStarted?.(event.slot);
    });

    // Slot change
    this._consensus.on('slot:change', (event) => {
      // Periodic status every 100 slots
      if (event.currentSlot % 100 === 0) {
        const stats = this._consensus.getStats();
        log.debug('Slot status', { slot: event.currentSlot, finalized: stats.blocksFinalized, pending: stats.pendingBlocks });
      }

      this.emit('slot:change', event);
      handlers.onSlotChange?.(event.currentSlot, event.previousSlot);
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Block Operations
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Create a judgment block
   *
   * @param {Object} options - Block options
   * @param {Array} options.judgments - Judgments to include
   * @param {string} options.previousHash - Previous block hash
   * @param {number} [options.timestamp] - Block timestamp
   * @returns {Object} Created block
   */
  createBlock(options) {
    const currentSlot = this._consensus.getCurrentSlot();

    return createJudgmentBlock({
      judgments: options.judgments,
      previousHash: options.previousHash,
      timestamp: options.timestamp || Date.now(),
      slot: currentSlot,
      proposer: this._publicKey,
    });
  }

  /**
   * Propose a block to consensus
   *
   * @param {Object} block - Block to propose
   * @returns {Object|null} Consensus record or null if not participating
   */
  proposeBlock(block) {
    if (!this._config.enabled) return null;
    if (this._consensus.state !== ConsensusState.PARTICIPATING) return null;

    return this._consensus.proposeBlock(block);
  }

  /**
   * Check if a block is finalized
   * @param {string} blockHash - Block hash
   * @returns {boolean} True if finalized
   */
  isBlockFinalized(blockHash) {
    return this._consensus.isFinalized(blockHash);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Validator Management
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Register a peer as validator
   *
   * @param {Object} validator - Validator info
   * @param {string} validator.publicKey - Validator public key
   * @param {number} [validator.eScore=50] - E-Score
   * @param {number} [validator.burned=0] - Burned amount
   * @param {number} [validator.uptime=1.0] - Uptime ratio
   */
  registerValidator(validator) {
    this._consensus.registerValidator({
      publicKey: validator.publicKey,
      eScore: validator.eScore || 50,
      burned: validator.burned || 0,
      uptime: validator.uptime || 1.0,
    });
  }

  /**
   * Remove a validator
   * @param {string} publicKey - Validator public key
   */
  removeValidator(publicKey) {
    this._consensus.removeValidator(publicKey);
  }

  /**
   * Get number of validators
   * @returns {number} Validator count
   */
  get validatorCount() {
    return this._consensus.validators.size;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // State & Stats
  // ═══════════════════════════════════════════════════════════════════════════

  /** @returns {boolean} Whether consensus is enabled */
  get enabled() {
    return this._config.enabled;
  }

  /** @returns {string} Current consensus state */
  get state() {
    return this._consensus.state;
  }

  /** @returns {boolean} Whether actively participating */
  get isParticipating() {
    return this._consensus.state === ConsensusState.PARTICIPATING;
  }

  /** @returns {number} Current slot */
  get currentSlot() {
    return this._consensus.getCurrentSlot();
  }

  /** @returns {number} Last finalized slot */
  get lastFinalizedSlot() {
    return this._consensus.lastFinalizedSlot;
  }

  /**
   * Get consensus stats
   * @returns {Object} Stats
   */
  getStats() {
    return this._consensus.getStats();
  }

  /**
   * Get component info
   * @returns {Object} Component info
   */
  getInfo() {
    return {
      enabled: this._config.enabled,
      state: this._consensus.state,
      confirmations: this._config.confirmationsForFinality,
      stats: this.getStats(),
      validators: this.validatorCount,
      currentSlot: this.currentSlot,
      lastFinalizedSlot: this.lastFinalizedSlot,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Accessors for backward compatibility
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get raw consensus engine
   * @returns {ConsensusEngine} Consensus engine
   * @deprecated Use component methods instead
   */
  get engine() {
    return this._consensus;
  }

  /**
   * Get consensus-gossip bridge
   * @returns {ConsensusGossip|null} Bridge
   * @deprecated Use component methods instead
   */
  get bridge() {
    return this._consensusGossip;
  }
}

export { ConsensusState, BlockStatus };
export default ConsensusComponent;
