/**
 * Cosmos Actor - C7.4 (COSMOS × ACT)
 *
 * Factory-generated from cosmos-actor.config.js + create-actor.js.
 * Domain logic in config, template code in factory.
 *
 * "Le chien agit pour les étoiles" - κυνικός
 *
 * @module @cynic/node/cosmos/cosmos-actor
 */

'use strict';

import { createActor } from '../cycle/create-actor.js';
import { cosmosActorConfig, CosmosActionType } from '../cycle/configs/cosmos-actor.config.js';
import { ActionStatus as CosmosActionStatus } from '../cycle/shared-enums.js';

const { Class: CosmosActor, getInstance, resetInstance } = createActor(cosmosActorConfig);

export { CosmosActionType, CosmosActionStatus, CosmosActor };
export const getCosmosActor = getInstance;
export const resetCosmosActor = resetInstance;

export default { CosmosActor, CosmosActionType, CosmosActionStatus, getCosmosActor, resetCosmosActor };
