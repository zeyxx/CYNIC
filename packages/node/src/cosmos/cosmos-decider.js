/**
 * Cosmos Decider - C7.3 (COSMOS × DECIDE)
 *
 * Factory-generated from cosmos-decider.config.js + create-decider.js.
 * Domain logic in config, template code in factory.
 *
 * "Le chien décide pour les étoiles" - κυνικός
 *
 * @module @cynic/node/cosmos/cosmos-decider
 */

'use strict';

import { createDecider } from '../cycle/create-decider.js';
import { cosmosDeciderConfig, CosmosDecisionType } from '../cycle/configs/cosmos-decider.config.js';

const { Class: CosmosDecider, getInstance, resetInstance } = createDecider(cosmosDeciderConfig);

export { CosmosDecisionType, CosmosDecider };
export const getCosmosDecider = getInstance;
export const resetCosmosDecider = resetInstance;

export default { CosmosDecider, CosmosDecisionType, getCosmosDecider, resetCosmosDecider };
