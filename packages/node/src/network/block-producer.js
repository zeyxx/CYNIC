/**
 * BlockProducer - Slot-based block production
 *
 * Extracted SRP component for CYNICNetworkNode (PHASE 2).
 *
 * Collects JUDGMENT_CREATED events into a pending pool,
 * produces blocks on SlotManager timer when this node is leader.
 *
 * "The dog builds the chain" - κυνικός
 *
 * @module @cynic/node/network/block-producer
 */

'use strict';

import crypto from 'crypto';
import { EventEmitter } from 'events';
import { createLogger, globalEventBus, EventType } from '@cynic/core';
import { SlotManager } from '@cynic/protocol';

const log = createLogger('BlockProducer');

export class BlockProducer extends EventEmitter {
  /**
   * @param {Object} options
   * @param {string} options.publicKey - This node's public key
   * @param {number} [options.genesisTime] - Genesis timestamp for SlotManager
   * @param {number} [options.slotDuration] - Slot duration in ms (default 400)
   * @param {number} [options.maxJudgmentsPerBlock=100] - Max judgments per block
   */
  constructor(options = {}) {
    super();

    this._publicKey = options.publicKey;
    this._maxJudgmentsPerBlock = options.maxJudgmentsPerBlock || 100;
    this._pendingJudgments = [];
    this._lastBlockHash = '0'.repeat(64);
    this._running = false;

    this._slotManager = new SlotManager({
      genesisTime: options.genesisTime || Date.now(),
      slotDuration: options.slotDuration,
    });

    // Injected via wire()
    this._proposeBlock = null;
    this._getValidators = null;

    this._stats = {
      blocksProduced: 0,
      emptyBlocks: 0,
      judgmentsIncluded: 0,
      slotsAsLeader: 0,
      slotsTotal: 0,
    };
  }

  /**
   * Wire external dependencies
   * @param {Object} deps
   * @param {Function} deps.proposeBlock - (block) => record
   * @param {Function} deps.getValidators - () => Array<{publicKey}>
   */
  wire({ proposeBlock, getValidators }) {
    if (proposeBlock) this._proposeBlock = proposeBlock;
    if (getValidators) this._getValidators = getValidators;
  }

  /**
   * Start block production
   */
  start() {
    if (this._running) return;
    this._running = true;

    // Collect pending judgments from event bus
    this._judgmentHandler = (event) => {
      if (this._pendingJudgments.length < this._maxJudgmentsPerBlock * 3) {
        this._pendingJudgments.push({
          judgment_id: event.id || event.payload?.id || `jdg_${Date.now().toString(36)}`,
          q_score: event.payload?.qScore ?? event.payload?.score ?? 50,
          verdict: event.payload?.verdict || 'BARK',
          timestamp: Date.now(),
        });
      }
    };
    globalEventBus.on(EventType.JUDGMENT_CREATED, this._judgmentHandler);

    // Sync validators to SlotManager
    this._syncValidators();

    // Start slot timer
    this._slotManager.start((slot, isNewEpoch) => {
      this._onSlot(slot, isNewEpoch);
    });

    log.info('BlockProducer started', {
      publicKey: this._publicKey?.slice(0, 16),
      slotDuration: this._slotManager.slotDuration,
    });

    this.emit('started');
  }

  /**
   * Stop block production
   */
  stop() {
    if (!this._running) return;
    this._running = false;

    this._slotManager.stop();

    if (this._judgmentHandler) {
      globalEventBus.removeListener(EventType.JUDGMENT_CREATED, this._judgmentHandler);
      this._judgmentHandler = null;
    }

    log.info('BlockProducer stopped', { stats: this._stats });
    this.emit('stopped');
  }

  /**
   * Called on each slot tick
   * @private
   */
  _onSlot(slot, isNewEpoch) {
    this._stats.slotsTotal++;

    if (isNewEpoch) {
      this._syncValidators();
    }

    // Check if we're leader for this slot
    if (!this._slotManager.isLeader(this._publicKey)) return;

    this._stats.slotsAsLeader++;

    // Drain pending judgments (up to max per block)
    const judgments = this._pendingJudgments.splice(0, this._maxJudgmentsPerBlock);

    // Produce block (even empty blocks maintain chain liveness)
    const block = this._createBlock(slot, judgments);

    if (judgments.length === 0) {
      this._stats.emptyBlocks++;
    } else {
      this._stats.judgmentsIncluded += judgments.length;
    }

    // Propose to consensus
    try {
      const record = this._proposeBlock?.(block);
      if (record) {
        this._lastBlockHash = block.hash;
        this._stats.blocksProduced++;

        log.info('Block produced', {
          slot,
          hash: block.hash?.slice(0, 16),
          judgments: judgments.length,
        });

        this.emit('block:produced', {
          slot,
          hash: block.hash,
          judgmentCount: judgments.length,
        });
      }
    } catch (error) {
      log.warn('Block proposal failed', { slot, error: error.message });
    }
  }

  /**
   * Sync validator set to SlotManager
   * @private
   */
  _syncValidators() {
    const validators = this._getValidators?.() || [];
    const validatorKeys = validators.map(v => v.publicKey);

    // Ensure self is always in the validator set
    if (!validatorKeys.includes(this._publicKey)) {
      validatorKeys.push(this._publicKey);
    }

    this._slotManager.setValidators(validatorKeys);
  }

  /**
   * Create a block from pending judgments
   * @private
   */
  _createBlock(slot, judgments) {
    const merkleRoot = this._computeMerkleRoot(judgments);
    const prevHash = this._lastBlockHash;

    const blockData = `${slot}|${prevHash}|${merkleRoot}|${this._publicKey}|${Date.now()}`;
    const hash = crypto.createHash('sha256').update(blockData).digest('hex');

    return {
      slot,
      proposer: this._publicKey,
      hash,
      block_hash: hash,
      prev_hash: prevHash,
      merkle_root: merkleRoot,
      judgments_root: merkleRoot,
      judgments,
      judgment_count: judgments.length,
      judgment_ids: judgments.map(j => j.judgment_id),
      timestamp: Date.now(),
    };
  }

  /**
   * Compute merkle root from judgments
   * @private
   */
  _computeMerkleRoot(judgments) {
    if (judgments.length === 0) {
      return '0'.repeat(64);
    }

    // Simple merkle: hash all judgment IDs together
    // For production: use a proper binary merkle tree
    const data = judgments.map(j => j.judgment_id).join('|');
    return crypto.createHash('sha256').update(data).digest('hex');
  }

  /** @returns {Object} SlotManager reference */
  get slotManager() {
    return this._slotManager;
  }

  /** @returns {number} Pending judgment count */
  get pendingCount() {
    return this._pendingJudgments.length;
  }

  /** @returns {Object} Stats */
  get stats() {
    return { ...this._stats };
  }

  /** @returns {boolean} Whether running */
  get running() {
    return this._running;
  }
}

export default BlockProducer;
