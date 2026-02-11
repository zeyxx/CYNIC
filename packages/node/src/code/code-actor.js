/**
 * CYNIC Code Actor - C1.4 (CODE × ACT)
 *
 * Factory-generated from code-actor.config.js + create-actor.js.
 * Domain logic in config, template code in factory.
 *
 * "Le chien aboie, le code obéit" - κυνικός
 *
 * @module @cynic/node/code/code-actor
 */

'use strict';

import { createActor } from '../cycle/create-actor.js';
import { codeActorConfig, CodeActionType } from '../cycle/configs/code-actor.config.js';
import { ActionStatus as CodeActionStatus } from '../cycle/shared-enums.js';

const { Class: CodeActor, getInstance, resetInstance } = createActor(codeActorConfig);

// Domain-specific method: technical debt log access (used by code-learner.js)
CodeActor.prototype.getDebtLog = function(limit = 21) {
  return (this._debtLog || []).slice(-limit);
};

export { CodeActionType, CodeActionStatus, CodeActor };
export const getCodeActor = getInstance;
export const resetCodeActor = resetInstance;

export default { CodeActor, CodeActionType, CodeActionStatus, getCodeActor, resetCodeActor };
