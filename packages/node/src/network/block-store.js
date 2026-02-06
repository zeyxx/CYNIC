/**
 * Block Store
 *
 * Persistent block storage backed by PostgreSQL.
 * Provides getBlocks/storeBlock callbacks for StateSyncManager.
 *
 * Wire into NetworkNode via:
 *   node.wireBlockStore(blockStore.callbacks());
 *
 * "Chaque bloc est une vérité gravée" - κυνικός
 *
 * @module @cynic/node/network/block-store
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger } from '@cynic/core';

const log = createLogger('BlockStore');

export class BlockStore extends EventEmitter {
  /**
   * @param {Object} [options]
   * @param {Object} [options.pool] - PostgreSQL connection pool
   */
  constructor(options = {}) {
    super();
    this._pool = options.pool || null;
    this._memoryStore = new Map(); // slot -> block (fallback when no pool)
    this._stats = {
      blocksStored: 0,
      blocksRetrieved: 0,
      errors: 0,
    };
  }

  /**
   * Store a block. Uses PostgreSQL if available, memory otherwise.
   *
   * @param {Object} block
   * @param {number} block.slot
   * @param {string} block.hash
   * @param {string} block.proposer
   * @param {string} [block.merkle_root]
   * @param {Array} [block.judgments]
   * @param {number} [block.judgment_count]
   * @param {string} [block.prev_hash]
   * @param {number} [block.timestamp]
   */
  async storeBlock(block) {
    if (!block || block.slot === undefined) return;

    if (this._pool) {
      try {
        await this._pool.query(
          `INSERT INTO blocks (slot, hash, proposer, merkle_root, judgments, judgment_count, parent_hash, timestamp)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
           ON CONFLICT (slot) DO UPDATE SET
             hash = EXCLUDED.hash,
             proposer = EXCLUDED.proposer,
             merkle_root = EXCLUDED.merkle_root,
             judgments = EXCLUDED.judgments,
             judgment_count = EXCLUDED.judgment_count,
             parent_hash = EXCLUDED.parent_hash`,
          [
            block.slot,
            block.hash || block.block_hash,
            block.proposer,
            block.merkle_root || block.judgments_root || null,
            JSON.stringify(block.judgments || []),
            block.judgment_count || (block.judgments?.length || 0),
            block.prev_hash || block.parent_hash || null,
            block.timestamp || Date.now(),
          ]
        );
      } catch (err) {
        this._stats.errors++;
        log.error('storeBlock failed', { slot: block.slot, error: err.message });
        // Fallback to memory
        this._memoryStore.set(block.slot, block);
        return;
      }
    } else {
      this._memoryStore.set(block.slot, block);
    }

    this._stats.blocksStored++;
    this.emit('block:stored', { slot: block.slot, hash: block.hash || block.block_hash });
  }

  /**
   * Get a block by slot.
   *
   * @param {number} slot
   * @returns {Promise<Object|null>}
   */
  async getBlock(slot) {
    if (this._pool) {
      try {
        const { rows: [row] } = await this._pool.query(
          'SELECT * FROM blocks WHERE slot = $1',
          [slot]
        );
        if (row) {
          this._stats.blocksRetrieved++;
          return this._normalizeRow(row);
        }
        return null;
      } catch (err) {
        this._stats.errors++;
        log.error('getBlock failed', { slot, error: err.message });
      }
    }

    const block = this._memoryStore.get(slot);
    if (block) this._stats.blocksRetrieved++;
    return block || null;
  }

  /**
   * Get blocks in a range (inclusive).
   * Used by StateSyncManager for state sync.
   *
   * @param {number} fromSlot
   * @param {number} toSlot
   * @returns {Promise<Object[]>}
   */
  async getBlocks(fromSlot, toSlot) {
    if (this._pool) {
      try {
        const { rows } = await this._pool.query(
          'SELECT * FROM blocks WHERE slot >= $1 AND slot <= $2 ORDER BY slot ASC',
          [fromSlot, toSlot]
        );
        this._stats.blocksRetrieved += rows.length;
        return rows.map(r => this._normalizeRow(r));
      } catch (err) {
        this._stats.errors++;
        log.error('getBlocks failed', { fromSlot, toSlot, error: err.message });
      }
    }

    // Memory fallback
    const blocks = [];
    for (let slot = fromSlot; slot <= toSlot; slot++) {
      const block = this._memoryStore.get(slot);
      if (block) blocks.push(block);
    }
    this._stats.blocksRetrieved += blocks.length;
    return blocks;
  }

  /**
   * Get the latest block.
   *
   * @returns {Promise<Object|null>}
   */
  async getLatestBlock() {
    if (this._pool) {
      try {
        const { rows: [row] } = await this._pool.query(
          'SELECT * FROM blocks ORDER BY slot DESC LIMIT 1'
        );
        return row ? this._normalizeRow(row) : null;
      } catch (err) {
        this._stats.errors++;
      }
    }

    // Memory fallback
    let maxSlot = -1;
    let latest = null;
    for (const [slot, block] of this._memoryStore) {
      if (slot > maxSlot) { maxSlot = slot; latest = block; }
    }
    return latest;
  }

  /**
   * Return callbacks for NetworkNode.wireBlockStore()
   *
   * @returns {{ getBlocks: Function, storeBlock: Function }}
   */
  callbacks() {
    return {
      getBlocks: this.getBlocks.bind(this),
      storeBlock: this.storeBlock.bind(this),
    };
  }

  /**
   * Normalize a PostgreSQL row to a block object.
   * @private
   */
  _normalizeRow(row) {
    return {
      slot: Number(row.slot),
      hash: row.hash,
      proposer: row.proposer,
      merkle_root: row.merkle_root,
      judgments: row.judgments || [],
      judgment_count: row.judgment_count || 0,
      prev_hash: row.parent_hash,
      parent_hash: row.parent_hash,
      timestamp: Number(row.timestamp),
    };
  }

  /** @returns {Object} */
  get stats() {
    return {
      ...this._stats,
      memorySize: this._memoryStore.size,
    };
  }
}
