/**
 * PoJ Blocks Repository
 *
 * PostgreSQL persistence for Proof of Judgment blockchain.
 * SHA-256 chain for verifiable judgment history.
 *
 * "The chain remembers what dogs forget" - κυνικός
 *
 * Implements: BaseRepository
 *
 * @module @cynic/persistence/repositories/poj-blocks
 */

'use strict';

import crypto from 'crypto';
import { getPool } from '../client.js';
import { BaseRepository } from '../../interfaces/IRepository.js';

/**
 * PoJ Blocks Repository
 *
 * LSP compliant - implements standard repository interface.
 * Note: PoJ blocks are append-only (blockchain).
 *
 * @extends BaseRepository
 */
export class PoJBlockRepository extends BaseRepository {
  constructor(db = null) {
    super(db || getPool());
  }

  /**
   * Store a new PoJ block and link judgments
   * @param {Object} block - Block to store
   * @returns {Promise<Object>} Stored block record
   */
  async create(block) {
    // Extract judgment IDs from block
    const judgmentIds = (block.judgments || []).map(j =>
      j.judgment_id || j.id || `jdg_${Date.now().toString(36)}`
    );

    const blockHash = block.hash || block.block_hash;
    const blockNumber = block.slot;

    const { rows } = await this.db.query(`
      INSERT INTO poj_blocks (
        block_number, block_hash, prev_hash, merkle_root,
        judgment_count, judgment_ids, timestamp
      ) VALUES ($1, $2, $3, $4, $5, $6, $7)
      ON CONFLICT (block_number) DO NOTHING
      RETURNING *
    `, [
      blockNumber,
      blockHash,
      block.prev_hash,
      block.judgments_root || block.merkle_root || '',
      judgmentIds.length,
      judgmentIds,
      new Date(block.timestamp),
    ]);

    const storedBlock = rows[0] || null;

    // CRITICAL: Link judgments to this block
    // This is the L2 chain linkage - judgments become part of the block
    if (storedBlock && judgmentIds.length > 0) {
      await this.db.query(`
        UPDATE judgments
        SET block_hash = $1,
            block_number = $2,
            prev_hash = COALESCE(prev_hash, $3)
        WHERE judgment_id = ANY($4)
          AND block_hash IS NULL
      `, [
        blockHash,
        blockNumber,
        block.prev_hash,
        judgmentIds,
      ]);
    }

    return storedBlock;
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
   * Re-link orphaned judgments to their blocks
   * Run this to repair existing data where judgments have NULL block_hash
   * @returns {Promise<Object>} Repair result
   */
  async relinkOrphanedJudgments() {
    // Find all blocks with their judgment_ids
    const { rows: blocks } = await this.db.query(`
      SELECT block_number, block_hash, prev_hash, judgment_ids
      FROM poj_blocks
      WHERE judgment_ids IS NOT NULL
        AND array_length(judgment_ids, 1) > 0
      ORDER BY block_number ASC
    `);

    let totalLinked = 0;
    const results = [];

    for (const block of blocks) {
      if (!block.judgment_ids || block.judgment_ids.length === 0) continue;

      const { rowCount } = await this.db.query(`
        UPDATE judgments
        SET block_hash = $1,
            block_number = $2,
            prev_hash = COALESCE(prev_hash, $3)
        WHERE judgment_id = ANY($4)
          AND block_hash IS NULL
      `, [
        block.block_hash,
        block.block_number,
        block.prev_hash,
        block.judgment_ids,
      ]);

      if (rowCount > 0) {
        totalLinked += rowCount;
        results.push({
          blockNumber: parseInt(block.block_number),
          blockHash: block.block_hash.slice(0, 16) + '...',
          linked: rowCount,
        });
      }
    }

    return {
      totalLinked,
      blocksProcessed: blocks.length,
      results,
    };
  }

  /**
   * Get count of unlinked judgments
   * @returns {Promise<number>} Count of judgments with NULL block_hash
   */
  async countUnlinkedJudgments() {
    const { rows } = await this.db.query(`
      SELECT COUNT(*) FROM judgments WHERE block_hash IS NULL
    `);
    return parseInt(rows[0].count);
  }

  /**
   * Adopt orphaned judgments into a new recovery block
   * Creates a special block to include judgments that were never added to any block
   * @returns {Promise<Object>} Adoption result
   */
  async adoptOrphanedJudgments() {
    // Find all orphaned judgments
    const { rows: orphans } = await this.db.query(`
      SELECT judgment_id, q_score, verdict, created_at
      FROM judgments
      WHERE block_hash IS NULL
      ORDER BY created_at ASC
    `);

    if (orphans.length === 0) {
      return {
        adopted: 0,
        block: null,
        message: 'No orphaned judgments to adopt',
      };
    }

    // Get current head block
    const head = await this.getHead();
    const prevHash = head?.block_hash || '0'.repeat(64);
    const nextSlot = head ? parseInt(head.block_number) + 1 : 0;

    // Build judgment data for merkle root
    const judgmentIds = orphans.map(j => j.judgment_id);
    const judgmentsData = orphans.map(j => ({
      id: j.judgment_id,
      q_score: parseFloat(j.q_score),
      verdict: j.verdict,
    }));

    // Calculate merkle root (simple hash of all judgment IDs)
    const merkleRoot = crypto
      .createHash('sha256')
      .update(judgmentIds.join('|'))
      .digest('hex');

    // Create block hash
    const blockData = `${nextSlot}|${prevHash}|${merkleRoot}|${Date.now()}`;
    const blockHash = crypto
      .createHash('sha256')
      .update(blockData)
      .digest('hex');

    // Create the recovery block
    const { rows: blockRows } = await this.db.query(`
      INSERT INTO poj_blocks (
        block_number, block_hash, prev_hash, merkle_root,
        judgment_count, judgment_ids, timestamp
      ) VALUES ($1, $2, $3, $4, $5, $6, NOW())
      RETURNING *
    `, [
      nextSlot,
      blockHash,
      prevHash,
      merkleRoot,
      judgmentIds.length,
      judgmentIds,
    ]);

    const block = blockRows[0];

    // Link orphaned judgments to this block
    const { rowCount } = await this.db.query(`
      UPDATE judgments
      SET block_hash = $1,
          block_number = $2,
          prev_hash = $3
      WHERE judgment_id = ANY($4)
    `, [
      blockHash,
      nextSlot,
      prevHash,
      judgmentIds,
    ]);

    return {
      adopted: rowCount,
      block: {
        slot: nextSlot,
        hash: blockHash.slice(0, 16) + '...',
        prevHash: prevHash.slice(0, 16) + '...',
        judgmentCount: judgmentIds.length,
      },
      judgments: judgmentIds,
      message: `*tail wag* Adopted ${rowCount} orphans into recovery block ${nextSlot}`,
    };
  }

  /**
   * Reset the entire PoJ chain and related data
   * ⚠️ DESTRUCTIVE: Clears all judgments, blocks, patterns, knowledge
   * @param {string} confirmPhrase - Must be "BURN_IT_ALL" to confirm
   * @returns {Promise<Object>} Reset result with counts
   */
  async resetAll(confirmPhrase) {
    if (confirmPhrase !== 'BURN_IT_ALL') {
      throw new Error('Reset requires confirmation phrase: BURN_IT_ALL');
    }

    // Get counts before reset
    const beforeCounts = {};
    const tables = ['judgments', 'poj_blocks', 'patterns', 'knowledge', 'sessions', 'feedback'];

    for (const table of tables) {
      try {
        const { rows } = await this.db.query(`SELECT COUNT(*) FROM ${table}`);
        beforeCounts[table] = parseInt(rows[0].count);
      } catch (e) {
        beforeCounts[table] = 0;
      }
    }

    // Truncate in correct order (foreign key safe)
    await this.db.query(`
      TRUNCATE judgments, poj_blocks, patterns, knowledge, sessions, feedback RESTART IDENTITY CASCADE
    `);

    return {
      reset: true,
      tablesCleared: tables,
      beforeCounts,
      timestamp: new Date().toISOString(),
    };
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

  // ═══════════════════════════════════════════════════════════════════════════
  // BaseRepository Interface Methods (LSP compliance)
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Find block by ID (block number or hash)
   * @param {string|number} id - Block number or hash
   * @returns {Promise<Object|null>}
   */
  async findById(id) {
    if (typeof id === 'number' || /^\d+$/.test(id)) {
      return this.findByNumber(parseInt(id, 10));
    }
    return this.findByHash(id);
  }

  /**
   * List blocks with pagination
   * @param {Object} [options={}] - Query options
   * @returns {Promise<Object[]>}
   */
  async list(options = {}) {
    const { limit = 10, offset = 0 } = options;

    const { rows } = await this.db.query(`
      SELECT * FROM poj_blocks
      ORDER BY block_number DESC
      LIMIT $1 OFFSET $2
    `, [limit, offset]);

    return rows.map(r => this._toBlock(r));
  }

  /**
   * Update is not supported for append-only blockchain
   * @throws {Error} PoJ blocks are immutable
   */
  async update(id, data) {
    throw new Error('PoJ blocks are append-only and cannot be updated');
  }

  /**
   * Delete is not supported for append-only blockchain
   * @throws {Error} PoJ blocks are immutable
   */
  async delete(id) {
    throw new Error('PoJ blocks are append-only and cannot be deleted');
  }

}

export default PoJBlockRepository;
