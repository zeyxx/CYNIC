/**
 * CYNIC Pricing Oracle
 *
 * Real-time pricing for all LLM providers.
 * NOTHING IS FREE - CYNIC knows real costs.
 *
 * Prices as of 2026-02:
 * - Anthropic: $3-15/M tokens (input), $15-75/M tokens (output)
 * - OpenAI: $0.5-15/M tokens
 * - Gemini: $0-3.5/M tokens
 * - Ollama: GPU electricity + amortized hardware
 * - Claude Code: $20/month subscription (amortized)
 *
 * @module @cynic/llm/pricing/oracle
 */

'use strict';

import { createLogger, PHI, PHI_INV } from '@cynic/core';

const log = createLogger('PricingOracle');

/**
 * Real pricing data (USD per 1M tokens)
 * Source: Official API pricing pages (Feb 2026)
 */
export const PROVIDER_PRICING = {
  anthropic: {
    'claude-opus-4-5-20251101': {
      input: 15.0,  // $15/M
      output: 75.0, // $75/M
      context: 200000,
    },
    'claude-sonnet-4-5-20251101': {
      input: 3.0,   // $3/M
      output: 15.0, // $15/M
      context: 200000,
    },
    'claude-haiku-3-5-20251101': {
      input: 0.8,   // $0.8/M
      output: 4.0,  // $4/M
      context: 200000,
    },
    default: {
      input: 3.0,
      output: 15.0,
      context: 200000,
    },
  },
  openai: {
    'gpt-4o': {
      input: 2.5,
      output: 10.0,
      context: 128000,
    },
    'gpt-4o-mini': {
      input: 0.15,
      output: 0.6,
      context: 128000,
    },
    'o1': {
      input: 15.0,
      output: 60.0,
      context: 200000,
    },
    default: {
      input: 2.5,
      output: 10.0,
      context: 128000,
    },
  },
  google: {
    'gemini-2.0-flash': {
      input: 0.0,   // Free until quota
      output: 0.0,
      context: 1000000,
    },
    'gemini-1.5-pro': {
      input: 1.25,
      output: 5.0,
      context: 2000000,
    },
    'gemini-1.5-flash': {
      input: 0.075,
      output: 0.3,
      context: 1000000,
    },
    default: {
      input: 0.075,
      output: 0.3,
      context: 1000000,
    },
  },
  ollama: {
    // Ollama is "free" but has real costs:
    // - GPU: $0.30-0.50/hour electricity (RTX 4090 ~350W)
    // - Hardware amortized: ~$1500 GPU / 3 years / 8760 hours = $0.057/hour
    // - Total: ~$0.36-0.56/hour for inference
    default: {
      input: 0.0,   // GPU electricity is real cost but hard to calculate per-token
      output: 0.0,
      costPerHour: 0.40, // Average GPU cost/hour
      model: 'llama3.3:70b', // Default model
    },
  },
  claudeCode: {
    // Claude Code subscription: $20/month
    // Average usage: ~2M tokens/month = $10/M effective
    // But this is FIXED cost regardless of usage
    default: {
      input: 0,  // Subscription-based, not per-token
      output: 0,
      subscription: 20.0, // $/month
      amortizedPerMToken: 10.0, // If using 2M tokens/month
    },
  },
  websocketClaude: {
    // Same as Claude Code - uses subscription
    default: {
      input: 0,
      output: 0,
      subscription: 20.0,
      amortizedPerMToken: 10.0,
    },
  },
  airllm: {
    // AirLLM uses disk offloading - no GPU needed
    // But has SSD I/O costs and CPU costs
    default: {
      input: 0,
      output: 0,
      costPerHour: 0.05, // Just SSD/CPU cost
    },
  },
};

/**
 * Default configuration for cost calculation
 */
export const DEFAULT_CONFIG = {
  // Electricity cost (USD per kWh)
  electricityRate: 0.12,
  // Claude Code subscription ($/month)
  claudeSubscription: 20.0,
  // Expected Claude Code usage (tokens/month)
  expectedClaudeCodeUsage: 2000000,
  // GPU hourly cost (electricity + amortized)
  gpuCostPerHour: 0.40,
  // AirLLM cost per hour (SSD I/O + CPU)
  airllmCostPerHour: 0.05,
};

/**
 * Calculate real cost for a request
 */
export function calculateCost(provider, model, inputTokens, outputTokens, config = DEFAULT_CONFIG) {
  const pricing = PROVIDER_PRICING[provider];
  if (!pricing) {
    log.warn('Unknown provider', { provider });
    return { cost: 0, isEstimate: true };
  }

  const modelPricing = pricing[model] || pricing.default;
  
  // Handle subscription-based providers
  if (modelPricing.subscription) {
    const amortized = modelPricing.amortizedPerMToken || 10.0;
    const inputCost = (inputTokens / 1000000) * amortized;
    const outputCost = (outputTokens / 1000000) * amortized;
    return {
      cost: inputCost + outputCost,
      isEstimate: false,
      type: 'subscription',
      details: {
        subscription: modelPricing.subscription,
        amortizedPerM: amortized,
      },
    };
  }

  // Handle hourly rate providers (Ollama, AirLLM)
  if (modelPricing.costPerHour) {
    // Estimate based on average tokens/second for inference
    const avgTokensPerSecond = 50; // Conservative estimate
    const seconds = (inputTokens + outputTokens) / avgTokensPerSecond;
    const hours = seconds / 3600;
    const cost = hours * modelPricing.costPerHour;
    return {
      cost,
      isEstimate: true,
      type: 'hourly',
      details: {
        costPerHour: modelPricing.costPerHour,
        estimatedDuration: seconds,
      },
    };
  }

  // Handle per-token providers (Anthropic, OpenAI, Google)
  const inputCost = (inputTokens / 1000000) * (modelPricing.input || 0);
  const outputCost = (outputTokens / 1000000) * (modelPricing.output || 0);
  
  return {
    cost: inputCost + outputCost,
    isEstimate: false,
    type: 'perToken',
    details: {
      inputRate: modelPricing.input,
      outputRate: modelPricing.output,
      inputTokens,
      outputTokens,
    },
  };
}

/**
 * Get pricing for a provider
 */
export function getProviderPricing(provider) {
  return PROVIDER_PRICING[provider] || null;
}

/**
 * Calculate effective cost per 1M tokens for subscription providers
 */
export function calculateEffectiveRate(subscriptionCost, monthlyTokens) {
  if (monthlyTokens <= 0) return Infinity;
  return (subscriptionCost / monthlyTokens) * 1000000;
}

/**
 * Compare costs between providers
 */
export function compareCosts(providers, inputTokens, outputTokens, config = DEFAULT_CONFIG) {
  const results = [];

  for (const { provider, model } of providers) {
    const cost = calculateCost(provider, model || 'default', inputTokens, outputTokens, config);
    results.push({
      provider,
      model: model || 'default',
      cost: cost.cost,
      isEstimate: cost.isEstimate,
      type: cost.type,
    });
  }

  // Sort by cost
  results.sort((a, b) => a.cost - b.cost);

  return results;
}

/**
 * PricingOracle class - manages pricing and cost calculation
 */
export class PricingOracle {
  constructor(config = DEFAULT_CONFIG) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    this.pricing = { ...PROVIDER_PRICING };
    this.customPrices = new Map();
    this.listeners = [];
  }

  /**
   * Get cost for a request
   */
  getCost(provider, model, inputTokens, outputTokens) {
    // Check custom prices first
    const customKey = `${provider}:${model}`;
    if (this.customPrices.has(customKey)) {
      const custom = this.customPrices.get(customKey);
      return {
        cost: (inputTokens / 1000000) * custom.input + (outputTokens / 1000000) * custom.output,
        isEstimate: false,
        type: 'custom',
      };
    }

    return calculateCost(provider, model, inputTokens, outputTokens, this.config);
  }

  /**
   * Update pricing for a provider
   */
  setPrice(provider, model, prices) {
    if (!this.pricing[provider]) {
      this.pricing[provider] = {};
    }
    this.pricing[provider][model] = prices;
    this.customPrices.set(`${provider}:${model}`, prices);
    log.info('Price updated', { provider, model, prices });
    this._notifyListeners({ type: 'priceUpdate', provider, model, prices });
  }

  /**
   * Subscribe to price changes
   */
  subscribe(listener) {
    this.listeners.push(listener);
    return () => {
      this.listeners = this.listeners.filter(l => l !== listener);
    };
  }

  /**
   * Notify listeners of changes
   * @private
   */
  _notifyListeners(event) {
    for (const listener of this.listeners) {
      try {
        listener(event);
      } catch (e) {
        log.error('Listener error', { error: e.message });
      }
    }
  }

  /**
   * Get all available providers
   */
  getProviders() {
    return Object.keys(this.pricing);
  }

  /**
   * Get summary for logging
   */
  getSummary() {
    const summary = {};
    for (const [provider, models] of Object.entries(this.pricing)) {
      summary[provider] = Object.keys(models);
    }
    return summary;
  }
}

/**
 * Create PricingOracle instance
 */
export function createPricingOracle(config) {
  return new PricingOracle(config);
}

/**
 * Singleton instance
 */
let _oracle = null;

export function getOracle() {
  if (!_oracle) {
    _oracle = new PricingOracle();
  }
  return _oracle;
}

export default PricingOracle;
