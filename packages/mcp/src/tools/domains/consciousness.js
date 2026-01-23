/**
 * Consciousness Domain Tools
 *
 * Tools for emergence and self-awareness:
 * - Emergence: Consciousness detection
 * - SelfMod: Self-modification tracking
 * - Milestone: Development milestones
 * - Patterns: Pattern recognition
 *
 * @module @cynic/mcp/tools/domains/consciousness
 */

'use strict';

/**
 * Factory for consciousness domain tools
 */
export const consciousnessFactory = {
  name: 'consciousness',
  domain: 'consciousness',
  requires: ['judge'],

  /**
   * Create all consciousness domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const { judge, persistence } = options;

    const tools = [];

    const {
      createEmergenceTool,
      createSelfModTool,
      createMilestoneHistoryTool,
      createPatternsTool,
    } = require('../index.js');

    // Emergence detection
    if (judge) {
      tools.push(createEmergenceTool(judge, persistence));
    }

    // Self-modification tracking
    tools.push(createSelfModTool());

    // Milestone history
    tools.push(createMilestoneHistoryTool(persistence));

    // Patterns tool
    if (judge) {
      tools.push(createPatternsTool(judge, persistence));
    }

    return tools;
  },
};
