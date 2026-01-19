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
import { createMetaTool } from '../meta-dashboard.js';
import { createCodeAnalyzer } from '../code-analyzer.js';
import {
  createSearchIndexTool,
  createTimelineTool,
  createGetObservationsTool,
  createProgressiveSearchTools,
} from './search-progressive.js';

/**
 * Create judge tool definition
 * @param {Object} judge - CYNICJudge instance
 * @param {Object} [persistence] - PersistenceManager instance (for storing judgments)
 * @param {Object} [sessionManager] - SessionManager instance (for user/session context)
 * @param {Object} [pojChainManager] - PoJChainManager instance (for blockchain)
 * @param {Object} [graphIntegration] - JudgmentGraphIntegration instance (for graph edges)
 * @param {Function} [onJudgment] - Callback when judgment is completed (for SSE broadcast)
 * @returns {Object} Tool definition
 */
export function createJudgeTool(judge, persistence = null, sessionManager = null, pojChainManager = null, graphIntegration = null, onJudgment = null) {
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

      // Use graph integration if available, otherwise direct judge
      const judgment = graphIntegration
        ? await graphIntegration.judgeWithGraph(item, context)
        : judge.judge(item, context);

      // Generate fallback ID (used if persistence unavailable)
      let judgmentId = `jdg_${Date.now().toString(36)}`;

      // Get session context for user isolation
      const sessionContext = sessionManager?.getSessionContext() || {};

      // Store judgment in persistence FIRST to get the real DB ID
      if (persistence) {
        try {
          const stored = await persistence.storeJudgment({
            item,
            itemType: item.type || 'unknown',
            itemContent: typeof item.content === 'string' ? item.content : JSON.stringify(item),
            qScore: judgment.qScore,
            globalScore: judgment.global_score,
            confidence: Math.round(judgment.confidence * 1000) / 1000,
            verdict: judgment.qVerdict?.verdict || judgment.verdict,
            axiomScores: judgment.axiomScores,
            dimensionScores: judgment.dimensionScores || null,
            weaknesses: judgment.weaknesses,
            context,
            // Session context for multi-user isolation
            userId: sessionContext.userId || null,
            sessionId: sessionContext.sessionId || null,
          });

          // Use the DB-generated ID for consistency
          if (stored?.judgment_id) {
            judgmentId = stored.judgment_id;
          }

          // Add to PoJ chain (batched block creation)
          if (pojChainManager && stored) {
            await pojChainManager.addJudgment({
              judgment_id: stored.judgment_id,
              q_score: judgment.qScore,
              verdict: judgment.qVerdict?.verdict || judgment.verdict,
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

      // Build response with consistent ID
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

      // Include graph edge info if available
      if (judgment.graphEdge) {
        result.graphEdge = judgment.graphEdge;
      }

      // Emit event for SSE broadcast
      if (onJudgment) {
        try {
          onJudgment({
            judgmentId,
            qScore: result.score,
            verdict: result.verdict,
            confidence: result.confidence,
            itemType: item.type || 'unknown',
            timestamp: result.timestamp,
          });
        } catch (e) {
          // Non-blocking - don't fail judgment for callback errors
          console.error('Judgment callback error:', e.message);
        }
      }

      return result;
    },
  };
}

/**
 * Create self-refinement tool definition
 * @param {Object} judge - Judge instance
 * @param {Object} persistence - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createRefineTool(judge, persistence = null) {
  // Dynamic import to avoid circular dependencies
  let refinement = null;

  return {
    name: 'brain_cynic_refine',
    description: 'Self-refine a judgment by critiquing and re-judging. CYNIC critiques its own judgment, identifies weaknesses, and suggests improved scores. Returns original vs refined comparison.',
    inputSchema: {
      type: 'object',
      properties: {
        judgmentId: {
          type: 'string',
          description: 'ID of a previous judgment to refine (e.g., jdg_abc123)',
        },
        judgment: {
          type: 'object',
          description: 'Or provide judgment directly: { Q, breakdown: { PHI, VERIFY, CULTURE, BURN }, verdict }',
        },
        context: {
          type: 'object',
          description: 'Optional context for critique (source, type, etc.)',
        },
        maxIterations: {
          type: 'number',
          description: 'Max refinement iterations (default: 3)',
          default: 3,
        },
      },
    },
    handler: async (params) => {
      const { judgmentId, judgment: directJudgment, context = {}, maxIterations = 3 } = params;

      // Lazy load refinement module
      if (!refinement) {
        try {
          const core = await import('@cynic/core');
          refinement = {
            critiqueJudgment: core.critiqueJudgment,
            selfRefine: core.selfRefine,
            extractLearning: core.extractLearning,
          };
        } catch (e) {
          throw new Error(`Refinement module not available: ${e.message}`);
        }
      }

      let judgment = directJudgment;

      // If judgmentId provided, fetch from persistence
      if (judgmentId && !judgment) {
        if (!persistence) {
          throw new Error('Persistence required to fetch judgment by ID');
        }
        const stored = await persistence.getJudgment(judgmentId);
        if (!stored) {
          throw new Error(`Judgment not found: ${judgmentId}`);
        }
        judgment = {
          Q: stored.q_score,
          qScore: stored.q_score,
          verdict: stored.verdict,
          breakdown: stored.axiom_scores || stored.axiomScores,
          axiomScores: stored.axiom_scores || stored.axiomScores,
          confidence: stored.confidence,
        };
      }

      if (!judgment) {
        throw new Error('Must provide either judgmentId or judgment object');
      }

      // Normalize judgment format
      const normalizedJudgment = {
        Q: judgment.Q || judgment.qScore || judgment.q_score,
        qScore: judgment.Q || judgment.qScore || judgment.q_score,
        breakdown: judgment.breakdown || judgment.axiomScores || judgment.axiom_scores,
        verdict: judgment.verdict,
        confidence: judgment.confidence,
      };

      // Run self-refinement
      const result = refinement.selfRefine(normalizedJudgment, context, { maxIterations });

      // Extract learnings
      const learnings = refinement.extractLearning(result);

      // Store learnings if improved
      if (persistence && learnings.improved) {
        try {
          // Store as a pattern for future reference
          for (const learning of learnings.learnings) {
            await persistence.storePattern({
              type: 'refinement_learning',
              signature: `${learning.type}:${learning.axiom || learning.pattern}`,
              description: learning.pattern || learning.correction,
              count: 1,
            });
          }
        } catch (e) {
          // Non-blocking
          console.error('Error storing refinement learnings:', e.message);
        }
      }

      return {
        original: result.original,
        refined: result.final,
        improved: result.improved,
        improvement: result.totalImprovement,
        iterations: result.iterationCount,
        critiques: result.iterations.map(i => ({
          iteration: i.iteration,
          issues: i.critique?.critiques?.length || 0,
          severity: i.critique?.severity,
          adjustments: i.refinement?.adjustments?.length || 0,
        })),
        learnings: learnings.learnings,
        meta: {
          judgmentId: judgmentId || 'direct',
          maxIterations,
          timestamp: Date.now(),
        },
      };
    },
  };
}

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

        case 'task':
          if (!taskId) throw new Error('taskId required for task action');
          const task = orchestrator.getTask(taskId);
          if (!task) throw new Error(`Task not found: ${taskId}`);
          return task.toJSON();

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

/**
 * Create learning loop tool definition
 * @param {Object} [options] - Options
 * @param {Object} [options.persistence] - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createLearningTool(options = {}) {
  const { persistence } = options;

  // Lazy load learning loop
  let learningLoop = null;

  return {
    name: 'brain_learning',
    description: `Learning loop that improves CYNIC's judgments based on feedback.
Actions:
- feedback: Process feedback on a judgment (correct/incorrect/partial)
- calibrate: Trigger weight calibration from accumulated feedback
- biases: Detect systematic biases in judgments
- state: Get current learning state (weights, accuracy, biases)
- summary: Get learning summary
- reset: Reset learning state`,
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['feedback', 'calibrate', 'biases', 'state', 'summary', 'reset'],
          description: 'Action to perform',
        },
        judgmentId: {
          type: 'string',
          description: 'Judgment ID for feedback',
        },
        judgment: {
          type: 'object',
          description: 'Judgment object if not using ID',
        },
        outcome: {
          type: 'string',
          enum: ['correct', 'incorrect', 'partial'],
          description: 'Feedback outcome',
        },
        actualScore: {
          type: 'number',
          description: 'What the Q-Score should have been (0-100)',
        },
        reason: {
          type: 'string',
          description: 'Explanation for feedback',
        },
        context: {
          type: 'object',
          description: 'Additional context',
        },
      },
      required: ['action'],
    },
    handler: async (params) => {
      const {
        action,
        judgmentId,
        judgment: directJudgment,
        outcome,
        actualScore,
        reason,
        context = {},
      } = params;

      // Lazy load learning module
      if (!learningLoop) {
        try {
          const core = await import('@cynic/core');
          learningLoop = new core.LearningLoop({
            autoCalibrate: true,
            calibrateThreshold: 21, // Fib(8)
          });

          // Try to load existing state from persistence
          if (persistence) {
            try {
              const state = await persistence.getLearningState();
              if (state) {
                learningLoop.import(state);
              }
            } catch (e) {
              // No saved state, start fresh
            }
          }
        } catch (e) {
          throw new Error(`Learning module not available: ${e.message}`);
        }
      }

      switch (action) {
        case 'state':
          return learningLoop.getState();

        case 'summary':
          return learningLoop.getSummary();

        case 'reset':
          learningLoop.reset();
          return { reset: true, message: 'Learning state reset' };

        case 'calibrate': {
          const result = learningLoop.calibrate();

          // Save state if calibration occurred
          if (persistence && result.updated) {
            try {
              await persistence.saveLearningState(learningLoop.export());
            } catch (e) {
              // Non-blocking
            }
          }

          return result;
        }

        case 'biases': {
          return learningLoop.detectBiases();
        }

        case 'feedback': {
          if (!outcome) {
            throw new Error('outcome required for feedback action');
          }

          let judgment = directJudgment;

          // Fetch judgment by ID if needed
          if (judgmentId && !judgment && persistence) {
            try {
              const stored = await persistence.getJudgment(judgmentId);
              if (stored) {
                judgment = {
                  Q: stored.q_score,
                  qScore: stored.q_score,
                  verdict: stored.verdict,
                  breakdown: stored.axiom_scores,
                  confidence: stored.confidence,
                };
              }
            } catch (e) {
              // Continue without judgment
            }
          }

          const feedback = {
            judgment,
            outcome,
            actualScore,
            reason,
            context,
            judgmentId,
            timestamp: Date.now(),
          };

          const result = learningLoop.processFeedback(feedback);

          // Save state after feedback
          if (persistence) {
            try {
              await persistence.saveLearningState(learningLoop.export());
            } catch (e) {
              // Non-blocking
            }
          }

          return {
            action: 'feedback',
            outcome,
            processed: true,
            stats: result.stats,
            calibration: result.calibration,
            biases: result.biases?.biases?.length || 0,
          };
        }

        default:
          throw new Error(`Unknown action: ${action}`);
      }
    },
  };
}

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
              const state = await persistence.getTriggersState();
              if (state) {
                triggerManager.import(state);
              }
            } catch (e) {
              // No saved state, start fresh
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
            trigger: trigger.toJSON(),
          };
        }

        case 'unregister': {
          if (!triggerId) {
            throw new Error('triggerId required for unregister');
          }

          const removed = triggerManager.unregister(triggerId);

          // Save state
          if (persistence) {
            try {
              await persistence.saveTriggersState(triggerManager.export());
            } catch (e) {
              // Non-blocking
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

      // Basic response - include debug info in stats
      const statsWithDebug = {
        ...summary.stats,
        debugDigesterSelections: summary._debug?.digesterSelections || 0,
      };

      const response = {
        enabled: summary.enabled,
        stats: statsWithDebug,
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
          return {
            action: 'status',
            ...status,
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
export function createAgentDiagnosticTool(agents) {
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

      if (!agents) {
        return { error: 'Agents not available', timestamp: Date.now() };
      }

      // Create test event
      const testEvent = {
        type: eventType,
        content: testContent,
        tool: 'brain_agent_diagnostic',
        timestamp: Date.now(),
      };

      // Get the agents object directly
      const digester = agents.agents?.digester;
      const observer = agents.agents?.observer;

      // Test shouldTrigger directly
      const results = {
        testEvent,
        digester: digester ? {
          exists: true,
          trigger: digester.trigger,
          shouldTriggerResult: digester.shouldTrigger(testEvent),
          invocations: digester.stats?.invocations,
          _shouldTriggerCallCount: digester._shouldTriggerCallCount,
        } : { exists: false },
        observer: observer ? {
          exists: true,
          trigger: observer.trigger,
          shouldTriggerResult: observer.shouldTrigger(testEvent),
          invocations: observer.stats?.invocations,
        } : { exists: false },
        managerStats: agents.stats,
        timestamp: Date.now(),
      };

      // Also try directly calling agents.process to see what happens
      try {
        const processResult = await agents.process(testEvent, {});
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
 * Create all tools
 * @param {Object} options - Tool options
 * @param {Object} options.judge - CYNICJudge instance
 * @param {Object} [options.node] - CYNICNode instance
 * @param {Object} [options.persistence] - PersistenceManager instance (handles fallback)
 * @param {Object} [options.agents] - AgentManager instance (The Four Dogs)
 * @param {Object} [options.sessionManager] - SessionManager instance for multi-user sessions
 * @param {Object} [options.pojChainManager] - PoJChainManager instance for blockchain
 * @param {Object} [options.librarian] - LibrarianService instance for documentation caching
 * @param {Object} [options.ecosystem] - EcosystemService instance for pre-loaded docs
 * @param {Object} [options.integrator] - IntegratorService instance for cross-project sync
 * @param {Object} [options.metrics] - MetricsService instance for monitoring
 * @param {Object} [options.graphIntegration] - JudgmentGraphIntegration instance for graph edges
 * @param {Object} [options.codebaseOptions] - Options for code analyzer (rootPath, etc)
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
    onJudgment = null, // SSE broadcast callback
  } = options;

  if (!judge) throw new Error('judge is required');

  const tools = {};
  const toolDefs = [
    createJudgeTool(judge, persistence, sessionManager, pojChainManager, graphIntegration, onJudgment),
    createRefineTool(judge, persistence), // Self-refinement: critique → refine → learn
    createOrchestrationTool({ judge, agents, persistence }), // Multi-agent parallel execution
    createVectorSearchTool({ persistence }), // Semantic search with embeddings
    createLearningTool({ persistence }), // Learning loop: feedback → calibration → improvement
    createTriggersTool({ judge, persistence }), // Auto-judgment triggers
    createDigestTool(persistence, sessionManager),
    createHealthTool(node, judge, persistence),
    createSearchTool(persistence),
    // Progressive Search Tools (3-layer retrieval for 10x token savings)
    createSearchIndexTool(persistence),
    createTimelineTool(persistence),
    createGetObservationsTool(persistence),
    createPatternsTool(judge, persistence),
    createFeedbackTool(persistence, sessionManager),
    createAgentsStatusTool(agents),
    createCollectiveStatusTool(collective), // NEW: The Five Dogs + CYNIC (Keter)
    createAgentDiagnosticTool(agents),
    createSessionStartTool(sessionManager),
    createSessionEndTool(sessionManager),
    createDocsTool(librarian, persistence),
    createEcosystemTool(ecosystem),
    createPoJChainTool(pojChainManager, persistence),
    createIntegratorTool(integrator),
    createMetricsTool(metrics),
    createMetaTool(), // CYNIC self-analysis dashboard
    createCodebaseTool(codebaseOptions), // Code structure analyzer
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
  createDocsTool,
  createEcosystemTool,
  createPoJChainTool,
  createIntegratorTool,
  createMetricsTool,
  createMetaTool,
  createCodebaseTool,
  createAllTools,
};
