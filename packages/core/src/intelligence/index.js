/**
 * CYNIC Intelligence — Prompt classification and context routing
 *
 * @module @cynic/core/intelligence
 */

'use strict';

export {
  classifyPrompt,
  scoreContextRelevance,
  selectSections,
  DOMAIN_KEYWORDS,
  INTENT_PATTERNS,
  TOKEN_BUDGETS,
  SKIP_PATTERNS,
} from './prompt-classifier.js';

export {
  createPhiGovernor,
  SETPOINT,
  LOWER_BOUND,
  classifyZone,
} from './phi-governor.js';

export {
  createExperimentRunner,
  computeStats,
  pairedTTest,
  generateTestPrompts,
} from './experiment-runner.js';
