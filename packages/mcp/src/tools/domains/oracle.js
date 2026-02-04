/**
 * Oracle Domain Tools
 *
 * MCP tools for CYNIC's Oracle token scoring system.
 * Exposes the 17-dimension φ-governed token judgment via MCP.
 *
 * - brain_oracle_score: Score a Solana token (17-dim + Q-Score + K-Score)
 * - brain_oracle_watchlist: Manage token watchlist (add/remove/list/check/alerts)
 * - brain_oracle_stats: Aggregate stats of all Oracle judgments
 *
 * "φ distrusts φ" — Every score capped at 61.8% confidence
 *
 * @module @cynic/mcp/tools/domains/oracle
 */

'use strict';

import { PHI_INV } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// TOOL IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create brain_oracle_score tool
 *
 * Fetches on-chain data, scores across 17 dimensions, stores verdict.
 *
 * @param {Object} oracle - Oracle services { fetcher, scorer, memory, watchlist }
 * @returns {Object} Tool definition
 */
export function createOracleScoreTool(oracle) {
  return {
    name: 'brain_oracle_score',
    description: `Score a Solana token across 17 φ-governed dimensions.
Fetches real on-chain data (Helius DAS + DexScreener), computes:
- Q-Score (0-100): Unified quality score
- K-Score: Metal tier (Rust → Diamond)
- Verdict: HOWL (strong) / WAG (acceptable) / GROWL (caution) / BARK (danger)
- 17 dimension breakdown grouped by 4 axioms (PHI, VERIFY, CULTURE, BURN)
- Trajectory (if previously judged)

Requires HELIUS_API_KEY for full data. Max confidence: ${(PHI_INV * 100).toFixed(1)}%.`,
    inputSchema: {
      type: 'object',
      properties: {
        mint: {
          type: 'string',
          description: 'Solana token mint address (32-44 chars base58)',
        },
        label: {
          type: 'string',
          description: 'Optional human label (e.g. "SOL", "$asdfasdfa")',
        },
      },
      required: ['mint'],
    },
    handler: async (params) => {
      const { mint, label } = params;

      if (!oracle?.fetcher || !oracle?.scorer) {
        return {
          success: false,
          error: 'Oracle not available. Requires @cynic/observatory and HELIUS_API_KEY.',
        };
      }

      try {
        // 1. Fetch on-chain data
        const tokenData = await oracle.fetcher.getTokenData(mint);

        // 2. Get trajectory (previous judgments)
        let trajectory = null;
        if (oracle.memory) {
          try {
            trajectory = await oracle.memory.getTrajectory(mint);
          } catch { /* memory optional */ }
        }

        // 3. Score across 17 dimensions
        const verdict = oracle.scorer.score(tokenData);

        // 4. Store judgment
        if (oracle.memory) {
          try {
            await oracle.memory.store({
              ...verdict,
              mint,
              name: tokenData.name,
              symbol: tokenData.symbol,
            });
          } catch { /* storage optional */ }
        }

        return {
          success: true,
          mint,
          label: label || tokenData.symbol || null,
          name: tokenData.name,
          symbol: tokenData.symbol,
          verdict: verdict.verdict,
          qScore: verdict.qScore,
          kScore: verdict.kScore,
          tier: verdict.tier,
          confidence: verdict.confidence,
          dimensions: verdict.dimensions,
          axiomScores: verdict.axiomScores,
          weaknesses: verdict.weaknesses,
          trajectory: trajectory ? {
            judgmentCount: trajectory.length,
            previous: trajectory[0] ? {
              verdict: trajectory[0].verdict,
              qScore: trajectory[0].q_score,
              judgedAt: trajectory[0].judged_at,
            } : null,
            trend: trajectory.length >= 2
              ? (trajectory[0].q_score > trajectory[1].q_score ? 'improving' : 'declining')
              : null,
          } : null,
          tokenData: {
            price: tokenData.pricePerToken,
            marketCap: tokenData.marketCap,
            holders: tokenData.holderCount,
            supply: tokenData.circulatingSupply,
            age: tokenData.age,
          },
        };
      } catch (err) {
        return {
          success: false,
          error: err.message,
          mint,
        };
      }
    },
  };
}

/**
 * Create brain_oracle_watchlist tool
 *
 * Manage the autonomous token watchlist.
 *
 * @param {Object} oracle - Oracle services { fetcher, scorer, memory, watchlist }
 * @returns {Object} Tool definition
 */
export function createOracleWatchlistTool(oracle) {
  return {
    name: 'brain_oracle_watchlist',
    description: `Manage CYNIC's Oracle token watchlist.
Actions:
- add: Add a mint to watch (with optional label)
- remove: Remove a mint from watchlist
- list: List all watched tokens with last verdict
- check: Re-score all watched tokens now
- alerts: View recent verdict changes/alerts`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['add', 'remove', 'list', 'check', 'alerts'],
          description: 'Watchlist action',
        },
        mint: {
          type: 'string',
          description: 'Token mint address (for add/remove)',
        },
        label: {
          type: 'string',
          description: 'Human label for the token (for add)',
        },
        limit: {
          type: 'number',
          description: 'Max items to return (default: 20)',
        },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const { action, mint, label, limit = 20 } = params;

      if (!oracle?.watchlist) {
        return {
          success: false,
          error: 'Oracle watchlist not available. Requires PostgreSQL persistence.',
        };
      }

      try {
        switch (action) {
          case 'add': {
            if (!mint) return { success: false, error: 'mint required for add' };
            await oracle.watchlist.add(mint, label);
            return { success: true, action: 'add', mint, label };
          }

          case 'remove': {
            if (!mint) return { success: false, error: 'mint required for remove' };
            await oracle.watchlist.remove(mint);
            return { success: true, action: 'remove', mint };
          }

          case 'list': {
            const items = await oracle.watchlist.list(limit);
            return {
              success: true,
              action: 'list',
              count: items.length,
              items: items.map(item => ({
                mint: item.mint,
                label: item.label,
                lastVerdict: item.last_verdict,
                lastQScore: item.last_q_score,
                lastKScore: item.last_k_score,
                addedAt: item.added_at,
                lastCheckedAt: item.last_checked_at,
              })),
            };
          }

          case 'check': {
            const results = await oracle.watchlist.checkAll();
            return {
              success: true,
              action: 'check',
              checked: results.checked,
              alerts: results.alerts,
              errors: results.errors,
            };
          }

          case 'alerts': {
            const alerts = await oracle.watchlist.getAlerts(limit);
            return {
              success: true,
              action: 'alerts',
              count: alerts.length,
              alerts: alerts.map(a => ({
                mint: a.mint,
                type: a.alert_type,
                oldVerdict: a.old_verdict,
                newVerdict: a.new_verdict,
                oldQScore: a.old_q_score,
                newQScore: a.new_q_score,
                message: a.message,
                createdAt: a.created_at,
              })),
            };
          }

          default:
            return { success: false, error: `Unknown action: ${action}` };
        }
      } catch (err) {
        return {
          success: false,
          error: err.message,
          action,
        };
      }
    },
  };
}

/**
 * Create brain_oracle_stats tool
 *
 * Aggregate statistics from all Oracle judgments.
 *
 * @param {Object} oracle - Oracle services { fetcher, scorer, memory, watchlist }
 * @returns {Object} Tool definition
 */
export function createOracleStatsTool(oracle) {
  return {
    name: 'brain_oracle_stats',
    description: 'Get aggregate statistics from all Oracle token judgments.',
    inputSchema: {
      type: 'object',
      properties: {},
    },
    handler: async () => {
      if (!oracle?.memory) {
        return {
          success: false,
          error: 'Oracle memory not available. Requires PostgreSQL persistence.',
        };
      }

      try {
        const stats = await oracle.memory.getStats();
        return {
          success: true,
          stats,
        };
      } catch (err) {
        return {
          success: false,
          error: err.message,
        };
      }
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY (OCP-compliant)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Oracle domain factory
 */
export const oracleFactory = {
  name: 'oracle',
  domain: 'oracle',
  requires: [],

  create(options) {
    const { oracle } = options;
    if (!oracle) return [];

    const tools = [
      createOracleScoreTool(oracle),
    ];

    if (oracle.watchlist) {
      tools.push(createOracleWatchlistTool(oracle));
    }

    if (oracle.memory) {
      tools.push(createOracleStatsTool(oracle));
    }

    return tools;
  },
};

export default oracleFactory;
