/**
 * Orchestration Domain - KETER (Conscience Centrale)
 *
 * "φ distrusts φ" - L'orchestrateur route vers les bons Sefirot
 *
 * brain_orchestrate = Point d'entrée unique du MCP
 * - Reçoit tous les événements
 * - Charge profil utilisateur (E-Score 7D)
 * - Route vers le bon Sefirah
 * - Adapte intervention selon contexte
 *
 * Phase 19: Now uses UnifiedOrchestrator internally for
 * coordinated decision-making across all layers.
 *
 * @module @cynic/mcp/tools/domains/orchestration
 */

'use strict';

import { PHI, PHI_INV, THRESHOLDS, createLogger } from '@cynic/core';
import {
  UnifiedOrchestrator,
  createUnifiedOrchestrator,
  getOrchestrator,
} from '@cynic/node/orchestration/unified-orchestrator.js';
import {
  DecisionEvent,
  EventSource,
  DecisionOutcome,
} from '@cynic/node/orchestration/decision-event.js';
import { createDecisionTracer } from '@cynic/node/orchestration/decision-tracer.js';
import { createSkillRegistry } from '@cynic/node/orchestration/skill-registry.js';

const log = createLogger('OrchestrationTools');

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Event types that trigger orchestration
 */
export const EVENT_TYPES = {
  USER_PROMPT: 'user_prompt',
  TOOL_USE: 'tool_use',
  SESSION_START: 'session_start',
  SESSION_END: 'session_end',
  FILE_CHANGE: 'file_change',
  ERROR: 'error',
  JUDGMENT_REQUEST: 'judgment_request',
};

/**
 * Intervention levels (adaptive based on user E-Score and context)
 */
export const INTERVENTION_LEVELS = {
  SILENT: 'silent',       // No intervention, just observe
  NOTIFY: 'notify',       // Inform user but don't block
  ASK: 'ask',             // Ask for confirmation
  BLOCK: 'block',         // Block action, require explicit permission
};

/**
 * Sefirot routing - which agent/tool handles which domain
 */
export const SEFIROT_ROUTING = {
  // Chochmah (Sage) - Wisdom queries
  wisdom: {
    sefirah: 'Chochmah',
    agent: 'cynic-sage',
    tools: ['brain_search', 'brain_wisdom'],
    triggers: ['wisdom', 'philosophy', 'why', 'meaning', 'understanding'],
  },
  // Binah (Architect) - Design and planning
  design: {
    sefirah: 'Binah',
    agent: 'cynic-architect',
    tools: ['brain_patterns'],
    triggers: ['design', 'architect', 'plan', 'structure', 'refactor'],
  },
  // Daat (Archivist) - Memory and learning
  memory: {
    sefirah: 'Daat',
    agent: 'cynic-archivist',
    tools: ['brain_learning', 'brain_search', 'brain_patterns'],
    triggers: ['remember', 'learn', 'recall', 'history', 'past', 'before', 'already', 'déjà', 'souviens'],
  },
  // Chesed (Analyst) - Pattern analysis
  analysis: {
    sefirah: 'Chesed',
    agent: 'cynic-analyst',
    tools: ['brain_patterns', 'brain_emergence'],
    triggers: ['analyze', 'pattern', 'trend', 'insight', 'data'],
  },
  // Gevurah (Guardian) - Protection and verification
  protection: {
    sefirah: 'Gevurah',
    agent: 'cynic-guardian',
    tools: ['brain_cynic_judge'],
    triggers: ['danger', 'secure', 'verify', 'protect', 'risk', 'delete', 'rm'],
  },
  // Tiferet (Oracle) - Visualization and insights
  visualization: {
    sefirah: 'Tiferet',
    agent: 'cynic-oracle',
    tools: ['brain_render'],
    triggers: ['dashboard', 'visualize', 'show', 'display', 'status'],
  },
  // Netzach (Scout) - Exploration
  exploration: {
    sefirah: 'Netzach',
    agent: 'cynic-scout',
    tools: ['brain_code_analyze', 'brain_code_deps'],
    triggers: ['find', 'search', 'explore', 'where', 'locate'],
  },
  // Yesod (Janitor) - Cleanup and simplification
  cleanup: {
    sefirah: 'Yesod',
    agent: 'cynic-simplifier',
    tools: [],
    triggers: ['simplify', 'clean', 'reduce', 'remove', 'prune'],
  },
  // Hod (Deployer) - Deployment and infrastructure
  deployment: {
    sefirah: 'Hod',
    agent: 'cynic-deployer',
    tools: ['brain_ecosystem_monitor'],
    triggers: ['deploy', 'release', 'build', 'ci', 'cd', 'infrastructure'],
  },
  // Malkhut (Cartographer) - Mapping reality
  mapping: {
    sefirah: 'Malkhut',
    agent: 'cynic-cartographer',
    tools: ['brain_ecosystem_monitor'],
    triggers: ['map', 'overview', 'codebase', 'structure', 'repos'],
  },
};

/**
 * Trust level thresholds from E-Score 7D
 */
export const TRUST_THRESHOLDS = {
  GUARDIAN: PHI_INV * 100,       // 61.8% - Auto-approve most actions
  STEWARD: PHI_INV ** 2 * 100,   // 38.2% - Notify on significant actions
  BUILDER: 30,                    // Ask on potentially dangerous actions
  CONTRIBUTOR: 15,                // Ask on most actions
  OBSERVER: 0,                    // Block dangerous, ask on others
};

// ═══════════════════════════════════════════════════════════════════════════
// ORCHESTRATOR LOGIC
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Determine which Sefirah should handle the request
 *
 * @param {string} content - The prompt or event content
 * @param {string} eventType - Type of event
 * @returns {Object|null} Sefirot routing info or null if no specific handler
 */
export function routeToSefirah(content, eventType) {
  const lowerContent = content.toLowerCase();

  // Check each sefirah's triggers
  for (const [domain, routing] of Object.entries(SEFIROT_ROUTING)) {
    for (const trigger of routing.triggers) {
      if (lowerContent.includes(trigger)) {
        return { domain, ...routing };
      }
    }
  }

  // Default based on event type
  switch (eventType) {
    case EVENT_TYPES.JUDGMENT_REQUEST:
      return { domain: 'protection', ...SEFIROT_ROUTING.protection };
    case EVENT_TYPES.ERROR:
      return { domain: 'analysis', ...SEFIROT_ROUTING.analysis };
    case EVENT_TYPES.FILE_CHANGE:
      return { domain: 'mapping', ...SEFIROT_ROUTING.mapping };
    default:
      return null;
  }
}

/**
 * Determine intervention level based on user E-Score and action risk
 *
 * @param {number} eScore - User's E-Score (0-100)
 * @param {string} actionRisk - 'low' | 'medium' | 'high' | 'critical'
 * @returns {string} Intervention level
 */
export function determineIntervention(eScore, actionRisk) {
  // Risk matrix based on E-Score trust level and action risk
  const matrix = {
    critical: {
      GUARDIAN: INTERVENTION_LEVELS.ASK,
      STEWARD: INTERVENTION_LEVELS.ASK,
      BUILDER: INTERVENTION_LEVELS.BLOCK,
      CONTRIBUTOR: INTERVENTION_LEVELS.BLOCK,
      OBSERVER: INTERVENTION_LEVELS.BLOCK,
    },
    high: {
      GUARDIAN: INTERVENTION_LEVELS.NOTIFY,
      STEWARD: INTERVENTION_LEVELS.ASK,
      BUILDER: INTERVENTION_LEVELS.ASK,
      CONTRIBUTOR: INTERVENTION_LEVELS.BLOCK,
      OBSERVER: INTERVENTION_LEVELS.BLOCK,
    },
    medium: {
      GUARDIAN: INTERVENTION_LEVELS.SILENT,
      STEWARD: INTERVENTION_LEVELS.NOTIFY,
      BUILDER: INTERVENTION_LEVELS.NOTIFY,
      CONTRIBUTOR: INTERVENTION_LEVELS.ASK,
      OBSERVER: INTERVENTION_LEVELS.ASK,
    },
    low: {
      GUARDIAN: INTERVENTION_LEVELS.SILENT,
      STEWARD: INTERVENTION_LEVELS.SILENT,
      BUILDER: INTERVENTION_LEVELS.SILENT,
      CONTRIBUTOR: INTERVENTION_LEVELS.NOTIFY,
      OBSERVER: INTERVENTION_LEVELS.NOTIFY,
    },
  };

  // Determine trust level from E-Score
  let trustLevel = 'OBSERVER';
  if (eScore >= TRUST_THRESHOLDS.GUARDIAN) trustLevel = 'GUARDIAN';
  else if (eScore >= TRUST_THRESHOLDS.STEWARD) trustLevel = 'STEWARD';
  else if (eScore >= TRUST_THRESHOLDS.BUILDER) trustLevel = 'BUILDER';
  else if (eScore >= TRUST_THRESHOLDS.CONTRIBUTOR) trustLevel = 'CONTRIBUTOR';

  return matrix[actionRisk]?.[trustLevel] || INTERVENTION_LEVELS.ASK;
}

/**
 * Detect action risk level from content
 *
 * @param {string} content - The prompt or action content
 * @returns {string} Risk level: 'low' | 'medium' | 'high' | 'critical'
 */
export function detectActionRisk(content) {
  const lowerContent = content.toLowerCase();

  // Critical risk patterns (irreversible, destructive)
  const criticalPatterns = [
    'rm -rf', 'drop database', 'delete all', 'format',
    'reset --hard', 'force push', 'push --force',
    'truncate', 'destroy', 'wipe',
  ];

  // High risk patterns (significant changes)
  const highPatterns = [
    'delete', 'remove', 'drop', 'reset', 'revert',
    'migrate', 'deploy prod', 'production', 'main branch',
    'master branch', 'secret', 'credential', 'api key',
  ];

  // Medium risk patterns (modifications)
  const mediumPatterns = [
    'modify', 'change', 'update', 'edit', 'refactor',
    'rename', 'move', 'install', 'upgrade', 'downgrade',
  ];

  // Check patterns in order of severity
  for (const pattern of criticalPatterns) {
    if (lowerContent.includes(pattern)) return 'critical';
  }
  for (const pattern of highPatterns) {
    if (lowerContent.includes(pattern)) return 'high';
  }
  for (const pattern of mediumPatterns) {
    if (lowerContent.includes(pattern)) return 'medium';
  }

  return 'low';
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIFIED ORCHESTRATOR SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _orchestratorInstance = null;

/**
 * Get or create the UnifiedOrchestrator singleton
 *
 * @param {Object} options - Creation options
 * @returns {UnifiedOrchestrator}
 */
function getOrCreateOrchestrator(options = {}) {
  if (!_orchestratorInstance) {
    const tracer = createDecisionTracer({ maxTraces: 500 });
    const skillRegistry = createSkillRegistry({ mcpClient: options.mcpClient });

    _orchestratorInstance = createUnifiedOrchestrator({
      persistence: options.persistence,
      judge: options.judge,
      dogOrchestrator: options.dogOrchestrator,
      engineOrchestrator: options.engineOrchestrator,
      tracer,
      skillRegistry,
    });

    log.debug('UnifiedOrchestrator singleton created');
  }
  return _orchestratorInstance;
}

// ═══════════════════════════════════════════════════════════════════════════
// BRAIN_KETER TOOL (Routing Only - Lightweight)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create the brain_keter tool (KETER routing)
 *
 * Lightweight routing-only tool. For full orchestration use brain_orchestrate.
 *
 * @param {Object} options
 * @param {Object} options.persistence - PersistenceManager instance
 * @param {Object} options.judge - CYNICJudge instance
 * @returns {Object} Tool definition
 */
export function createOrchestrateTool(options = {}) {
  const { persistence, judge } = options;

  // Session state cache
  const sessionState = {
    userId: null,
    eScore: 50, // Default to middle
    trustLevel: 'BUILDER',
    recentActions: [],
    currentProject: null,
  };

  return {
    name: 'brain_keter',
    description: `Central consciousness router (KETER - Crown).
Routes events to appropriate Sefirot (specialized agents/tools).
Returns routing decisions and intervention level.

For FULL orchestration (routing + judgment + synthesis + skill invocation),
use brain_orchestrate instead.`,
    inputSchema: {
      type: 'object',
      properties: {
        event: {
          type: 'string',
          enum: Object.values(EVENT_TYPES),
          description: 'Type of event triggering orchestration',
        },
        data: {
          type: 'object',
          description: 'Event-specific data (prompt, tool params, error info, etc.)',
          properties: {
            content: { type: 'string', description: 'Main content (prompt, error message, etc.)' },
            source: { type: 'string', description: 'Source of the event' },
            metadata: { type: 'object', description: 'Additional metadata' },
          },
        },
        context: {
          type: 'object',
          description: 'Current context information',
          properties: {
            user: { type: 'string', description: 'User identifier' },
            project: { type: 'string', description: 'Current project' },
            gitBranch: { type: 'string', description: 'Current git branch' },
            recentActions: {
              type: 'array',
              items: { type: 'string' },
              description: 'Recent actions taken',
            },
          },
        },
      },
      required: ['event', 'data'],
    },
    handler: async (params) => {
      const { event, data, context = {} } = params;
      const content = data.content || '';

      // ═══ 1. UPDATE SESSION STATE ═══
      if (context.user && context.user !== sessionState.userId) {
        sessionState.userId = context.user;

        // PREFER E-Score from context (calculated by hooks)
        if (context.eScore !== undefined) {
          sessionState.eScore = context.eScore;
          sessionState.trustLevel = context.trustLevel || 'BUILDER';
        }
        // Fallback: Load user E-Score from persistence
        else if (persistence) {
          try {
            const result = await persistence.query(
              `SELECT e_score, trust_level FROM user_profiles WHERE user_id = $1`,
              [context.user]
            );
            if (result?.rows?.[0]) {
              sessionState.eScore = result.rows[0].e_score || 50;
              sessionState.trustLevel = result.rows[0].trust_level || 'BUILDER';
            }
          } catch (e) {
            // User not found, use defaults
          }
        }
      }
      // Always update E-Score if provided in context (for returning users)
      else if (context.eScore !== undefined) {
        sessionState.eScore = context.eScore;
        sessionState.trustLevel = context.trustLevel || sessionState.trustLevel;
      }

      if (context.project) {
        sessionState.currentProject = context.project;
      }

      // ═══ 2. ROUTE TO SEFIRAH ═══
      const routing = routeToSefirah(content, event);

      // ═══ 3. DETECT RISK LEVEL ═══
      const actionRisk = detectActionRisk(content);

      // ═══ 4. DETERMINE INTERVENTION ═══
      const intervention = determineIntervention(sessionState.eScore, actionRisk);

      // ═══ 5. BUILD RESPONSE ═══
      const response = {
        // Routing decision
        routing: routing ? {
          sefirah: routing.sefirah,
          domain: routing.domain,
          suggestedAgent: routing.agent,
          suggestedTools: routing.tools,
        } : {
          sefirah: 'Keter',
          domain: 'general',
          suggestedAgent: null,
          suggestedTools: [],
        },

        // Intervention decision
        intervention: {
          level: intervention,
          reason: intervention === INTERVENTION_LEVELS.BLOCK
            ? `Action risk (${actionRisk}) requires explicit permission for trust level ${sessionState.trustLevel}`
            : intervention === INTERVENTION_LEVELS.ASK
              ? `Confirming ${actionRisk} risk action`
              : null,
          userTrustLevel: sessionState.trustLevel,
          userEScore: sessionState.eScore,
          actionRisk,
        },

        // State updates
        stateUpdates: {
          lastEvent: event,
          lastEventTime: Date.now(),
          currentProject: sessionState.currentProject,
        },

        // Actions to take
        actions: [],

        // Metadata
        timestamp: Date.now(),
        confidence: PHI_INV, // Max 61.8% confidence
      };

      // ═══ 6. ADD SUGGESTED ACTIONS ═══

      // If Guardian should be involved for risky actions
      if (actionRisk === 'critical' || actionRisk === 'high') {
        response.actions.push({
          tool: 'brain_cynic_judge',
          priority: 'high',
          reason: `Evaluate ${actionRisk} risk action before proceeding`,
        });
      }

      // If specific sefirah routing found
      if (routing && routing.tools.length > 0) {
        response.actions.push({
          tool: routing.tools[0],
          priority: 'normal',
          reason: `${routing.sefirah} (${routing.agent}) handles ${routing.domain} domain`,
        });
      }

      // For session start, suggest awakening
      if (event === EVENT_TYPES.SESSION_START) {
        response.actions.push({
          tool: 'brain_session_awaken',
          priority: 'normal',
          reason: 'Session awakening ritual',
        });
      }

      // ═══ 7. RECORD TO PERSISTENCE ═══
      if (persistence) {
        try {
          await persistence.query(
            `INSERT INTO orchestration_log (event_type, user_id, sefirah, intervention, risk_level, created_at)
             VALUES ($1, $2, $3, $4, $5, NOW())`,
            [
              event,
              sessionState.userId,
              response.routing.sefirah,
              intervention,
              actionRisk,
            ]
          );
        } catch (e) {
          // Log error but don't fail
          log.error('Error logging orchestration', { error: e.message });
        }
      }

      // Track recent action
      sessionState.recentActions.unshift({
        event,
        sefirah: response.routing.sefirah,
        time: Date.now(),
      });
      sessionState.recentActions = sessionState.recentActions.slice(0, 10);

      return response;
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// BRAIN_ORCHESTRATE TOOL (Full Orchestration)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create the brain_orchestrate tool
 *
 * Full orchestration: routing + judgment + synthesis + skill invocation.
 * Uses UnifiedOrchestrator internally.
 *
 * @param {Object} options
 * @returns {Object} Tool definition
 */
export function createFullOrchestrateTool(options = {}) {
  return {
    name: 'brain_orchestrate',
    description: `Full CYNIC orchestration (Phase 19).
Coordinates all layers: KETER routing → Dogs judgment → Engines synthesis → Skill invocation.
Returns complete decision trace with:
- Routing decision (which Sefirah/domain)
- Judgment (if requested)
- Synthesis (philosophical grounding)
- Skill result (auto-invoked if applicable)
- Full decision trace for transparency

Use this when you need coordinated decision-making across CYNIC's brain.
For lightweight routing only, use brain_keter.`,
    inputSchema: {
      type: 'object',
      properties: {
        content: {
          type: 'string',
          description: 'Content to process (prompt, code, text)',
        },
        eventType: {
          type: 'string',
          enum: Object.values(EVENT_TYPES),
          description: 'Type of event (defaults to user_prompt)',
        },
        requestJudgment: {
          type: 'boolean',
          description: 'Whether to request judgment from Dogs (default: auto-detect based on risk)',
        },
        requestSynthesis: {
          type: 'boolean',
          description: 'Whether to request synthesis from Engines (default: false)',
        },
        autoInvokeSkill: {
          type: 'boolean',
          description: 'Whether to auto-invoke skill based on routing (default: true)',
        },
        context: {
          type: 'object',
          description: 'Additional context',
          properties: {
            userId: { type: 'string', description: 'User identifier' },
            project: { type: 'string', description: 'Current project' },
            metadata: { type: 'object', description: 'Additional metadata' },
          },
        },
      },
      required: ['content'],
    },
    handler: async (params) => {
      const {
        content,
        eventType = EVENT_TYPES.USER_PROMPT,
        requestJudgment,
        requestSynthesis = false,
        autoInvokeSkill = true,
        context = {},
      } = params;

      // Get or create orchestrator
      const orchestrator = getOrCreateOrchestrator(options);

      // Build user context
      const userContext = {
        userId: context.userId || 'anonymous',
        metadata: context.metadata || {},
      };

      // Process through UnifiedOrchestrator
      const result = await orchestrator.process({
        eventType,
        content,
        source: EventSource.TOOL,
        userContext,
        requestJudgment,
        requestSynthesis,
        autoInvokeSkill,
      });

      // Format response
      return {
        success: result.outcome === DecisionOutcome.ALLOW ||
                 result.outcome === DecisionOutcome.MODIFIED,
        outcome: result.outcome,
        decisionId: result.id,

        // Routing info
        routing: result.routing,

        // Intervention level
        intervention: result.intervention,

        // Judgment (if requested)
        judgment: result.judgment ? {
          qScore: result.judgment.qScore,
          verdict: result.judgment.verdict,
          reasoning: result.judgment.reasoning,
        } : null,

        // Synthesis (if requested)
        synthesis: result.synthesis,

        // Skill result (if auto-invoked)
        skillResult: result.skillResult,

        // Decision trace for transparency
        trace: result.trace,

        // Metadata
        timestamp: result.timestamp,
        confidence: PHI_INV,
      };
    },
  };
}

// ═══════════════════════════════════════════════════════════════════════════
// FACTORY
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Factory for orchestration domain tools
 */
export const orchestrationFactory = {
  name: 'orchestration',
  domain: 'orchestration',
  requires: [],

  /**
   * Create orchestration domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    return [
      createOrchestrateTool(options),        // brain_keter (lightweight routing)
      createFullOrchestrateTool(options),    // brain_orchestrate (full orchestration)
    ];
  },
};

export default {
  EVENT_TYPES,
  INTERVENTION_LEVELS,
  SEFIROT_ROUTING,
  TRUST_THRESHOLDS,
  routeToSefirah,
  determineIntervention,
  detectActionRisk,
  createOrchestrateTool,
  createFullOrchestrateTool,
  getOrCreateOrchestrator,
  orchestrationFactory,
};
