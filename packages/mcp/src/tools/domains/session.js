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

import { PHI_INV } from '@cynic/core';

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
        console.error('Profile sync error:', err.message);
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
        console.error('Profile load error:', err.message);
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
      tools.push(createSessionStartTool(sessionManager));
      tools.push(createSessionEndTool(sessionManager));
    }

    // Profile management
    if (persistence) {
      tools.push(createProfileSyncTool(persistence));
      tools.push(createProfileLoadTool(persistence));
      tools.push(createPsychologyTool(persistence));
    }

    return tools;
  },
};
