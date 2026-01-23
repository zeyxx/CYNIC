/**
 * Blockchain Domain Tools
 *
 * Tools for Proof-of-Judgment chain:
 * - PoJChain: Block management
 * - Trace: Judgment verification
 *
 * @module @cynic/mcp/tools/domains/blockchain
 */

'use strict';

/**
 * Factory for blockchain domain tools
 */
export const blockchainFactory = {
  name: 'blockchain',
  domain: 'blockchain',
  requires: ['pojChainManager'],

  /**
   * Create all blockchain domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const { pojChainManager, persistence } = options;

    const tools = [];

    const {
      createPoJChainTool,
      createTraceTool,
    } = require('../index.js');

    // PoJ Chain tool
    if (pojChainManager) {
      tools.push(createPoJChainTool(pojChainManager, persistence));
    }

    // Trace tool
    if (persistence) {
      tools.push(createTraceTool(persistence, pojChainManager));
    }

    return tools;
  },
};
