/**
 * CYNIC Human Actor - C5.4 (HUMAN × ACT)
 *
 * Factory-generated from human-actor.config.js + create-actor.js.
 * Domain logic in config, template code in factory.
 *
 * "Le chien agit — il ne délibère pas éternellement" - κυνικός
 *
 * @module @cynic/node/symbiosis/human-actor
 */

'use strict';

import { createActor } from '../cycle/create-actor.js';
import { humanActorConfig, HumanActionType as ActionType } from '../cycle/configs/human-actor.config.js';
import { ActionStatus } from '../cycle/shared-enums.js';

const { Class: HumanActor, getInstance, resetInstance } = createActor(humanActorConfig);

// Domain-specific method: convenience celebrate wrapper
HumanActor.prototype.celebrate = function(reason, details = {}) {
  return this.act(
    { type: 'CELEBRATE', urgency: 'low', reason, ...details },
    {}
  );
};

// Domain-specific override: track dismiss/acknowledge in extra stats
const _originalRecordResponse = HumanActor.prototype.recordResponse;
HumanActor.prototype.recordResponse = function(actionType, response) {
  _originalRecordResponse.call(this, actionType, response);
  if (response === 'dismiss') this._stats.dismissed++;
  else this._stats.acknowledged++;
};

export { ActionType, ActionStatus, HumanActor };
export const getHumanActor = getInstance;
export const resetHumanActor = resetInstance;

export default { HumanActor, ActionType, ActionStatus, getHumanActor, resetHumanActor };
