/**
 * CYNIC Human Learner - C5.5 (HUMAN × LEARN)
 *
 * Factory-generated from human-learner.config.js + create-learner.js.
 * Wraps HumanLearning in φ-factory pattern for pipeline integration.
 *
 * "Le chien apprend les habitudes de son maître" - κυνικός
 *
 * @module @cynic/node/symbiosis/human-learner
 */

'use strict';

import { createLearner } from '../cycle/create-learner.js';
import { humanLearnerConfig, LearningCategory } from '../cycle/configs/human-learner.config.js';

const { Class: HumanLearner, getInstance, resetInstance } = createLearner(humanLearnerConfig);

export { LearningCategory, HumanLearner };
export const getHumanLearner = getInstance;
export const resetHumanLearner = resetInstance;

export default { HumanLearner, LearningCategory, getHumanLearner, resetHumanLearner };
