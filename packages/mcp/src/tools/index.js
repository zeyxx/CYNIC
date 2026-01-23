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

import {
  PHI_INV,
  PHI_INV_2,
  THRESHOLDS,
  EMERGENCE,
  IDENTITY,
  getVerdictFromScore,
  EcosystemMonitor,
  summarizeUpdates,
} from '@cynic/core';
import { createMetaTool } from '../meta-dashboard.js';
import { createCodeAnalyzer } from '../code-analyzer.js';
import { LSPService, createLSPTools } from '../lsp-service.js';
import { JSONRenderService, createJSONRenderTool } from '../json-render.js';
import {
  createSearchIndexTool,
  createTimelineTool,
  createGetObservationsTool,
  createProgressiveSearchTools,
} from './search-progressive.js';

// ═══════════════════════════════════════════════════════════════════════════
// OCP: Re-export from domain modules
// ═══════════════════════════════════════════════════════════════════════════

// Judgment domain (judge, refine, feedback, learning)
// Import for local use + re-export
import {
  createJudgeTool,
  createRefineTool,
  createFeedbackTool,
  createLearningTool,
  judgmentFactory,
} from './domains/judgment.js';

export {
  createJudgeTool,
  createRefineTool,
  createFeedbackTool,
  createLearningTool,
  judgmentFactory,
};

// NOTE: createJudgeTool and createRefineTool implementations moved to domains/judgment.js

// Knowledge domain (search, digest, docs)
// Import for local use + re-export
import {
  createSearchTool,
  createDigestTool,
  createDocsTool,
  knowledgeFactory,
} from './domains/knowledge.js';

export {
  createSearchTool,
  createDigestTool,
  createDocsTool,
  knowledgeFactory,
};

// NOTE: createDigestTool, createSearchTool, createDocsTool implementations moved to domains/knowledge.js

// Blockchain domain (poj_chain, trace)
// Import for local use + re-export
import {
  createPoJChainTool,
  createTraceTool,
  blockchainFactory,
} from './domains/blockchain.js';

export {
  createPoJChainTool,
  createTraceTool,
  blockchainFactory,
};

// NOTE: createPoJChainTool, createTraceTool implementations moved to domains/blockchain.js

// Consciousness domain (patterns, milestone_history, self_mod, emergence)
// Import for local use + re-export
import {
  createPatternsTool,
  createMilestoneHistoryTool,
  createSelfModTool,
  createEmergenceTool,
  consciousnessFactory,
} from './domains/consciousness.js';

export {
  createPatternsTool,
  createMilestoneHistoryTool,
  createSelfModTool,
  createEmergenceTool,
  consciousnessFactory,
};

// NOTE: createPatternsTool, createMilestoneHistoryTool, createSelfModTool, createEmergenceTool moved to domains/consciousness.js

// Session domain (psychology, session_start, session_end, profile_sync, profile_load)
// Import for local use + re-export
import {
  createPsychologyTool,
  createSessionStartTool,
  createSessionEndTool,
  createProfileSyncTool,
  createProfileLoadTool,
  sessionFactory,
} from './domains/session.js';

export {
  createPsychologyTool,
  createSessionStartTool,
  createSessionEndTool,
  createProfileSyncTool,
  createProfileLoadTool,
  sessionFactory,
};

// NOTE: Session tools moved to domains/session.js

// Ecosystem domain (ecosystem, ecosystem_monitor, integrator, discovery)
import {
  createEcosystemTool,
  createEcosystemMonitorTool,
  createIntegratorTool,
  createDiscoveryTool,
  ecosystemFactory,
} from './domains/ecosystem.js';

export {
  createEcosystemTool,
  createEcosystemMonitorTool,
  createIntegratorTool,
  createDiscoveryTool,
  ecosystemFactory,
};

// NOTE: Ecosystem tools moved to domains/ecosystem.js

// System domain (health, metrics, collective_status, agents_status, agent_diagnostic)
import {
  createHealthTool,
  createAgentsStatusTool,
  createMetricsTool,
  createCollectiveStatusTool,
  createAgentDiagnosticTool,
  systemFactory,
} from './domains/system.js';

export {
  createHealthTool,
  createAgentsStatusTool,
  createMetricsTool,
  createCollectiveStatusTool,
  createAgentDiagnosticTool,
  systemFactory,
};

// NOTE: System tools moved to domains/system.js

// Automation domain (orchestration, triggers)
import {
  createOrchestrationTool,
  createTriggersTool,
  automationFactory,
} from './domains/automation.js';

export {
  createOrchestrationTool,
  createTriggersTool,
  automationFactory,
};

// NOTE: Automation tools moved to domains/automation.js

/**
 * Create vector search tool definition
 * @param {Object} [options] - Options
 * @param {Object} [options.persistence] - PersistenceManager instance
 * @param {Object} [options.embeddingConfig] - External embedding config (optional)
 * @returns {Object} Tool definition
 */
export function createVectorSearchTool(options = {}) {
  const { persistence, embeddingConfig } = options;

  // Lazy load semantic search
  let semanticSearch = null;
  let initialized = false;

  return {
    name: 'brain_vector_search',
    description: `Semantic vector search for finding similar content.
Uses TF-IDF embeddings by default (no external API needed).
Actions:
- search: Find similar documents by semantic meaning
- add: Add document to search index
- remove: Remove document from index
- stats: Get search engine statistics
- initialize: Initialize with existing documents`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['search', 'add', 'remove', 'stats', 'initialize'],
          description: 'Action to perform',
        },
        query: {
          type: 'string',
          description: 'Search query text (for search action)',
        },
        documentId: {
          type: 'string',
          description: 'Document ID (for add/remove actions)',
        },
        text: {
          type: 'string',
          description: 'Document text (for add action)',
        },
        metadata: {
          type: 'object',
          description: 'Document metadata (for add action)',
        },
        documents: {
          type: 'array',
          description: 'Array of { id, text, metadata } for initialize action',
        },
        limit: {
          type: 'number',
          description: 'Max results (default: 10)',
          default: 10,
        },
        threshold: {
          type: 'number',
          description: 'Minimum similarity threshold 0-1 (default: 0.382)',
          default: 0.382,
        },
        filter: {
          type: 'object',
          description: 'Metadata filter for search',
        },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const {
        action,
        query,
        documentId,
        text,
        metadata = {},
        documents,
        limit = 10,
        threshold = 0.382,
        filter = null,
      } = params;

      // Lazy load vector module
      if (!semanticSearch) {
        try {
          const core = await import('@cynic/core');

          // Choose embedder based on config
          let embedder;
          if (embeddingConfig?.apiKey) {
            embedder = new core.ExternalEmbedder(embeddingConfig);
          } else {
            embedder = new core.TfIdfEmbedder();
          }

          semanticSearch = new core.SemanticSearch({ embedder });
        } catch (e) {
          throw new Error(`Vector search module not available: ${e.message}`);
        }
      }

      switch (action) {
        case 'stats':
          return {
            ...semanticSearch.getStats(),
            initialized,
          };

        case 'initialize': {
          if (!documents || !Array.isArray(documents)) {
            // Try to load from persistence
            if (persistence) {
              try {
                const knowledge = await persistence.getRecentKnowledge(100);
                const docs = knowledge.map(k => ({
                  id: k.knowledge_id,
                  text: k.content || k.summary || '',
                  metadata: {
                    sourceType: k.source_type,
                    category: k.category,
                  },
                })).filter(d => d.text);

                await semanticSearch.initialize(docs);
                initialized = true;

                return {
                  action: 'initialize',
                  documentsLoaded: docs.length,
                  source: 'persistence',
                };
              } catch (e) {
                throw new Error(`Failed to load from persistence: ${e.message}`);
              }
            }
            throw new Error('documents array required for initialize action');
          }

          await semanticSearch.initialize(documents);
          initialized = true;

          return {
            action: 'initialize',
            documentsLoaded: documents.length,
            source: 'provided',
          };
        }

        case 'add': {
          if (!documentId) throw new Error('documentId required for add action');
          if (!text) throw new Error('text required for add action');

          await semanticSearch.addDocument(documentId, text, metadata);

          return {
            action: 'add',
            documentId,
            textLength: text.length,
            metadata,
          };
        }

        case 'remove': {
          if (!documentId) throw new Error('documentId required for remove action');

          semanticSearch.removeDocument(documentId);

          return {
            action: 'remove',
            documentId,
            removed: true,
          };
        }

        case 'search': {
          if (!query) throw new Error('query required for search action');

          // Auto-initialize if empty
          if (!initialized && semanticSearch.index.size() === 0) {
            // Try to initialize from persistence
            if (persistence) {
              try {
                const knowledge = await persistence.getRecentKnowledge(100);
                const docs = knowledge.map(k => ({
                  id: k.knowledge_id,
                  text: k.content || k.summary || '',
                  metadata: {
                    sourceType: k.source_type,
                    category: k.category,
                  },
                })).filter(d => d.text);

                if (docs.length > 0) {
                  await semanticSearch.initialize(docs);
                  initialized = true;
                }
              } catch (e) {
                // Non-blocking, continue with empty index
              }
            }
          }

          const results = await semanticSearch.search(query, {
            limit,
            threshold,
            filter,
          });

          return {
            action: 'search',
            query,
            results: results.map(r => ({
              id: r.id,
              similarity: r.similarity,
              confidence: r.confidence,
              textPreview: r.text ? r.text.slice(0, 200) + (r.text.length > 200 ? '...' : '') : null,
              metadata: r.metadata,
            })),
            totalResults: results.length,
          };
        }

        default:
          throw new Error(`Unknown action: ${action}`);
      }
    },
  };
}

// NOTE: createLearningTool implementation moved to domains/judgment.js

/**
 * Create codebase analyzer tool definition
 * @param {Object} [options] - Options including rootPath
 * @returns {Object} Tool definition
 */
export function createCodebaseTool(options = {}) {
  // Lazily create analyzer on first use
  let analyzer = null;

  const getAnalyzer = () => {
    if (!analyzer) {
      analyzer = createCodeAnalyzer(options);
    }
    return analyzer;
  };

  return {
    name: 'brain_codebase',
    description: 'Analyze CYNIC codebase structure for 3D visualization. Get package hierarchy, search symbols, and view codebase metrics.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['tree', 'package', 'search', 'stats', 'invalidate'],
          description: 'Action: tree (full hierarchy), package (single package details), search (find symbols), stats (codebase metrics), invalidate (clear cache)',
        },
        name: {
          type: 'string',
          description: 'Package name for "package" action (e.g., "node", "mcp")',
        },
        query: {
          type: 'string',
          description: 'Search query for "search" action (symbol name)',
        },
      },
    },
    handler: async (params) => {
      const { action = 'tree', name, query } = params;
      const codeAnalyzer = getAnalyzer();

      switch (action) {
        case 'tree': {
          const tree = await codeAnalyzer.getTree();
          return {
            action: 'tree',
            root: tree.root,
            packages: tree.packages.map(pkg => ({
              name: pkg.name,
              shortName: pkg.shortName,
              path: pkg.path,
              color: pkg.color,
              stats: pkg.stats,
              modules: pkg.modules.map(mod => ({
                name: mod.name,
                path: mod.path,
                lines: mod.lines,
                classes: mod.classes?.map(cls => ({
                  name: cls.name,
                  line: cls.line,
                  methodCount: cls.methods?.length || 0,
                  methods: cls.methods?.map(m => ({
                    name: m.name,
                    line: m.line,
                    params: m.params,
                    visibility: m.visibility,
                  })),
                })) || [],
                functions: mod.functions?.map(fn => ({
                  name: fn.name,
                  line: fn.line,
                  params: fn.params,
                  exported: fn.exported,
                })) || [],
              })),
            })),
            stats: tree.stats,
            message: `*tail wag* Scanned ${tree.stats.packages} packages, ${tree.stats.classes} classes, ${tree.stats.methods} methods.`,
            timestamp: Date.now(),
          };
        }

        case 'package': {
          if (!name) {
            return {
              error: 'name required for package action',
              hint: 'Provide package name like "node", "mcp", "core"',
              timestamp: Date.now(),
            };
          }
          const pkg = await codeAnalyzer.getPackage(name);
          if (!pkg) {
            return {
              error: `Package "${name}" not found`,
              timestamp: Date.now(),
            };
          }
          return {
            action: 'package',
            package: pkg,
            message: `*ears perk* Package ${pkg.name}: ${pkg.stats.modules} modules, ${pkg.stats.classes} classes.`,
            timestamp: Date.now(),
          };
        }

        case 'search': {
          if (!query) {
            return {
              error: 'query required for search action',
              hint: 'Search for class, method, or function names',
              timestamp: Date.now(),
            };
          }
          const results = await codeAnalyzer.search(query);
          return {
            action: 'search',
            query,
            results: results.slice(0, 50), // Limit results
            total: results.length,
            message: results.length > 0
              ? `*sniff* Found ${results.length} symbols matching "${query}".`
              : `*head tilt* No symbols found matching "${query}".`,
            timestamp: Date.now(),
          };
        }

        case 'stats': {
          const stats = await codeAnalyzer.getStats();
          return {
            action: 'stats',
            stats,
            message: `*tail wag* Codebase: ${stats.packages} packages, ${stats.modules} modules, ${stats.lines} lines.`,
            timestamp: Date.now(),
          };
        }

        case 'invalidate': {
          codeAnalyzer.invalidateCache();
          return {
            action: 'invalidate',
            message: '*growl* Cache invalidated. Next request will rescan.',
            timestamp: Date.now(),
          };
        }

        default:
          return {
            error: `Unknown action: ${action}`,
            validActions: ['tree', 'package', 'search', 'stats', 'invalidate'],
            timestamp: Date.now(),
          };
      }
    },
  };
}

/**
 * Create all tools
 * @param {Object} options - Tool options
 * @param {Object} options.judge - CYNICJudge instance
 * @param {Object} [options.node] - CYNICNode instance
 * @param {Object} [options.persistence] - PersistenceManager instance (handles fallback)
 * @param {Object} [options.agents] - DEPRECATED: Legacy AgentManager (use collective instead)
 * @param {Object} [options.sessionManager] - SessionManager instance for multi-user sessions
 * @param {Object} [options.pojChainManager] - PoJChainManager instance for blockchain
 * @param {Object} [options.librarian] - LibrarianService instance for documentation caching
 * @param {Object} [options.ecosystem] - EcosystemService instance for pre-loaded docs
 * @param {Object} [options.integrator] - IntegratorService instance for cross-project sync
 * @param {Object} [options.metrics] - MetricsService instance for monitoring
 * @param {Object} [options.graphIntegration] - JudgmentGraphIntegration instance for graph edges
 * @param {Object} [options.codebaseOptions] - Options for code analyzer (rootPath, etc)
 * @param {Object} [options.discovery] - DiscoveryService instance for MCP/plugin/node discovery
 * @param {Object} [options.learningService] - LearningService instance for RLHF-style learning
 * @param {Object} [options.eScoreCalculator] - EScoreCalculator instance for vote weight
 * @param {Function} [options.onJudgment] - Callback when judgment is completed (for SSE broadcast)
 * @returns {Object} All tools keyed by name
 */
export function createAllTools(options = {}) {
  const {
    judge,
    node = null,
    persistence = null,
    agents = null,
    collective = null, // CollectivePack (The Five Dogs + CYNIC)
    sessionManager = null,
    pojChainManager = null,
    librarian = null,
    ecosystem = null,
    integrator = null,
    metrics = null,
    graphIntegration = null, // JudgmentGraphIntegration for graph edges
    codebaseOptions = {},
    lspOptions = {}, // LSP service options (rootPath, extensions, cacheTTL)
    discovery = null, // DiscoveryService for MCP/plugin/node discovery
    learningService = null, // LearningService for RLHF feedback
    eScoreCalculator = null, // EScoreCalculator for vote weight
    onJudgment = null, // SSE broadcast callback
  } = options;

  // Initialize LSP service for code intelligence
  const lsp = new LSPService({
    rootPath: lspOptions.rootPath || codebaseOptions.rootPath || process.cwd(),
    ...lspOptions,
  });

  // Initialize JSON render service for streaming UI
  const jsonRenderer = new JSONRenderService();

  if (!judge) throw new Error('judge is required');

  const tools = {};
  const toolDefs = [
    createJudgeTool(judge, persistence, sessionManager, pojChainManager, graphIntegration, onJudgment),
    createRefineTool(judge, persistence), // Self-refinement: critique → refine → learn
    createOrchestrationTool({ judge, agents, persistence }), // Multi-agent parallel execution
    createVectorSearchTool({ persistence }), // Semantic search with embeddings
    createLearningTool({ learningService, persistence }), // Learning service: feedback → weight modifiers → improvement
    createTriggersTool({ judge, persistence }), // Auto-judgment triggers
    createDigestTool(persistence, sessionManager),
    createHealthTool(node, judge, persistence),
    createPsychologyTool(persistence), // Human psychology dashboard
    createSearchTool(persistence),
    // Progressive Search Tools (3-layer retrieval for 10x token savings)
    createSearchIndexTool(persistence),
    createTimelineTool(persistence),
    createGetObservationsTool(persistence),
    createPatternsTool(judge, persistence),
    createFeedbackTool(persistence, sessionManager),
    createAgentsStatusTool(collective), // DEPRECATED: redirects to Collective
    createCollectiveStatusTool(collective), // The Eleven Dogs + CYNIC (Keter)
    createAgentDiagnosticTool(collective),
    createSessionStartTool(sessionManager),
    createSessionEndTool(sessionManager),
    createProfileSyncTool(persistence),  // Cross-session memory: sync profile to DB
    createProfileLoadTool(persistence),  // Cross-session memory: load profile from DB
    createDocsTool(librarian, persistence),
    createEcosystemTool(ecosystem),
    createEcosystemMonitorTool({ judge, persistence }), // External sources: GitHub, Twitter, Web + auto-analysis
    createDiscoveryTool(discovery), // MCP servers, plugins, CYNIC nodes
    createPoJChainTool(pojChainManager, persistence),
    createTraceTool(persistence, pojChainManager),
    createIntegratorTool(integrator),
    createMetricsTool(metrics),
    createMetaTool(), // CYNIC self-analysis dashboard
    createCodebaseTool(codebaseOptions), // Code structure analyzer
    // Dashboard real-data tools (Singularity Index components)
    createMilestoneHistoryTool(persistence), // Historical singularity scores
    createSelfModTool(), // Git history analysis
    createEmergenceTool(judge, persistence), // Consciousness signals
    // LSP Tools (code intelligence: symbols, references, call graphs, refactoring)
    ...createLSPTools(lsp),
    // JSON Render (streaming UI components)
    createJSONRenderTool(jsonRenderer),
  ];

  for (const tool of toolDefs) {
    tools[tool.name] = tool;
  }

  return tools;
}

// Re-export meta tool
export { createMetaTool } from '../meta-dashboard.js';

// Re-export progressive search tools
export {
  createSearchIndexTool,
  createTimelineTool,
  createGetObservationsTool,
  createProgressiveSearchTools,
} from './search-progressive.js';

// Re-export LSP tools
export { LSPService, createLSPTools } from '../lsp-service.js';

// Re-export JSON render
export { JSONRenderService, createJSONRenderTool } from '../json-render.js';

// ═══════════════════════════════════════════════════════════════════════════
// OCP: TOOL REGISTRY (Open for Extension, Closed for Modification)
// Add new tools by registering factories, not modifying this file
// ═══════════════════════════════════════════════════════════════════════════

export { ToolRegistry, defaultRegistry, registerTool, registerTools } from './registry.js';

export default {
  createJudgeTool,
  createDigestTool,
  createHealthTool,
  createSearchTool,
  // Progressive Search (3-layer retrieval)
  createSearchIndexTool,
  createTimelineTool,
  createGetObservationsTool,
  createProgressiveSearchTools,
  createPatternsTool,
  createFeedbackTool,
  createAgentsStatusTool,
  createCollectiveStatusTool, // NEW: The Five Dogs + CYNIC
  createSessionStartTool,
  createSessionEndTool,
  createProfileSyncTool,
  createProfileLoadTool,
  createDocsTool,
  createEcosystemTool,
  createDiscoveryTool,
  createPoJChainTool,
  createTraceTool,
  createIntegratorTool,
  createMetricsTool,
  createMetaTool,
  createCodebaseTool,
  // Dashboard real-data tools
  createMilestoneHistoryTool,
  createSelfModTool,
  createEmergenceTool,
  // LSP Tools (code intelligence)
  LSPService,
  createLSPTools,
  // JSON Render
  JSONRenderService,
  createJSONRenderTool,
  createAllTools,
};

