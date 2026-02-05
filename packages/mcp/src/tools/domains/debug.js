/**
 * Debug Domain Tools
 *
 * Tools for debugging CYNIC internal state:
 * - brain_debug_patterns: Query SharedMemory patterns
 * - brain_debug_qlearning: Query Q-Learning state
 * - brain_debug_memory: Query collective memory state
 * - brain_debug_routing: Query recent routing decisions
 * - brain_debug_errors: Query recent errors/failures
 *
 * "φ distrusts φ, but φ can see φ" - κυνικός
 *
 * @module @cynic/mcp/tools/domains/debug
 */

'use strict';

import { PHI_INV } from '@cynic/core';

/**
 * Create brain_debug_patterns tool
 * Query SharedMemory patterns for debugging
 *
 * @param {Object} options
 * @param {Object} options.sharedMemory - SharedMemory instance
 * @returns {Object} Tool definition
 */
export function createDebugPatternsTool(options = {}) {
  const { sharedMemory } = options;

  return {
    name: 'brain_debug_patterns',
    description: 'DEBUG: Query SharedMemory patterns. Shows pattern weights, Fisher scores, and usage statistics. Use for debugging routing and memory issues.',
    inputSchema: {
      type: 'object',
      properties: {
        query: {
          type: 'string',
          description: 'Search query to filter patterns by signature or content',
        },
        limit: {
          type: 'number',
          description: 'Maximum patterns to return (default: 20)',
          default: 20,
        },
        sortBy: {
          type: 'string',
          enum: ['weight', 'uses', 'recent', 'fisher'],
          description: 'Sort order: weight (highest weight), uses (most used), recent (recently added), fisher (highest Fisher score)',
          default: 'weight',
        },
        includeEWC: {
          type: 'boolean',
          description: 'Include EWC++ Fisher importance scores',
          default: false,
        },
      },
    },
    handler: async (params) => {
      const { query = '', limit = 20, sortBy = 'weight', includeEWC = false } = params;

      if (!sharedMemory) {
        return {
          error: 'SharedMemory not available',
          hint: 'Collective must be initialized before debugging patterns',
          timestamp: Date.now(),
        };
      }

      try {
        // Get patterns from SharedMemory
        let patterns = [];

        // Use internal _patterns Map
        if (sharedMemory._patterns) {
          for (const [id, pattern] of sharedMemory._patterns.entries()) {
            // Filter by query if provided
            if (query) {
              const searchable = `${pattern.signature || ''} ${pattern.content || ''} ${pattern.type || ''}`.toLowerCase();
              if (!searchable.includes(query.toLowerCase())) {
                continue;
              }
            }

            patterns.push({
              id,
              signature: pattern.signature || id,
              type: pattern.type || 'unknown',
              weight: pattern.weight ?? 1.0,
              useCount: pattern.useCount ?? 0,
              lastUsed: pattern.lastUsed || pattern.createdAt || null,
              fisherImportance: includeEWC ? (pattern.fisherImportance ?? 0) : undefined,
              consolidationLocked: includeEWC ? (pattern.consolidationLocked ?? false) : undefined,
            });
          }
        }

        // Sort patterns
        switch (sortBy) {
          case 'weight':
            patterns.sort((a, b) => b.weight - a.weight);
            break;
          case 'uses':
            patterns.sort((a, b) => b.useCount - a.useCount);
            break;
          case 'recent':
            patterns.sort((a, b) => (b.lastUsed || 0) - (a.lastUsed || 0));
            break;
          case 'fisher':
            patterns.sort((a, b) => (b.fisherImportance || 0) - (a.fisherImportance || 0));
            break;
        }

        // Limit results
        patterns = patterns.slice(0, limit);

        // Calculate summary stats
        const totalPatterns = sharedMemory._patterns?.size || 0;
        const lockedPatterns = includeEWC
          ? Array.from(sharedMemory._patterns?.values() || []).filter(p => p.consolidationLocked).length
          : null;

        return {
          status: 'success',
          patterns,
          summary: {
            total: totalPatterns,
            returned: patterns.length,
            query: query || null,
            sortBy,
            lockedByEWC: lockedPatterns,
          },
          stats: sharedMemory.stats || {},
          message: `*sniff* Found ${patterns.length}/${totalPatterns} patterns${query ? ` matching "${query}"` : ''}.`,
          timestamp: Date.now(),
        };
      } catch (e) {
        return {
          error: e.message,
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * Create brain_debug_qlearning tool
 * Query Q-Learning state for debugging
 *
 * @param {Object} options
 * @param {Function} options.getQLearningService - Function to get QLearningService
 * @returns {Object} Tool definition
 */
export function createDebugQLearningTool(options = {}) {
  const { getQLearningService } = options;

  return {
    name: 'brain_debug_qlearning',
    description: 'DEBUG: Query Q-Learning routing state. Shows Q-table entries, exploration rate, and episode history. Use for debugging why certain Dogs are selected.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['stats', 'table', 'episodes', 'best_dogs'],
          description: 'What to query: stats (summary), table (Q-table entries), episodes (recent learning episodes), best_dogs (recommended dogs per task type)',
          default: 'stats',
        },
        features: {
          type: 'array',
          items: { type: 'string' },
          description: 'Features to filter Q-table entries (e.g., ["task:security", "ctx:error"])',
        },
        limit: {
          type: 'number',
          description: 'Limit for episodes or table entries',
          default: 10,
        },
      },
    },
    handler: async (params) => {
      const { action = 'stats', features = [], limit = 10 } = params;

      let service = null;
      if (typeof getQLearningService === 'function') {
        service = getQLearningService();
      }

      if (!service) {
        return {
          error: 'QLearningService not available',
          hint: 'Q-Learning must be initialized before debugging',
          timestamp: Date.now(),
        };
      }

      try {
        switch (action) {
          case 'stats': {
            const stats = service.getStats();
            return {
              status: 'success',
              action: 'stats',
              stats,
              explorationRate: service.explorationRate,
              config: {
                learningRate: service.config?.learningRate,
                discountFactor: service.config?.discountFactor,
              },
              message: `*tail wag* Q-Learning: ${stats.episodes} episodes, ${stats.qTableStats?.states || 0} states learned.`,
              timestamp: Date.now(),
            };
          }

          case 'table': {
            const qTable = service.qTable;
            const entries = [];

            // Get entries, optionally filtered by features
            if (qTable?.table) {
              for (const [stateKey, actions] of qTable.table.entries()) {
                // If features provided, check if state matches
                if (features.length > 0) {
                  const stateFeatures = stateKey.split('|');
                  const hasAllFeatures = features.every(f => stateFeatures.includes(f));
                  if (!hasAllFeatures) continue;
                }

                entries.push({
                  state: stateKey,
                  actions: Object.entries(actions).map(([a, q]) => ({
                    dog: a,
                    qValue: Math.round(q * 1000) / 1000,
                  })).sort((a, b) => b.qValue - a.qValue),
                });

                if (entries.length >= limit) break;
              }
            }

            return {
              status: 'success',
              action: 'table',
              entries,
              totalStates: qTable?.stats?.states || 0,
              message: `*sniff* Q-table: ${entries.length} entries${features.length > 0 ? ` matching [${features.join(', ')}]` : ''}.`,
              timestamp: Date.now(),
            };
          }

          case 'episodes': {
            const episodes = (service.episodeHistory || [])
              .slice(-limit)
              .reverse()
              .map(ep => ({
                id: ep.id,
                features: ep.features,
                actions: ep.actions?.map(a => a.action) || [],
                outcome: ep.outcome?.type || (ep.outcome?.success ? 'success' : 'unknown'),
                reward: ep.reward,
                durationMs: ep.duration,
              }));

            return {
              status: 'success',
              action: 'episodes',
              episodes,
              totalInMemory: service.episodeHistory?.length || 0,
              message: `*ears perk* Last ${episodes.length} learning episodes.`,
              timestamp: Date.now(),
            };
          }

          case 'best_dogs': {
            const bestDogs = service.getBestDogsPerTask?.() || {};
            return {
              status: 'success',
              action: 'best_dogs',
              byTaskType: bestDogs,
              message: '*tail wag* Best Dogs per task type based on learned Q-values.',
              timestamp: Date.now(),
            };
          }

          default:
            return {
              error: `Unknown action: ${action}`,
              validActions: ['stats', 'table', 'episodes', 'best_dogs'],
              timestamp: Date.now(),
            };
        }
      } catch (e) {
        return {
          error: e.message,
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * Create brain_debug_memory tool
 * Query collective memory state for debugging
 *
 * @param {Object} options
 * @param {Object} options.sharedMemory - SharedMemory instance
 * @param {Object} options.collective - CollectivePack instance
 * @returns {Object} Tool definition
 */
export function createDebugMemoryTool(options = {}) {
  const { sharedMemory, collective } = options;

  return {
    name: 'brain_debug_memory',
    description: 'DEBUG: Query collective memory state. Shows dimension weights, feedback log, and procedures. Use for understanding CYNIC\'s learned preferences.',
    inputSchema: {
      type: 'object',
      properties: {
        include: {
          type: 'array',
          items: {
            type: 'string',
            enum: ['weights', 'feedback', 'procedures', 'all'],
          },
          description: 'What to include: weights (dimension weights), feedback (feedback log), procedures (scoring procedures), all (everything)',
          default: ['weights'],
        },
        feedbackLimit: {
          type: 'number',
          description: 'Limit for feedback entries (default: 20)',
          default: 20,
        },
      },
    },
    handler: async (params) => {
      const { include = ['weights'], feedbackLimit = 20 } = params;

      if (!sharedMemory) {
        return {
          error: 'SharedMemory not available',
          timestamp: Date.now(),
        };
      }

      const result = {
        status: 'success',
        timestamp: Date.now(),
      };

      const includeAll = include.includes('all');

      // Dimension weights
      if (includeAll || include.includes('weights')) {
        result.dimensionWeights = sharedMemory._dimensionWeights || {};
        result.weightCount = Object.keys(result.dimensionWeights).length;
      }

      // Feedback log
      if (includeAll || include.includes('feedback')) {
        const feedbackLog = sharedMemory._feedbackLog || [];
        result.feedback = feedbackLog.slice(-feedbackLimit).map(f => ({
          type: f.type,
          judgmentId: f.judgmentId,
          correct: f.correct,
          timestamp: f.timestamp,
        }));
        result.totalFeedback = feedbackLog.length;
      }

      // Procedures
      if (includeAll || include.includes('procedures')) {
        result.procedures = {};
        if (sharedMemory._procedures) {
          for (const [type, proc] of sharedMemory._procedures.entries()) {
            result.procedures[type] = {
              stepCount: proc.steps?.length || 0,
              lastUpdated: proc.lastUpdated,
            };
          }
        }
        result.scoringRules = {};
        if (sharedMemory._scoringRules) {
          for (const [type, rules] of sharedMemory._scoringRules.entries()) {
            result.scoringRules[type] = {
              ruleCount: rules.length || 0,
            };
          }
        }
      }

      // Collective state if available
      if (collective) {
        result.collectiveState = collective.getCollectiveState?.() || null;
      }

      result.message = `*sniff* Memory state: ${result.weightCount || 0} dimension weights, ${result.totalFeedback || 0} feedback entries.`;

      return result;
    },
  };
}

/**
 * Create brain_debug_routing tool
 * Query recent routing decisions for debugging
 *
 * @param {Object} options
 * @param {Object} options.persistence - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createDebugRoutingTool(options = {}) {
  const { persistence } = options;

  return {
    name: 'brain_debug_routing',
    description: 'DEBUG: Query recent orchestration routing decisions. Shows which Dogs were selected and why. Use for debugging unexpected routing behavior.',
    inputSchema: {
      type: 'object',
      properties: {
        limit: {
          type: 'number',
          description: 'Number of recent decisions to return (default: 10)',
          default: 10,
        },
        dog: {
          type: 'string',
          description: 'Filter by specific Dog name',
        },
      },
    },
    handler: async (params) => {
      const { limit = 10, dog = null } = params;

      if (!persistence?.query) {
        return {
          error: 'Persistence not available for routing history',
          hint: 'Routing decisions are stored in PostgreSQL',
          timestamp: Date.now(),
        };
      }

      try {
        // Query orchestration_decisions table
        let query = `
          SELECT
            id, request_id, selected_dogs, scores, context,
            router_version, duration_ms, created_at
          FROM orchestration_decisions
        `;
        const queryParams = [];

        if (dog) {
          query += ` WHERE $1 = ANY(selected_dogs)`;
          queryParams.push(dog);
        }

        query += ` ORDER BY created_at DESC LIMIT $${queryParams.length + 1}`;
        queryParams.push(limit);

        const result = await persistence.query(query, queryParams);

        const decisions = (result.rows || []).map(row => ({
          id: row.id,
          requestId: row.request_id,
          selectedDogs: row.selected_dogs,
          scores: row.scores,
          context: row.context,
          routerVersion: row.router_version,
          durationMs: row.duration_ms,
          createdAt: row.created_at,
        }));

        return {
          status: 'success',
          decisions,
          count: decisions.length,
          filter: dog ? { dog } : null,
          message: `*tail wag* ${decisions.length} routing decisions${dog ? ` for ${dog}` : ''}.`,
          timestamp: Date.now(),
        };
      } catch (e) {
        // Table might not exist
        if (e.message.includes('does not exist')) {
          return {
            status: 'no_data',
            message: '*head tilt* No routing history table found. Run migration 017.',
            timestamp: Date.now(),
          };
        }
        return {
          error: e.message,
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * Create brain_debug_errors tool
 * Query recent errors and failures for debugging
 *
 * @param {Object} options
 * @param {Object} options.persistence - PersistenceManager instance
 * @param {Object} options.errorBuffer - In-memory error buffer (if available)
 * @returns {Object} Tool definition
 */
export function createDebugErrorsTool(options = {}) {
  const { persistence, errorBuffer } = options;

  return {
    name: 'brain_debug_errors',
    description: 'DEBUG: Query recent errors and failures. Shows tool failures, hook errors, and system issues. Essential for debugging CYNIC problems.',
    inputSchema: {
      type: 'object',
      properties: {
        limit: {
          type: 'number',
          description: 'Number of recent errors to return (default: 20)',
          default: 20,
        },
        severity: {
          type: 'string',
          enum: ['all', 'error', 'warning', 'critical'],
          description: 'Filter by severity level',
          default: 'all',
        },
        source: {
          type: 'string',
          description: 'Filter by error source (e.g., "hook", "tool", "judge")',
        },
      },
    },
    handler: async (params) => {
      const { limit = 20, severity = 'all', source = null } = params;

      const errors = [];

      // Check in-memory buffer first
      if (errorBuffer && Array.isArray(errorBuffer)) {
        const filtered = errorBuffer
          .filter(e => severity === 'all' || e.severity === severity)
          .filter(e => !source || e.source === source)
          .slice(-limit);
        errors.push(...filtered);
      }

      // Check persistence for stored errors
      if (persistence?.query && errors.length < limit) {
        try {
          const remaining = limit - errors.length;
          let query = `
            SELECT id, error_type, message, source, severity, context, created_at
            FROM cynic_errors
            WHERE 1=1
          `;
          const queryParams = [];
          let paramIndex = 1;

          if (severity !== 'all') {
            query += ` AND severity = $${paramIndex}`;
            queryParams.push(severity);
            paramIndex++;
          }

          if (source) {
            query += ` AND source = $${paramIndex}`;
            queryParams.push(source);
            paramIndex++;
          }

          query += ` ORDER BY created_at DESC LIMIT $${paramIndex}`;
          queryParams.push(remaining);

          const result = await persistence.query(query, queryParams);

          for (const row of result.rows || []) {
            errors.push({
              id: row.id,
              type: row.error_type,
              message: row.message,
              source: row.source,
              severity: row.severity,
              context: row.context,
              timestamp: row.created_at,
            });
          }
        } catch (e) {
          // Table might not exist - that's ok, continue with what we have
          if (!e.message.includes('does not exist')) {
            errors.push({
              type: 'debug_error',
              message: `Error querying error log: ${e.message}`,
              source: 'brain_debug_errors',
              severity: 'warning',
              timestamp: Date.now(),
            });
          }
        }
      }

      // Sort by timestamp descending
      errors.sort((a, b) => (b.timestamp || 0) - (a.timestamp || 0));

      // Truncate to limit
      const limitedErrors = errors.slice(0, limit);

      // Summary by severity
      const bySeverity = {
        critical: limitedErrors.filter(e => e.severity === 'critical').length,
        error: limitedErrors.filter(e => e.severity === 'error').length,
        warning: limitedErrors.filter(e => e.severity === 'warning').length,
      };

      // Summary by source
      const bySource = {};
      for (const e of limitedErrors) {
        const src = e.source || 'unknown';
        bySource[src] = (bySource[src] || 0) + 1;
      }

      const hasErrors = limitedErrors.length > 0;
      const hasCritical = bySeverity.critical > 0;

      return {
        status: hasErrors ? (hasCritical ? 'critical' : 'has_errors') : 'clean',
        errors: limitedErrors,
        count: limitedErrors.length,
        summary: {
          bySeverity,
          bySource,
        },
        filters: {
          severity: severity !== 'all' ? severity : null,
          source: source || null,
        },
        message: hasErrors
          ? hasCritical
            ? `*GROWL* ${bySeverity.critical} critical errors found!`
            : `*ears perk* ${limitedErrors.length} errors/warnings found.`
          : '*tail wag* No errors found. System healthy.',
        timestamp: Date.now(),
      };
    },
  };
}

/**
 * Factory for debug domain tools
 */
export const debugFactory = {
  name: 'debug',
  domain: 'debug',
  requires: [],

  /**
   * Create all debug domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const {
      sharedMemory,
      getQLearningService,
      collective,
      persistence,
      errorBuffer,
    } = options;

    const tools = [];

    // Debug patterns tool
    if (sharedMemory) {
      tools.push(createDebugPatternsTool({ sharedMemory }));
    }

    // Debug Q-Learning tool
    if (getQLearningService) {
      tools.push(createDebugQLearningTool({ getQLearningService }));
    }

    // Debug memory tool
    if (sharedMemory || collective) {
      tools.push(createDebugMemoryTool({ sharedMemory, collective }));
    }

    // Debug routing tool
    if (persistence) {
      tools.push(createDebugRoutingTool({ persistence }));
    }

    // Debug errors tool (always available, uses what it can)
    tools.push(createDebugErrorsTool({ persistence, errorBuffer }));

    return tools;
  },
};
