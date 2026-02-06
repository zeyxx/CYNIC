/**
 * PoJ Chain Exporter
 *
 * Handles export/import of PoJ chain data for backup and restore.
 *
 * "The chain remembers" - κυνικός
 *
 * @module @cynic/mcp/poj-chain/chain-exporter
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('PoJChainExporter');

/**
 * Handles chain export and import operations
 */
export class ChainExporter {
  /**
   * Export chain data for backup
   * @param {Object} persistence - PersistenceManager instance
   * @param {Object} [options] - Export options
   * @returns {Promise<Object>} Exportable chain data
   */
  async exportChain(persistence, options = {}) {
    const { fromBlock = 0, limit = 1000 } = options;

    if (!persistence?.capabilities?.pojChain) {
      return { error: 'Persistence not available', blocks: [] };
    }

    // Use PersistenceManager methods which handle fallback internally
    const blocks = await persistence.getRecentPoJBlocks(limit);
    const stats = await persistence.getPoJStats();

    // Filter blocks by fromBlock slot
    const filteredBlocks = blocks.filter(b => b.slot >= fromBlock);

    return {
      version: '1.0.0',
      exportedAt: new Date().toISOString(),
      chainStats: stats,
      blocks: filteredBlocks.map(b => ({
        slot: b.slot,
        hash: b.hash || b.block_hash,
        prevHash: b.prev_hash,
        merkleRoot: b.merkle_root || b.judgments_root,
        judgmentCount: b.judgment_count,
        judgmentIds: b.judgment_ids,
        timestamp: b.timestamp instanceof Date ? b.timestamp.toISOString() : b.timestamp,
      })),
      totalBlocks: filteredBlocks.length,
    };
  }

  /**
   * Import chain data from backup
   * @param {Object} persistence - PersistenceManager instance
   * @param {Object} chainData - Exported chain data
   * @param {Object} [options] - Import options
   * @returns {Promise<Object>} Import result
   */
  async importChain(persistence, chainData, options = {}) {
    const { validateLinks = true, skipExisting = true } = options;

    if (!persistence?.capabilities?.pojChain) {
      return { error: 'Persistence not available', imported: 0 };
    }

    if (!chainData?.blocks || !Array.isArray(chainData.blocks)) {
      return { error: 'Invalid chain data format', imported: 0 };
    }

    const results = {
      imported: 0,
      skipped: 0,
      errors: [],
    };

    // Sort blocks by slot
    const sortedBlocks = [...chainData.blocks].sort((a, b) => a.slot - b.slot);

    // Validate chain links if requested
    if (validateLinks && sortedBlocks.length > 1) {
      for (let i = 1; i < sortedBlocks.length; i++) {
        const block = sortedBlocks[i];
        const prevBlock = sortedBlocks[i - 1];
        if (block.prevHash !== prevBlock.hash) {
          results.errors.push({
            slot: block.slot,
            error: `Invalid prev_hash: expected ${prevBlock.hash}, got ${block.prevHash}`,
          });
        }
      }

      if (results.errors.length > 0) {
        return {
          error: 'Chain validation failed',
          ...results,
        };
      }
    }

    // Import blocks
    for (const block of sortedBlocks) {
      try {
        // Check if exists
        if (skipExisting) {
          const existing = await persistence.getPoJBlockBySlot(block.slot);
          if (existing) {
            results.skipped++;
            continue;
          }
        }

        // Store block
        await persistence.storePoJBlock({
          slot: block.slot,
          hash: block.hash,
          block_hash: block.hash,
          prev_hash: block.prevHash,
          judgments_root: block.merkleRoot,
          merkle_root: block.merkleRoot,
          judgments: block.judgmentIds?.map(id => ({ judgment_id: id })) || [],
          timestamp: new Date(block.timestamp).getTime(),
        });

        results.imported++;
      } catch (err) {
        results.errors.push({
          slot: block.slot,
          error: err.message,
        });
      }
    }

    log.info('Chain import complete', {
      imported: results.imported,
      skipped: results.skipped,
      errors: results.errors.length,
    });

    return results;
  }

  /**
   * Verify chain integrity
   * @param {Object} persistence - PersistenceManager instance
   * @returns {Promise<Object>} Verification result
   */
  async verifyIntegrity(persistence) {
    if (!persistence?.capabilities?.pojChain) {
      return { valid: true, blocksChecked: 0, errors: [] };
    }
    return await persistence.verifyPoJChain();
  }
}

export default ChainExporter;
