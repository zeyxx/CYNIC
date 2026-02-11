/**
 * Social Actor - C4.4 (SOCIAL × ACT)
 *
 * Factory-generated from social-actor.config.js + create-actor.js.
 * Executes advisory social actions from SocialDecider decisions.
 *
 * "Le chien protège la meute sociale" - κυνικός
 *
 * @module @cynic/node/social/social-actor
 */

'use strict';

import { createActor } from '../cycle/create-actor.js';
import { socialActorConfig, SocialActionType } from '../cycle/configs/social-actor.config.js';
import { ActionStatus as SocialActionStatus } from '../cycle/shared-enums.js';

const { Class: SocialActor, getInstance, resetInstance } = createActor(socialActorConfig);

export { SocialActionType, SocialActionStatus, SocialActor };
export const getSocialActor = getInstance;
export const resetSocialActor = resetInstance;

export default { SocialActor, SocialActionType, SocialActionStatus, getSocialActor, resetSocialActor };
