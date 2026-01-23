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

    const {
      createSessionStartTool,
      createSessionEndTool,
      createProfileSyncTool,
      createProfileLoadTool,
      createPsychologyTool,
    } = require('../index.js');

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
