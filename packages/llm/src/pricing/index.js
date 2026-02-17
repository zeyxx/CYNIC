/**
 * CYNIC Pricing Module
 *
 * Real-time pricing for all LLM providers.
 * NOTHING IS FREE - CYNIC knows real costs.
 *
 * @module @cynic/llm/pricing
 */

'use strict';

export {
  PricingOracle,
  createPricingOracle,
  getOracle,
  PROVIDER_PRICING,
  DEFAULT_CONFIG,
  calculateCost,
  getProviderPricing,
  calculateEffectiveRate,
  compareCosts,
} from './oracle.js';

// Re-export for convenience
export { default as Oracle } from './oracle.js';
