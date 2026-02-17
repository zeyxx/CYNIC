/**
 * CynicAccountant — C6.6 (CYNIC × ACCOUNT)
 *
 * Tracks costs and value of CYNIC's internal operations.
 * Factory-instantiated from cynic-accountant.config.js.
 *
 * "Le chien compte ses propres pensées" — self-awareness of cost
 *
 * Cell: C6.6 (CYNIC row × ACCOUNT column)
 * Pattern: Factory-based (60% template, 40% config)
 *
 * @module @cynic/node/cynic/cynic-accountant
 */

'use strict';

import { createAccountant } from '../cycle/create-accountant.js';
import { cynicAccountantConfig } from '../cycle/configs/cynic-accountant.config.js';

const { Class: CynicAccountant, getInstance, resetInstance } = createAccountant(cynicAccountantConfig);

// Named exports
const getCynicAccountant = getInstance;
const resetCynicAccountant = resetInstance;

export {
  CynicAccountant,
  getCynicAccountant,
  resetCynicAccountant,
};

export default {
  CynicAccountant,
  getCynicAccountant,
  resetCynicAccountant,
};
