/**
 * CynicAccountant Config — C6.6 (CYNIC × ACCOUNT)
 *
 * Domain-specific accounting logic for CYNIC's internal operations.
 * Tracks costs of self-maintenance: memory operations, event routing,
 * learning loops, and meta-cognition.
 *
 * "Le chien compte ses propres pensées" — self-awareness of cost
 *
 * @module @cynic/node/cycle/configs/cynic-accountant
 */

'use strict';

import { PHI_INV_2, PHI_INV_3 } from '@cynic/core';
import { roundTo } from '@cynic/core';

/**
 * Accounting categories for CYNIC domain
 */
export const CynicAccountingCategories = {
  MEMORY: 'memory',           // DB queries, cache operations
  EVENTS: 'events',           // Event bus routing, bridge forwarding
  LEARNING: 'learning',       // Q-Learning, Thompson Sampling, SONA
  JUDGMENT: 'judgment',       // Judge operations, dimension scoring
  ROUTING: 'routing',         // Dog routing, KabbalisticRouter
  EMERGENCE: 'emergence',     // Emergence detection, residual analysis
  MAINTENANCE: 'maintenance', // GC, health checks, calibration
};

/**
 * Cost multipliers by category (φ-aligned)
 */
const COST_MULTIPLIERS = {
  [CynicAccountingCategories.MEMORY]: 1.0,
  [CynicAccountingCategories.EVENTS]: PHI_INV_3,      // 0.236 - lightweight
  [CynicAccountingCategories.LEARNING]: PHI_INV_2,    // 0.382 - moderate
  [CynicAccountingCategories.JUDGMENT]: 1.0,          // Full cost
  [CynicAccountingCategories.ROUTING]: PHI_INV_3,     // 0.236 - lightweight
  [CynicAccountingCategories.EMERGENCE]: PHI_INV_2,   // 0.382 - moderate
  [CynicAccountingCategories.MAINTENANCE]: PHI_INV_3, // 0.236 - lightweight
};

export const cynicAccountantConfig = {
  name: 'CynicAccountant',
  cell: 'C6.6',
  dimension: 'CYNIC',
  eventPrefix: 'cynic',
  categories: CynicAccountingCategories,

  /**
   * Account for a CYNIC operation.
   *
   * @param {string} category - CynicAccountingCategories value
   * @param {Object} data - Operation data
   * @param {Object} context - Additional context
   * @returns {Object} Cost record
   */
  account(category, data, context) {
    const {
      dbQueries = 0,
      dbLatencyMs = 0,
      eventsRouted = 0,
      llmTokens = 0,           // If CYNIC triggers LLM (rare)
      computeMs = 0,
      memoryDeltaMB = 0,
      apiCalls = 0,
    } = data;

    const timestamp = Date.now();
    const multiplier = COST_MULTIPLIERS[category] || 1.0;

    // Calculate costs
    const costs = {
      // Database costs (assume local PostgreSQL, near-zero monetary cost)
      database: {
        queries: dbQueries,
        latencyMs: dbLatencyMs,
        estimatedCostUSD: dbQueries * 0.00001, // Nominal cost for tracking
      },

      // Event routing costs (in-memory, zero monetary cost)
      events: {
        routedCount: eventsRouted,
        estimatedCostUSD: 0,
      },

      // LLM costs (if CYNIC triggers meta-cognition via LLM)
      llm: {
        tokens: llmTokens,
        estimatedCostUSD: this._estimateLLMCost(llmTokens),
      },

      // Compute costs (CPU time, local)
      compute: {
        ms: computeMs,
        estimatedCostUSD: computeMs * 0.0000001, // Nominal cost
      },

      // Memory costs
      memory: {
        deltaMB: memoryDeltaMB,
        estimatedCostUSD: memoryDeltaMB * 0.000001, // Nominal cost
      },

      // External API calls
      api: {
        calls: apiCalls,
        estimatedCostUSD: apiCalls * 0.0001, // Nominal cost per call
      },
    };

    // Total cost (weighted by category)
    const totalCostUSD = multiplier * (
      costs.database.estimatedCostUSD +
      costs.events.estimatedCostUSD +
      costs.llm.estimatedCostUSD +
      costs.compute.estimatedCostUSD +
      costs.memory.estimatedCostUSD +
      costs.api.estimatedCostUSD
    );

    // Calculate value score (CYNIC operations produce meta-value)
    const valueScore = this._calculateValue(category, data, costs);

    return {
      timestamp,
      category,
      costs,
      costUSD: roundTo(totalCostUSD, 6),
      tokens: llmTokens,
      computeMs,
      apiCalls,
      valueScore: roundTo(valueScore, 3),
      efficiency: totalCostUSD > 0 ? roundTo(valueScore / totalCostUSD, 2) : 0,
    };
  },

  /**
   * Estimate LLM cost from tokens (delegates to CostLedger if available).
   * @private
   */
  _estimateLLMCost(tokens) {
    if (!tokens) return 0;
    // Assume Sonnet pricing: ~$3/1M input, ~$15/1M output (50/50 split)
    const avgCostPer1M = (3 + 15) / 2; // $9/1M
    return roundTo((tokens / 1_000_000) * avgCostPer1M, 6);
  },

  /**
   * Calculate value score for CYNIC operations.
   * @private
   */
  _calculateValue(category, data, costs) {
    let value = 0;

    // Learning operations produce high value
    if (category === CynicAccountingCategories.LEARNING) {
      value = (data.learningSamples || 0) * PHI_INV_2; // 0.382 per sample
    }

    // Judgments produce value (quality improvement)
    if (category === CynicAccountingCategories.JUDGMENT) {
      value = (data.judgmentsCount || 0) * PHI_INV_3; // 0.236 per judgment
    }

    // Emergence detection produces high value
    if (category === CynicAccountingCategories.EMERGENCE) {
      value = (data.patternsDetected || 0) * PHI_INV_2; // 0.382 per pattern
    }

    // Event routing produces baseline value
    if (category === CynicAccountingCategories.EVENTS) {
      value = (costs.events.routedCount || 0) * 0.01; // Small per-event value
    }

    // Memory operations produce utility value
    if (category === CynicAccountingCategories.MEMORY) {
      value = (costs.database.queries || 0) * 0.005; // Small per-query value
    }

    // Maintenance produces stability value
    if (category === CynicAccountingCategories.MAINTENANCE) {
      value = (data.checksPerformed || 0) * 0.1; // Moderate per-check value
    }

    return Math.max(0, value);
  },

  /**
   * Custom health check for CynicAccountant.
   */
  healthCheck(stats, summary) {
    const total = stats.total;

    // Return idle status when no operations recorded
    if (total === 0) {
      return {
        status: 'idle',
        score: 0,
        totalRecords: 0,
        avgCostUSD: 0,
        totalCostUSD: 0,
        efficiency: 0,
      };
    }

    const avgCost = summary.totalCostUSD / total;

    // CYNIC operations should be mostly Tier 1+2 (zero/low cost)
    const isHealthy = avgCost < 0.001; // < $0.001 per operation

    return {
      status: isHealthy ? 'efficient' : 'review_costs',
      score: isHealthy ? PHI_INV_2 : 0.2,
      totalRecords: total,
      avgCostUSD: roundTo(avgCost, 6),
      totalCostUSD: roundTo(summary.totalCostUSD, 4),
      efficiency: summary.totalCostUSD > 0
        ? roundTo(stats.totalValue / summary.totalCostUSD, 2)
        : 0,
    };
  },

  extraStatFields: ['efficiency'],
};
