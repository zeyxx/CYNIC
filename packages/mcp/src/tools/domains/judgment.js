/**
 * Judgment Domain Tools
 *
 * Tools for CYNIC judgment operations:
 * - Judge: 25-dimension evaluation
 * - Refine: Self-refinement of judgments
 * - Feedback: Learning from corrections
 * - Learn: Pattern learning
 *
 * @module @cynic/mcp/tools/domains/judgment
 */

'use strict';

// Re-export from main tools (gradual migration)
export {
  createJudgeTool,
  createRefineTool,
  createFeedbackTool,
  createLearningTool,
} from '../index.js';

/**
 * Factory for judgment domain tools
 */
export const judgmentFactory = {
  name: 'judgment',
  domain: 'judgment',
  requires: ['judge'],

  /**
   * Create all judgment domain tools
   * @param {Object} options
   * @returns {Object[]} Tool definitions
   */
  create(options) {
    const {
      judge,
      persistence,
      sessionManager,
      pojChainManager,
      graphIntegration,
      onJudgment,
    } = options;

    const tools = [];

    // Import dynamically to avoid circular dependency
    const {
      createJudgeTool,
      createRefineTool,
      createFeedbackTool,
      createLearningTool,
    } = require('../index.js');

    // Judge tool (core)
    if (judge) {
      tools.push(createJudgeTool(
        judge,
        persistence,
        sessionManager,
        pojChainManager,
        graphIntegration,
        onJudgment
      ));
    }

    // Refine tool
    if (judge) {
      tools.push(createRefineTool(judge, persistence));
    }

    // Feedback tool
    if (persistence) {
      tools.push(createFeedbackTool(persistence, sessionManager));
    }

    // Learning tool
    if (persistence) {
      tools.push(createLearningTool({ persistence }));
    }

    return tools;
  },
};
