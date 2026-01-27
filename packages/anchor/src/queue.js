/**
 * @cynic/anchor - Anchor Queue
 *
 * Batches items and triggers periodic anchoring.
 *
 * "Onchain is truth" - κυνικός
 *
 * @module @cynic/anchor/queue
 */

'use strict';

import { createHash } from 'crypto';
import {
  AnchorStatus,
  DEFAULT_CONFIG,
} from './constants.js';

/**
 * Queued item for anchoring
 * @typedef {Object} QueuedItem
 * @property {string} id - Item ID (e.g., judgment ID)
 * @property {string} hash - SHA-256 hash of item content
 * @property {number} timestamp - When queued
 * @property {AnchorStatus} status - Current status
 */

/**
 * Anchor batch
 * @typedef {Object} AnchorBatch
 * @property {string} batchId - Unique batch ID
 * @property {string} merkleRoot - Computed merkle root
 * @property {QueuedItem[]} items - Items in batch
 * @property {number} createdAt - Batch creation time
 * @property {AnchorStatus} status - Batch status
 */

/**
 * Anchor Queue
 *
 * Collects items and batches them for periodic anchoring.
 * Triggers anchor when batch is full OR timer expires.
 */
export class AnchorQueue {
  /**
   * @param {Object} config - Configuration
   * @param {SolanaAnchorer} config.anchorer - Anchorer instance
   * @param {number} [config.batchSize] - Items per batch
   * @param {number} [config.intervalMs] - Max time between anchors
   * @param {boolean} [config.autoStart] - Start timer automatically
   * @param {Function} [config.onBatchReady] - Callback when batch ready
   * @param {Function} [config.onAnchorComplete] - Callback when anchor done
   */
  constructor(config = {}) {
    this.anchorer = config.anchorer;
    this.batchSize = config.batchSize || DEFAULT_CONFIG.batchSize;
    this.intervalMs = config.intervalMs || DEFAULT_CONFIG.intervalMs;
    this.onBatchReady = config.onBatchReady;
    this.onAnchorComplete = config.onAnchorComplete;

    // Queue state
    this.queue = [];
    this.batches = new Map();
    this.timer = null;
    this.lastAnchorTime = null;

    // Stats
    this.stats = {
      totalQueued: 0,
      totalBatches: 0,
      totalAnchored: 0,
    };

    // Auto-start if configured
    if (config.autoStart) {
      this.startTimer();
    }
  }

  /**
   * Generate batch ID
   * @returns {string}
   */
  generateBatchId() {
    const hash = createHash('sha256')
      .update(`batch:${Date.now()}:${Math.random()}`)
      .digest('hex')
      .slice(0, 12);
    return `batch_${hash}`;
  }

  /**
   * Compute merkle root from items
   *
   * Simple merkle tree: hash pairs until one root remains.
   *
   * @param {QueuedItem[]} items - Items to hash
   * @returns {string} 64-char hex merkle root
   */
  computeMerkleRoot(items) {
    if (items.length === 0) {
      // Empty tree - return zero hash
      return '0'.repeat(64);
    }

    // Get leaf hashes
    let hashes = items.map((item) => item.hash);

    // Pad to power of 2 with last hash
    while (hashes.length > 1 && (hashes.length & (hashes.length - 1)) !== 0) {
      hashes.push(hashes[hashes.length - 1]);
    }

    // Build tree bottom-up
    while (hashes.length > 1) {
      const nextLevel = [];
      for (let i = 0; i < hashes.length; i += 2) {
        const left = hashes[i];
        const right = hashes[i + 1] || left;
        const combined = createHash('sha256')
          .update(left + right)
          .digest('hex');
        nextLevel.push(combined);
      }
      hashes = nextLevel;
    }

    return hashes[0];
  }

  /**
   * Add item to queue
   *
   * @param {string} id - Item ID
   * @param {string|Object} content - Item content (string or object)
   * @returns {QueuedItem}
   */
  enqueue(id, content) {
    // Compute hash
    const contentStr =
      typeof content === 'string' ? content : JSON.stringify(content);
    const hash = createHash('sha256').update(contentStr).digest('hex');

    const item = {
      id,
      hash,
      timestamp: Date.now(),
      status: AnchorStatus.QUEUED,
    };

    this.queue.push(item);
    this.stats.totalQueued++;

    // Check if batch is ready
    if (this.queue.length >= this.batchSize) {
      this._processBatch();
    }

    return item;
  }

  /**
   * Process current queue into a batch
   * @private
   */
  async _processBatch() {
    if (this.queue.length === 0) {
      return null;
    }

    // Take items from queue
    const items = this.queue.splice(0, this.batchSize);

    // Compute merkle root
    const merkleRoot = this.computeMerkleRoot(items);

    // Create batch
    const batchId = this.generateBatchId();
    const batch = {
      batchId,
      merkleRoot,
      items,
      createdAt: Date.now(),
      status: AnchorStatus.QUEUED,
    };

    this.batches.set(batchId, batch);
    this.stats.totalBatches++;

    // Callback
    if (this.onBatchReady) {
      this.onBatchReady(batch);
    }

    // Anchor if we have an anchorer
    if (this.anchorer) {
      await this._anchorBatch(batch);
    }

    return batch;
  }

  /**
   * Anchor a batch
   * @param {AnchorBatch} batch - Batch to anchor
   * @private
   */
  async _anchorBatch(batch) {
    try {
      const itemIds = batch.items.map((i) => i.id);
      const result = await this.anchorer.anchor(batch.merkleRoot, itemIds);

      if (result.success) {
        batch.status = AnchorStatus.ANCHORED;
        batch.signature = result.signature;
        batch.slot = result.slot;
        batch.anchoredAt = result.timestamp;

        // Update item statuses
        for (const item of batch.items) {
          item.status = AnchorStatus.ANCHORED;
        }

        this.stats.totalAnchored++;
        this.lastAnchorTime = result.timestamp;

        if (this.onAnchorComplete) {
          this.onAnchorComplete(batch, result);
        }
      } else {
        batch.status = AnchorStatus.FAILED;
        batch.error = result.error;
      }

      return result;
    } catch (error) {
      batch.status = AnchorStatus.FAILED;
      batch.error = error.message;
      throw error;
    }
  }

  /**
   * Force process current queue
   * @returns {Promise<AnchorBatch|null>}
   */
  async flush() {
    return this._processBatch();
  }

  /**
   * Start the anchor timer
   */
  startTimer() {
    if (this.timer) {
      return;
    }

    this.timer = setInterval(async () => {
      if (this.queue.length > 0) {
        await this._processBatch();
      }
    }, this.intervalMs);
  }

  /**
   * Stop the anchor timer
   */
  stopTimer() {
    if (this.timer) {
      clearInterval(this.timer);
      this.timer = null;
    }
  }

  /**
   * Get queue length
   * @returns {number}
   */
  getQueueLength() {
    return this.queue.length;
  }

  /**
   * Get batch by ID
   * @param {string} batchId - Batch ID
   * @returns {AnchorBatch|undefined}
   */
  getBatch(batchId) {
    return this.batches.get(batchId);
  }

  /**
   * Get batch containing item
   * @param {string} itemId - Item ID
   * @returns {AnchorBatch|undefined}
   */
  getBatchForItem(itemId) {
    for (const batch of this.batches.values()) {
      if (batch.items.some((i) => i.id === itemId)) {
        return batch;
      }
    }
    return undefined;
  }

  /**
   * Get item status
   * @param {string} itemId - Item ID
   * @returns {AnchorStatus|null}
   */
  getItemStatus(itemId) {
    // Check queue
    const queued = this.queue.find((i) => i.id === itemId);
    if (queued) {
      return queued.status;
    }

    // Check batches
    const batch = this.getBatchForItem(itemId);
    if (batch) {
      const item = batch.items.find((i) => i.id === itemId);
      return item ? item.status : null;
    }

    return null;
  }

  /**
   * Get anchor proof for item
   * @param {string} itemId - Item ID
   * @returns {Object|null} Proof with merkle path and signature
   */
  getProof(itemId) {
    const batch = this.getBatchForItem(itemId);
    if (!batch || batch.status !== AnchorStatus.ANCHORED) {
      return null;
    }

    const item = batch.items.find((i) => i.id === itemId);
    if (!item) {
      return null;
    }

    // Build merkle proof (path from leaf to root)
    const proof = this._buildMerkleProof(batch.items, item);

    return {
      itemId,
      itemHash: item.hash,
      merkleRoot: batch.merkleRoot,
      merkleProof: proof,
      signature: batch.signature,
      slot: batch.slot,
      anchoredAt: batch.anchoredAt,
    };
  }

  /**
   * Build merkle proof for an item
   * @param {QueuedItem[]} items - All items in batch
   * @param {QueuedItem} target - Target item
   * @returns {Array<{hash: string, position: 'left'|'right'}>}
   * @private
   */
  _buildMerkleProof(items, target) {
    const proof = [];
    let hashes = items.map((item) => item.hash);
    let index = items.findIndex((i) => i.id === target.id);

    if (index === -1) {
      return [];
    }

    // Pad to power of 2
    while (hashes.length > 1 && (hashes.length & (hashes.length - 1)) !== 0) {
      hashes.push(hashes[hashes.length - 1]);
    }

    // Build proof
    while (hashes.length > 1) {
      const siblingIndex = index % 2 === 0 ? index + 1 : index - 1;
      const siblingHash = hashes[siblingIndex] || hashes[index];
      const position = index % 2 === 0 ? 'right' : 'left';

      proof.push({ hash: siblingHash, position });

      // Move to next level
      const nextLevel = [];
      for (let i = 0; i < hashes.length; i += 2) {
        const left = hashes[i];
        const right = hashes[i + 1] || left;
        nextLevel.push(createHash('sha256').update(left + right).digest('hex'));
      }

      hashes = nextLevel;
      index = Math.floor(index / 2);
    }

    return proof;
  }

  /**
   * Verify a merkle proof
   * @param {string} itemHash - Item hash (leaf)
   * @param {string} merkleRoot - Expected root
   * @param {Array} proof - Merkle proof path
   * @returns {boolean}
   */
  verifyProof(itemHash, merkleRoot, proof) {
    let hash = itemHash;

    for (const step of proof) {
      const left = step.position === 'right' ? hash : step.hash;
      const right = step.position === 'right' ? step.hash : hash;
      hash = createHash('sha256').update(left + right).digest('hex');
    }

    return hash === merkleRoot;
  }

  /**
   * Get queue statistics
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.stats,
      queueLength: this.queue.length,
      batchCount: this.batches.size,
      lastAnchorTime: this.lastAnchorTime,
      timerActive: !!this.timer,
      batchSize: this.batchSize,
      intervalMs: this.intervalMs,
    };
  }

  /**
   * Export queue state
   * @returns {Object}
   */
  export() {
    return {
      queue: [...this.queue],
      batches: Array.from(this.batches.entries()),
      stats: { ...this.stats },
      lastAnchorTime: this.lastAnchorTime,
    };
  }

  /**
   * Import queue state
   * @param {Object} state - Exported state
   */
  import(state) {
    if (state.queue) {
      this.queue = [...state.queue];
    }
    if (state.batches) {
      this.batches = new Map(state.batches);
    }
    if (state.stats) {
      this.stats = { ...this.stats, ...state.stats };
    }
    if (state.lastAnchorTime) {
      this.lastAnchorTime = state.lastAnchorTime;
    }
  }

  /**
   * Cleanup - stop timer
   */
  destroy() {
    this.stopTimer();
  }
}

/**
 * Create an anchor queue instance
 * @param {Object} [config] - Configuration
 * @returns {AnchorQueue}
 */
export function createAnchorQueue(config = {}) {
  return new AnchorQueue(config);
}
