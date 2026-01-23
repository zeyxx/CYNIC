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
    const { persistence, collective, scheduler } = options;

    const tools = [];

    const {
      createTriggersTool,
      createOrchestrationTool,
    } = require('../index.js');

    // Triggers tool
    tools.push(createTriggersTool({ persistence, collective, scheduler }));

    // Orchestration tool
    tools.push(createOrchestrationTool({ persistence }));

    return tools;
  },
};
