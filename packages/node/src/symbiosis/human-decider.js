/**
 * CYNIC Human Decider - C5.3 (HUMAN × DECIDE)
 *
 * Factory-generated from human-decider.config.js + create-decider.js.
 * Converts HumanJudge verdicts into intervention decisions.
 *
 * "Le chien décide pour le bien du maître" - κυνικός
 *
 * @module @cynic/node/symbiosis/human-decider
 */

'use strict';

import { createDecider } from '../cycle/create-decider.js';
import { humanDeciderConfig, HumanDecisionType } from '../cycle/configs/human-decider.config.js';

const { Class: HumanDecider, getInstance, resetInstance } = createDecider(humanDeciderConfig);

export { HumanDecisionType, HumanDecider };
export const getHumanDecider = getInstance;
export const resetHumanDecider = resetInstance;

export default { HumanDecider, HumanDecisionType, getHumanDecider, resetHumanDecider };
