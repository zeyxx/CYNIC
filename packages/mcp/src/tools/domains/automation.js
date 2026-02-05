/**
 * Automation Domain Tools
 *
 * Tools for automation and orchestration:
 * - Triggers: Event-driven automation
 * - Orchestration: Multi-tool orchestration
 *
 * @module @cynic/mcp/tools/domains/automation
 */

'use strict';

import { createLogger } from '@cynic/core';

const log = createLogger('AutomationTools');

// P4: createOrchestrationTool REMOVED — dead code, superseded by brain_orchestrate in orchestration.js
// Was: brain_orchestrate (duplicate name, never registered in toolDefs)
function _dead_createOrchestrationTool(options = {}) {
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
              log.info('CYNIC alert', alert);
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
                    log.warn('Failed to load trigger', { triggerId: t.triggerId, error: regErr.message });
                  }
                }
                log.info('Loaded triggers from PostgreSQL', { count: dbTriggers.length });
              } else {
                // Fall back to legacy state (file/memory)
                const state = await persistence.getTriggersState();
                if (state) {
                  triggerManager.import(state);
                  log.info('Loaded triggers from legacy state');
                }
              }
            } catch (e) {
              // No saved state, start fresh
              log.debug('Triggers starting fresh', { error: e.message });
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
 * Factory for automation domain tools
 */
export const automationFactory = {
  name: 'automation',
  domain: 'automation',
  requires: [],

  /**
   * Create all automation domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const { judge, agents, persistence, collective, scheduler } = options;

    const tools = [];

    // P4: createOrchestrationTool removed — use brain_orchestrate from orchestration.js

    // Triggers tool
    tools.push(createTriggersTool({ judge, persistence, collective, scheduler }));

    return tools;
  },
};
