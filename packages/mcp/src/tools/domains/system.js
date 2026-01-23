/**
 * System Domain Tools
 *
 * Tools for system operations:
 * - Health: System health checks
 * - Metrics: Performance metrics
 * - Diagnostics: Agent diagnostics
 * - Collective: Multi-agent status
 *
 * @module @cynic/mcp/tools/domains/system
 */

'use strict';

/**
 * Factory for system domain tools
 */
export const systemFactory = {
  name: 'system',
  domain: 'system',
  requires: [],

  /**
   * Create all system domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const {
      node,
      judge,
      persistence,
      metricsService,
      collective,
    } = options;

    const tools = [];

    const {
      createHealthTool,
      createMetricsTool,
      createAgentDiagnosticTool,
      createCollectiveStatusTool,
      createAgentsStatusTool,
    } = require('../index.js');

    // Health tool
    if (node && judge) {
      tools.push(createHealthTool(node, judge, persistence));
    }

    // Metrics tool
    if (metricsService) {
      tools.push(createMetricsTool(metricsService));
    }

    // Collective status
    if (collective) {
      tools.push(createCollectiveStatusTool(collective));
      tools.push(createAgentsStatusTool(collective));
      tools.push(createAgentDiagnosticTool(collective));
    }

    return tools;
  },
};
