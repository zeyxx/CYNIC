/**
 * Market Actor - C3.4 (MARKET × ACT)
 *
 * φ-factory wrapper for market action logic.
 *
 * @module @cynic/node/market/market-actor
 */

'use strict';

import { createActor } from '../cycle/create-actor.js';
import marketActorConfig, { MarketActionType, MarketActionStatus } from '../cycle/configs/market-actor.config.js';

// Create actor using φ-factory
const { Class: MarketActor, ActionStatus, getInstance, resetInstance } = createActor(marketActorConfig);

/**
 * Get singleton instance
 */
export const getMarketActor = getInstance;

/**
 * Reset singleton (for testing)
 */
export const resetMarketActor = resetInstance;

/**
 * Export action types
 */
export { MarketActionType, ActionStatus as MarketActionStatus };

/**
 * Export class
 */
export { MarketActor };

export default MarketActor;
