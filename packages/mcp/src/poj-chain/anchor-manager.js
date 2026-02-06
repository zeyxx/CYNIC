/**
 * PoJ Anchor Manager
 *
 * Handles Solana anchoring for PoJ blocks.
 * "Onchain is truth" - κυνικός
 *
 * @module @cynic/mcp/poj-chain/anchor-manager
 */

'use strict';

import { createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('PoJAnchorManager');

/**
 * Anchor status for blocks
 */
export const AnchorStatus = {
  /** Block not yet anchored */
  PENDING: 'PENDING',
  /** Block queued for anchoring */
  QUEUED: 'QUEUED',
  /** Block anchored to Solana */
  ANCHORED: 'ANCHORED',
  /** Anchoring failed */
  FAILED: 'FAILED',
};

/**
 * Blockchain events for anchoring
 */
export const BlockchainEvent = {
  /** PoJ block created from batched judgments */
  BLOCK_CREATED: 'poj:block:created',
  /** PoJ block anchored to Solana */
  BLOCK_ANCHORED: 'poj:block:anchored',
  /** Anchor failed for a block */
  ANCHOR_FAILED: 'poj:anchor:failed',
};

/**
 * Manages Solana anchoring for PoJ blocks
 */
export class AnchorManager {
  /**
   * @param {Object} [options] - Configuration
   * @param {Object} [options.anchorQueue] - AnchorQueue instance
   * @param {boolean} [options.autoAnchor=true] - Auto-anchor blocks
   */
  constructor(options = {}) {
    this._anchorQueue = options.anchorQueue || null;
    this._autoAnchor = options.autoAnchor ?? true;

    // Track anchor status per block
    this._anchorStatus = new Map();

    // Stats
    this._stats = {
      blocksAnchored: 0,
      anchorsFailed: 0,
    };
  }

  /**
   * Check if anchoring is enabled
   */
  get isEnabled() {
    return this._anchorQueue !== null;
  }

  /**
   * Get stats
   */
  get stats() {
    return { ...this._stats };
  }

  /**
   * Set anchor queue
   * @param {Object} anchorQueue - AnchorQueue instance
   */
  setAnchorQueue(anchorQueue) {
    this._anchorQueue = anchorQueue;

    // Set up callback for anchor completion
    if (anchorQueue && anchorQueue.onAnchorComplete === undefined) {
      anchorQueue.onAnchorComplete = (batch, result) => {
        this._onAnchorComplete(batch, result);
      };
    }
  }

  /**
   * Queue a block for anchoring
   * @param {Object} block - Block to anchor
   */
  async anchorBlock(block) {
    if (!this._anchorQueue) {
      log.warn('Anchor queue not available');
      return;
    }

    const blockId = `poj_block_${block.slot}`;

    // Mark as queued
    this._anchorStatus.set(block.hash, {
      status: AnchorStatus.QUEUED,
      slot: block.slot,
      queuedAt: Date.now(),
    });

    try {
      // Queue the block's merkle root for anchoring
      this._anchorQueue.enqueue(blockId, {
        type: 'poj_block',
        slot: block.slot,
        hash: block.hash,
        merkleRoot: block.judgments_root,
        judgmentCount: block.judgments?.length || 0,
        timestamp: block.timestamp,
      });

      log.info('PoJ block queued for anchoring', { slot: block.slot });
    } catch (err) {
      log.error('Error queuing block for anchor', { error: err.message });
      this._anchorStatus.set(block.hash, {
        status: AnchorStatus.FAILED,
        slot: block.slot,
        error: err.message,
      });
      this._stats.anchorsFailed++;
    }
  }

  /**
   * Mark a block as pending anchor
   * @param {Object} block - Block to mark
   * @param {Object} [extra] - Extra status fields
   */
  markPending(block, extra = {}) {
    this._anchorStatus.set(block.hash, {
      status: AnchorStatus.PENDING,
      slot: block.slot,
      ...extra,
    });
  }

  /**
   * Handle anchor completion callback
   * @param {Object} batch - Anchored batch
   * @param {Object} result - Anchor result
   * @private
   */
  _onAnchorComplete(batch, result) {
    if (!result.success) {
      // Emit anchor failure event
      try {
        globalEventBus.publish(BlockchainEvent.ANCHOR_FAILED, {
          batchId: batch.id,
          itemCount: batch.items?.length || 0,
          error: result.error || 'Unknown error',
          timestamp: Date.now(),
        }, { source: 'PoJAnchorManager' });
      } catch (e) { /* non-blocking */ }
      return;
    }

    // Update anchor status for all blocks in this batch
    for (const item of batch.items) {
      if (item.id.startsWith('poj_block_')) {
        const slot = parseInt(item.id.replace('poj_block_', ''), 10);

        // Find block by slot
        for (const [hash, status] of this._anchorStatus.entries()) {
          if (status.slot === slot) {
            this._anchorStatus.set(hash, {
              status: AnchorStatus.ANCHORED,
              slot,
              signature: result.signature,
              anchoredAt: result.timestamp,
            });
            this._stats.blocksAnchored++;
            log.info('PoJ block anchored', { slot, signature: result.signature.slice(0, 16) });

            // Emit blockchain event - block is now ON-CHAIN TRUTH
            try {
              globalEventBus.publish(BlockchainEvent.BLOCK_ANCHORED, {
                slot,
                hash,
                signature: result.signature,
                solanaSlot: result.slot,
                merkleRoot: item.merkleRoot || batch.merkleRoot,
                anchoredAt: result.timestamp,
                judgmentCount: status.judgmentCount || 0,
                batchSize: batch.items?.length || 1,
              }, { source: 'PoJAnchorManager' });
            } catch (eventErr) {
              log.warn('Anchor event emission failed', { error: eventErr.message });
            }

            break;
          }
        }
      }
    }
  }

  /**
   * Get anchor status for a block
   * @param {string} blockHash - Block hash
   * @returns {Object|null} Anchor status
   */
  getStatus(blockHash) {
    return this._anchorStatus.get(blockHash) || null;
  }

  /**
   * Get all pending anchor statuses
   * @returns {Object[]} Pending anchors
   */
  getPending() {
    const pending = [];
    for (const [hash, status] of this._anchorStatus.entries()) {
      if (status.status === AnchorStatus.PENDING || status.status === AnchorStatus.QUEUED) {
        pending.push({ hash, ...status });
      }
    }
    return pending;
  }

  /**
   * Count anchored and pending blocks
   * @returns {{anchored: number, pending: number}}
   */
  getCounts() {
    let anchored = 0;
    let pending = 0;
    for (const status of this._anchorStatus.values()) {
      if (status.status === AnchorStatus.ANCHORED) anchored++;
      else if (status.status === AnchorStatus.PENDING || status.status === AnchorStatus.QUEUED) {
        pending++;
      }
    }
    return { anchored, pending };
  }
}

export default AnchorManager;
