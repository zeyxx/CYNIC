/**
 * CYNIC Learner - C6.5 (CYNIC × LEARN)
 *
 * Self-improvement: CYNIC learns from self-governance outcomes.
 * Sits after C6.4 (ACT) — actions become outcomes become learning updates.
 *
 * "Le chien apprend de ses propres pas" - κυνικός
 *
 * Learns from:
 * - Budget governance effectiveness (Ollama shifts)
 * - Memory optimization results (compression, GC)
 * - Learning velocity changes (SONA prioritization)
 * - Dog rotation outcomes (consensus quality)
 * - Threshold calibration accuracy
 *
 * Predictions:
 * - Predict budget savings from Ollama shift
 * - Predict memory freed from optimization
 * - Decide when to rotate dogs
 *
 * Generated via createLearner factory (50% template + 50% config).
 *
 * @module @cynic/node/cynic/cynic-learner
 */

'use strict';

import { createLearner } from '../cycle/create-learner.js';
import {
  cynicLearnerConfig,
  CynicLearningCategory,
} from '../cycle/configs/cynic-learner.config.js';

// Generate the CynicLearner class from config
const { Class: CynicLearner, getInstance, resetInstance } =
  createLearner(cynicLearnerConfig);

// Create aliases for consistency with naming convention
const getCynicLearner = getInstance;
const resetCynicLearner = resetInstance;

// Export for external use
export {
  CynicLearner,
  CynicLearningCategory,
  getCynicLearner,
  resetCynicLearner,
};

export default {
  CynicLearner,
  CynicLearningCategory,
  getCynicLearner,
  resetCynicLearner,
};
