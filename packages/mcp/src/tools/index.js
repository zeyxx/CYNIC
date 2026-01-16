/**
 * CYNIC MCP Tools
 *
 * Tool definitions for MCP server
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/mcp/tools
 */

'use strict';

import { PHI_INV, PHI_INV_2, IDENTITY, getVerdictFromScore } from '@cynic/core';

/**
 * Create judge tool definition
 * @param {Object} judge - CYNICJudge instance
 * @param {Object} [persistence] - PersistenceManager instance (for storing judgments)
 * @param {Object} [sessionManager] - SessionManager instance (for user/session context)
 * @param {Object} [pojChainManager] - PoJChainManager instance (for blockchain)
 * @returns {Object} Tool definition
 */
export function createJudgeTool(judge, persistence = null, sessionManager = null, pojChainManager = null) {
  return {
    name: 'brain_cynic_judge',
    description: `Judge an item using CYNIC's 25-dimension evaluation across 4 axioms (PHI, VERIFY, CULTURE, BURN). Returns Q-Score (0-100), verdict (HOWL/WAG/GROWL/BARK), confidence (max ${(PHI_INV * 100).toFixed(1)}%), and dimension breakdown.`,
    inputSchema: {
      type: 'object',
      properties: {
        item: {
          type: 'object',
          description: 'The item to judge. Can contain: type, content, sources, verified, scores (explicit dimension scores)',
        },
        context: {
          type: 'object',
          description: 'Optional context: source, type, kScore (for Final score calculation)',
        },
      },
      required: ['item'],
    },
    handler: async (params) => {
      const { item, context = {} } = params;
      if (!item) throw new Error('Missing required parameter: item');

      const judgment = judge.judge(item, context);
      const judgmentId = `jdg_${Date.now().toString(36)}`;

      // Get session context for user isolation
      const sessionContext = sessionManager?.getSessionContext() || {};

      const result = {
        requestId: judgmentId,
        score: judgment.qScore,
        globalScore: judgment.global_score,
        verdict: judgment.qVerdict?.verdict || judgment.verdict,
        confidence: Math.round(judgment.confidence * 1000) / 1000,
        axiomScores: judgment.axiomScores,
        weaknesses: judgment.weaknesses,
        finalScore: judgment.finalScore || null,
        phi: { maxConfidence: PHI_INV, minDoubt: PHI_INV_2 },
        timestamp: Date.now(),
      };

      // Store judgment in persistence (PostgreSQL → File → Memory fallback)
      if (persistence) {
        try {
          const stored = await persistence.storeJudgment({
            item,
            itemType: item.type || 'unknown',
            itemContent: typeof item.content === 'string' ? item.content : JSON.stringify(item),
            qScore: result.score,
            globalScore: result.globalScore,
            confidence: result.confidence,
            verdict: result.verdict,
            axiomScores: result.axiomScores,
            dimensionScores: judgment.dimensionScores || null,
            weaknesses: result.weaknesses,
            context,
            // Session context for multi-user isolation
            userId: sessionContext.userId || null,
            sessionId: sessionContext.sessionId || null,
          });

          // Add to PoJ chain (batched block creation)
          if (pojChainManager && stored) {
            await pojChainManager.addJudgment({
              judgment_id: stored.judgment_id,
              q_score: result.score,
              verdict: result.verdict,
              created_at: stored.created_at,
            });
          }

          // Increment session counter
          if (sessionManager) {
            await sessionManager.incrementCounter('judgmentCount');
          }
        } catch (e) {
          // Log but don't fail the judgment - persistence is best-effort
          console.error('Error persisting judgment:', e.message);
        }
      }

      return result;
    },
  };
}

/**
 * Create digest tool definition
 * @param {Object} persistence - PersistenceManager instance (handles fallback automatically)
 * @param {Object} [sessionManager] - SessionManager instance (for user/session context)
 * @returns {Object} Tool definition
 */
export function createDigestTool(persistence = null, sessionManager = null) {
  return {
    name: 'brain_cynic_digest',
    description: 'Digest text content and extract patterns, insights, and knowledge. Stores extracted knowledge for future retrieval.',
    inputSchema: {
      type: 'object',
      properties: {
        content: { type: 'string', description: 'Text content to digest' },
        source: { type: 'string', description: 'Source identifier (url, file, conversation)' },
        type: { type: 'string', enum: ['code', 'conversation', 'document', 'decision'], description: 'Content type' },
      },
      required: ['content'],
    },
    handler: async (params) => {
      const { content, source = 'unknown', type = 'document' } = params;
      if (!content) throw new Error('Missing required parameter: content');

      // Get session context for user isolation
      const sessionContext = sessionManager?.getSessionContext() || {};

      const words = content.split(/\s+/).length;
      const sentences = content.split(/[.!?]+/).filter(s => s.trim()).length;

      // Extract patterns
      const patterns = [];
      const codeBlocks = content.match(/```[\s\S]*?```/g) || [];
      const urls = content.match(/https?:\/\/[^\s]+/g) || [];
      const decisions = content.match(/(?:decided|chose|selected|will use|going with)/gi) || [];
      const todos = content.match(/(?:TODO|FIXME|XXX|HACK):/gi) || [];

      if (codeBlocks.length > 0) patterns.push({ type: 'code', count: codeBlocks.length });
      if (urls.length > 0) patterns.push({ type: 'links', count: urls.length, items: urls.slice(0, 5) });
      if (decisions.length > 0) patterns.push({ type: 'decisions', count: decisions.length });
      if (todos.length > 0) patterns.push({ type: 'todos', count: todos.length });

      const digest = {
        digestId: `dig_${Date.now().toString(36)}`,
        source,
        type,
        stats: {
          words,
          sentences,
          estimatedReadTime: Math.ceil(words / 200),
        },
        patterns,
        timestamp: Date.now(),
      };

      // Store in persistence (handles PostgreSQL → File → Memory fallback)
      if (persistence) {
        try {
          await persistence.storeKnowledge({
            sourceType: type,
            sourceRef: source,
            summary: content.slice(0, 500),  // Summary for quick display
            content: content,                 // Full content for FTS
            insights: patterns.map(p => `${p.type}: ${p.count}`),
            patterns: patterns,
            category: type,
            // Session context for multi-user isolation
            userId: sessionContext.userId || null,
            sessionId: sessionContext.sessionId || null,
          });

          // Increment session counter
          if (sessionManager) {
            await sessionManager.incrementCounter('digestCount');
          }
        } catch (e) {
          console.error('Error storing digest:', e.message);
        }
      }

      return digest;
    },
  };
}

/**
 * Create health tool definition
 * @param {Object} node - CYNICNode instance (optional)
 * @param {Object} judge - CYNICJudge instance
 * @param {Object} persistence - PersistenceManager instance (optional)
 * @returns {Object} Tool definition
 */
export function createHealthTool(node, judge, persistence = null) {
  return {
    name: 'brain_health',
    description: 'Get CYNIC system health status including node status, judge statistics, and capability metrics.',
    inputSchema: {
      type: 'object',
      properties: {
        verbose: { type: 'boolean', description: 'Include detailed statistics' },
      },
    },
    handler: async (params) => {
      const { verbose = false } = params;

      const health = {
        status: 'healthy',
        identity: {
          name: IDENTITY.name,
          greek: IDENTITY.greek,
        },
        phi: { maxConfidence: PHI_INV, minDoubt: PHI_INV_2 },
        timestamp: Date.now(),
      };

      if (node) {
        const info = node.getInfo();
        health.node = {
          status: info.status,
          uptime: info.uptime,
          id: info.id?.slice(0, 16) + '...',
        };
      }

      // Add persistence health
      if (persistence) {
        try {
          health.persistence = await persistence.health();
          health.persistence.capabilities = persistence.capabilities;
        } catch (e) {
          health.persistence = { status: 'error', error: e.message };
        }
      }

      if (verbose) {
        health.judge = judge.getStats();
        health.tools = ['brain_cynic_judge', 'brain_cynic_digest', 'brain_health', 'brain_search', 'brain_patterns', 'brain_cynic_feedback', 'brain_agents_status'];

        // Add judgment stats from persistence
        if (persistence?.judgments) {
          try {
            health.judgmentStats = await persistence.getJudgmentStats();
          } catch (e) {
            // Ignore
          }
        }
      }

      return health;
    },
  };
}

/**
 * Create search tool definition
 * @param {Object} persistence - PersistenceManager instance (handles fallback automatically)
 * @returns {Object} Tool definition
 */
export function createSearchTool(persistence = null) {
  return {
    name: 'brain_search',
    description: 'Search CYNIC knowledge base for past judgments, patterns, and decisions.',
    inputSchema: {
      type: 'object',
      properties: {
        query: { type: 'string', description: 'Search query' },
        type: { type: 'string', enum: ['judgment', 'pattern', 'decision', 'all'], description: 'Type of knowledge to search' },
        limit: { type: 'number', description: 'Maximum results (default 10)' },
      },
      required: ['query'],
    },
    handler: async (params) => {
      const { query, type = 'all', limit = 10 } = params;
      if (!query) throw new Error('Missing required parameter: query');

      const results = [];

      // Search judgments (PersistenceManager handles PostgreSQL → File → Memory fallback)
      if (persistence && (type === 'all' || type === 'judgment')) {
        try {
          const judgments = await persistence.searchJudgments(query, { limit });
          for (const j of judgments) {
            results.push({
              type: 'judgment',
              id: j.judgment_id,
              score: j.q_score,
              verdict: j.verdict,
              itemType: j.item_type,
              timestamp: j.created_at,
            });
            if (results.length >= limit) break;
          }
        } catch (e) {
          console.error('Error searching judgments:', e.message);
        }
      }

      // Search knowledge
      if (persistence && (type === 'all' || type === 'decision' || type === 'pattern')) {
        try {
          const knowledge = await persistence.searchKnowledge(query, { limit: limit - results.length });
          for (const k of knowledge) {
            results.push({
              type: k.source_type || 'knowledge',
              id: k.knowledge_id,
              summary: k.summary?.slice(0, 100),
              category: k.category,
              timestamp: k.created_at,
            });
          }
        } catch (e) {
          console.error('Error searching knowledge:', e.message);
        }
      }

      return {
        query,
        type,
        results: results.slice(0, limit),
        total: results.length,
        timestamp: Date.now(),
      };
    },
  };
}

/**
 * Create patterns tool definition
 * @param {Object} judge - CYNICJudge instance
 * @param {Object} persistence - PersistenceManager instance (optional)
 * @returns {Object} Tool definition
 */
export function createPatternsTool(judge, persistence = null) {
  return {
    name: 'brain_patterns',
    description: 'List detected patterns from CYNIC observations and anomalies.',
    inputSchema: {
      type: 'object',
      properties: {
        category: { type: 'string', enum: ['anomaly', 'verdict', 'dimension', 'all'], description: 'Filter by category' },
        limit: { type: 'number', description: 'Maximum patterns (default 10)' },
      },
    },
    handler: async (params) => {
      const { category = 'all', limit = 10 } = params;

      const patterns = [];

      // Get patterns from persistence (PostgreSQL) first
      if (persistence?.patterns && (category === 'all' || category !== 'anomaly' && category !== 'verdict')) {
        try {
          const dbPatterns = await persistence.getPatterns({ category: category !== 'all' ? category : undefined, limit });
          for (const p of dbPatterns) {
            patterns.push({
              category: p.category,
              id: p.pattern_id,
              name: p.name,
              description: p.description,
              confidence: p.confidence,
              frequency: p.frequency,
            });
          }
        } catch (e) {
          console.error('Error getting patterns:', e.message);
        }
      }

      // Get anomaly candidates from judge
      if (category === 'all' || category === 'anomaly') {
        const anomalies = judge.getAnomalyCandidates?.() || [];
        for (const a of anomalies.slice(0, limit)) {
          patterns.push({
            category: 'anomaly',
            residual: a.residual,
            itemType: a.item?.type,
            timestamp: a.timestamp,
          });
        }
      }

      // Get verdict distribution
      if (category === 'all' || category === 'verdict') {
        // Use persistence stats if available
        if (persistence?.judgments) {
          try {
            const stats = await persistence.getJudgmentStats();
            patterns.push({
              category: 'verdict',
              distribution: stats.verdicts,
              total: stats.total,
              avgScore: stats.avgScore,
              source: 'persistence',
            });
          } catch (e) {
            // Fall back to judge stats
          }
        }

        // Fallback to judge stats
        const stats = judge.getStats();
        if (stats.verdicts) {
          patterns.push({
            category: 'verdict',
            distribution: stats.verdicts,
            total: stats.totalJudgments,
            avgScore: stats.avgScore,
            source: 'memory',
          });
        }
      }

      return {
        category,
        patterns: patterns.slice(0, limit),
        total: patterns.length,
        timestamp: Date.now(),
      };
    },
  };
}

/**
 * Create feedback tool definition
 * @param {Object} persistence - PersistenceManager instance (handles fallback automatically)
 * @param {Object} [sessionManager] - SessionManager instance (for user/session context)
 * @returns {Object} Tool definition
 */
export function createFeedbackTool(persistence = null, sessionManager = null) {
  return {
    name: 'brain_cynic_feedback',
    description: 'Provide feedback on a past judgment to improve CYNIC learning. Mark judgments as correct/incorrect with reasoning.',
    inputSchema: {
      type: 'object',
      properties: {
        judgmentId: { type: 'string', description: 'ID of judgment to provide feedback on (e.g., jdg_abc123)' },
        outcome: { type: 'string', enum: ['correct', 'incorrect', 'partial'], description: 'Was the judgment correct?' },
        reason: { type: 'string', description: 'Explanation of why the judgment was correct/incorrect' },
        actualScore: { type: 'number', description: 'What the score should have been (0-100)' },
      },
      required: ['judgmentId', 'outcome'],
    },
    handler: async (params) => {
      const { judgmentId, outcome, reason = '', actualScore } = params;
      if (!judgmentId) throw new Error('Missing required parameter: judgmentId');
      if (!outcome) throw new Error('Missing required parameter: outcome');

      // Get session context for user isolation
      const sessionContext = sessionManager?.getSessionContext() || {};

      const feedback = {
        feedbackId: `fb_${Date.now().toString(36)}`,
        judgmentId,
        outcome,
        reason,
        actualScore,
        timestamp: Date.now(),
        // Session context for multi-user isolation
        userId: sessionContext.userId || null,
        sessionId: sessionContext.sessionId || null,
      };

      let learningDelta = null;
      let originalScore = null;

      // Store feedback (PersistenceManager handles PostgreSQL → File → Memory fallback)
      if (persistence) {
        try {
          await persistence.storeFeedback({
            judgmentId,
            outcome,
            reason,
            actualScore,
            // Session context for multi-user isolation
            userId: sessionContext.userId || null,
            sessionId: sessionContext.sessionId || null,
          });

          // Increment session counter
          if (sessionManager) {
            await sessionManager.incrementCounter('feedbackCount');
          }

          // Try to get original judgment for delta calculation
          if (typeof actualScore === 'number') {
            const judgments = await persistence.getRecentJudgments(100);
            const original = judgments.find(j => j.judgment_id === judgmentId);
            if (original) {
              originalScore = original.q_score;
              learningDelta = actualScore - originalScore;
            }
          }
        } catch (e) {
          console.error('Error storing feedback:', e.message);
        }
      }

      return {
        ...feedback,
        learningDelta,
        originalScore,
        message: `Feedback recorded for ${judgmentId}. ${outcome === 'correct' ? '*wag*' : outcome === 'incorrect' ? '*growl* Learning...' : '*sniff* Partially noted.'}`,
      };
    },
  };
}

/**
 * Create agents status tool definition
 * @param {Object} agents - AgentManager instance (The Four Dogs)
 * @returns {Object} Tool definition
 */
export function createAgentsStatusTool(agents) {
  return {
    name: 'brain_agents_status',
    description: 'Get status and statistics for The Four Dogs agents (Observer, Digester, Guardian, Mentor). Shows blocks, warnings, patterns detected, and wisdom shared.',
    inputSchema: {
      type: 'object',
      properties: {
        verbose: { type: 'boolean', description: 'Include detailed per-agent statistics' },
        agent: { type: 'string', enum: ['observer', 'digester', 'guardian', 'mentor'], description: 'Get status for specific agent only' },
      },
    },
    handler: async (params) => {
      const { verbose = false, agent = null } = params;

      if (!agents) {
        return {
          status: 'unavailable',
          message: 'Agent system not initialized',
          timestamp: Date.now(),
        };
      }

      const summary = agents.getSummary();

      // If specific agent requested
      if (agent && summary.agents[agent]) {
        return {
          agent,
          ...summary.agents[agent],
          managerStats: summary.stats,
          enabled: summary.enabled,
          timestamp: Date.now(),
        };
      }

      // Basic response
      const response = {
        enabled: summary.enabled,
        stats: summary.stats,
        dogs: {
          guardian: {
            description: 'The Watchdog - blocks dangerous operations',
            blocks: summary.agents.guardian?.decisionsBlocked || 0,
            warnings: summary.agents.guardian?.decisionsWarned || 0,
          },
          observer: {
            description: 'The Silent Watcher - detects patterns',
            patterns: summary.agents.observer?.patternsDetected || 0,
            observations: summary.agents.observer?.totalObservations || 0,
          },
          digester: {
            description: 'The Archivist - extracts knowledge',
            digests: summary.agents.digester?.totalDigests || 0,
          },
          mentor: {
            description: 'The Wise Elder - shares wisdom',
            wisdomShared: summary.agents.mentor?.wisdomShared || 0,
          },
        },
        timestamp: Date.now(),
      };

      // Add verbose details
      if (verbose) {
        response.agents = summary.agents;
      }

      return response;
    },
  };
}

/**
 * Create session start tool
 * @param {Object} sessionManager - SessionManager instance
 * @returns {Object} Tool definition
 */
export function createSessionStartTool(sessionManager) {
  return {
    name: 'brain_session_start',
    description: 'Start a new CYNIC session for a user. Sessions provide isolation and tracking of judgments, digests, and feedback.',
    inputSchema: {
      type: 'object',
      properties: {
        userId: {
          type: 'string',
          description: 'User identifier (wallet address, email, username). Required for session isolation.',
        },
        project: {
          type: 'string',
          description: 'Optional project name for context',
        },
        metadata: {
          type: 'object',
          description: 'Optional metadata to store with the session',
        },
      },
      required: ['userId'],
    },
    handler: async (params) => {
      const { userId, project, metadata } = params;

      if (!userId) throw new Error('userId is required');

      if (!sessionManager) {
        return {
          success: false,
          error: 'Session management not available',
        };
      }

      const session = await sessionManager.startSession(userId, { project, metadata });

      return {
        success: true,
        sessionId: session.sessionId,
        userId: session.userId,
        project: session.project,
        createdAt: session.createdAt,
        message: `*tail wag* Session started. Your data is now isolated.`,
      };
    },
  };
}

/**
 * Create session end tool
 * @param {Object} sessionManager - SessionManager instance
 * @returns {Object} Tool definition
 */
export function createSessionEndTool(sessionManager) {
  return {
    name: 'brain_session_end',
    description: 'End the current CYNIC session. Returns session summary with statistics.',
    inputSchema: {
      type: 'object',
      properties: {
        sessionId: {
          type: 'string',
          description: 'Session ID to end. If not provided, ends the current session.',
        },
      },
    },
    handler: async (params) => {
      const { sessionId } = params;

      if (!sessionManager) {
        return {
          success: false,
          error: 'Session management not available',
        };
      }

      // Use provided sessionId or current session
      const targetSessionId = sessionId || sessionManager.getSessionContext().sessionId;

      if (!targetSessionId) {
        return {
          success: false,
          error: 'No active session to end',
        };
      }

      const result = await sessionManager.endSession(targetSessionId);

      if (!result.ended) {
        return {
          success: false,
          error: result.reason,
        };
      }

      return {
        success: true,
        ...result.summary,
        message: `*yawn* Session ended. ${result.summary.judgmentCount} judgments recorded.`,
      };
    },
  };
}

/**
 * Create all tools
 * @param {Object} options - Tool options
 * @param {Object} options.judge - CYNICJudge instance
 * @param {Object} [options.node] - CYNICNode instance
 * @param {Object} [options.persistence] - PersistenceManager instance (handles fallback)
 * @param {Object} [options.agents] - AgentManager instance (The Four Dogs)
 * @param {Object} [options.sessionManager] - SessionManager instance for multi-user sessions
 * @returns {Object} All tools keyed by name
 */
export function createAllTools(options = {}) {
  const {
    judge,
    node = null,
    persistence = null,
    agents = null,
    sessionManager = null,
    pojChainManager = null,
  } = options;

  if (!judge) throw new Error('judge is required');

  const tools = {};
  const toolDefs = [
    createJudgeTool(judge, persistence, sessionManager, pojChainManager),
    createDigestTool(persistence, sessionManager),
    createHealthTool(node, judge, persistence),
    createSearchTool(persistence),
    createPatternsTool(judge, persistence),
    createFeedbackTool(persistence, sessionManager),
    createAgentsStatusTool(agents),
    createSessionStartTool(sessionManager),
    createSessionEndTool(sessionManager),
  ];

  for (const tool of toolDefs) {
    tools[tool.name] = tool;
  }

  return tools;
}

export default {
  createJudgeTool,
  createDigestTool,
  createHealthTool,
  createSearchTool,
  createPatternsTool,
  createFeedbackTool,
  createAgentsStatusTool,
  createSessionStartTool,
  createSessionEndTool,
  createAllTools,
};
