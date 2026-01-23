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

/**
 * Create orchestration tool definition
 * @param {Object} options - Orchestration options
 * @param {Object} options.judge - CYNICJudge instance
 * @param {Object} options.agents - Agents registry
 * @param {Object} [options.persistence] - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createOrchestrationTool(options = {}) {
  const { judge, agents, persistence } = options;

  // Lazy load orchestration module
  let orchestrator = null;

  return {
    name: 'brain_orchestrate',
    description: `Background agent orchestration for parallel task execution.
Run tasks across multiple agents with different strategies:
- parallel: Run same task on multiple agents, aggregate results
- batch: Run different tasks in parallel
- route: Route a task to the best agent
- status: Get orchestrator status`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['parallel', 'batch', 'route', 'submit', 'status', 'task'],
          description: 'Action: parallel (multi-agent consensus), batch (multiple different tasks), route (find best agent), submit (single task), status, task (get task status)',
        },
        taskType: {
          type: 'string',
          description: 'Task type (e.g., "judge", "analyze", "search")',
        },
        payload: {
          type: 'object',
          description: 'Task payload/data',
        },
        tasks: {
          type: 'array',
          description: 'For batch: array of { type, payload, priority } task definitions',
        },
        taskId: {
          type: 'string',
          description: 'For task action: ID of task to check',
        },
        agentCount: {
          type: 'number',
          description: 'For parallel: number of agents to use (default: 3)',
          default: 3,
        },
        strategy: {
          type: 'string',
          enum: ['first', 'best', 'consensus', 'all', 'merge'],
          description: 'Aggregation strategy for parallel execution',
          default: 'consensus',
        },
        priority: {
          type: 'string',
          enum: ['critical', 'high', 'normal', 'low', 'idle'],
          description: 'Task priority',
          default: 'normal',
        },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const {
        action,
        taskType,
        payload,
        tasks,
        taskId,
        agentCount = 3,
        strategy = 'consensus',
        priority = 'normal',
      } = params;

      // Lazy load orchestration module
      if (!orchestrator) {
        try {
          const core = await import('@cynic/core');
          orchestrator = new core.Orchestrator({
            agents: agents instanceof Map ? agents : new Map(Object.entries(agents || {})),
            maxConcurrent: 21, // Fib(8)
          });

          // Register judge as default agent
          if (judge) {
            orchestrator.registerAgent('judge', judge, {
              taskTypes: ['judge', 'analyze', 'evaluate'],
              maxConcurrent: 5,
            });
          }
        } catch (e) {
          throw new Error(`Orchestration module not available: ${e.message}`);
        }
      }

      switch (action) {
        case 'status':
          return orchestrator.getStatus();

        case 'task': {
          if (!taskId) throw new Error('taskId required for task action');
          const task = orchestrator.getTask(taskId);
          if (!task) throw new Error(`Task not found: ${taskId}`);
          return task.toJSON();
        }

        case 'route': {
          if (!taskType) throw new Error('taskType required for route action');
          const available = orchestrator.getAvailableAgents(taskType);
          return {
            taskType,
            availableAgents: available.map(a => ({
              agentId: a.agentId,
              load: a.load,
              averageTime: a.averageTime,
            })),
            recommended: available[0]?.agentId || null,
          };
        }

        case 'submit': {
          if (!taskType) throw new Error('taskType required for submit action');
          const submitted = orchestrator.submit({
            type: taskType,
            payload: payload || {},
            priority,
          });
          return {
            taskId: submitted.id,
            status: submitted.status,
            priority: submitted.priority,
            message: 'Task submitted to queue',
          };
        }

        case 'parallel': {
          if (!taskType) throw new Error('taskType required for parallel action');
          const result = await orchestrator.parallel(
            { type: taskType, payload: payload || {}, priority },
            { count: agentCount, strategy }
          );
          return {
            action: 'parallel',
            taskType,
            agentCount,
            strategy,
            ...result,
          };
        }

        case 'batch': {
          if (!tasks || !Array.isArray(tasks)) {
            throw new Error('tasks array required for batch action');
          }
          const batchTasks = tasks.map(t => ({
            type: t.type || t.taskType,
            payload: t.payload || {},
            priority: t.priority || 'normal',
          }));
          const results = await orchestrator.batch(batchTasks);
          return {
            action: 'batch',
            submitted: batchTasks.length,
            results: results.map(t => ({
              taskId: t.id,
              type: t.type,
              status: t.status,
              duration: t.getDuration(),
              hasResult: !!t.result,
              error: t.error,
            })),
          };
        }

        default:
          throw new Error(`Unknown action: ${action}`);
      }
    },
  };
}

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
 * Create auto-triggers tool definition
 * @param {Object} [options] - Options
 * @param {Object} [options.judge] - CYNICJudge instance
 * @param {Object} [options.persistence] - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createTriggersTool(options = {}) {
  const { judge, persistence } = options;

  // Lazy load trigger manager
  let triggerManager = null;

  return {
    name: 'brain_triggers',
    description: `Auto-judgment triggers for automatic evaluation of events.
Actions:
- register: Register a new trigger
- unregister: Remove a trigger
- list: List all triggers
- enable/disable: Toggle trigger state
- process: Manually process an event
- status: Get trigger manager status
- defaults: Register predefined default triggers`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['register', 'unregister', 'list', 'enable', 'disable', 'process', 'status', 'defaults'],
          description: 'Action to perform',
        },
        triggerId: {
          type: 'string',
          description: 'Trigger ID (for unregister/enable/disable)',
        },
        trigger: {
          type: 'object',
          description: 'Trigger configuration for register',
          properties: {
            name: { type: 'string' },
            type: { type: 'string', enum: ['event', 'periodic', 'pattern', 'threshold', 'composite'] },
            condition: { type: 'object' },
            action: { type: 'string', enum: ['judge', 'log', 'alert', 'block', 'review', 'notify'] },
            config: { type: 'object' },
          },
        },
        event: {
          type: 'object',
          description: 'Event data for process action',
          properties: {
            type: { type: 'string' },
            data: { type: 'object' },
          },
        },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const {
        action,
        triggerId,
        trigger: triggerConfig,
        event,
      } = params;

      // Lazy load trigger module
      if (!triggerManager) {
        try {
          const core = await import('@cynic/core');
          triggerManager = new core.TriggerManager({
            judgeCallback: judge ? async (input) => {
              return judge.judge(input);
            } : null,
            alertCallback: async (alert) => {
              console.log('[CYNIC Alert]', alert);
              return alert;
            },
          });

          // Try to load existing triggers from persistence
          if (persistence) {
            try {
              // First, try to load from PostgreSQL (new method)
              const dbTriggers = await persistence.getEnabledTriggers();
              if (dbTriggers && dbTriggers.length > 0) {
                // Register each trigger from DB
                for (const t of dbTriggers) {
                  try {
                    triggerManager.register({
                      id: t.triggerId,
                      name: t.name,
                      type: t.triggerType,
                      condition: t.condition,
                      action: t.action,
                      config: t.actionConfig,
                      enabled: t.enabled,
                      priority: t.priority,
                    });
                  } catch (regErr) {
                    // Skip invalid triggers
                    console.error(`[Triggers] Failed to load trigger ${t.triggerId}: ${regErr.message}`);
                  }
                }
                console.error(`[Triggers] Loaded ${dbTriggers.length} triggers from PostgreSQL`);
              } else {
                // Fall back to legacy state (file/memory)
                const state = await persistence.getTriggersState();
                if (state) {
                  triggerManager.import(state);
                  console.error(`[Triggers] Loaded triggers from legacy state`);
                }
              }
            } catch (e) {
              // No saved state, start fresh
              console.error(`[Triggers] Starting fresh: ${e.message}`);
            }
          }
        } catch (e) {
          throw new Error(`Triggers module not available: ${e.message}`);
        }
      }

      switch (action) {
        case 'status':
          return triggerManager.getStatus();

        case 'list':
          return { triggers: triggerManager.listTriggers() };

        case 'defaults': {
          triggerManager.registerDefaults();

          // Save state
          if (persistence) {
            try {
              await persistence.saveTriggersState(triggerManager.export());
            } catch (e) {
              // Non-blocking
            }
          }

          return {
            registered: true,
            triggers: triggerManager.listTriggers(),
          };
        }

        case 'register': {
          if (!triggerConfig) {
            throw new Error('trigger configuration required for register');
          }

          const trigger = triggerManager.register(triggerConfig);

          // Save to PostgreSQL (primary) and legacy state (fallback)
          if (persistence) {
            try {
              // Try PostgreSQL first
              await persistence.createTrigger({
                triggerId: trigger.id,
                name: triggerConfig.name,
                triggerType: triggerConfig.type,
                condition: triggerConfig.condition,
                action: triggerConfig.action,
                actionConfig: triggerConfig.config,
                enabled: triggerConfig.enabled !== false,
                priority: triggerConfig.priority || 50,
              });
            } catch (e) {
              // Fall back to legacy state
              try {
                await persistence.saveTriggersState(triggerManager.export());
              } catch (e2) {
                // Non-blocking
              }
            }
          }

          return {
            registered: true,
            trigger: trigger.toJSON(),
          };
        }

        case 'unregister': {
          if (!triggerId) {
            throw new Error('triggerId required for unregister');
          }

          const removed = triggerManager.unregister(triggerId);

          // Delete from PostgreSQL (primary) and save legacy state (fallback)
          if (persistence) {
            try {
              await persistence.deleteTrigger(triggerId);
            } catch (e) {
              // Fall back to legacy state
              try {
                await persistence.saveTriggersState(triggerManager.export());
              } catch (e2) {
                // Non-blocking
              }
            }
          }

          return { unregistered: removed, triggerId };
        }

        case 'enable':
        case 'disable': {
          if (!triggerId) {
            throw new Error('triggerId required');
          }

          const enabled = action === 'enable';
          const success = triggerManager.setEnabled(triggerId, enabled);

          // Update in PostgreSQL
          if (persistence && success) {
            try {
              if (enabled) {
                await persistence.enableTrigger(triggerId);
              } else {
                await persistence.disableTrigger(triggerId);
              }
            } catch (e) {
              // Non-blocking
            }
          }

          return { success, triggerId, enabled };
        }

        case 'process': {
          if (!event) {
            throw new Error('event required for process');
          }

          const results = await triggerManager.processEvent(event);

          return {
            processed: true,
            event: { type: event.type },
            triggersActivated: results.length,
            results,
          };
        }

        default:
          throw new Error(`Unknown action: ${action}`);
      }
    },
  };
}

/**
 * Create ecosystem monitor tool definition
 * Tracks external sources: GitHub, Twitter (TODO), Web
 * @param {Object} [options] - Options
 * @param {Object} [options.judge] - CYNICJudge instance for commit analysis
 * @param {Object} [options.persistence] - PersistenceManager for storing digests
 * @returns {Object} Tool definition
 */
export function createEcosystemMonitorTool(options = {}) {
  const { judge = null, persistence = null } = options;

  // Singleton monitor instance (created on first use)
  let monitorInstance;

  const getMonitor = () => {
    if (!monitorInstance) {
      monitorInstance = new EcosystemMonitor();
    }
    return monitorInstance;
  };

  /**
   * Digest a single commit/update for learning
   * @param {Object} update - Update from ecosystem monitor
   * @returns {Promise<Object>} Digest result
   */
  const digestUpdate = async (update) => {
    const content = `${update.title || ''}\n\n${update.description || ''}`;
    const source = update.url || `${update.source}:${update.id}`;

    // Extract patterns from commit message
    const patterns = [];
    const commitType = update.title?.match(/^(feat|fix|docs|chore|refactor|test|style|perf|ci|build)(\(.*?\))?:/i);
    if (commitType) {
      patterns.push({ type: 'commit_convention', value: commitType[1].toLowerCase() });
    }

    // Extract insights
    const insights = [];
    if (update.priority === 'CRITICAL' || update.priority === 'HIGH') {
      insights.push({ importance: 'high', reason: `Priority: ${update.priority}` });
    }
    if (update.type === 'RELEASE') {
      insights.push({ type: 'release', version: update.id });
    }

    const digest = {
      id: `dig_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 6)}`,
      source,
      sourceType: 'commit',
      content: content.slice(0, 500),
      patterns,
      insights,
      metadata: {
        updateType: update.type,
        priority: update.priority,
        author: update.author,
        repo: update.meta?.repo,
        sha: update.meta?.sha,
        timestamp: update.timestamp,
      },
      digestedAt: Date.now(),
    };

    // Store if persistence available
    if (persistence?.storeDigest) {
      try {
        await persistence.storeDigest(digest);
      } catch (e) {
        // Non-blocking
      }
    }

    // If judge available, create a quick judgment for pattern extraction
    if (judge) {
      try {
        const judgment = await judge.judge({
          type: 'commit',
          content: content.slice(0, 200),
          sources: [source],
        });
        digest.qScore = judgment.Q;
        digest.verdict = judgment.verdict;
      } catch (e) {
        // Non-blocking
      }
    }

    return digest;
  };

  return {
    name: 'brain_ecosystem_monitor',
    description: `Monitor external ecosystem sources for updates.
Actions:
- track: Add a GitHub repo to track (owner, repo required)
- untrack: Remove a source (sourceId required)
- sources: List all tracked sources
- fetch: Fetch updates from one source (sourceId) or all sources
- updates: Get recent updates from cache
- defaults: Register default Solana ecosystem sources
- discover: Discover new relevant sources
- status: Get monitor status
- analyze: Analyze a specific update (updateIndex required)
- analyze_all: Analyze all unanalyzed updates (batch learning)`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['track', 'untrack', 'sources', 'fetch', 'updates', 'defaults', 'discover', 'status', 'analyze', 'analyze_all'],
          description: 'Action to perform',
        },
        owner: {
          type: 'string',
          description: 'GitHub repo owner (for track action)',
        },
        repo: {
          type: 'string',
          description: 'GitHub repo name (for track action)',
        },
        sourceId: {
          type: 'string',
          description: 'Source ID (for untrack, fetch actions)',
        },
        trackReleases: {
          type: 'boolean',
          description: 'Track releases (default true)',
        },
        trackCommits: {
          type: 'boolean',
          description: 'Track commits (default true)',
        },
        limit: {
          type: 'number',
          description: 'Max results (for updates action)',
        },
        minPriority: {
          type: 'string',
          enum: ['CRITICAL', 'HIGH', 'MEDIUM', 'LOW', 'INFO'],
          description: 'Minimum priority filter (for updates action)',
        },
        updateIndex: {
          type: 'number',
          description: 'Index of update to analyze (for analyze action)',
        },
        autoAnalyze: {
          type: 'boolean',
          description: 'Automatically analyze updates after fetch (default false)',
        },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const {
        action,
        owner,
        repo,
        sourceId,
        trackReleases = true,
        trackCommits = true,
        limit = 20,
        minPriority,
        updateIndex,
        autoAnalyze = false,
      } = params;

      const monitor = getMonitor();

      switch (action) {
        case 'track': {
          if (!owner || !repo) {
            throw new Error('owner and repo required for track action');
          }

          const id = monitor.trackGitHubRepo(owner, repo, {
            trackReleases,
            trackCommits,
          });

          return {
            success: true,
            sourceId: id,
            message: `*ears perk* Now tracking ${owner}/${repo}`,
            timestamp: Date.now(),
          };
        }

        case 'untrack': {
          if (!sourceId) {
            throw new Error('sourceId required for untrack action');
          }

          const success = monitor.unregisterSource(sourceId);

          return {
            success,
            sourceId,
            message: success ? '*nod* Source removed' : 'Source not found',
            timestamp: Date.now(),
          };
        }

        case 'sources': {
          const sources = monitor.listSources();

          return {
            sources,
            total: sources.length,
            message: `*sniff* Tracking ${sources.length} sources`,
            timestamp: Date.now(),
          };
        }

        case 'fetch': {
          if (sourceId) {
            // Fetch single source
            const result = await monitor.fetchSource(sourceId);

            // Auto-analyze if enabled
            let analyzed = 0;
            if (autoAnalyze && result.success && result.updates?.length > 0) {
              for (const update of result.updates.slice(0, 10)) {
                try {
                  await digestUpdate(update);
                  analyzed++;
                } catch (e) {
                  // Non-blocking
                }
              }
            }

            return {
              ...result,
              analyzed: autoAnalyze ? analyzed : undefined,
              message: result.success
                ? `*tail wag* Fetched ${result.updates?.length || 0} updates${autoAnalyze ? `, analyzed ${analyzed}` : ''}`
                : `*head tilt* ${result.reason || result.error}`,
              timestamp: Date.now(),
            };
          } else {
            // Fetch all
            const results = await monitor.fetchAll();
            const summary = summarizeUpdates(results.updates);

            // Auto-analyze if enabled
            let analyzed = 0;
            if (autoAnalyze && results.updates.length > 0) {
              for (const update of results.updates.slice(0, 20)) {
                try {
                  await digestUpdate(update);
                  analyzed++;
                } catch (e) {
                  // Non-blocking
                }
              }
            }

            return {
              fetched: results.fetched,
              skipped: results.skipped,
              errors: results.errors,
              totalUpdates: results.updates.length,
              analyzed: autoAnalyze ? analyzed : undefined,
              summary,
              message: `*sniff* Fetched ${results.updates.length} updates from ${results.fetched} sources${autoAnalyze ? `, analyzed ${analyzed}` : ''}`,
              timestamp: Date.now(),
            };
          }
        }

        case 'updates': {
          const updates = monitor.getRecentUpdates({
            limit,
            minPriority,
          });

          const summary = summarizeUpdates(updates);

          return {
            updates: updates.map(u => ({
              type: u.type,
              title: u.title,
              url: u.url,
              priority: u.priority,
              source: u.source,
              timestamp: u.timestamp,
            })),
            summary,
            total: updates.length,
            message: `*ears perk* ${updates.length} recent updates`,
            timestamp: Date.now(),
          };
        }

        case 'defaults': {
          const sources = monitor.registerSolanaDefaults();

          return {
            sources,
            total: sources.length,
            message: '*tail wag* Solana ecosystem defaults registered',
            timestamp: Date.now(),
          };
        }

        case 'discover': {
          const suggestions = await monitor.discoverSources();

          return {
            suggestions,
            total: suggestions.length,
            message: `*sniff* Found ${suggestions.length} potential sources`,
            timestamp: Date.now(),
          };
        }

        case 'status': {
          const status = monitor.getStatus();

          return {
            ...status,
            message: `*nod* Monitoring ${status.sources.length} sources`,
            timestamp: Date.now(),
          };
        }

        case 'analyze': {
          if (updateIndex === undefined) {
            throw new Error('updateIndex required for analyze action');
          }

          const updates = monitor.getRecentUpdates({ limit: 100 });
          if (updateIndex < 0 || updateIndex >= updates.length) {
            throw new Error(`Invalid updateIndex: ${updateIndex} (have ${updates.length} updates)`);
          }

          const update = updates[updateIndex];
          const digest = await digestUpdate(update);

          return {
            digest,
            update: {
              type: update.type,
              title: update.title,
              url: update.url,
            },
            message: `*sniff* Analyzed commit: ${digest.verdict || 'digested'}`,
            timestamp: Date.now(),
          };
        }

        case 'analyze_all': {
          const updates = monitor.getRecentUpdates({ limit: limit || 50 });
          const results = {
            analyzed: 0,
            skipped: 0,
            digests: [],
            byVerdict: {},
            byType: {},
          };

          for (const update of updates) {
            try {
              const digest = await digestUpdate(update);
              results.analyzed++;
              results.digests.push({
                id: digest.id,
                source: digest.source,
                qScore: digest.qScore,
                verdict: digest.verdict,
              });

              // Track verdicts
              if (digest.verdict) {
                results.byVerdict[digest.verdict] = (results.byVerdict[digest.verdict] || 0) + 1;
              }

              // Track types
              const commitType = digest.patterns?.find(p => p.type === 'commit_convention')?.value || 'other';
              results.byType[commitType] = (results.byType[commitType] || 0) + 1;
            } catch (e) {
              results.skipped++;
            }
          }

          return {
            ...results,
            total: updates.length,
            message: `*tail wag* Analyzed ${results.analyzed} commits, learned ${Object.keys(results.byType).length} patterns`,
            timestamp: Date.now(),
          };
        }

        default:
          throw new Error(`Unknown action: ${action}`);
      }
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

// NOTE: createFeedbackTool implementation moved to domains/judgment.js


/**
 * Create agents status tool definition
 * DEPRECATED: Use brain_collective_status instead
 * @param {Object} collective - CollectivePack instance (The 11 Dogs)
 * @returns {Object} Tool definition
 */
export function createAgentsStatusTool(collective) {
  return {
    name: 'brain_agents_status',
    description: 'DEPRECATED: Use brain_collective_status instead. Returns collective status for backwards compatibility.',
    inputSchema: {
      type: 'object',
      properties: {
        verbose: { type: 'boolean', description: 'Include detailed per-agent statistics' },
        agent: { type: 'string', enum: ['guardian', 'analyst', 'scholar', 'architect', 'sage', 'cynic', 'janitor', 'scout', 'cartographer', 'oracle', 'deployer'], description: 'Get status for specific agent only' },
      },
    },
    handler: async (params) => {
      const { verbose = false, agent = null } = params;

      // Redirect to collective
      if (!collective) {
        return {
          status: 'unavailable',
          message: 'Collective not initialized. Use brain_collective_status.',
          deprecated: true,
          timestamp: Date.now(),
        };
      }

      const summary = collective.getSummary();

      // If specific agent requested
      if (agent && summary.agents[agent]) {
        return {
          agent,
          ...summary.agents[agent],
          deprecated: true,
          useInstead: 'brain_collective_status',
          timestamp: Date.now(),
        };
      }

      // Return summary with deprecation notice
      return {
        deprecated: true,
        useInstead: 'brain_collective_status',
        dogCount: summary.dogCount,
        profileLevel: summary.profileLevel,
        dogs: Object.fromEntries(
          Object.entries(summary.agents).map(([name, data]) => [
            name,
            {
              invocations: data.invocations || 0,
              actions: data.actions || 0,
              blocks: data.blocks || 0,
              warnings: data.warnings || 0,
            },
          ])
        ),
        timestamp: Date.now(),
      };
    },
  };
}

/**
 * Create ecosystem docs tool definition
 * @param {Object} ecosystem - EcosystemService instance
 * @returns {Object} Tool definition
 */
export function createEcosystemTool(ecosystem) {
  return {
    name: 'brain_ecosystem',
    description: 'Access pre-loaded ecosystem documentation (CLAUDE.md, API docs, architecture). Provides context for the $ASDFASDFA ecosystem projects.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['get', 'search', 'list', 'context', 'refresh', 'stats'],
          description: 'Action: get (specific doc), search (query), list (all docs), context (relevant docs for task), refresh (reload), stats',
        },
        project: {
          type: 'string',
          description: 'Project name (cynic, holdex, gasdf, ecosystem, asdf-brain)',
        },
        docType: {
          type: 'string',
          description: 'Document type (claude_md, api_readme, architecture, harmony)',
        },
        query: {
          type: 'string',
          description: 'Search query or context description',
        },
        limit: {
          type: 'number',
          description: 'Maximum results (default 10)',
        },
      },
    },
    handler: async (params) => {
      const { action = 'list', project, docType, query, limit = 10 } = params;

      if (!ecosystem) {
        return {
          error: 'Ecosystem service not available',
          hint: 'Ecosystem docs provide pre-loaded project context',
          timestamp: Date.now(),
        };
      }

      switch (action) {
        case 'get': {
          if (!project || !docType) {
            return {
              error: 'Both project and docType required for get action',
              hint: 'Use action="list" to see available documents',
              timestamp: Date.now(),
            };
          }
          const doc = await ecosystem.get(project, docType);
          if (!doc) {
            return {
              error: `Document not found: ${project}/${docType}`,
              timestamp: Date.now(),
            };
          }
          return {
            action: 'get',
            project: doc.project,
            docType: doc.doc_type || doc.docType,
            content: doc.content,
            digest: doc.digest,
            hasDigest: !!doc.digest,
            message: `*ears perk* Found ${project}/${docType}.`,
            timestamp: Date.now(),
          };
        }

        case 'search': {
          if (!query) {
            return {
              error: 'query required for search action',
              timestamp: Date.now(),
            };
          }
          const results = await ecosystem.search(query, { limit });
          return {
            action: 'search',
            query,
            results,
            total: results.length,
            message: `*sniff* Found ${results.length} matches.`,
            timestamp: Date.now(),
          };
        }

        case 'list': {
          const docs = await ecosystem.list();
          return {
            action: 'list',
            documents: docs.map(d => ({
              project: d.project,
              docType: d.doc_type || d.docType,
              filePath: d.file_path || d.filePath,
              hasDigest: !!d.digest,
            })),
            total: docs.length,
            message: `*tail wag* ${docs.length} ecosystem docs loaded.`,
            timestamp: Date.now(),
          };
        }

        case 'context': {
          if (!query) {
            return {
              error: 'query required for context action (describe your task)',
              timestamp: Date.now(),
            };
          }
          const context = await ecosystem.getContextFor(query, { maxDocs: limit });
          return {
            action: 'context',
            query,
            documents: context.documents.map(d => ({
              project: d.project,
              docType: d.docType,
              contentLength: d.content?.length || 0,
              hasDigest: !!d.digest,
            })),
            totalLength: context.totalLength,
            count: context.count,
            message: `*head tilt* Selected ${context.count} relevant docs (${context.totalLength} chars).`,
            timestamp: Date.now(),
          };
        }

        case 'refresh': {
          const results = await ecosystem.refresh();
          return {
            action: 'refresh',
            ...results,
            message: `*sniff* Refreshed: ${results.loaded} loaded, ${results.skipped} unchanged, ${results.failed} failed.`,
            timestamp: Date.now(),
          };
        }

        case 'stats': {
          const stats = await ecosystem.getStats();
          return {
            action: 'stats',
            ...stats,
            message: `*tail wag* ${stats.total_docs || stats.loadCount || 0} docs, ${stats.searchCount || 0} searches, ${stats.hitCount || 0} hits.`,
            timestamp: Date.now(),
          };
        }

        default:
          return {
            error: `Unknown action: ${action}`,
            validActions: ['get', 'search', 'list', 'context', 'refresh', 'stats'],
            timestamp: Date.now(),
          };
      }
    },
  };
}


/**
 * Create integrator tool definition
 * @param {Object} integrator - IntegratorService instance
 * @returns {Object} Tool definition
 */
export function createIntegratorTool(integrator) {
  return {
    name: 'brain_integrator',
    description: 'Cross-project integration and synchronization. Check sync status, detect drift, get suggestions for keeping shared modules aligned.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['check', 'drifts', 'suggest', 'projects', 'modules', 'stats'],
          description: 'Action: check (sync status), drifts (current drifts), suggest (sync suggestions), projects (project status), modules (shared modules), stats (service stats)',
        },
        project: {
          type: 'string',
          description: 'Project name (for projects action)',
        },
      },
    },
    handler: async (params) => {
      const { action = 'check', project } = params;

      if (!integrator) {
        return {
          error: 'Integrator service not available',
          hint: 'IntegratorService tracks shared modules across projects',
          timestamp: Date.now(),
        };
      }

      switch (action) {
        case 'check': {
          const report = await integrator.checkSync();
          return {
            action: 'check',
            allSynced: report.allSynced,
            modulesChecked: report.modules.length,
            driftsFound: report.drifts.length,
            modules: report.modules.map(m => ({
              name: m.name,
              synced: m.synced,
              mirrorsCount: m.mirrors?.length || 0,
              driftsCount: m.drifts?.length || 0,
            })),
            message: report.allSynced
              ? `*tail wag* All ${report.modules.length} modules in sync!`
              : `*GROWL* Found ${report.drifts.length} drifts across ${report.modules.filter(m => !m.synced).length} modules.`,
            timestamp: Date.now(),
          };
        }

        case 'drifts': {
          const drifts = integrator.getDrifts();
          return {
            action: 'drifts',
            drifts: drifts.map(d => ({
              type: d.type,
              module: d.module,
              canonical: d.canonical,
              drifted: d.drifted,
              critical: d.critical,
            })),
            total: drifts.length,
            critical: drifts.filter(d => d.critical).length,
            message: drifts.length === 0
              ? `*tail wag* No drifts detected.`
              : `*sniff* ${drifts.length} drifts (${drifts.filter(d => d.critical).length} critical).`,
            timestamp: Date.now(),
          };
        }

        case 'suggest': {
          const suggestions = integrator.getSyncSuggestions();
          return {
            action: 'suggest',
            suggestions: suggestions.map(s => ({
              action: s.action,
              priority: s.priority,
              from: s.from,
              to: s.to,
              reason: s.reason,
              command: s.command,
            })),
            total: suggestions.length,
            highPriority: suggestions.filter(s => s.priority === 'high').length,
            message: suggestions.length === 0
              ? `*yawn* No sync actions needed.`
              : `*ears perk* ${suggestions.length} sync actions suggested (${suggestions.filter(s => s.priority === 'high').length} high priority).`,
            timestamp: Date.now(),
          };
        }

        case 'projects': {
          const status = await integrator.getProjectStatus(project);
          return {
            action: 'projects',
            ...status,
            message: `*sniff* ${status.available}/${status.total} projects available.`,
            timestamp: Date.now(),
          };
        }

        case 'modules': {
          const modules = integrator.getSharedModules();
          return {
            action: 'modules',
            modules,
            total: modules.length,
            critical: modules.filter(m => m.critical).length,
            message: `*tail wag* Tracking ${modules.length} shared modules (${modules.filter(m => m.critical).length} critical).`,
            timestamp: Date.now(),
          };
        }

        case 'stats': {
          const stats = integrator.getStats();
          return {
            action: 'stats',
            ...stats,
            message: `*ears perk* ${stats.checksPerformed} checks, ${stats.driftsDetected} drifts detected.`,
            timestamp: Date.now(),
          };
        }

        default:
          return {
            error: `Unknown action: ${action}`,
            validActions: ['check', 'drifts', 'suggest', 'projects', 'modules', 'stats'],
            timestamp: Date.now(),
          };
      }
    },
  };
}

/**
 * Create metrics tool definition
 * @param {Object} metricsService - MetricsService instance
 * @returns {Object} Tool definition
 */
export function createMetricsTool(metricsService) {
  return {
    name: 'brain_metrics',
    description: 'Get CYNIC metrics in various formats. Prometheus format for monitoring, JSON for inspection, HTML for dashboard.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['collect', 'prometheus', 'alerts', 'clear_alert', 'stats', 'html'],
          description: 'Action: collect (raw metrics), prometheus (Prometheus format), alerts (active alerts), clear_alert (acknowledge alert), stats (service stats), html (dashboard)',
        },
        alertType: {
          type: 'string',
          description: 'Alert type to clear (for clear_alert action)',
        },
      },
    },
    handler: async (params) => {
      const { action = 'collect', alertType } = params;

      if (!metricsService) {
        return {
          error: 'Metrics service not available',
          hint: 'MetricsService provides monitoring capabilities',
          timestamp: Date.now(),
        };
      }

      switch (action) {
        case 'collect': {
          const metrics = await metricsService.collect();
          return {
            action: 'collect',
            metrics,
            alerts: metricsService.getAlerts(),
            message: `*sniff* Collected metrics in ${metricsService.getStats().lastCollectMs}ms.`,
            timestamp: Date.now(),
          };
        }

        case 'prometheus': {
          const prometheus = await metricsService.toPrometheus();
          return {
            action: 'prometheus',
            format: 'text/plain',
            content: prometheus,
            message: '*tail wag* Metrics exported in Prometheus format.',
            timestamp: Date.now(),
          };
        }

        case 'alerts': {
          const alerts = metricsService.getAlerts();
          return {
            action: 'alerts',
            alerts,
            total: alerts.length,
            critical: alerts.filter(a => a.level === 'critical').length,
            warning: alerts.filter(a => a.level === 'warning').length,
            message: alerts.length === 0
              ? '*tail wag* No active alerts.'
              : `*ears perk* ${alerts.length} active alerts (${alerts.filter(a => a.level === 'critical').length} critical).`,
            timestamp: Date.now(),
          };
        }

        case 'clear_alert': {
          if (!alertType) {
            return {
              error: 'alertType required for clear_alert action',
              availableAlerts: metricsService.getAlerts().map(a => a.type),
              timestamp: Date.now(),
            };
          }
          const cleared = metricsService.clearAlert(alertType);
          return {
            action: 'clear_alert',
            alertType,
            cleared,
            message: cleared
              ? `*yawn* Alert '${alertType}' acknowledged.`
              : `*head tilt* Alert '${alertType}' not found.`,
            timestamp: Date.now(),
          };
        }

        case 'stats': {
          const stats = metricsService.getStats();
          return {
            action: 'stats',
            ...stats,
            message: `*sniff* ${stats.collectCount} collections, ${stats.alertsTriggered} alerts triggered.`,
            timestamp: Date.now(),
          };
        }

        case 'html': {
          const html = await metricsService.toHTML();
          return {
            action: 'html',
            format: 'text/html',
            content: html,
            message: '*tail wag* Dashboard HTML generated.',
            timestamp: Date.now(),
          };
        }

        default:
          return {
            error: `Unknown action: ${action}`,
            validActions: ['collect', 'prometheus', 'alerts', 'clear_alert', 'stats', 'html'],
            timestamp: Date.now(),
          };
      }
    },
  };
}

/**
 * Create collective status tool definition
 * @param {Object} collective - CollectivePack instance (The Five Dogs + CYNIC)
 * @returns {Object} Tool definition
 */
export function createCollectiveStatusTool(collective) {
  return {
    name: 'brain_collective_status',
    description: 'Get status and statistics for The Collective (11 Dogs + CYNIC meta-consciousness). Shows all agents with Sefirot mappings, event bus stats, and CYNIC\'s current state.',
    inputSchema: {
      type: 'object',
      properties: {
        verbose: { type: 'boolean', description: 'Include detailed per-agent statistics' },
        agent: {
          type: 'string',
          enum: ['guardian', 'analyst', 'scholar', 'architect', 'sage', 'cynic', 'janitor', 'scout', 'cartographer', 'oracle', 'deployer'],
          description: 'Get status for specific agent only',
        },
      },
    },
    handler: async (params) => {
      const { verbose = false, agent = null } = params;

      if (!collective) {
        return {
          status: 'unavailable',
          message: '*growl* Collective not initialized. Only legacy agents available.',
          hint: 'Use brain_agents_status for legacy Four Dogs.',
          timestamp: Date.now(),
        };
      }

      const summary = collective.getSummary();
      const collectiveState = collective.getCollectiveState();

      // Sefirot mapping for display
      const sefirotMap = {
        guardian: { sefira: 'Gevurah', meaning: 'Strength', role: 'Security & Protection' },
        analyst: { sefira: 'Binah', meaning: 'Understanding', role: 'Pattern Analysis' },
        scholar: { sefira: 'Daat', meaning: 'Knowledge', role: 'Knowledge Extraction' },
        architect: { sefira: 'Chesed', meaning: 'Kindness', role: 'Design Review' },
        sage: { sefira: 'Chochmah', meaning: 'Wisdom', role: 'Guidance & Teaching' },
        cynic: { sefira: 'Keter', meaning: 'Crown', role: 'Meta-Consciousness' },
        janitor: { sefira: 'Yesod', meaning: 'Foundation', role: 'Code Quality' },
        scout: { sefira: 'Netzach', meaning: 'Victory', role: 'Discovery' },
        cartographer: { sefira: 'Malkhut', meaning: 'Kingdom', role: 'Reality Mapping' },
        oracle: { sefira: 'Tiferet', meaning: 'Beauty', role: 'Visualization' },
        deployer: { sefira: 'Hod', meaning: 'Splendor', role: 'Deployment' },
      };

      // If specific agent requested
      if (agent && summary.agents[agent]) {
        const agentSummary = summary.agents[agent];
        const sefira = sefirotMap[agent];
        return {
          agent,
          sefira: sefira?.sefira,
          meaning: sefira?.meaning,
          role: sefira?.role,
          ...agentSummary,
          profileLevel: summary.profileLevel,
          timestamp: Date.now(),
        };
      }

      // Build dogs summary with Sefirot info
      const dogs = {};
      for (const [name, info] of Object.entries(sefirotMap)) {
        const agentData = summary.agents[name];
        dogs[name] = {
          sefira: info.sefira,
          meaning: info.meaning,
          role: info.role,
          active: !!agentData,
          ...(agentData ? {
            invocations: agentData.invocations || agentData.stats?.invocations || 0,
          } : {}),
        };
      }

      // Basic response
      const response = {
        status: 'active',
        dogCount: summary.dogCount,
        agentCount: summary.agentCount,
        profileLevel: summary.profileLevel,
        cynicState: collectiveState?.metaState || 'unknown',
        eventBusStats: summary.eventBusStats,
        dogs,
        collectiveStats: summary.collectiveStats,
        message: `*tail wag* Collective active. CYNIC at ${collectiveState?.metaState || 'unknown'} state. ${summary.agentCount} agents ready.`,
        timestamp: Date.now(),
      };

      // Add verbose details
      if (verbose) {
        response.agents = summary.agents;
        response.collectiveState = collectiveState;
      }

      return response;
    },
  };
}

/**
 * Create agent diagnostic tool - tests agent routing directly
 * @param {Object} agents - AgentManager instance
 * @returns {Object} Tool definition
 */
export function createAgentDiagnosticTool(collective) {
  return {
    name: 'brain_agent_diagnostic',
    description: 'INTERNAL: Test agent routing and shouldTrigger logic directly. For debugging only.',
    inputSchema: {
      type: 'object',
      properties: {
        eventType: { type: 'string', description: 'Event type to test (e.g., PostConversation)' },
        testContent: { type: 'string', description: 'Test content for the event' },
      },
    },
    handler: async (params) => {
      const { eventType = 'PostConversation', testContent = 'Test content' } = params;

      if (!collective) {
        return { error: 'Collective not available', timestamp: Date.now() };
      }

      // Create test event
      const testEvent = {
        type: eventType,
        content: testContent,
        tool: 'brain_agent_diagnostic',
        timestamp: Date.now(),
      };

      // Get all agents from collective and test shouldTrigger
      const agentResults = {};
      for (const agent of collective.agents) {
        const shouldTrigger = agent.shouldTrigger(testEvent);
        agentResults[agent.name] = {
          exists: true,
          sefirah: agent.sefirah,
          trigger: agent.trigger,
          shouldTriggerResult: shouldTrigger,
          invocations: agent.stats?.invocations || 0,
        };
      }

      const results = {
        testEvent,
        agents: agentResults,
        collectiveStats: collective.getStats?.() || {},
        timestamp: Date.now(),
      };

      // Test full pipeline via processEvent
      try {
        const processResult = await collective.processEvent(testEvent, {});
        results.processResult = processResult;
      } catch (e) {
        results.processError = e.message;
      }

      return results;
    },
  };
}

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
 * Create discovery tool definition
 * Discovers MCP servers, Claude Code plugins, and CYNIC nodes
 * @param {Object} discovery - DiscoveryService instance
 * @returns {Object} Tool definition
 */
export function createDiscoveryTool(discovery) {
  return {
    name: 'brain_discovery',
    description: `Discover MCP servers, Claude Code plugins, and CYNIC nodes from repositories or endpoints.
Actions:
- scan_repo: Scan a GitHub repo for .mcp.json and plugin.json
- mcp_servers: List discovered MCP servers
- plugins: List discovered plugins
- nodes: List discovered CYNIC nodes
- register_node: Register a new CYNIC node
- discover_node: Probe an endpoint to discover a node
- health_check: Run health checks on all nodes
- stats: Get discovery statistics`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['scan_repo', 'mcp_servers', 'plugins', 'nodes', 'register_node', 'discover_node', 'health_check', 'stats'],
          description: 'Action to perform',
        },
        owner: { type: 'string', description: 'GitHub repo owner (for scan_repo)' },
        repo: { type: 'string', description: 'GitHub repo name (for scan_repo)' },
        endpoint: { type: 'string', description: 'Node endpoint URL (for register_node, discover_node)' },
        nodeName: { type: 'string', description: 'Node name (for register_node)' },
        status: { type: 'string', description: 'Filter by status' },
        limit: { type: 'number', description: 'Max results (default 50)' },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const { action, owner, repo, endpoint, nodeName, status, limit = 50 } = params;

      if (!discovery) {
        return {
          error: 'Discovery service not available',
          hint: 'Ensure DiscoveryService is initialized',
          timestamp: Date.now(),
        };
      }

      switch (action) {
        case 'scan_repo': {
          if (!owner || !repo) {
            return { error: 'owner and repo required for scan_repo' };
          }
          const results = await discovery.scanRepo(owner, repo);
          return {
            action: 'scan_repo',
            ...results,
            message: `*sniff* Scanned ${owner}/${repo}: ${results.mcpServers.length} MCP servers, ${results.plugin ? 1 : 0} plugin.`,
            timestamp: Date.now(),
          };
        }

        case 'mcp_servers': {
          const servers = await discovery.getMcpServers({ status, limit });
          return {
            action: 'mcp_servers',
            servers,
            total: servers.length,
            message: `*ears perk* Found ${servers.length} MCP servers.`,
            timestamp: Date.now(),
          };
        }

        case 'plugins': {
          const plugins = await discovery.getPlugins({ status, limit });
          return {
            action: 'plugins',
            plugins,
            total: plugins.length,
            message: `*tail wag* Found ${plugins.length} plugins.`,
            timestamp: Date.now(),
          };
        }

        case 'nodes': {
          const nodes = await discovery.getNodes({ status, limit });
          return {
            action: 'nodes',
            nodes,
            total: nodes.length,
            message: `*nod* Found ${nodes.length} CYNIC nodes.`,
            timestamp: Date.now(),
          };
        }

        case 'register_node': {
          if (!endpoint) {
            return { error: 'endpoint required for register_node' };
          }
          const node = await discovery.registerNode({ endpoint, nodeName });
          return {
            action: 'register_node',
            node,
            message: `*tail wag* Registered node at ${endpoint}.`,
            timestamp: Date.now(),
          };
        }

        case 'discover_node': {
          if (!endpoint) {
            return { error: 'endpoint required for discover_node' };
          }
          const node = await discovery.discoverNode(endpoint);
          if (node) {
            return {
              action: 'discover_node',
              node,
              message: `*ears perk* Discovered node at ${endpoint}.`,
              timestamp: Date.now(),
            };
          }
          return {
            action: 'discover_node',
            error: 'Node not reachable or not a CYNIC node',
            endpoint,
            timestamp: Date.now(),
          };
        }

        case 'health_check': {
          const results = await discovery.runNodeHealthChecks();
          return {
            action: 'health_check',
            ...results,
            message: `*sniff* Health check: ${results.healthy}/${results.checked} healthy.`,
            timestamp: Date.now(),
          };
        }

        case 'stats': {
          const stats = await discovery.getStats();
          return {
            action: 'stats',
            stats,
            message: `*nod* ${stats.mcp_servers || 0} MCP servers, ${stats.plugins || 0} plugins, ${stats.nodes || 0} nodes.`,
            timestamp: Date.now(),
          };
        }

        default:
          return {
            error: `Unknown action: ${action}`,
            validActions: ['scan_repo', 'mcp_servers', 'plugins', 'nodes', 'register_node', 'discover_node', 'health_check', 'stats'],
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

