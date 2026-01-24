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
 * @module @cynic/mcp/tools/domains/orchestration
 */

'use strict';

import { PHI, PHI_INV, THRESHOLDS } from '@cynic/core';

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
// BRAIN_ORCHESTRATE TOOL
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Create the brain_orchestrate tool
 *
 * This is KETER - the central consciousness that routes all events
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
    description: `Central consciousness orchestrator (KETER - Crown).
Routes events to appropriate Sefirot (specialized agents/tools).
Adapts intervention level based on user E-Score and action risk.
Returns routing decisions, suggested tools, and intervention level.

This is the "brain" of CYNIC - all events pass through KETER for routing.`,
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

        // Load user E-Score from persistence
        if (persistence) {
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
          console.error('Error logging orchestration:', e.message);
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
    return [createOrchestrateTool(options)];
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
  orchestrationFactory,
};
