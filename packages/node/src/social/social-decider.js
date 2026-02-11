/**
 * Social Decider - C4.3 (SOCIAL × DECIDE)
 *
 * Factory-generated from social-decider.config.js + create-decider.js.
 * Decides social actions based on social:judgment events.
 *
 * "Le chien écoute le vent social" - κυνικός
 *
 * @module @cynic/node/social/social-decider
 */

'use strict';

import { createDecider } from '../cycle/create-decider.js';
import { socialDeciderConfig, SocialDecisionType } from '../cycle/configs/social-decider.config.js';

const { Class: SocialDecider, getInstance, resetInstance } = createDecider(socialDeciderConfig);

export { SocialDecisionType, SocialDecider };
export const getSocialDecider = getInstance;
export const resetSocialDecider = resetInstance;

export default { SocialDecider, SocialDecisionType, getSocialDecider, resetSocialDecider };
