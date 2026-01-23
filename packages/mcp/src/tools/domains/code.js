/**
 * Code Domain Tools
 *
 * Tools for code analysis:
 * - Codebase: Codebase analysis
 * - VectorSearch: Semantic code search
 *
 * @module @cynic/mcp/tools/domains/code
 */

'use strict';

/**
 * Factory for code domain tools
 */
export const codeFactory = {
  name: 'code',
  domain: 'code',
  requires: [],

  /**
   * Create all code domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const tools = [];

    const {
      createCodebaseTool,
      createVectorSearchTool,
    } = require('../index.js');

    // Codebase analysis tool
    tools.push(createCodebaseTool(options));

    // Vector search tool
    tools.push(createVectorSearchTool(options));

    return tools;
  },
};
