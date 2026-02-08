/**
 * CYNIC Cost Optimizer
 *
 * Extracted from TieredRouter - provides cost optimization
 * as a wrapper around KabbalisticRouter.
 *
 * "The smallest dog that can do the job" - κυνικός
 *
 * Features preserved:
 * - 3-tier cost structure (LOCAL/LIGHT/FULL)
 * - Cost tracking & savings calculation
 * - Latency tracking
 * - Complexity classification
 *
 * @module @cynic/node/routing/cost-optimizer
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV } from '@cynic/core';

// =============================================================================
// COMPLEXITY TIERS
// =============================================================================

/**
 * Complexity tiers for routing
 */
export const ComplexityTier = Object.freeze({
  LOCAL: 'local',   // No LLM needed - local resolution
  LIGHT: 'light',   // Simple LLM (Haiku-level)
  FULL: 'full',     // Complex LLM (Sonnet/Opus)
});

/**
 * Handler costs (relative units)
 */
export const TIER_COSTS = Object.freeze({
  [ComplexityTier.LOCAL]: 0,      // $0 - no API call
  [ComplexityTier.LIGHT]: 1,      // Baseline (Haiku)
  [ComplexityTier.FULL]: 15,      // ~15x Haiku cost (Sonnet/Opus)
});

/**
 * Handler latencies (approximate ms)
 */
export const TIER_LATENCIES = Object.freeze({
  [ComplexityTier.LOCAL]: 1,      // <1ms local
  [ComplexityTier.LIGHT]: 500,    // ~500ms Haiku
  [ComplexityTier.FULL]: 3000,    // ~3s Sonnet/Opus
});

/**
 * Complexity thresholds for classification
 */
export const COMPLEXITY_THRESHOLDS = Object.freeze({
  // Content length thresholds
  SHORT_CONTENT: 100,
  LONG_CONTENT: 500,

  // Keyword density for complexity
  SIMPLE_KEYWORDS: ['list', 'show', 'get', 'check', 'status'],
  COMPLEX_KEYWORDS: ['analyze', 'design', 'architect', 'refactor', 'optimize', 'security', 'review'],
});

// =============================================================================
// COMPLEXITY CLASSIFIER
// =============================================================================

/**
 * Classifies request complexity for tier selection
 */
export class ComplexityClassifier {
  constructor() {
    this.stats = {
      classifications: 0,
      byTier: { local: 0, light: 0, full: 0 },
    };
  }

  /**
   * Classify request complexity
   *
   * @param {Object} request - Request to classify
   * @param {string} request.content - Request content
   * @param {string} [request.type] - Request type
   * @param {Object} [request.context] - Additional context
   * @returns {Object} Classification result
   */
  classify(request) {
    const content = (request.content || '').toLowerCase();
    const type = request.type || '';

    this.stats.classifications++;

    // Check for LOCAL tier patterns
    if (this._isLocalResolvable(content, type)) {
      this.stats.byTier.local++;
      return {
        tier: ComplexityTier.LOCAL,
        complexity: 'low',
        reason: 'Pattern matches local resolution',
      };
    }

    // Check for FULL tier patterns
    if (this._isComplex(content, type, request.context)) {
      this.stats.byTier.full++;
      return {
        tier: ComplexityTier.FULL,
        complexity: 'high',
        reason: 'Complex analysis required',
      };
    }

    // Default to LIGHT
    this.stats.byTier.light++;
    return {
      tier: ComplexityTier.LIGHT,
      complexity: 'medium',
      reason: 'Standard processing',
    };
  }

  /**
   * Check if request can be resolved locally
   * @private
   */
  _isLocalResolvable(content, type) {
    // File existence checks
    if (content.match(/^(does|is) .+ exist/i)) return true;

    // List operations
    if (content.match(/^(list|show|get) (all )?(files?|dirs?)/i)) return true;

    // Git status
    if (content.match(/^git status/i)) return true;

    // Format/lint
    if (content.match(/^(format|lint|prettify)/i)) return true;

    // Simple keywords at start
    for (const keyword of COMPLEXITY_THRESHOLDS.SIMPLE_KEYWORDS) {
      if (content.startsWith(keyword)) return true;
    }

    return false;
  }

  /**
   * Check if request is complex
   * @private
   */
  _isComplex(content, type, context) {
    // When local LLM is available, raise the bar for FULL tier
    // LIGHT (Ollama) can handle medium complexity — save FULL for truly hard work
    const hasLocalLLM = context?.hasLocalLLM === true;
    const complexThreshold = hasLocalLLM
      ? COMPLEXITY_THRESHOLDS.LONG_CONTENT * 2  // 1000 chars with local LLM
      : COMPLEXITY_THRESHOLDS.LONG_CONTENT;      // 500 chars without

    // Long content is complex
    if (content.length > complexThreshold) return true;

    // Complex keywords — but with local LLM, only multi-keyword triggers FULL
    const complexHits = COMPLEXITY_THRESHOLDS.COMPLEX_KEYWORDS.filter(k => content.includes(k));
    if (hasLocalLLM ? complexHits.length >= 2 : complexHits.length >= 1) return true;

    // Security-related — always FULL regardless of local LLM
    if (content.match(/security|vulnerab|credential|auth/i)) return true;

    // Multi-step operations
    if (content.match(/and then|after that|finally|step \d/i)) return true;

    // Context hints
    if (context?.complexity === 'high') return true;
    if (context?.risk === 'high' || context?.risk === 'critical') return true;

    return false;
  }

  /**
   * Get classification stats
   */
  getStats() {
    return { ...this.stats };
  }

  /**
   * Reset stats
   */
  resetStats() {
    this.stats = {
      classifications: 0,
      byTier: { local: 0, light: 0, full: 0 },
    };
  }
}

// =============================================================================
// COST OPTIMIZER
// =============================================================================

/**
 * CostOptimizer - Wraps routing with cost optimization
 *
 * Usage:
 * const optimizer = new CostOptimizer();
 * const { tier, shouldRoute } = optimizer.optimize(request);
 * if (shouldRoute) {
 *   // Route through KabbalisticRouter
 * }
 * optimizer.recordOutcome(tier, success, latency);
 */
export class CostOptimizer extends EventEmitter {
  /**
   * @param {Object} [options] - Options
   * @param {ComplexityClassifier} [options.classifier] - Custom classifier
   * @param {Object} [options.budgetLimits] - Budget constraints
   * @param {boolean} [options.trackSavings=true] - Track cost savings
   */
  constructor(options = {}) {
    super();

    this.classifier = options.classifier || new ComplexityClassifier();
    this.trackSavings = options.trackSavings !== false;

    // Budget limits (optional)
    this.budgetLimits = {
      maxTier: options.budgetLimits?.maxTier || ComplexityTier.FULL,
      maxCostPerRequest: options.budgetLimits?.maxCostPerRequest || TIER_COSTS.full,
      dailyBudget: options.budgetLimits?.dailyBudget || Infinity,
    };

    // Statistics
    this.stats = {
      optimized: 0,
      byTier: { local: 0, light: 0, full: 0 },
      costSaved: 0,
      totalCost: 0,
      avgLatency: 0,
      budgetExceeded: 0,
    };

    // Daily tracking
    this._dailyCost = 0;
    this._dailyReset = this._getNextMidnight();
  }

  /**
   * Optimize a request - determine best tier
   *
   * @param {Object} request - Request to optimize
   * @returns {Object} Optimization result
   */
  optimize(request) {
    // Reset daily cost if needed
    this._checkDailyReset();

    // Classify complexity
    const classification = this.classifier.classify(request);
    let { tier, complexity, reason } = classification;

    // Apply budget constraints
    const budgetResult = this._applyBudgetConstraints(tier);
    if (budgetResult.constrained) {
      tier = budgetResult.tier;
      reason = budgetResult.reason;
    }

    this.stats.optimized++;
    this.stats.byTier[tier]++;

    this.emit('optimized', { request, tier, complexity, reason });

    return {
      tier,
      complexity,
      reason,
      cost: TIER_COSTS[tier],
      estimatedLatency: TIER_LATENCIES[tier],
      shouldRoute: tier !== ComplexityTier.LOCAL,
    };
  }

  /**
   * Record outcome after routing
   *
   * @param {string} tier - Tier used
   * @param {boolean} success - Whether routing succeeded
   * @param {number} latency - Actual latency in ms
   */
  recordOutcome(tier, success, latency) {
    const cost = TIER_COSTS[tier];

    // Update costs
    this.stats.totalCost += cost;
    this._dailyCost += cost;

    // Calculate savings (compared to always using FULL)
    if (this.trackSavings && tier !== ComplexityTier.FULL) {
      const saved = TIER_COSTS.full - cost;
      this.stats.costSaved += saved;
    }

    // Update latency average
    const total = this.stats.byTier.local + this.stats.byTier.light + this.stats.byTier.full;
    this.stats.avgLatency = (this.stats.avgLatency * (total - 1) + latency) / total;

    this.emit('outcome', { tier, success, latency, cost });
  }

  /**
   * Apply budget constraints to tier selection
   * @private
   */
  _applyBudgetConstraints(requestedTier) {
    const requestedCost = TIER_COSTS[requestedTier];

    // Check max tier
    const tierOrder = [ComplexityTier.LOCAL, ComplexityTier.LIGHT, ComplexityTier.FULL];
    const maxTierIndex = tierOrder.indexOf(this.budgetLimits.maxTier);
    const requestedIndex = tierOrder.indexOf(requestedTier);

    if (requestedIndex > maxTierIndex) {
      return {
        constrained: true,
        tier: this.budgetLimits.maxTier,
        reason: `Budget constraint: max tier is ${this.budgetLimits.maxTier}`,
      };
    }

    // Check per-request cost
    if (requestedCost > this.budgetLimits.maxCostPerRequest) {
      // Find highest tier within budget
      for (let i = requestedIndex - 1; i >= 0; i--) {
        if (TIER_COSTS[tierOrder[i]] <= this.budgetLimits.maxCostPerRequest) {
          return {
            constrained: true,
            tier: tierOrder[i],
            reason: `Budget constraint: max cost per request is ${this.budgetLimits.maxCostPerRequest}`,
          };
        }
      }
    }

    // Check daily budget
    if (this._dailyCost + requestedCost > this.budgetLimits.dailyBudget) {
      this.stats.budgetExceeded++;
      // Force local if over daily budget
      return {
        constrained: true,
        tier: ComplexityTier.LOCAL,
        reason: 'Daily budget exceeded',
      };
    }

    return { constrained: false };
  }

  /**
   * Check and reset daily cost
   * @private
   */
  _checkDailyReset() {
    if (Date.now() >= this._dailyReset) {
      this._dailyCost = 0;
      this._dailyReset = this._getNextMidnight();
    }
  }

  /**
   * Get next midnight timestamp
   * @private
   */
  _getNextMidnight() {
    const now = new Date();
    const midnight = new Date(now);
    midnight.setHours(24, 0, 0, 0);
    return midnight.getTime();
  }

  /**
   * Get optimization statistics
   */
  getStats() {
    const total = this.stats.optimized || 1;

    // Calculate percentages
    const localPercent = (this.stats.byTier.local / total) * 100;
    const lightPercent = (this.stats.byTier.light / total) * 100;
    const fullPercent = (this.stats.byTier.full / total) * 100;

    // Calculate savings percentage
    const maxPossibleCost = total * TIER_COSTS.full;
    const savingsPercent = maxPossibleCost > 0
      ? (this.stats.costSaved / maxPossibleCost) * 100
      : 0;

    return {
      ...this.stats,
      localPercent: Math.round(localPercent * 10) / 10,
      lightPercent: Math.round(lightPercent * 10) / 10,
      fullPercent: Math.round(fullPercent * 10) / 10,
      savingsPercent: Math.round(savingsPercent * 10) / 10,
      dailyCost: this._dailyCost,
      dailyBudgetRemaining: this.budgetLimits.dailyBudget - this._dailyCost,
      classifierStats: this.classifier.getStats(),
    };
  }

  /**
   * Reset statistics
   */
  resetStats() {
    this.stats = {
      optimized: 0,
      byTier: { local: 0, light: 0, full: 0 },
      costSaved: 0,
      totalCost: 0,
      avgLatency: 0,
      budgetExceeded: 0,
    };
    this.classifier.resetStats();
  }

  /**
   * Set budget limits
   *
   * @param {Object} limits - New budget limits
   */
  setBudgetLimits(limits) {
    this.budgetLimits = {
      ...this.budgetLimits,
      ...limits,
    };
  }

  /**
   * Get tier info
   *
   * @param {string} tier - Tier name
   * @returns {Object} Cost and latency info
   */
  static getTierInfo(tier) {
    return {
      tier,
      cost: TIER_COSTS[tier],
      latency: TIER_LATENCIES[tier],
    };
  }
}

/**
 * Create a CostOptimizer instance
 */
export function createCostOptimizer(options = {}) {
  return new CostOptimizer(options);
}

// Singleton instance
let _optimizer = null;

/**
 * Get the global CostOptimizer instance
 */
export function getCostOptimizer(options) {
  if (!_optimizer) {
    _optimizer = createCostOptimizer(options);
  }
  return _optimizer;
}

export default {
  CostOptimizer,
  ComplexityClassifier,
  ComplexityTier,
  TIER_COSTS,
  TIER_LATENCIES,
  COMPLEXITY_THRESHOLDS,
  createCostOptimizer,
  getCostOptimizer,
};
