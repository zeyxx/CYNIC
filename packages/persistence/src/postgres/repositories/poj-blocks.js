/**
 * PoJ Blocks Repository
 *
 * PostgreSQL persistence for Proof of Judgment blockchain.
 * SHA-256 chain for verifiable judgment history.
 *
 * "The chain remembers what dogs forget" - κυνικός
 *
 * @module @cynic/persistence/repositories/poj-blocks
 */

'use strict';

import { getPool } from '../client.js';

export class PoJBlockRepository {
  constructor(db = null) {
    this.db = db || getPool();
  }

  /**
   * Store a new PoJ block
   * @param {Object} block - Block to store
   * @returns {Promise<Object>} Stored block record
   */
  async create(block) {
    // Extract judgment IDs from block
    const judgmentIds = (block.judgments || []).map(j =>
      j.judgment_id || j.id || `jdg_${Date.now().toString(36)}`
    );

    const { rows } = await this.db.query(`
      INSERT INTO poj_blocks (
        block_number, block_hash, prev_hash, merkle_root,
        judgment_count, judgment_ids, timestamp
      ) VALUES ($1, $2, $3, $4, $5, $6, $7)
      ON CONFLICT (block_number) DO NOTHING
      RETURNING *
    `, [
      block.slot,
      block.hash || block.block_hash,
      block.prev_hash,
      block.judgments_root || block.merkle_root || '',
      judgmentIds.length,
      judgmentIds,
      new Date(block.timestamp),
    ]);

    return rows[0] || null;
  }

  /**
   * Get the head (latest) block
   * @returns {Promise<Object|null>} Head block or null
   */
  async getHead() {
    const { rows } = await this.db.query(`
      SELECT * FROM poj_blocks
      ORDER BY block_number DESC
      LIMIT 1
    `);

    return rows[0] ? this._toBlock(rows[0]) : null;
  }

  /**
   * Get block by number (slot)
   * @param {number} blockNumber - Block number
   * @returns {Promise<Object|null>} Block or null
   */
  async findByNumber(blockNumber) {
    const { rows } = await this.db.query(
      'SELECT * FROM poj_blocks WHERE block_number = $1',
      [blockNumber]
    );

    return rows[0] ? this._toBlock(rows[0]) : null;
  }

  /**
   * Get block by hash
   * @param {string} hash - Block hash
   * @returns {Promise<Object|null>} Block or null
   */
  async findByHash(hash) {
    const { rows } = await this.db.query(
      'SELECT * FROM poj_blocks WHERE block_hash = $1',
      [hash]
    );

    return rows[0] ? this._toBlock(rows[0]) : null;
  }

  /**
   * Get blocks since a specific block number
   * @param {number} blockNumber - Starting block number (exclusive)
   * @param {number} [limit=100] - Maximum blocks to return
   * @returns {Promise<Object[]>} Array of blocks
   */
  async findSince(blockNumber, limit = 100) {
    const { rows } = await this.db.query(`
      SELECT * FROM poj_blocks
      WHERE block_number > $1
      ORDER BY block_number ASC
      LIMIT $2
    `, [blockNumber, limit]);

    return rows.map(r => this._toBlock(r));
  }

  /**
   * Get recent blocks
   * @param {number} [limit=10] - Number of blocks
   * @returns {Promise<Object[]>} Recent blocks
   */
  async findRecent(limit = 10) {
    const { rows } = await this.db.query(`
      SELECT * FROM poj_blocks
      ORDER BY block_number DESC
      LIMIT $1
    `, [limit]);

    return rows.map(r => this._toBlock(r));
  }

  /**
   * Get chain statistics
   * @returns {Promise<Object>} Chain stats
   */
  async getStats() {
    const { rows } = await this.db.query(`
      SELECT
        COUNT(*) as total_blocks,
        MAX(block_number) as head_slot,
        MIN(block_number) as genesis_slot,
        SUM(judgment_count) as total_judgments,
        AVG(judgment_count) as avg_judgments_per_block,
        MIN(timestamp) as chain_start,
        MAX(timestamp) as last_block_time
      FROM poj_blocks
    `);

    const stats = rows[0];
    return {
      totalBlocks: parseInt(stats.total_blocks) || 0,
      headSlot: parseInt(stats.head_slot) || 0,
      genesisSlot: parseInt(stats.genesis_slot) || 0,
      totalJudgments: parseInt(stats.total_judgments) || 0,
      avgJudgmentsPerBlock: parseFloat(stats.avg_judgments_per_block) || 0,
      chainStart: stats.chain_start,
      lastBlockTime: stats.last_block_time,
    };
  }

  /**
   * Verify chain integrity (check hash links)
   * @param {number} [fromBlock=0] - Starting block number
   * @param {number} [limit=100] - Max blocks to verify
   * @returns {Promise<Object>} Verification result
   */
  async verifyIntegrity(fromBlock = 0, limit = 100) {
    const { rows } = await this.db.query(`
      SELECT block_number, block_hash, prev_hash
      FROM poj_blocks
      WHERE block_number >= $1
      ORDER BY block_number ASC
      LIMIT $2
    `, [fromBlock, limit]);

    const errors = [];
    for (let i = 1; i < rows.length; i++) {
      const block = rows[i];
      const prevBlock = rows[i - 1];

      if (block.prev_hash !== prevBlock.block_hash) {
        errors.push({
          blockNumber: parseInt(block.block_number),
          expected: prevBlock.block_hash,
          actual: block.prev_hash,
        });
      }
    }

    return {
      valid: errors.length === 0,
      blocksChecked: rows.length,
      errors,
    };
  }

  /**
   * Count total blocks
   * @returns {Promise<number>} Block count
   */
  async count() {
    const { rows } = await this.db.query('SELECT COUNT(*) FROM poj_blocks');
    return parseInt(rows[0].count);
  }

  /**
   * Convert database row to block format
   * @private
   */
  _toBlock(row) {
    return {
      id: row.id,
      slot: parseInt(row.block_number),
      block_number: parseInt(row.block_number),
      hash: row.block_hash,
      block_hash: row.block_hash,
      prev_hash: row.prev_hash,
      merkle_root: row.merkle_root,
      judgments_root: row.merkle_root,
      judgment_count: row.judgment_count,
      judgment_ids: row.judgment_ids || [],
      timestamp: row.timestamp,
      created_at: row.created_at,
    };
  }
}

export default PoJBlockRepository;
