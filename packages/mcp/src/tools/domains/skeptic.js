/**
 * Skeptic Domain Tools
 *
 * MCP tools for CYNIC's Kabbalistic verification service.
 * The 18th path between Binah and Gevurah.
 *
 * TZIMTZUM (צמצום) → BERUR (בירור) → TIKKUN (תיקון)
 * Contraction    →  Clarification →  Repair
 *
 * "גם זו לטובה" — "This too is for the good"
 *
 * @module @cynic/mcp/tools/domains/skeptic
 */

'use strict';

import { PHI_INV, createLogger } from '@cynic/core';

const log = createLogger('SkepticTools');

// ═══════════════════════════════════════════════════════════════════════════
// TOOL IMPLEMENTATIONS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create brain_skeptic tool definition
 *
 * @param {Object} skepticService - SkepticService instance
 * @param {Object} [persistence] - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createSkepticTool(skepticService, persistence = null) {
  return {
    name: 'brain_skeptic',
    description: `Verify content through Kabbalistic skepticism. Extracts claims (TZIMTZUM), verifies against facts (BERUR), suggests corrections (TIKKUN). Returns verdict (TRUST/DOUBT/MIXED/VERIFY), claim analysis, and corrections. Max confidence: ${(PHI_INV * 100).toFixed(1)}%.`,
    inputSchema: {
      type: 'object',
      properties: {
        content: {
          type: 'string',
          description: 'The content to verify (response, claim, code explanation)',
        },
        action: {
          type: 'string',
          enum: ['verify', 'stats', 'quick'],
          description: 'Action: verify (full pipeline), quick (heuristics only), stats (service stats)',
          default: 'verify',
        },
        context: {
          type: 'object',
          description: 'Optional context for verification (source, type, related judgments)',
        },
      },
      required: ['content'],
    },
    handler: async (params) => {
      const { content, action = 'verify', context = {} } = params;

      if (!content && action !== 'stats') {
        throw new Error('Missing required parameter: content');
      }

      switch (action) {
        case 'stats':
          return {
            stats: skepticService.getStats(),
            philosophy: {
              process: 'TZIMTZUM → BERUR → TIKKUN',
              meaning: 'Contraction → Clarification → Repair',
              path: '18th path (Binah → Gevurah)',
              maxConfidence: PHI_INV,
            },
          };

        case 'quick': {
          // Heuristic-only verification (fast, no LLM)
          const claims = skepticService._extractClaimsHeuristic(content);

          // Quick bounds checking
          const results = claims.map(claim => {
            if (claim.type === 'numerical') {
              const bounds = skepticService._checkNumericalBounds(claim);
              return { ...claim, ...bounds };
            }
            if (claim.flag === 'OVERCERTAINTY') {
              return {
                ...claim,
                status: 'overclaimed',
                reason: 'Absolute certainty violates φ axiom',
              };
            }
            return { ...claim, status: 'unknown' };
          });

          const disputed = results.filter(c => c.status === 'disputed' || c.status === 'overclaimed').length;

          return {
            verdict: disputed === 0 ? 'TRUST' : disputed > claims.length / 2 ? 'DOUBT' : 'MIXED',
            claims: results,
            claimCount: claims.length,
            disputedCount: disputed,
            mode: 'quick (heuristics only)',
            confidence: disputed === 0 ? PHI_INV : PHI_INV * (1 - disputed / claims.length),
          };
        }

        case 'verify':
        default: {
          // Full TZIMTZUM → BERUR → TIKKUN pipeline
          const result = await skepticService.verify(content, context);

          // Store verification result if persistence available
          if (persistence && result.corrections.length > 0) {
            try {
              await persistence.upsertPattern({
                category: 'skeptic_correction',
                name: `skeptic_${result.verdict.toLowerCase()}`,
                description: result.message,
                confidence: result.confidence,
                sourceJudgments: [],
                tags: ['skeptic', 'verification', result.verdict.toLowerCase()],
                data: {
                  verdict: result.verdict,
                  corrections: result.corrections.length,
                  stats: result.stats,
                },
              });
            } catch (e) {
              log.warn('Failed to store skeptic pattern', { error: e.message });
            }
          }

          return {
            verdict: result.verdict,
            message: result.message,
            confidence: result.confidence,
            claims: result.claims.map(c => ({
              text: c.text,
              type: c.type,
              status: c.status,
              reason: c.reason,
            })),
            corrections: result.corrections,
            stats: result.stats,
            duration: result.duration,
            philosophy: {
              process: 'TZIMTZUM → BERUR → TIKKUN',
              phase1: 'Contraction - Extract claims',
              phase2: 'Clarification - Verify truth',
              phase3: 'Repair - Suggest corrections',
            },
          };
        }
      }
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY (OCP-compliant)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Factory for skeptic domain tools
 */
export const skepticFactory = {
  name: 'skeptic',
  domain: 'verification',
  requires: [],

  /**
   * Create all skeptic domain tools
   * @param {Object} options
   * @returns {Promise<Object[]>} Tool definitions
   */
  async create(options) {
    const { skepticService, persistence, factsRepository } = options;
    const tools = [];

    // Lazy-create skepticService if not provided
    let service = skepticService;
    if (!service) {
      try {
        const { createSkepticService } = await import('@cynic/node/services');
        service = createSkepticService({
          factsRepo: factsRepository,
        });
      } catch (e) {
        log.debug('SkepticService not available', { error: e.message });
        return [];
      }
    }

    tools.push(createSkepticTool(service, persistence));

    return tools;
  },
};
