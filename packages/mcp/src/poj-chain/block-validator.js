/**
 * PoJ Block Validator
 *
 * Validates and receives blocks from other operators.
 * Ensures chain integrity and signature verification.
 *
 * "Don't trust, verify" - κυνικός
 *
 * @module @cynic/mcp/poj-chain/block-validator
 */

'use strict';

import { createLogger } from '@cynic/core';
import { sha256 } from './crypto-utils.js';

const log = createLogger('PoJBlockValidator');

/**
 * Validates PoJ blocks and handles reception from other operators
 */
export class BlockValidator {
  /**
   * @param {Object} [options] - Configuration
   * @param {Object} [options.operatorRegistry] - Multi-operator registry
   * @param {boolean} [options.requireSignatures=false] - Require signed blocks
   * @param {boolean} [options.verifyReceivedBlocks=true] - Verify blocks from others
   */
  constructor(options = {}) {
    this._operatorRegistry = options.operatorRegistry || null;
    this._requireSignatures = options.requireSignatures || false;
    this._verifyReceivedBlocks = options.verifyReceivedBlocks ?? true;

    // Stats
    this._stats = {
      blocksReceived: 0,
      blocksRejected: 0,
      signatureVerifications: 0,
      signatureFailures: 0,
    };
  }

  /**
   * Get stats
   */
  get stats() {
    return { ...this._stats };
  }

  /**
   * Check if multi-operator mode is enabled
   */
  get isMultiOperator() {
    return this._operatorRegistry !== null;
  }

  /**
   * Set operator registry
   * @param {Object} registry - OperatorRegistry instance
   */
  setOperatorRegistry(registry) {
    this._operatorRegistry = registry;
  }

  /**
   * Validate a received block
   * @param {Object} block - Block to validate
   * @param {Object} head - Current chain head
   * @returns {{valid: boolean, error?: string}}
   */
  validate(block, head) {
    // Verify signature if multi-operator mode and verification enabled
    if (this._verifyReceivedBlocks && this._operatorRegistry) {
      const verification = this._operatorRegistry.verifyBlock(block);
      if (!verification.valid) {
        this._stats.signatureFailures++;
        return { valid: false, error: verification.error };
      }
      this._stats.signatureVerifications++;
    } else if (this._requireSignatures && !block.signature) {
      return { valid: false, error: 'Block signature required but not present' };
    }

    // Validate chain integrity
    const expectedPrevHash = head?.hash || head?.block_hash || sha256('CYNIC_GENESIS_φ');
    if (block.prev_hash !== expectedPrevHash) {
      return { valid: false, error: `Chain mismatch: expected prev_hash ${expectedPrevHash.slice(0, 16)}...` };
    }

    const expectedSlot = (head?.slot || 0) + 1;
    if (block.slot !== expectedSlot) {
      return { valid: false, error: `Slot mismatch: expected ${expectedSlot}, got ${block.slot}` };
    }

    // Verify block hash
    const computedHash = sha256({
      slot: block.slot,
      prev_hash: block.prev_hash,
      judgments_root: block.judgments_root,
      judgments: block.judgments,
      timestamp: block.timestamp,
      operator: block.operator,
      operator_name: block.operator_name,
      signature: block.signature,
    });

    if (block.hash && block.hash !== computedHash) {
      return { valid: false, error: 'Invalid block hash' };
    }

    return { valid: true, computedHash };
  }

  /**
   * Receive a block from another operator
   * @param {Object} block - Block to receive
   * @param {Object} head - Current chain head
   * @param {Object} persistence - PersistenceManager
   * @returns {Promise<{accepted: boolean, newHead?: Object, error?: string}>}
   */
  async receiveBlock(block, head, persistence) {
    // Validate the block
    const validation = this.validate(block, head);
    if (!validation.valid) {
      this._stats.blocksRejected++;
      return { accepted: false, error: validation.error };
    }

    // Store block
    try {
      const stored = await persistence.storePoJBlock(block);
      if (stored) {
        const newHead = {
          slot: block.slot,
          hash: block.hash || validation.computedHash,
          block_hash: block.hash || validation.computedHash,
          prev_hash: block.prev_hash,
          judgment_count: block.judgments?.length || 0,
        };
        this._stats.blocksReceived++;

        log.info('PoJ block received', {
          slot: block.slot,
          operator: (block.operator_name || block.operator || 'unknown').slice(0, 16),
        });

        return { accepted: true, newHead };
      }
    } catch (err) {
      this._stats.blocksRejected++;
      return { accepted: false, error: err.message };
    }

    return { accepted: false, error: 'Unknown error storing block' };
  }
}

export default BlockValidator;
