/**
 * Memory Domain Tools
 *
 * MCP tools for Total Memory system:
 * - brain_memory_search: Hybrid search (FTS + vector)
 * - brain_memory_store: Store memories, decisions, lessons
 * - brain_memory_stats: Memory statistics
 * - brain_goals: Manage autonomous goals
 * - brain_notifications: Proactive notifications
 *
 * "φ remembers everything" - CYNIC
 *
 * @module @cynic/mcp/tools/domains/memory
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('MemoryTools');

// φ constants
const PHI_FTS = 0.382;
const PHI_VECTOR = 0.618;

/**
 * Create memory search tool
 * @param {Object} memoryRetriever - MemoryRetriever instance
 * @returns {Object} Tool definition
 */
export function createMemorySearchTool(memoryRetriever) {
  // Track embedder type for warnings
  let embedderType = null;
  let embedderChecked = false;

  return {
    name: 'brain_memory_search',
    description: 'Search CYNIC\'s Total Memory using φ-weighted hybrid search (FTS + vector). Returns memories, decisions, and lessons learned.',
    inputSchema: {
      type: 'object',
      properties: {
        query: {
          type: 'string',
          description: 'Natural language search query',
        },
        userId: {
          type: 'string',
          description: 'User ID for scoped search (defaults to current session user)',
        },
        limit: {
          type: 'number',
          description: 'Maximum results per source (default: 5)',
        },
        sources: {
          type: 'array',
          items: { type: 'string', enum: ['memories', 'decisions', 'lessons'] },
          description: 'Which memory sources to search (default: all)',
        },
        minRelevance: {
          type: 'number',
          description: 'Minimum relevance score 0-1 (default: 0.3)',
        },
      },
      required: ['query'],
    },
    handler: async (params, context) => {
      const {
        query,
        userId = context?.userId || 'default',
        limit = 5,
        sources = ['memories', 'decisions', 'lessons'],
        minRelevance = 0.3,
      } = params;

      if (!query) {
        throw new Error('Missing required parameter: query');
      }

      if (!memoryRetriever) {
        return {
          success: false,
          error: 'MemoryRetriever not available',
          timestamp: Date.now(),
        };
      }

      // Check embedder type once
      if (!embedderChecked) {
        try {
          const { getEmbedder } = await import('@cynic/persistence');
          const embedder = getEmbedder();
          if (embedder._detect) await embedder._detect();
          embedderType = embedder.type;
        } catch {
          embedderType = 'unknown';
        }
        embedderChecked = true;
      }

      try {
        const results = await memoryRetriever.search(userId, query, {
          limit,
          sources,
          minRelevance,
        });

        const totalResults = Object.values(results.sources || {}).flat().length;

        // Warning for mock embeddings
        const mockWarning = embedderType === 'mock'
          ? '⚠️ Using mock embeddings - vector similarity is approximated. Install Ollama for semantic search: https://ollama.ai'
          : null;

        return {
          success: true,
          query,
          userId,
          sources: results.sources,
          totalResults,
          searchParams: {
            phiFTS: PHI_FTS,
            phiVector: PHI_VECTOR,
            minRelevance,
          },
          embedderType,
          warning: mockWarning,
          message: totalResults > 0
            ? `*sniff* Found ${totalResults} relevant memories.`
            : '*head tilt* No matching memories found.',
          timestamp: Date.now(),
        };
      } catch (error) {
        log.error('Memory search failed', { error: error.message });
        return {
          success: false,
          error: error.message,
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * Create memory store tool
 * @param {Object} memoryRetriever - MemoryRetriever instance
 * @returns {Object} Tool definition
 */
export function createMemoryStoreTool(memoryRetriever) {
  return {
    name: 'brain_memory_store',
    description: 'Store a memory, decision, or lesson in CYNIC\'s Total Memory system.',
    inputSchema: {
      type: 'object',
      properties: {
        type: {
          type: 'string',
          enum: ['memory', 'decision', 'lesson'],
          description: 'Type of content to store',
        },
        userId: {
          type: 'string',
          description: 'User ID (defaults to current session user)',
        },
        // Memory fields
        memoryType: {
          type: 'string',
          enum: ['summary', 'key_moment', 'decision', 'preference'],
          description: 'For memories: type of memory',
        },
        content: {
          type: 'string',
          description: 'For memories: the content to remember',
        },
        importance: {
          type: 'number',
          description: 'For memories: importance score 0-1 (default: 0.5)',
        },
        // Decision fields
        projectPath: {
          type: 'string',
          description: 'For decisions: project path',
        },
        decisionType: {
          type: 'string',
          enum: ['pattern', 'technology', 'structure', 'naming', 'other'],
          description: 'For decisions: type of decision',
        },
        title: {
          type: 'string',
          description: 'For decisions/lessons: title',
        },
        description: {
          type: 'string',
          description: 'For decisions: detailed description',
        },
        rationale: {
          type: 'string',
          description: 'For decisions: why this choice was made',
        },
        alternatives: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              option: { type: 'string' },
              reason_rejected: { type: 'string' },
            },
          },
          description: 'For decisions: alternatives considered',
        },
        // Lesson fields
        category: {
          type: 'string',
          enum: ['bug', 'architecture', 'process', 'communication', 'other'],
          description: 'For lessons: category',
        },
        mistake: {
          type: 'string',
          description: 'For lessons: what went wrong',
        },
        correction: {
          type: 'string',
          description: 'For lessons: how it was fixed',
        },
        prevention: {
          type: 'string',
          description: 'For lessons: how to prevent in future',
        },
        severity: {
          type: 'string',
          enum: ['low', 'medium', 'high', 'critical'],
          description: 'For lessons: severity level',
        },
        // Common fields
        context: {
          type: 'object',
          description: 'Additional context metadata',
        },
      },
      required: ['type'],
    },
    handler: async (params, context) => {
      const {
        type,
        userId = context?.userId || 'default',
      } = params;

      if (!memoryRetriever) {
        return {
          success: false,
          error: 'MemoryRetriever not available',
          timestamp: Date.now(),
        };
      }

      try {
        let result;

        switch (type) {
          case 'memory': {
            const { memoryType = 'summary', content, importance = 0.5, context: ctx } = params;
            if (!content) throw new Error('content required for memory');

            result = await memoryRetriever.rememberConversation(userId, memoryType, content, {
              importance,
              context: ctx,
            });
            break;
          }

          case 'decision': {
            const {
              projectPath,
              decisionType = 'other',
              title,
              description,
              rationale,
              alternatives = [],
              context: ctx,
            } = params;
            if (!title || !description) throw new Error('title and description required for decision');

            result = await memoryRetriever.rememberDecision(userId, {
              projectPath,
              decisionType,
              title,
              description,
              rationale,
              alternatives,
              consequences: ctx?.consequences || {},
            });
            break;
          }

          case 'lesson': {
            const {
              category = 'other',
              mistake,
              correction,
              prevention,
              severity = 'medium',
            } = params;
            if (!mistake || !correction) throw new Error('mistake and correction required for lesson');

            result = await memoryRetriever.rememberLesson(userId, {
              category,
              mistake,
              correction,
              prevention,
              severity,
            });
            break;
          }

          default:
            throw new Error(`Unknown type: ${type}`);
        }

        return {
          success: true,
          type,
          id: result.id,
          message: `*tail wag* ${type} stored successfully.`,
          timestamp: Date.now(),
        };
      } catch (error) {
        log.error('Memory store failed', { error: error.message });
        return {
          success: false,
          error: error.message,
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * Create memory stats tool
 * @param {Object} memoryRetriever - MemoryRetriever instance
 * @returns {Object} Tool definition
 */
export function createMemoryStatsTool(memoryRetriever) {
  return {
    name: 'brain_memory_stats',
    description: 'Get statistics about CYNIC\'s Total Memory for a user.',
    inputSchema: {
      type: 'object',
      properties: {
        userId: {
          type: 'string',
          description: 'User ID (defaults to current session user)',
        },
      },
    },
    handler: async (params, context) => {
      const userId = params.userId || context?.userId || 'default';

      if (!memoryRetriever) {
        return {
          success: false,
          error: 'MemoryRetriever not available',
          timestamp: Date.now(),
        };
      }

      try {
        const stats = await memoryRetriever.getStats(userId);

        return {
          success: true,
          userId,
          stats: {
            memories: stats.totals.memories,
            decisions: stats.totals.decisions,
            lessons: stats.totals.lessons,
            total: stats.totals.combined,
          },
          breakdown: stats.breakdown,
          message: `*sniff* User has ${stats.totals.combined} total memories.`,
          timestamp: Date.now(),
        };
      } catch (error) {
        log.error('Memory stats failed', { error: error.message });
        return {
          success: false,
          error: error.message,
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * Create self-correction check tool
 * @param {Object} memoryRetriever - MemoryRetriever instance
 * @returns {Object} Tool definition
 */
export function createSelfCorrectionTool(memoryRetriever) {
  return {
    name: 'brain_self_correction',
    description: 'Check if a proposed action matches any past mistakes/lessons learned. CYNIC warns before repeating history.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          description: 'Description of the action being considered',
        },
        userId: {
          type: 'string',
          description: 'User ID (defaults to current session user)',
        },
      },
      required: ['action'],
    },
    handler: async (params, context) => {
      const {
        action,
        userId = context?.userId || 'default',
      } = params;

      if (!action) {
        throw new Error('Missing required parameter: action');
      }

      if (!memoryRetriever) {
        return {
          success: false,
          error: 'MemoryRetriever not available',
          timestamp: Date.now(),
        };
      }

      try {
        const check = await memoryRetriever.checkForMistakes(userId, action);

        return {
          success: true,
          action,
          warning: check.warning,
          severity: check.severity,
          similarLessons: check.lessons || [],
          message: check.warning
            ? `*GROWL* ${check.message}`
            : '*tail wag* No similar mistakes found. Proceed.',
          prevention: check.prevention,
          timestamp: Date.now(),
        };
      } catch (error) {
        log.error('Self-correction check failed', { error: error.message });
        return {
          success: false,
          error: error.message,
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * Create goals management tool
 * @param {Object} goalsRepo - AutonomousGoalsRepository instance
 * @returns {Object} Tool definition
 */
export function createGoalsTool(goalsRepo) {
  return {
    name: 'brain_goals',
    description: 'Manage CYNIC\'s autonomous goals. Create, update progress, list active goals.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['list', 'create', 'update', 'complete'],
          description: 'Action to perform',
        },
        userId: {
          type: 'string',
          description: 'User ID (defaults to current session user)',
        },
        // For create
        goalType: {
          type: 'string',
          enum: ['quality', 'learning', 'maintenance', 'monitoring'],
          description: 'Type of goal',
        },
        title: {
          type: 'string',
          description: 'Goal title',
        },
        description: {
          type: 'string',
          description: 'Goal description',
        },
        successCriteria: {
          type: 'array',
          items: {
            type: 'object',
            properties: {
              criterion: { type: 'string' },
              weight: { type: 'number' },
            },
          },
          description: 'Success criteria with weights',
        },
        priority: {
          type: 'number',
          description: 'Priority 0-100 (default: 50)',
        },
        // For update
        goalId: {
          type: 'string',
          description: 'Goal ID (for update/complete)',
        },
        progress: {
          type: 'number',
          description: 'Progress 0-1 (for update)',
        },
        criterionIndex: {
          type: 'number',
          description: 'Index of criterion to mark complete',
        },
      },
      required: ['action'],
    },
    handler: async (params, context) => {
      const {
        action,
        userId = context?.userId || 'default',
      } = params;

      if (!goalsRepo) {
        return {
          success: false,
          error: 'GoalsRepository not available',
          timestamp: Date.now(),
        };
      }

      try {
        switch (action) {
          case 'list': {
            const goals = await goalsRepo.findActive(userId);
            return {
              success: true,
              action: 'list',
              goals: goals.map(g => ({
                id: g.id,
                type: g.goalType,
                title: g.title,
                progress: Math.round(g.progress * 100),
                priority: g.priority,
              })),
              message: `*sniff* Found ${goals.length} active goals.`,
              timestamp: Date.now(),
            };
          }

          case 'create': {
            const { goalType, title, description, successCriteria = [], priority = 50 } = params;
            if (!title) throw new Error('title required for create');

            const goal = await goalsRepo.create({
              userId,
              goalType: goalType || 'learning',
              title,
              description,
              successCriteria: successCriteria.map(c => ({
                criterion: c.criterion,
                weight: c.weight || 1,
                met: false,
              })),
              priority,
            });

            return {
              success: true,
              action: 'create',
              goal: {
                id: goal.id,
                title: goal.title,
                progress: 0,
              },
              message: `*tail wag* Goal "${title}" created.`,
              timestamp: Date.now(),
            };
          }

          case 'update': {
            const { goalId, progress, criterionIndex } = params;
            if (!goalId) throw new Error('goalId required for update');

            let goal;
            if (typeof criterionIndex === 'number') {
              goal = await goalsRepo.markCriterionMet(goalId, criterionIndex);
            } else if (typeof progress === 'number') {
              goal = await goalsRepo.updateProgress(goalId, progress);
            } else {
              throw new Error('progress or criterionIndex required for update');
            }

            return {
              success: true,
              action: 'update',
              goal: {
                id: goal.id,
                title: goal.title,
                progress: Math.round(goal.progress * 100),
                status: goal.status,
              },
              message: `*tail wag* Goal progress: ${Math.round(goal.progress * 100)}%`,
              timestamp: Date.now(),
            };
          }

          case 'complete': {
            const { goalId } = params;
            if (!goalId) throw new Error('goalId required for complete');

            const goal = await goalsRepo.complete(goalId);

            return {
              success: true,
              action: 'complete',
              goal: {
                id: goal.id,
                title: goal.title,
                completedAt: goal.completedAt,
              },
              message: `*celebration howl* Goal "${goal.title}" completed!`,
              timestamp: Date.now(),
            };
          }

          default:
            throw new Error(`Unknown action: ${action}`);
        }
      } catch (error) {
        log.error('Goals operation failed', { error: error.message });
        return {
          success: false,
          error: error.message,
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * Create notifications tool
 * @param {Object} notificationsRepo - ProactiveNotificationsRepository instance
 * @returns {Object} Tool definition
 */
export function createNotificationsTool(notificationsRepo) {
  return {
    name: 'brain_notifications',
    description: 'Manage CYNIC\'s proactive notifications. These are delivered at session start.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['list', 'create', 'acknowledge'],
          description: 'Action to perform',
        },
        userId: {
          type: 'string',
          description: 'User ID (defaults to current session user)',
        },
        // For create
        notificationType: {
          type: 'string',
          enum: ['insight', 'warning', 'reminder', 'achievement'],
          description: 'Type of notification',
        },
        title: {
          type: 'string',
          description: 'Notification title',
        },
        message: {
          type: 'string',
          description: 'Notification message',
        },
        priority: {
          type: 'number',
          description: 'Priority 0-100 (default: 50)',
        },
        expiresInHours: {
          type: 'number',
          description: 'Hours until expiration (optional)',
        },
        // For acknowledge
        notificationId: {
          type: 'string',
          description: 'Notification ID (for acknowledge)',
        },
      },
      required: ['action'],
    },
    handler: async (params, context) => {
      const {
        action,
        userId = context?.userId || 'default',
      } = params;

      if (!notificationsRepo) {
        return {
          success: false,
          error: 'NotificationsRepository not available',
          timestamp: Date.now(),
        };
      }

      try {
        switch (action) {
          case 'list': {
            const notifications = await notificationsRepo.getPending(userId, 10);
            return {
              success: true,
              action: 'list',
              notifications: notifications.map(n => ({
                id: n.id,
                type: n.notificationType,
                title: n.title,
                message: n.message,
                priority: n.priority,
                createdAt: n.createdAt,
              })),
              message: `*sniff* ${notifications.length} pending notifications.`,
              timestamp: Date.now(),
            };
          }

          case 'create': {
            const {
              notificationType = 'insight',
              title,
              message: msg,
              priority = 50,
              expiresInHours,
              context: ctx,
            } = params;
            if (!title || !msg) throw new Error('title and message required for create');

            const notification = await notificationsRepo.create({
              userId,
              notificationType,
              title,
              message: msg,
              priority,
              context: ctx,
              expiresAt: expiresInHours
                ? new Date(Date.now() + expiresInHours * 60 * 60 * 1000)
                : null,
            });

            return {
              success: true,
              action: 'create',
              notification: {
                id: notification.id,
                title: notification.title,
              },
              message: `*tail wag* Notification scheduled for next session.`,
              timestamp: Date.now(),
            };
          }

          case 'acknowledge': {
            const { notificationId } = params;
            if (!notificationId) throw new Error('notificationId required for acknowledge');

            const notification = await notificationsRepo.markDelivered(notificationId);

            return {
              success: true,
              action: 'acknowledge',
              notification: {
                id: notification.id,
                title: notification.title,
                deliveredAt: notification.deliveredAt,
              },
              message: `*nod* Notification acknowledged.`,
              timestamp: Date.now(),
            };
          }

          default:
            throw new Error(`Unknown action: ${action}`);
        }
      } catch (error) {
        log.error('Notifications operation failed', { error: error.message });
        return {
          success: false,
          error: error.message,
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * Create tasks management tool
 * @param {Object} tasksRepo - AutonomousTasksRepository instance
 * @returns {Object} Tool definition
 */
export function createTasksTool(tasksRepo) {
  return {
    name: 'brain_tasks',
    description: 'Manage CYNIC\'s autonomous background tasks. List pending, running, or completed tasks.',
    inputSchema: {
      type: 'object',
      properties: {
        action: {
          type: 'string',
          enum: ['list', 'status', 'retry', 'cancel'],
          description: 'Action to perform',
        },
        userId: {
          type: 'string',
          description: 'User ID (defaults to current session user)',
        },
        status: {
          type: 'string',
          enum: ['pending', 'running', 'completed', 'failed', 'all'],
          description: 'Filter by status (for list)',
        },
        taskId: {
          type: 'string',
          description: 'Task ID (for status/retry/cancel)',
        },
        limit: {
          type: 'number',
          description: 'Maximum tasks to return (default: 20)',
        },
      },
      required: ['action'],
    },
    handler: async (params, context) => {
      const {
        action,
        userId = context?.userId || 'default',
        status = 'all',
        taskId,
        limit = 20,
      } = params;

      if (!tasksRepo) {
        return {
          success: false,
          error: 'TasksRepository not available',
          timestamp: Date.now(),
        };
      }

      try {
        switch (action) {
          case 'list': {
            let tasks;
            if (status === 'all') {
              tasks = await tasksRepo.findByUser(userId, { limit });
            } else {
              tasks = await tasksRepo.findByStatus(status, { userId, limit });
            }

            return {
              success: true,
              action: 'list',
              tasks: tasks.map(t => ({
                id: t.id,
                type: t.taskType,
                status: t.status,
                priority: t.priority,
                scheduledFor: t.scheduledFor,
                createdAt: t.createdAt,
                retryCount: t.retryCount,
              })),
              message: `*sniff* Found ${tasks.length} tasks.`,
              timestamp: Date.now(),
            };
          }

          case 'status': {
            if (!taskId) throw new Error('taskId required for status');

            const task = await tasksRepo.findById(taskId);
            if (!task) {
              return {
                success: false,
                error: 'Task not found',
                timestamp: Date.now(),
              };
            }

            return {
              success: true,
              action: 'status',
              task: {
                id: task.id,
                type: task.taskType,
                status: task.status,
                priority: task.priority,
                payload: task.payload,
                result: task.result,
                error: task.errorMessage,
                scheduledFor: task.scheduledFor,
                startedAt: task.startedAt,
                completedAt: task.completedAt,
                retryCount: task.retryCount,
              },
              timestamp: Date.now(),
            };
          }

          case 'retry': {
            if (!taskId) throw new Error('taskId required for retry');

            const task = await tasksRepo.markForRetry(taskId);

            return {
              success: true,
              action: 'retry',
              task: {
                id: task.id,
                status: task.status,
                retryCount: task.retryCount,
              },
              message: `*tail wag* Task marked for retry.`,
              timestamp: Date.now(),
            };
          }

          case 'cancel': {
            if (!taskId) throw new Error('taskId required for cancel');

            const task = await tasksRepo.updateStatus(taskId, 'cancelled');

            return {
              success: true,
              action: 'cancel',
              task: {
                id: task.id,
                status: task.status,
              },
              message: `*nod* Task cancelled.`,
              timestamp: Date.now(),
            };
          }

          default:
            throw new Error(`Unknown action: ${action}`);
        }
      } catch (error) {
        log.error('Tasks operation failed', { error: error.message });
        return {
          success: false,
          error: error.message,
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * Factory for memory domain tools
 */
export const memoryFactory = {
  name: 'memory',
  domain: 'memory',
  requires: ['persistence'],

  /**
   * Create all memory domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const { memoryRetriever, goalsRepo, tasksRepo, notificationsRepo } = options;

    const tools = [];

    // Search tool
    if (memoryRetriever) {
      tools.push(createMemorySearchTool(memoryRetriever));
      tools.push(createMemoryStoreTool(memoryRetriever));
      tools.push(createMemoryStatsTool(memoryRetriever));
      tools.push(createSelfCorrectionTool(memoryRetriever));
    }

    // Goals tool
    if (goalsRepo) {
      tools.push(createGoalsTool(goalsRepo));
    }

    // Tasks tool
    if (tasksRepo) {
      tools.push(createTasksTool(tasksRepo));
    }

    // Notifications tool
    if (notificationsRepo) {
      tools.push(createNotificationsTool(notificationsRepo));
    }

    return tools;
  },
};

export default memoryFactory;
