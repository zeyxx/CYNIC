/**
 * Ecosystem Domain Tools
 *
 * Tools for ecosystem management:
 * - Ecosystem: Repository tracking and updates
 * - EcosystemMonitor: Real-time monitoring
 * - Integrator: Cross-project synchronization
 * - Discovery: Repository discovery
 *
 * @module @cynic/mcp/tools/domains/ecosystem
 */

'use strict';

/**
 * Factory for ecosystem domain tools
 */
export const ecosystemFactory = {
  name: 'ecosystem',
  domain: 'ecosystem',
  requires: [],

  /**
   * Create all ecosystem domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const {
      ecosystem,
      integrator,
      discovery,
      persistence,
    } = options;

    const tools = [];

    // Import dynamically
    const {
      createEcosystemTool,
      createEcosystemMonitorTool,
      createIntegratorTool,
      createDiscoveryTool,
    } = require('../index.js');

    // Ecosystem tool
    if (ecosystem) {
      tools.push(createEcosystemTool(ecosystem));
    }

    // Ecosystem monitor tool
    tools.push(createEcosystemMonitorTool({ persistence }));

    // Integrator tool
    if (integrator) {
      tools.push(createIntegratorTool(integrator));
    }

    // Discovery tool
    if (discovery) {
      tools.push(createDiscoveryTool(discovery));
    }

    return tools;
  },
};
