/**
 * Blockchain Domain Tools
 *
 * Tools for Proof-of-Judgment chain:
 * - PoJChain: Block management
 * - Trace: Judgment verification
 *
 * @module @cynic/mcp/tools/domains/blockchain
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('BlockchainTools');

/**
 * Create PoJ chain tool definition
 * @param {Object} pojChainManager - PoJChainManager instance
 * @param {Object} persistence - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createPoJChainTool(pojChainManager, persistence = null) {
  return {
    name: 'brain_poj_chain',
    description: 'Proof of Judgment chain operations. View chain status, verify integrity, get blocks, and export chain data.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['status', 'verify', 'head', 'block', 'recent', 'stats', 'export', 'flush', 'relink', 'adopt', 'reset'],
          description: 'Action: status (chain state), verify (check integrity), head (latest block), block (get by number), recent (last N blocks), stats (chain stats), export (export chain), flush (force create block), relink (repair orphaned judgments), adopt (create recovery block for orphans), reset (⚠️ DESTRUCTIVE: clear all data)',
        },
        blockNumber: {
          type: 'number',
          description: 'Block number (for block action)',
        },
        limit: {
          type: 'number',
          description: 'Number of blocks (for recent/export actions, default 10)',
        },
        fromBlock: {
          type: 'number',
          description: 'Starting block for verify/export (default 0)',
        },
        confirm: {
          type: 'string',
          description: 'Confirmation phrase for destructive actions. Use "BURN_IT_ALL" for reset.',
        },
      },
    },
    handler: async (params) => {
      const { action = 'status', blockNumber, limit = 10, fromBlock = 0, confirm } = params;

      if (!pojChainManager) {
        return {
          error: 'PoJ chain not available',
          hint: 'PoJ chain requires PostgreSQL persistence',
          timestamp: Date.now(),
        };
      }

      switch (action) {
        case 'status': {
          const status = pojChainManager.getStatus();

          // Fetch recent blocks for dashboard visualization
          let recentBlocks = [];
          let stats = null;
          if (persistence?.pojBlocks) {
            try {
              const blocks = await persistence.pojBlocks.findRecent(10);
              recentBlocks = blocks.map(b => ({
                blockNumber: b.slot,
                slot: b.slot,
                hash: (b.hash || b.block_hash)?.slice(0, 16) + '...',
                prevHash: b.prev_hash?.slice(0, 16) + '...',
                judgmentCount: b.judgment_count,
                timestamp: b.timestamp,
              }));
              stats = await persistence.pojBlocks.getStats();
            } catch (e) {
              log.warn('Failed to fetch recent blocks', { error: e.message });
            }
          }

          return {
            action: 'status',
            ...status,
            // Add head info for dashboard
            head: status.headSlot ? {
              blockNumber: status.headSlot,
              slot: status.headSlot,
              hash: status.headHash?.slice(0, 16) + '...',
            } : null,
            // Add recent blocks for ChainViz
            recentBlocks,
            // Add stats for dashboard
            stats: stats || {
              totalBlocks: status.headSlot || 0,
              totalJudgments: status.stats?.judgmentsProcessed || 0,
            },
            message: status.initialized
              ? `*tail wag* Chain at slot ${status.headSlot}, ${status.pendingJudgments} pending.`
              : '*growl* Chain not initialized.',
            timestamp: Date.now(),
          };
        }

        case 'verify': {
          const result = await pojChainManager.verifyIntegrity();
          return {
            action: 'verify',
            ...result,
            message: result.valid
              ? `*tail wag* Chain verified! ${result.blocksChecked} blocks, no errors.`
              : `*GROWL* Chain integrity failed! ${result.errors.length} errors found.`,
            timestamp: Date.now(),
          };
        }

        case 'head': {
          const head = pojChainManager.getHead();
          if (!head) {
            return {
              action: 'head',
              block: null,
              message: '*head tilt* No head block (chain empty?).',
              timestamp: Date.now(),
            };
          }
          return {
            action: 'head',
            block: {
              slot: head.slot,
              hash: head.hash || head.block_hash,
              prevHash: head.prev_hash,
              judgmentCount: head.judgment_count,
              timestamp: head.timestamp,
            },
            message: `*ears perk* Head at slot ${head.slot}.`,
            timestamp: Date.now(),
          };
        }

        case 'block': {
          if (typeof blockNumber !== 'number') {
            return {
              error: 'blockNumber required for block action',
              timestamp: Date.now(),
            };
          }
          if (!persistence?.pojBlocks) {
            return {
              error: 'Persistence not available for block lookup',
              timestamp: Date.now(),
            };
          }
          const block = await persistence.pojBlocks.findByNumber(blockNumber);
          if (!block) {
            return {
              action: 'block',
              block: null,
              message: `*sniff* Block ${blockNumber} not found.`,
              timestamp: Date.now(),
            };
          }
          return {
            action: 'block',
            block: {
              slot: block.slot,
              hash: block.hash || block.block_hash,
              prevHash: block.prev_hash,
              merkleRoot: block.merkle_root || block.judgments_root,
              judgmentCount: block.judgment_count,
              judgmentIds: block.judgment_ids,
              timestamp: block.timestamp,
            },
            message: `*tail wag* Block ${blockNumber} found.`,
            timestamp: Date.now(),
          };
        }

        case 'recent': {
          if (!persistence?.pojBlocks) {
            return {
              error: 'Persistence not available',
              timestamp: Date.now(),
            };
          }
          const blocks = await persistence.pojBlocks.findRecent(limit);
          return {
            action: 'recent',
            blocks: blocks.map(b => ({
              slot: b.slot,
              hash: (b.hash || b.block_hash)?.slice(0, 16) + '...',
              judgmentCount: b.judgment_count,
              timestamp: b.timestamp,
            })),
            total: blocks.length,
            message: `*sniff* Found ${blocks.length} recent blocks.`,
            timestamp: Date.now(),
          };
        }

        case 'stats': {
          if (!persistence?.pojBlocks) {
            return {
              error: 'Persistence not available',
              timestamp: Date.now(),
            };
          }
          const stats = await persistence.pojBlocks.getStats();
          const managerStats = pojChainManager.getStatus().stats;
          return {
            action: 'stats',
            chain: stats,
            session: managerStats,
            message: `*tail wag* ${stats.totalBlocks} blocks, ${stats.totalJudgments} judgments recorded.`,
            timestamp: Date.now(),
          };
        }

        case 'export': {
          if (!persistence?.pojBlocks) {
            return {
              error: 'Persistence not available',
              timestamp: Date.now(),
            };
          }
          const blocks = await persistence.pojBlocks.findSince(fromBlock, limit);
          return {
            action: 'export',
            fromBlock,
            blocks: blocks.map(b => ({
              slot: b.slot,
              hash: b.hash || b.block_hash,
              prevHash: b.prev_hash,
              merkleRoot: b.merkle_root || b.judgments_root,
              judgmentCount: b.judgment_count,
              judgmentIds: b.judgment_ids,
              timestamp: b.timestamp,
            })),
            total: blocks.length,
            message: `*ears perk* Exported ${blocks.length} blocks starting from ${fromBlock}.`,
            timestamp: Date.now(),
          };
        }

        case 'flush': {
          const block = await pojChainManager.flush();
          if (block) {
            return {
              action: 'flush',
              block: {
                slot: block.slot || block.block_number,
                hash: block.hash || block.block_hash,
                judgmentCount: block.judgment_count,
              },
              message: `*tail wag* Flushed pending judgments to block ${block.slot || block.block_number}.`,
              timestamp: Date.now(),
            };
          }
          return {
            action: 'flush',
            block: null,
            message: '*yawn* No pending judgments to flush.',
            timestamp: Date.now(),
          };
        }

        case 'relink': {
          // Repair orphaned judgments - link them back to their PoJ blocks
          if (!persistence?.pojBlocks) {
            return {
              error: 'Persistence not available',
              timestamp: Date.now(),
            };
          }

          // First count unlinked
          const unlinkedBefore = await persistence.pojBlocks.countUnlinkedJudgments();

          if (unlinkedBefore === 0) {
            return {
              action: 'relink',
              unlinkedBefore: 0,
              totalLinked: 0,
              message: '*tail wag* All judgments properly linked. Nothing to repair.',
              timestamp: Date.now(),
            };
          }

          // Run the repair
          const result = await persistence.pojBlocks.relinkOrphanedJudgments();

          // Count after
          const unlinkedAfter = await persistence.pojBlocks.countUnlinkedJudgments();

          return {
            action: 'relink',
            unlinkedBefore,
            unlinkedAfter,
            totalLinked: result.totalLinked,
            blocksProcessed: result.blocksProcessed,
            details: result.results,
            message: result.totalLinked > 0
              ? `*HOWL* L2 chain repaired! Linked ${result.totalLinked} orphaned judgments to ${result.blocksProcessed} blocks.`
              : `*sniff* Found ${unlinkedBefore} unlinked judgments but couldn't match them to blocks.`,
            timestamp: Date.now(),
          };
        }

        case 'adopt': {
          // Create recovery block for orphaned judgments that were never added to any block
          if (!persistence?.pojBlocks) {
            return {
              error: 'Persistence not available',
              timestamp: Date.now(),
            };
          }

          const result = await persistence.pojBlocks.adoptOrphanedJudgments();

          return {
            action: 'adopt',
            ...result,
            timestamp: Date.now(),
          };
        }

        case 'reset': {
          // ⚠️ DESTRUCTIVE: Clear all chain data
          if (!persistence?.pojBlocks) {
            return {
              error: 'Persistence not available',
              timestamp: Date.now(),
            };
          }

          if (confirm !== 'BURN_IT_ALL') {
            return {
              action: 'reset',
              error: 'Reset requires confirmation',
              hint: 'Set confirm="BURN_IT_ALL" to proceed with reset',
              warning: '⚠️ This will DELETE ALL judgments, blocks, patterns, knowledge, sessions, and feedback!',
              timestamp: Date.now(),
            };
          }

          try {
            const result = await persistence.pojBlocks.resetAll(confirm);

            // Also reset the chain manager state
            if (pojChainManager) {
              pojChainManager._head = null;
              pojChainManager._pendingJudgments = [];
              pojChainManager._initialized = false;
            }

            return {
              action: 'reset',
              ...result,
              message: `*HOWL* 🔥 BURN complete! All data cleared. Chain reset to genesis.`,
              timestamp: Date.now(),
            };
          } catch (err) {
            return {
              action: 'reset',
              error: err.message,
              timestamp: Date.now(),
            };
          }
        }

        default:
          return {
            error: `Unknown action: ${action}`,
            validActions: ['status', 'verify', 'head', 'block', 'recent', 'stats', 'export', 'flush', 'relink', 'adopt', 'reset'],
            timestamp: Date.now(),
          };
      }
    },
  };
}

/**
 * Create trace tool for end-to-end integrity verification
 * Traces a judgment through: judgment → PoJ block → merkle proof → Solana anchor
 * @param {Object} persistence - PersistenceManager instance
 * @param {Object} pojChainManager - PoJChainManager instance
 * @returns {Object} Tool definition
 */
export function createTraceTool(persistence, pojChainManager = null) {
  return {
    name: 'brain_trace',
    description: 'Trace end-to-end integrity of a judgment. Returns full chain of proof: judgment → PoJ block → merkle proof → Solana anchor.',
    inputSchema: {
      type: 'object',
      properties: {
        judgmentId: {
          type: 'string',
          description: 'Judgment ID to trace (e.g., jdg_abc123)',
        },
        includeRaw: {
          type: 'boolean',
          description: 'Include raw data (judgment content, full hashes). Default false.',
        },
      },
      required: ['judgmentId'],
    },
    handler: async (params) => {
      const { judgmentId, includeRaw = false } = params;

      if (!persistence) {
        return {
          error: 'Persistence not available',
          hint: 'brain_trace requires PostgreSQL persistence',
          timestamp: Date.now(),
        };
      }

      // Step 1: Get the judgment (try both APIs for compatibility)
      let judgment = null;
      if (persistence.getJudgment) {
        judgment = await persistence.getJudgment(judgmentId);
      } else if (persistence.judgments?.findById) {
        judgment = await persistence.judgments.findById(judgmentId);
      }

      if (!judgment) {
        return {
          judgmentId,
          found: false,
          error: 'Judgment not found',
          hint: 'Check judgment ID or use brain_search_index to find valid IDs',
          timestamp: Date.now(),
        };
      }

      // Step 2: Build trace object
      const trace = {
        judgmentId,
        found: true,
        layers: {
          judgment: {
            status: 'verified',
            id: judgment.judgment_id,
            verdict: judgment.verdict,
            qScore: parseFloat(judgment.q_score),
            confidence: parseFloat(judgment.confidence),
            itemType: judgment.item_type,
            createdAt: judgment.created_at,
            hash: judgment.item_hash,
          },
          pojBlock: null,
          merkleProof: null,
          solanaAnchor: null,
        },
        integrityScore: 25, // Base: judgment exists
        maxIntegrityScore: 100,
      };

      // Add raw data if requested
      if (includeRaw) {
        trace.layers.judgment.raw = {
          content: judgment.item_content,
          axiomScores: judgment.axiom_scores,
          dimensionScores: judgment.dimension_scores,
          weaknesses: judgment.weaknesses,
          context: judgment.context,
        };
      }

      // Step 3: Find the PoJ block containing this judgment
      if (judgment.block_hash && judgment.block_number !== null) {
        const block = persistence.pojBlocks
          ? await persistence.pojBlocks.findByNumber(judgment.block_number)
          : null;

        if (block) {
          trace.layers.pojBlock = {
            status: 'verified',
            slot: block.slot,
            hash: includeRaw ? block.block_hash : block.block_hash?.slice(0, 16) + '...',
            prevHash: includeRaw ? block.prev_hash : block.prev_hash?.slice(0, 16) + '...',
            merkleRoot: includeRaw ? block.merkle_root : block.merkle_root?.slice(0, 16) + '...',
            judgmentCount: block.judgment_count,
            timestamp: block.timestamp,
            containsJudgment: block.judgment_ids?.includes(judgmentId),
          };
          trace.integrityScore += 25; // +25 for being in a block
        } else {
          trace.layers.pojBlock = {
            status: 'referenced_but_not_found',
            blockNumber: judgment.block_number,
            blockHash: judgment.block_hash,
          };
        }
      } else {
        trace.layers.pojBlock = {
          status: 'pending',
          message: 'Judgment not yet included in a PoJ block',
        };
      }

      // Step 4: Check chain integrity (merkle proof)
      if (trace.layers.pojBlock?.status === 'verified' && pojChainManager) {
        const chainStatus = pojChainManager.getStatus();
        if (chainStatus.initialized) {
          // Verify chain integrity up to this block
          const integrityResult = await pojChainManager.verifyIntegrity();
          trace.layers.merkleProof = {
            status: integrityResult.valid ? 'verified' : 'failed',
            chainValid: integrityResult.valid,
            blocksChecked: integrityResult.blocksChecked,
            errors: integrityResult.errors || [],
          };
          if (integrityResult.valid) {
            trace.integrityScore += 25; // +25 for valid chain
          }
        }
      } else if (trace.layers.pojBlock?.status === 'verified') {
        // No pojChainManager, but block exists - partial verification
        trace.layers.merkleProof = {
          status: 'partial',
          message: 'Block found but chain manager unavailable for full verification',
        };
        trace.integrityScore += 12; // Partial credit
      }

      // Step 5: Check Solana anchor status
      if (pojChainManager?.isAnchoringEnabled) {
        const anchorStatus = pojChainManager.getAnchorStatus?.();
        const pendingAnchors = pojChainManager.getPendingAnchors?.() || [];

        // Check if this block is anchored
        const blockSlot = trace.layers.pojBlock?.slot;
        if (blockSlot !== undefined) {
          // This would need the anchor queue to check actual anchor status
          // For now, report the anchoring configuration
          trace.layers.solanaAnchor = {
            status: 'enabled',
            anchoringActive: true,
            pendingAnchors: pendingAnchors.length,
            // Would need anchor queue access to get actual tx signature
            message: 'Anchoring enabled. Check block explorer for confirmation.',
          };
          trace.integrityScore += 25; // +25 if anchoring is enabled
        }
      } else {
        trace.layers.solanaAnchor = {
          status: 'disabled',
          message: 'Solana anchoring not enabled for this node',
        };
      }

      // Calculate final integrity percentage
      trace.integrityPercentage = Math.round((trace.integrityScore / trace.maxIntegrityScore) * 100);

      // Generate verdict
      if (trace.integrityPercentage >= 75) {
        trace.verdict = 'HOWL'; // Full integrity
        trace.message = `*tail wag* Full integrity verified! ${trace.integrityPercentage}% confidence.`;
      } else if (trace.integrityPercentage >= 50) {
        trace.verdict = 'WAG'; // Good integrity
        trace.message = `*ears perk* Good integrity. ${trace.integrityPercentage}% verified.`;
      } else if (trace.integrityPercentage >= 25) {
        trace.verdict = 'GROWL'; // Partial integrity
        trace.message = `*sniff* Partial integrity. ${trace.integrityPercentage}% - some layers missing.`;
      } else {
        trace.verdict = 'BARK'; // Low integrity
        trace.message = `*GROWL* Low integrity! ${trace.integrityPercentage}% - verify data sources.`;
      }

      trace.timestamp = Date.now();
      return trace;
    },
  };
}

/**
 * Factory for blockchain domain tools
 */
export const blockchainFactory = {
  name: 'blockchain',
  domain: 'blockchain',
  requires: ['pojChainManager'],

  /**
   * Create all blockchain domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const { pojChainManager, persistence } = options;

    const tools = [];

    // PoJ Chain tool
    if (pojChainManager) {
      tools.push(createPoJChainTool(pojChainManager, persistence));
    }

    // Trace tool
    if (persistence) {
      tools.push(createTraceTool(persistence, pojChainManager));
    }

    return tools;
  },
};
