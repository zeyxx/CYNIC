/**
 * Knowledge Domain Tools
 *
 * Tools for knowledge management:
 * - Search: Search judgments and patterns
 * - Digest: Extract knowledge from content
 * - Docs: Library documentation
 *
 * @module @cynic/mcp/tools/domains/knowledge
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('KnowledgeTools');

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
          log.error('Error storing digest', { error: e.message });
        }
      }

      return digest;
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
      required: [],  // Empty query allowed for listing recent items
    },
    handler: async (params) => {
      const { query = '', type = 'all', limit = 10 } = params;
      // Empty query is allowed - returns recent items without text filtering

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
          log.error('Error searching judgments', { error: e.message });
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
          log.error('Error searching knowledge', { error: e.message });
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
 * Create docs tool definition (library documentation cache)
 * @param {Object} librarian - LibrarianService instance
 * @param {Object} persistence - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createDocsTool(librarian, persistence = null) {
  return {
    name: 'brain_docs',
    description: 'Query library documentation with caching. Fetches from Context7 and caches results for faster future access. Can also show cache statistics.',
    inputSchema: {
      type: 'object',
      properties: {
        libraryId: {
          type: 'string',
          description: 'Context7 library ID (e.g., "/vercel/next.js", "/solana-labs/solana-web3.js")',
        },
        query: {
          type: 'string',
          description: 'Documentation query (e.g., "how to connect wallet", "API reference")',
        },
        action: {
          type: 'string',
          enum: ['query', 'stats', 'invalidate', 'list'],
          description: 'Action to perform: query (default), stats (cache stats), invalidate (clear cache), list (show cached libraries)',
        },
      },
    },
    handler: async (params) => {
      const { libraryId, query, action = 'query' } = params;

      // Handle non-query actions
      if (action === 'stats') {
        if (!librarian) {
          return { error: 'Librarian service not available', timestamp: Date.now() };
        }
        const stats = await librarian.getStats();
        return {
          action: 'stats',
          ...stats,
          message: `*sniff* Cache has ${stats.cache.activeEntries} active entries with ${(stats.hitRate * 100).toFixed(1)}% hit rate.`,
          timestamp: Date.now(),
        };
      }

      if (action === 'list') {
        if (!librarian) {
          return { error: 'Librarian service not available', timestamp: Date.now() };
        }
        const libraries = await librarian.getCachedLibraries(20);
        const ecosystem = librarian.getEcosystemLibraries();
        return {
          action: 'list',
          cachedLibraries: libraries,
          ecosystemLibraries: ecosystem.map(l => ({ id: l.id, name: l.name, priority: l.priority })),
          message: `*tail wag* ${libraries.length} libraries cached, ${ecosystem.length} in ecosystem.`,
          timestamp: Date.now(),
        };
      }

      if (action === 'invalidate') {
        if (!libraryId) {
          return { error: 'libraryId required for invalidate action', timestamp: Date.now() };
        }
        if (!librarian) {
          return { error: 'Librarian service not available', timestamp: Date.now() };
        }
        const result = await librarian.invalidate(libraryId);
        return {
          action: 'invalidate',
          ...result,
          message: `*growl* Invalidated ${result.invalidated} cache entries for ${libraryId}.`,
          timestamp: Date.now(),
        };
      }

      // Default: query action
      if (!libraryId || !query) {
        return {
          error: 'Both libraryId and query are required for documentation lookup',
          hint: 'Use action="list" to see available libraries, or action="stats" for cache statistics',
          timestamp: Date.now(),
        };
      }

      if (!librarian) {
        // Fallback: direct persistence lookup without librarian service
        if (persistence?.libraryCache) {
          const cached = await persistence.getLibraryDoc(libraryId, query);
          if (cached) {
            return {
              libraryId,
              query,
              content: cached.content,
              source: 'postgres',
              cached: true,
              hitCount: cached.hitCount,
              message: '*ears perk* Found in cache (direct lookup).',
              timestamp: Date.now(),
            };
          }
        }
        return {
          error: 'Librarian service not available and no cached content found',
          hint: 'Documentation must be fetched from Context7 - librarian service required',
          timestamp: Date.now(),
        };
      }

      // Query with librarian (no fetcher - cache-only for now)
      // Note: The actual Context7 fetcher would be injected by the client
      const result = await librarian.getDocumentation(libraryId, query);

      if (result.content) {
        return {
          libraryId,
          query,
          content: result.content,
          source: result.source,
          cached: result.cached,
          hitCount: result.hitCount || 0,
          message: result.cached
            ? `*tail wag* Found in ${result.source} cache!`
            : '*sniff* Fetched fresh from source.',
          timestamp: Date.now(),
        };
      }

      return {
        libraryId,
        query,
        content: null,
        source: 'none',
        cached: false,
        message: '*head tilt* No documentation found. Use Context7 to fetch first.',
        hint: 'Call context7.query-docs to fetch documentation, which will be cached automatically.',
        timestamp: Date.now(),
      };
    },
  };
}

/**
 * Create semantic patterns tool definition
 * Uses VectorStore + SemanticPatternMatcher for finding patterns by meaning.
 *
 * @param {Object} semanticMatcher - SemanticPatternMatcher instance
 * @returns {Object} Tool definition
 */
export function createSemanticPatternsTool(semanticMatcher = null) {
  // Lazy-loaded SemanticPatternMatcher
  let matcher = semanticMatcher;

  return {
    name: 'brain_semantic_patterns',
    description: 'Find semantically similar patterns using vector search. Store patterns for future retrieval, cluster by similarity, and recommend patterns for context.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['search', 'add', 'cluster', 'recommend', 'stats'],
          description: 'Action to perform',
        },
        query: { type: 'string', description: 'Search query or pattern description' },
        patternId: { type: 'string', description: 'Pattern ID (for add action)' },
        k: { type: 'number', description: 'Number of results (default: 10)' },
        metadata: { type: 'object', description: 'Pattern metadata (for add action)' },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const { action, query, patternId, k = 10, metadata = {} } = params;

      // Lazy initialize matcher
      if (!matcher) {
        try {
          const { createSemanticPatternMatcher } = await import('@cynic/persistence');
          matcher = createSemanticPatternMatcher({
            config: { embedder: 'mock', dimensions: 384 },
          });
          log.info('SemanticPatternMatcher initialized lazily');
        } catch (e) {
          return {
            error: 'SemanticPatternMatcher not available',
            hint: 'Install @cynic/persistence or check imports',
            timestamp: Date.now(),
          };
        }
      }

      switch (action) {
        case 'search': {
          if (!query) {
            return { error: 'query required for search action', timestamp: Date.now() };
          }
          const results = await matcher.findSimilar(query, k);
          return {
            action: 'search',
            query,
            results: results.map(r => ({
              id: r.pattern.id,
              description: r.pattern.description,
              score: r.score.toFixed(3),
              metadata: r.pattern.metadata,
            })),
            count: results.length,
            message: `*sniff* Found ${results.length} similar patterns.`,
            timestamp: Date.now(),
          };
        }

        case 'add': {
          if (!patternId || !query) {
            return { error: 'patternId and query (description) required for add action', timestamp: Date.now() };
          }
          const pattern = await matcher.addPattern(patternId, query, metadata);
          return {
            action: 'add',
            pattern: {
              id: pattern.id,
              description: pattern.description,
              metadata: pattern.metadata,
            },
            message: `*tail wag* Pattern "${patternId}" stored.`,
            timestamp: Date.now(),
          };
        }

        case 'cluster': {
          const clusters = await matcher.clusterPatterns({ minSize: 2, maxClusters: 21 });
          return {
            action: 'cluster',
            clusters: clusters.map(c => c.toJSON()),
            count: clusters.length,
            message: `*ears perk* Found ${clusters.length} pattern clusters.`,
            timestamp: Date.now(),
          };
        }

        case 'recommend': {
          if (!query) {
            return { error: 'query (context) required for recommend action', timestamp: Date.now() };
          }
          const recommendations = await matcher.recommendPatterns(query, k);
          return {
            action: 'recommend',
            context: query,
            recommendations: recommendations.map(r => ({
              id: r.pattern.id,
              description: r.pattern.description,
              relevance: r.relevance.toFixed(3),
            })),
            count: recommendations.length,
            message: `*head tilt* ${recommendations.length} patterns recommended for this context.`,
            timestamp: Date.now(),
          };
        }

        case 'stats': {
          const stats = matcher.stats();
          return {
            action: 'stats',
            ...stats,
            message: `*sniff* ${stats.patterns} patterns stored, ${stats.searches} searches, ${stats.clusters} clusters.`,
            timestamp: Date.now(),
          };
        }

        default:
          return {
            error: `Unknown action: ${action}`,
            validActions: ['search', 'add', 'cluster', 'recommend', 'stats'],
            timestamp: Date.now(),
          };
      }
    },
  };
}

/**
 * Factory for knowledge domain tools
 */
export const knowledgeFactory = {
  name: 'knowledge',
  domain: 'knowledge',
  requires: [],

  /**
   * Create all knowledge domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const { persistence, sessionManager, librarian, semanticMatcher } = options;

    const tools = [];

    // Search tool
    if (persistence) {
      tools.push(createSearchTool(persistence));
    }

    // Digest tool
    tools.push(createDigestTool(persistence, sessionManager));

    // Documentation tool
    if (librarian) {
      tools.push(createDocsTool(librarian, persistence));
    }

    // Semantic Patterns tool (V3 vector search integration)
    tools.push(createSemanticPatternsTool(semanticMatcher));

    return tools;
  },
};
