/**
 * Claude Flow Domain Tools
 *
 * Tools for intelligent routing, code transforms, and optimization
 * Phase 21: Claude Flow Integration
 *
 * @module @cynic/mcp/tools/domains/claude-flow
 */

'use strict';

import { PHI_INV, PHI_INV_2, PHI_INV_3 } from '@cynic/core';

// ═══════════════════════════════════════════════════════════════════════════
// COMPLEXITY CLASSIFIER TOOL
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create complexity classifier tool
 * Classifies request complexity for tiered routing
 *
 * @param {Object} classifier - ComplexityClassifier instance
 * @returns {Object} Tool definition
 */
export function createComplexityTool(classifier) {
  return {
    name: 'brain_complexity',
    description: `Classify request complexity for optimal routing.

Returns tier (LOCAL/LIGHT/FULL), confidence, and cost estimate.

LOCAL tier: Simple operations, no LLM needed (<1ms, $0)
LIGHT tier: Moderate complexity, fast LLM (~500ms, $)
FULL tier: Complex analysis, full reasoning (~3s, $$$)

φ-aligned thresholds:
- LOCAL: < ${PHI_INV_3.toFixed(3)} (0.236)
- LIGHT: < ${PHI_INV.toFixed(3)} (0.618)
- FULL: ≥ ${PHI_INV.toFixed(3)} (0.618)`,
    inputSchema: {
      type: 'object',
      properties: {
        content: {
          type: 'string',
          description: 'The request content to classify',
        },
        context: {
          type: 'object',
          description: 'Optional context for classification',
          properties: {
            domain: { type: 'string' },
            urgency: { type: 'string', enum: ['low', 'medium', 'high'] },
          },
        },
      },
      required: ['content'],
    },
    async handler({ content, context = {} }) {
      if (!classifier) {
        return {
          error: 'ComplexityClassifier not initialized',
          tier: 'FULL', // Fallback to full
          confidence: 0.5,
        };
      }

      const result = classifier.classify({ content, ...context });

      return {
        tier: result.tier,
        complexity: result.complexity,
        confidence: Math.min(result.confidence, PHI_INV), // φ⁻¹ max
        signals: result.signals,
        recommendation: getTierRecommendation(result.tier),
      };
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// AGENT BOOSTER TOOL
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create agent booster tool
 * Fast code transforms without LLM
 *
 * @param {Object} booster - AgentBooster instance
 * @returns {Object} Tool definition
 */
export function createBoosterTool(booster) {
  return {
    name: 'brain_boost',
    description: `Fast code transforms without LLM (<1ms, $0).

Available transforms:
- var-to-const: Convert var → const
- var-to-let: Convert var → let
- add-types: Add TypeScript type annotations
- add-async-await: Convert .then() → async/await
- add-error-handling: Wrap with try-catch
- add-logging: Add console.log entries
- remove-console: Strip console.* calls
- remove-debugger: Strip debugger statements
- add-semicolons: Add missing semicolons
- remove-unused-imports: Clean dead imports
- sort-imports: Alphabetize imports
- add-strict-mode: Add 'use strict'

Use for simple, deterministic code modifications.`,
    inputSchema: {
      type: 'object',
      properties: {
        code: {
          type: 'string',
          description: 'The code to transform',
        },
        intent: {
          type: 'string',
          description: 'Transform intent (e.g., "var-to-const")',
          enum: [
            'var-to-const',
            'var-to-let',
            'add-types',
            'add-async-await',
            'add-error-handling',
            'add-logging',
            'remove-console',
            'remove-debugger',
            'add-semicolons',
            'remove-unused-imports',
            'sort-imports',
            'add-strict-mode',
          ],
        },
        detect: {
          type: 'string',
          description: 'Natural language request to auto-detect intent',
        },
      },
      required: ['code'],
    },
    async handler({ code, intent, detect }) {
      if (!booster) {
        return {
          error: 'AgentBooster not initialized',
          code,
        };
      }

      // Auto-detect intent from natural language
      if (!intent && detect) {
        const detection = booster.canHandle(detect);
        if (!detection) {
          return {
            error: 'Could not detect transform intent',
            suggestion: 'Specify intent explicitly',
            availableIntents: booster.getAvailableIntents?.() || [],
          };
        }
        intent = detection.intent;
      }

      if (!intent) {
        return {
          error: 'No intent specified',
          availableIntents: booster.getAvailableIntents?.() || [],
        };
      }

      const result = booster.transform({ code, intent });

      return {
        code: result.code,
        intent: result.intent,
        changes: result.changes,
        elapsed: result.elapsed,
        cost: 0, // Always free
      };
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TOKEN OPTIMIZER TOOL
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create token optimizer tool
 * Compress and cache for token efficiency
 *
 * @param {Object} optimizer - TokenOptimizer instance
 * @returns {Object} Tool definition
 */
export function createOptimizerTool(optimizer) {
  return {
    name: 'brain_optimize',
    description: `Optimize tokens through compression and caching.

Strategies:
- whitespace: Collapse extra spaces and newlines (10-20% reduction)
- abbreviation: Replace common words (function→fn, implementation→impl)
- filler: Remove filler words (please, kindly, basically)
- dedup: Reference repeated content

Returns optimized content with compression ratio.
Target: ${PHI_INV.toFixed(3)} (φ⁻¹) = 38% reduction`,
    inputSchema: {
      type: 'object',
      properties: {
        content: {
          type: 'string',
          description: 'Content to optimize',
        },
        strategies: {
          type: 'array',
          items: {
            type: 'string',
            enum: ['whitespace', 'abbreviation', 'filler', 'dedup'],
          },
          description: 'Strategies to apply (default: all)',
        },
        cacheKey: {
          type: 'string',
          description: 'Optional cache key for repeated content',
        },
      },
      required: ['content'],
    },
    async handler({ content, strategies, cacheKey }) {
      if (!optimizer) {
        return {
          error: 'TokenOptimizer not initialized',
          optimized: content,
        };
      }

      // Check cache first
      if (cacheKey) {
        const cached = optimizer.getFromCache?.(cacheKey);
        if (cached) {
          return {
            optimized: cached.optimized,
            fromCache: true,
            originalTokens: cached.originalTokens,
            optimizedTokens: cached.optimizedTokens,
            compressionRatio: cached.compressionRatio,
          };
        }
      }

      const result = optimizer.optimize({
        content,
        strategies: strategies || ['whitespace', 'abbreviation', 'filler'],
      });

      // Cache if key provided
      if (cacheKey && result.savedTokens > 0) {
        optimizer.addToCache?.(cacheKey, result);
      }

      return {
        optimized: result.optimized,
        originalTokens: result.originalTokens,
        optimizedTokens: result.optimizedTokens,
        savedTokens: result.savedTokens,
        compressionRatio: result.compressionRatio,
        appliedStrategies: result.appliedStrategies,
      };
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// TIERED ROUTER TOOL
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create tiered router tool
 * Route requests to appropriate handlers
 *
 * @param {Object} router - TieredRouter instance
 * @returns {Object} Tool definition
 */
export function createRouterTool(router) {
  return {
    name: 'brain_route',
    description: `Route request to optimal handler based on complexity.

Analyzes request and returns routing decision with:
- Selected tier (LOCAL/LIGHT/FULL)
- Estimated cost and latency
- Handler recommendation

Use for request preprocessing and cost optimization.`,
    inputSchema: {
      type: 'object',
      properties: {
        content: {
          type: 'string',
          description: 'Request content to route',
        },
        preferLocal: {
          type: 'boolean',
          description: 'Prefer LOCAL tier when possible',
          default: true,
        },
        maxCost: {
          type: 'number',
          description: 'Maximum acceptable cost multiplier',
        },
      },
      required: ['content'],
    },
    async handler({ content, preferLocal = true, maxCost }) {
      if (!router) {
        return {
          error: 'TieredRouter not initialized',
          routing: { tier: 'FULL' },
        };
      }

      const result = await router.route({
        content,
        options: { preferLocal, maxCost },
      });

      return {
        routing: {
          tier: result.routing.tier,
          confidence: Math.min(result.routing.confidence, PHI_INV),
          cost: result.routing.cost,
          latency: result.routing.latency,
        },
        recommendation: result.routing.tier === 'LOCAL'
          ? 'Use AgentBooster or local handler'
          : result.routing.tier === 'LIGHT'
            ? 'Use Haiku or fast model'
            : 'Use Sonnet/Opus for full analysis',
      };
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// HYPERBOLIC SPACE TOOL
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create hyperbolic space tool
 * Hierarchical embeddings in Poincaré ball
 *
 * @param {Object} space - HyperbolicSpace instance
 * @returns {Object} Tool definition
 */
export function createHyperbolicTool(space) {
  return {
    name: 'brain_hyperbolic',
    description: `Query hierarchical embeddings in hyperbolic (Poincaré) space.

Hyperbolic geometry naturally represents hierarchies with:
- Exponential growth of space (vs polynomial in Euclidean)
- Low distortion for tree structures
- 8D hyperbolic ≈ 200D Euclidean

Operations:
- distance: Hyperbolic distance between embeddings
- nearest: K-nearest neighbors
- ancestors: Get parent chain
- centroid: Compute Fréchet mean`,
    inputSchema: {
      type: 'object',
      properties: {
        operation: {
          type: 'string',
          enum: ['distance', 'nearest', 'ancestors', 'descendants', 'centroid', 'add', 'stats'],
          description: 'Operation to perform',
        },
        id: {
          type: 'string',
          description: 'Primary entity ID',
        },
        id2: {
          type: 'string',
          description: 'Secondary entity ID (for distance)',
        },
        parentId: {
          type: 'string',
          description: 'Parent ID (for add operation)',
        },
        k: {
          type: 'number',
          description: 'Number of neighbors (for nearest)',
          default: 5,
        },
        ids: {
          type: 'array',
          items: { type: 'string' },
          description: 'Entity IDs (for centroid)',
        },
      },
      required: ['operation'],
    },
    async handler({ operation, id, id2, parentId, k = 5, ids }) {
      if (!space) {
        return {
          error: 'HyperbolicSpace not initialized',
        };
      }

      switch (operation) {
        case 'distance':
          if (!id || !id2) return { error: 'Need id and id2 for distance' };
          return { distance: space.distance(id, id2) };

        case 'nearest':
          if (!id) return { error: 'Need id for nearest' };
          return { neighbors: space.kNearest(id, k) };

        case 'ancestors':
          if (!id) return { error: 'Need id for ancestors' };
          return { ancestors: space.getAncestors(id) };

        case 'descendants':
          if (!id) return { error: 'Need id for descendants' };
          return { descendants: space.getDescendants?.(id) || [] };

        case 'centroid':
          if (!ids || ids.length === 0) return { error: 'Need ids for centroid' };
          return { centroid: space.centroid(ids) };

        case 'add':
          if (!id) return { error: 'Need id to add' };
          space.add(id, null, parentId);
          return { added: id, parent: parentId || 'root' };

        case 'stats':
          return {
            nodeCount: space.nodeCount,
            dim: space.dim,
            curvature: space.curvature,
          };

        default:
          return { error: `Unknown operation: ${operation}` };
      }
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// SONA TOOL
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create SONA tool
 * Self-Optimizing Neural Adaptation
 *
 * @param {Object} sona - SONA instance
 * @returns {Object} Tool definition
 */
export function createSONATool(sona) {
  return {
    name: 'brain_sona',
    description: `Query SONA (Self-Optimizing Neural Adaptation) state.

SONA correlates judgment dimensions with feedback to auto-adapt:
- Observes pattern→dimension relationships
- Processes success/failure feedback
- Computes dimension→outcome correlations
- Suggests weight adjustments

φ-aligned parameters:
- Adaptation rate: ${PHI_INV_3.toFixed(3)} (0.236)
- Max adaptation: ${PHI_INV_2.toFixed(3)} (0.382)
- Correlation threshold: ${PHI_INV_2.toFixed(3)} (0.382)`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['status', 'correlations', 'suggestions', 'observe', 'feedback'],
          description: 'Action to perform',
        },
        patternId: {
          type: 'string',
          description: 'Pattern ID (for observe/feedback)',
        },
        dimensionScores: {
          type: 'object',
          description: 'Dimension scores map (for observe)',
        },
        success: {
          type: 'boolean',
          description: 'Success indicator (for feedback)',
        },
        impact: {
          type: 'number',
          description: 'Impact magnitude 0-1 (for feedback)',
        },
      },
      required: ['action'],
    },
    async handler({ action, patternId, dimensionScores, success, impact }) {
      if (!sona) {
        return {
          error: 'SONA not initialized',
        };
      }

      switch (action) {
        case 'status':
          return {
            trackedPatterns: sona.getTrackedCount?.() || 0,
            correlationsComputed: sona.getCorrelationCount?.() || 0,
            adaptationRate: PHI_INV_3,
            status: 'active',
          };

        case 'correlations':
          return {
            correlations: sona.getCorrelations?.() || {},
          };

        case 'suggestions':
          return {
            suggestions: sona.getSuggestions?.() || [],
          };

        case 'observe':
          if (!patternId || !dimensionScores) {
            return { error: 'Need patternId and dimensionScores' };
          }
          sona.observe({ patternId, dimensionScores });
          return { observed: patternId };

        case 'feedback':
          if (!patternId || success === undefined) {
            return { error: 'Need patternId and success' };
          }
          sona.processFeedback({ patternId, success, impact: impact || 0.5 });
          return { processed: patternId, success };

        default:
          return { error: `Unknown action: ${action}` };
      }
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Claude Flow domain factory
 */
export const claudeFlowFactory = {
  name: 'claude-flow',
  domain: 'optimization',
  requires: ['complexityClassifier', 'tieredRouter', 'agentBooster', 'tokenOptimizer', 'hyperbolicSpace', 'sona'],
  create(options) {
    const tools = [];

    if (options.complexityClassifier) {
      tools.push(createComplexityTool(options.complexityClassifier));
    }

    if (options.agentBooster) {
      tools.push(createBoosterTool(options.agentBooster));
    }

    if (options.tokenOptimizer) {
      tools.push(createOptimizerTool(options.tokenOptimizer));
    }

    if (options.tieredRouter) {
      tools.push(createRouterTool(options.tieredRouter));
    }

    if (options.hyperbolicSpace) {
      tools.push(createHyperbolicTool(options.hyperbolicSpace));
    }

    if (options.sona) {
      tools.push(createSONATool(options.sona));
    }

    return tools;
  },
};

// ═══════════════════════════════════════════════════════════════════════════
// HELPERS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Get recommendation for tier
 */
function getTierRecommendation(tier) {
  switch (tier) {
    case 'LOCAL':
      return 'Use AgentBooster for code transforms, or local lookup for simple queries';
    case 'LIGHT':
      return 'Use Haiku or similar fast model for moderate complexity';
    case 'FULL':
      return 'Use Sonnet/Opus for deep analysis, architecture, or complex reasoning';
    default:
      return 'Unknown tier';
  }
}
