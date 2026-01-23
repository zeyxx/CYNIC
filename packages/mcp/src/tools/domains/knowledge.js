/**
 * Knowledge Domain Tools
 *
 * Tools for knowledge management:
 * - Search: Search judgments and patterns
 * - Digest: Extract knowledge from content
 * - Docs: Library documentation
 *
 * @module @cynic/mcp/tools/domains/knowledge
 */

'use strict';

/**
 * Factory for knowledge domain tools
 */
export const knowledgeFactory = {
  name: 'knowledge',
  domain: 'knowledge',
  requires: [],

  /**
   * Create all knowledge domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const { persistence, sessionManager, librarian } = options;

    const tools = [];

    const {
      createSearchTool,
      createDigestTool,
      createDocsTool,
    } = require('../index.js');

    // Search tool
    if (persistence) {
      tools.push(createSearchTool(persistence));
    }

    // Digest tool
    tools.push(createDigestTool(persistence, sessionManager));

    // Documentation tool
    if (librarian) {
      tools.push(createDocsTool(librarian, persistence));
    }

    return tools;
  },
};
