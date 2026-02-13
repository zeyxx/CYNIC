/**
 * Session Domain Tools
 *
 * Tools for session and user management:
 * - SessionStart: Begin a session
 * - SessionEnd: End a session
 * - ProfileSync: Sync user profile
 * - ProfileLoad: Load user profile
 * - Psychology: User psychology state
 *
 * @module @cynic/mcp/tools/domains/session
 */

'use strict';

import { PHI_INV, createLogger } from '@cynic/core';

const log = createLogger('SessionTools');

// φ⁻² = 0.382 (secondary threshold)
const PHI_INV_2 = 0.382;

/**
 * Create psychology tool definition
 * @param {Object} [persistence] - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createPsychologyTool(persistence = null) {
  return {
    name: 'brain_psychology',
    description: 'Get human psychology dashboard showing energy, focus, emotions, biases, and learning calibration. Use for /psy command.',
    inputSchema: {
      type: 'object',
      properties: {
        userId: { type: 'string', description: 'User ID (defaults to current user)' },
        includeHistory: { type: 'boolean', description: 'Include recent intervention history' },
        includeCalibration: { type: 'boolean', description: 'Include detailed calibration stats' },
      },
    },
    handler: async (params) => {
      const { userId, includeHistory = false, includeCalibration = true } = params;

      const result = {
        status: 'ok',
        timestamp: Date.now(),
        confidence: PHI_INV, // Max confidence for psychology assessment
      };

      // ═══════════════════════════════════════════════════════════════════════════
      // PSYCHOLOGY STATE: Load from persistence if available
      // ═══════════════════════════════════════════════════════════════════════════
      if (persistence && userId) {
        try {
          const psychology = await persistence.loadPsychology(userId);
          if (psychology) {
            result.dimensions = psychology.dimensions || {};
            result.emotions = psychology.emotions || {};
            result.temporal = psychology.temporal || {};
            result.sessionCount = psychology.sessionCount || 0;
            result.lastUpdated = psychology.lastUpdated;

            // Calculate composite states
            result.composites = calculatePsychologyComposites(psychology.dimensions, psychology.emotions);
          }
        } catch (e) {
          result.psychologyError = e.message;
        }
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // CALIBRATION: Learning loop accuracy
      // ═══════════════════════════════════════════════════════════════════════════
      if (includeCalibration && persistence && userId) {
        try {
          const calibration = await persistence.getCalibrationStats(userId);
          if (calibration) {
            result.calibration = calibration;
          }
        } catch (e) {
          // Calibration not available
        }
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // INTERVENTION HISTORY: Recent interventions and effectiveness
      // ═══════════════════════════════════════════════════════════════════════════
      if (includeHistory && persistence && userId) {
        try {
          const effectiveness = await persistence.getInterventionEffectiveness(userId);
          if (effectiveness) {
            result.interventionEffectiveness = effectiveness;
          }
        } catch (e) {
          // History not available
        }
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // AGGREGATE STATS: Overall psychology system stats
      // ═══════════════════════════════════════════════════════════════════════════
      if (persistence) {
        try {
          result.systemStats = await persistence.getPsychologyStats();
        } catch (e) {
          // Stats not available
        }
      }

      // ═══════════════════════════════════════════════════════════════════════════
      // DETERMINE OVERALL STATE
      // ═══════════════════════════════════════════════════════════════════════════
      result.overallState = determineOverallState(result.dimensions, result.composites);

      return result;
    },
  };
}

/**
 * Calculate composite psychological states from dimensions and emotions
 * @param {Object} dimensions - Core dimensions
 * @param {Object} emotions - Emotional spectrum
 * @returns {Object} Composite states
 */
function calculatePsychologyComposites(dimensions = {}, emotions = {}) {
  const d = dimensions;
  const e = emotions;

  // Extract values with defaults
  const energy = d.energy?.value ?? PHI_INV;
  const focus = d.focus?.value ?? PHI_INV;
  const creativity = d.creativity?.value ?? PHI_INV;
  const frustration = d.frustration?.value ?? PHI_INV_2;
  const curiosity = e.curiosity?.value ?? PHI_INV;
  const boredom = e.boredom?.value ?? 0;
  const riskAppetite = d.riskAppetite?.value ?? PHI_INV;

  return {
    // Flow: High energy + focus + creativity, low frustration
    flow: energy > PHI_INV && focus > PHI_INV && creativity > PHI_INV_2 && frustration < PHI_INV_2,

    // Burnout risk: Low energy + high frustration
    burnoutRisk: energy < PHI_INV_2 && frustration > PHI_INV,

    // Exploration: High curiosity + risk appetite
    exploration: curiosity > PHI_INV && riskAppetite > PHI_INV_2,

    // Grind: Low creativity, moderate focus (mechanical work)
    grind: creativity < PHI_INV_2 && focus > PHI_INV_2 && energy > PHI_INV_2,

    // Procrastination: Low focus, high boredom
    procrastination: focus < PHI_INV_2 && boredom > PHI_INV_2,

    // Breakthrough potential: High creativity + curiosity
    breakthrough: creativity > PHI_INV && curiosity > PHI_INV,
  };
}

/**
 * Determine overall psychological state label
 * @param {Object} dimensions - Core dimensions
 * @param {Object} composites - Composite states
 * @returns {Object} Overall state with label and emoji
 */
function determineOverallState(dimensions = {}, composites = {}) {
  if (composites?.burnoutRisk) {
    return { state: 'burnout_risk', label: 'Burnout Risk', emoji: '🔥' };
  }
  if (composites?.flow) {
    return { state: 'flow', label: 'Flow', emoji: '✨' };
  }
  if (composites?.exploration) {
    return { state: 'exploration', label: 'Exploration', emoji: '🔭' };
  }
  if (composites?.breakthrough) {
    return { state: 'breakthrough', label: 'Breakthrough Potential', emoji: '💡' };
  }
  if (composites?.procrastination) {
    return { state: 'procrastination', label: 'Low Focus', emoji: '😴' };
  }
  if (composites?.grind) {
    return { state: 'grind', label: 'Grind Mode', emoji: '⚙️' };
  }

  // Default based on energy/focus
  const energy = dimensions?.energy?.value ?? PHI_INV;
  const focus = dimensions?.focus?.value ?? PHI_INV;

  if (energy > PHI_INV && focus > PHI_INV) {
    return { state: 'productive', label: 'Productive', emoji: '🚀' };
  }
  if (energy < PHI_INV_2) {
    return { state: 'low_energy', label: 'Low Energy', emoji: '🔋' };
  }

  return { state: 'neutral', label: 'Neutral', emoji: '*tail wag*' };
}

/**
 * Create session start tool
 * @param {Object} sessionManager - SessionManager instance
 * @returns {Object} Tool definition
 */
export function createSessionStartTool(sessionManager, persistence = null) {
  return {
    name: 'brain_session_start',
    description: 'Start a new CYNIC session for a user. Sessions provide isolation and tracking of judgments, digests, and feedback. Automatically restores learned patterns and Q-Learning state from prior sessions.',
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

      // ═══════════════════════════════════════════════════════════════════════════
      // RESTORE: Load collective state from prior sessions
      // "Le chien se souvient" — without this, CYNIC has Alzheimer's
      // ═══════════════════════════════════════════════════════════════════════════
      let memoryRestored = { restored: [], stats: {} };
      if (persistence) {
        try {
          const { restoreCollectiveState } = await import('@cynic/node');
          memoryRestored = await restoreCollectiveState(persistence);
          log.info('Session memory restored', {
            sessionId: session.sessionId,
            components: memoryRestored.restored,
          });
        } catch (err) {
          log.warn('Memory restoration failed (session still valid)', { error: err.message });
        }
      }

      return {
        success: true,
        sessionId: session.sessionId,
        userId: session.userId,
        project: session.project,
        createdAt: session.createdAt,
        memoryRestored: memoryRestored.restored.length > 0,
        restoredComponents: memoryRestored.restored,
        restoredStats: memoryRestored.stats,
        message: memoryRestored.restored.length > 0
          ? `*tail wag* Session started. Memory restored: ${memoryRestored.restored.join(', ')}. Le chien se souvient.`
          : `*tail wag* Session started. No prior memory found (fresh start).`,
      };
    },
  };
}

/**
 * Create session end tool
 * @param {Object} sessionManager - SessionManager instance
 * @param {Object} [persistence] - PersistenceManager instance (for collective state persistence)
 * @returns {Object} Tool definition
 */
export function createSessionEndTool(sessionManager, persistence = null) {
  return {
    name: 'brain_session_end',
    description: 'End the current CYNIC session. Saves collective state and returns session summary with statistics.',
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

      // ═══════════════════════════════════════════════════════════════════════════
      // FIX #1: Save collective state (SharedMemory + Q-Learning + Patterns)
      // "Le chien se souvient" - Memory must survive between sessions
      // ═══════════════════════════════════════════════════════════════════════════
      let collectiveSaved = false;
      if (persistence) {
        try {
          // Import saveCollectiveState dynamically to avoid circular deps
          const { saveCollectiveState } = await import('@cynic/node');
          await saveCollectiveState(persistence);
          collectiveSaved = true;
          log.info('Collective state saved on session end');
        } catch (e) {
          log.warn('Failed to save collective state', { error: e.message });
        }
      }

      return {
        success: true,
        ...result.summary,
        collectiveSaved,
        message: `*yawn* Session ended. ${result.summary.judgmentCount} judgments recorded.${collectiveSaved ? ' Memory preserved.' : ''}`,
      };
    },
  };
}

/**
 * Create profile sync tool - saves user profile to database
 * Called at session end for cross-session memory persistence
 * @param {Object} persistence - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createProfileSyncTool(persistence) {
  return {
    name: 'brain_profile_sync',
    description: 'Sync user profile to database for cross-session memory. Called at session end.',
    inputSchema: {
      type: 'object',
      properties: {
        userId: {
          type: 'string',
          description: 'User ID (hook-generated usr_xxx format)',
        },
        profile: {
          type: 'object',
          description: 'Full user profile from hooks (identity, stats, patterns, preferences, memory)',
        },
      },
      required: ['userId', 'profile'],
    },
    handler: async (params) => {
      const { userId, profile } = params;

      if (!persistence?.userLearningProfiles) {
        return {
          success: false,
          error: 'Profile persistence not available (no PostgreSQL connection)',
          hint: 'Profile will be stored locally only',
          timestamp: Date.now(),
        };
      }

      try {
        const result = await persistence.userLearningProfiles.syncProfile(userId, profile);
        return {
          success: true,
          userId,
          sessionCount: result?.session_count || profile.stats?.sessions,
          message: `*tail wag* Profile synced. φ remembers across sessions.`,
          timestamp: Date.now(),
        };
      } catch (err) {
        log.error('Profile sync error', { userId, error: err.message });
        return {
          success: false,
          error: err.message,
          hint: 'Profile will be stored locally only',
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * Create profile load tool - loads user profile from database
 * Called at session start to restore cross-session memory
 * @param {Object} persistence - PersistenceManager instance
 * @returns {Object} Tool definition
 */
export function createProfileLoadTool(persistence) {
  return {
    name: 'brain_profile_load',
    description: 'Load user profile from database for cross-session memory. Called at session start.',
    inputSchema: {
      type: 'object',
      properties: {
        userId: {
          type: 'string',
          description: 'User ID (hook-generated usr_xxx format)',
        },
      },
      required: ['userId'],
    },
    handler: async (params) => {
      const { userId } = params;

      if (!persistence?.userLearningProfiles) {
        return {
          success: false,
          profile: null,
          error: 'Profile persistence not available (no PostgreSQL connection)',
          hint: 'Using local profile only',
          timestamp: Date.now(),
        };
      }

      try {
        const profile = await persistence.userLearningProfiles.loadProfile(userId);

        if (!profile) {
          return {
            success: true,
            profile: null,
            isNewUser: true,
            message: `*curious sniff* New user detected. Welcome.`,
            timestamp: Date.now(),
          };
        }

        return {
          success: true,
          profile,
          isNewUser: false,
          sessionCount: profile.meta?.sessionCount || 0,
          lastSession: profile.meta?.lastSession,
          message: `*tail wag* Welcome back! Session ${(profile.meta?.sessionCount || 0) + 1}.`,
          timestamp: Date.now(),
        };
      } catch (err) {
        log.error('Profile load error', { userId, error: err.message });
        return {
          success: false,
          profile: null,
          error: err.message,
          hint: 'Using local profile only',
          timestamp: Date.now(),
        };
      }
    },
  };
}

// P4: createPreferencesTool REMOVED — dead code, persistence.userPreferences never exists
// Superseded by brain_profile_sync + brain_profile_load
function _dead_createPreferencesTool(persistence) {
  return {
    name: 'brain_preferences',
    description: 'Get or set user preferences for judgment, automation, and UI. Use action="get" to retrieve, action="set" to update.',
    inputSchema: {
      type: 'object',
      properties: {
        userId: {
          type: 'string',
          description: 'User ID',
        },
        action: {
          type: 'string',
          enum: ['get', 'set', 'reset'],
          description: 'Action to perform (default: get)',
        },
        preferences: {
          type: 'object',
          description: 'Preferences to set (for action="set")',
          properties: {
            judgmentStrictness: { type: 'number', description: '0-1 strictness level (default: 0.5)' },
            autoJudge: { type: 'boolean', description: 'Enable auto-judgment (default: true)' },
            autoLearn: { type: 'boolean', description: 'Enable auto-learning (default: true)' },
            autoNotifications: { type: 'boolean', description: 'Enable proactive notifications (default: true)' },
            theme: { type: 'string', enum: ['dark', 'light'], description: 'UI theme (default: dark)' },
            notificationLevel: { type: 'string', enum: ['silent', 'minimal', 'normal', 'verbose'] },
          },
        },
      },
      required: ['userId'],
    },
    handler: async (params) => {
      const { userId, action = 'get', preferences = {} } = params;

      if (!persistence?.userPreferences) {
        return {
          success: false,
          error: 'User preferences not available (no PostgreSQL connection)',
          timestamp: Date.now(),
        };
      }

      try {
        switch (action) {
          case 'get': {
            const prefs = await persistence.userPreferences.get(userId);
            return {
              success: true,
              action: 'get',
              userId,
              preferences: prefs,
              message: `*tail wag* Preferences loaded for ${userId}.`,
              timestamp: Date.now(),
            };
          }

          case 'set': {
            // Convert camelCase to snake_case for database
            const updates = {};
            if (preferences.judgmentStrictness !== undefined) {
              updates.judgment_strictness = preferences.judgmentStrictness;
            }
            if (preferences.autoJudge !== undefined) {
              updates.auto_judge = preferences.autoJudge;
            }
            if (preferences.autoLearn !== undefined) {
              updates.auto_learn = preferences.autoLearn;
            }
            if (preferences.autoNotifications !== undefined) {
              updates.auto_notifications = preferences.autoNotifications;
            }
            if (preferences.theme !== undefined) {
              updates.theme = preferences.theme;
            }
            if (preferences.notificationLevel !== undefined) {
              updates.notification_level = preferences.notificationLevel;
            }

            if (Object.keys(updates).length === 0) {
              return {
                success: false,
                error: 'No valid preferences provided to update',
                timestamp: Date.now(),
              };
            }

            const updated = await persistence.userPreferences.update(userId, updates);
            return {
              success: true,
              action: 'set',
              userId,
              preferences: updated,
              updated: Object.keys(updates),
              message: `*tail wag* ${Object.keys(updates).length} preference(s) updated.`,
              timestamp: Date.now(),
            };
          }

          case 'reset': {
            const reset = await persistence.userPreferences.reset(userId);
            return {
              success: true,
              action: 'reset',
              userId,
              preferences: reset,
              message: `*sniff* Preferences reset to defaults.`,
              timestamp: Date.now(),
            };
          }

          default:
            return {
              success: false,
              error: `Unknown action: ${action}`,
              timestamp: Date.now(),
            };
        }
      } catch (err) {
        log.error('Preferences error', { userId, action, error: err.message });
        return {
          success: false,
          error: err.message,
          timestamp: Date.now(),
        };
      }
    },
  };
}

// =============================================================================
// SESSION PATTERNS TOOLS
// =============================================================================

// P4: createSessionPatternsSaveTool REMOVED — dead code, persistence.sessionPatterns never exists
function _dead_createSessionPatternsSaveTool(persistence) {
  return {
    name: 'brain_session_patterns_save',
    description: 'Save detected patterns from session to database. Called by sleep.js hook.',
    inputSchema: {
      type: 'object',
      properties: {
        sessionId: {
          type: 'string',
          description: 'Session identifier',
        },
        userId: {
          type: 'string',
          description: 'User UUID',
        },
        patterns: {
          type: 'array',
          description: 'Array of patterns to save',
          items: {
            type: 'object',
            properties: {
              type: { type: 'string', description: 'Pattern type (SEQUENCE, ANOMALY, etc.)' },
              name: { type: 'string', description: 'Pattern name' },
              confidence: { type: 'number', description: 'Confidence (0-0.618)' },
              occurrences: { type: 'number', description: 'Occurrence count' },
              context: { type: 'object', description: 'Pattern context data' },
            },
          },
        },
      },
      required: ['sessionId', 'userId', 'patterns'],
    },
    handler: async (params) => {
      const { sessionId, userId, patterns } = params;

      if (!persistence?.sessionPatterns) {
        return {
          success: false,
          error: 'Session patterns persistence not available',
          timestamp: Date.now(),
        };
      }

      if (!patterns || patterns.length === 0) {
        return {
          success: true,
          saved: 0,
          message: '*yawn* No patterns to save.',
          timestamp: Date.now(),
        };
      }

      try {
        const count = await persistence.sessionPatterns.savePatterns(sessionId, userId, patterns);
        return {
          success: true,
          saved: count,
          sessionId,
          message: `*tail wag* ${count} patterns persisted for future sessions.`,
          timestamp: Date.now(),
        };
      } catch (err) {
        log.error('Session patterns save error', { sessionId, error: err.message });
        return {
          success: false,
          error: err.message,
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * P4: createSessionPatternsLoadTool REMOVED — dead code
 */
function _dead_createSessionPatternsLoadTool(persistence) {
  return {
    name: 'brain_session_patterns_load',
    description: 'Load recent patterns from previous sessions. Called by awaken.js hook.',
    inputSchema: {
      type: 'object',
      properties: {
        userId: {
          type: 'string',
          description: 'User UUID',
        },
        limit: {
          type: 'number',
          description: 'Maximum patterns to load (default 50)',
        },
      },
      required: ['userId'],
    },
    handler: async (params) => {
      const { userId, limit = 50 } = params;

      if (!persistence?.sessionPatterns) {
        return {
          success: false,
          patterns: [],
          error: 'Session patterns persistence not available',
          timestamp: Date.now(),
        };
      }

      try {
        const patterns = await persistence.sessionPatterns.loadRecentPatterns(userId, limit);
        const stats = await persistence.sessionPatterns.getStats(userId);

        return {
          success: true,
          patterns,
          count: patterns.length,
          stats,
          message: patterns.length > 0
            ? `*ears perk* ${patterns.length} patterns restored from memory.`
            : `*curious sniff* No patterns from previous sessions.`,
          timestamp: Date.now(),
        };
      } catch (err) {
        log.error('Session patterns load error', { userId, error: err.message });
        return {
          success: false,
          patterns: [],
          error: err.message,
          timestamp: Date.now(),
        };
      }
    },
  };
}

/**
 * Factory for session domain tools
 */
export const sessionFactory = {
  name: 'session',
  domain: 'session',
  requires: [],

  /**
   * Create all session domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const { sessionManager, persistence } = options;

    const tools = [];

    // Session management
    if (sessionManager) {
      tools.push(createSessionStartTool(sessionManager, persistence));  // Pass persistence for memory restore
      tools.push(createSessionEndTool(sessionManager, persistence));  // FIX #1: Pass persistence for collective save
    }

    // Profile management
    if (persistence) {
      tools.push(createProfileSyncTool(persistence));
      tools.push(createProfileLoadTool(persistence));
      tools.push(createPsychologyTool(persistence));
      // P4: preferences + session_patterns tools removed (dead code)
    }

    return tools;
  },
};
