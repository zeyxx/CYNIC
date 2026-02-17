/**
 * Market Decider - C3.3 (MARKET × DECIDE)
 *
 * φ-factory wrapper for market decision logic.
 *
 * @module @cynic/node/market/market-decider
 */

'use strict';

import { createDecider } from '../cycle/create-decider.js';
import marketDeciderConfig, { MarketDecisionType } from '../cycle/configs/market-decider.config.js';

// Create decider using φ-factory
const { Class: MarketDecider, getInstance, resetInstance } = createDecider(marketDeciderConfig);

/**
 * Get singleton instance
 */
export const getMarketDecider = getInstance;

/**
 * Reset singleton (for testing)
 */
export const resetMarketDecider = resetInstance;

/**
 * Export decision types
 */
export { MarketDecisionType };

/**
 * Export class
 */
export { MarketDecider };

export default MarketDecider;
