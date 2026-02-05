/**
 * CYNIC Symbiosis Module
 *
 * Human-CYNIC symbiotic intelligence layer.
 * "Le chien amplifie l'humain, l'humain guide le chien"
 *
 * @module @cynic/node/symbiosis
 */

'use strict';

// C5.3: HUMAN × DECIDE
export {
  HumanAdvisor,
  InterventionType,
  UrgencyLevel,
  getHumanAdvisor,
  resetHumanAdvisor,
} from './human-advisor.js';

// C5.5: HUMAN × LEARN
export {
  HumanLearning,
  LearningCategory,
  getHumanLearning,
  resetHumanLearning,
} from './human-learning.js';

// C5.6: HUMAN × ACCOUNT
export {
  HumanAccountant,
  ActivityType,
  getHumanAccountant,
  resetHumanAccountant,
} from './human-accountant.js';

// C5.7: HUMAN × EMERGE
export {
  HumanEmergence,
  HumanPatternType,
  SignificanceLevel as HumanSignificanceLevel,
  getHumanEmergence,
  resetHumanEmergence,
} from './human-emergence.js';
