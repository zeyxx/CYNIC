/**
 * LLMRouter - Cost-Aware Multi-Model Routing
 *
 * Routes LLM requests to Ollama (free) or Anthropic (paid) based on:
 * - Task complexity
 * - Budget status
 * - Thompson Sampling exploration
 *
 * "Don't pay for what you don't need" - κυνικός
 *
 * @module @cynic/node/orchestration/llm-router
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { getModelIntelligence } from '../learning/model-intelligence.js';
import { getCostLedger } from '../accounting/cost-ledger.js';
import { getPool } from '@cynic/persistence';

const log = createLogger('LLMRouter');

/**
 * Task complexity tiers
 */
const COMPLEXITY = {
  SIMPLE: 'simple',     // Ollama Haiku tier
  MODERATE: 'moderate', // Depends on budget
  COMPLEX: 'complex',   // Anthropic Sonnet tier
};

/**
 * Provider configuration
 */
const PROVIDERS = {
  ollama: {
    name: 'ollama',
    model: 'llama3.2:latest', // Or qwen2.5-coder:7b for code tasks
    cost: 0, // Free
    tier: 'LIGHT',
  },
  anthropic: {
    name: 'anthropic',
    model: 'claude-sonnet-4-5-20250929',
    cost: 0.003, // $3 per 1M input tokens (estimate)
    tier: 'MEDIUM',
  },
};

/**
 * LLMRouter - Routes to cheapest capable model
 */
export class LLMRouter {
  constructor(options = {}) {
    this.modelIntelligence = options.modelIntelligence || getModelIntelligence();
    this.costLedger = options.costLedger || getCostLedger();
    this.db = options.db || null;

    // Stats
    this.stats = {
      routesTotal: 0,
      routesOllama: 0,
      routesAnthropic: 0,
      explorations: 0, // Thompson Sampling explorations
      costSaved: 0,
    };
  }

  /**
   * Route LLM request to optimal provider
   *
   * @param {Object} task - Task to route
   * @param {string} task.type - Task type (code, chat, analysis, etc.)
   * @param {string} task.complexity - Complexity tier (simple/moderate/complex)
   * @param {number} [task.estimatedTokens=1000] - Estimated token count
   * @param {Object} [task.context={}] - Additional context
   * @returns {Promise<Object>} Routing decision
   */
  async route(task) {
    const { type, complexity = COMPLEXITY.MODERATE, estimatedTokens = 1000, context = {} } = task;

    this.stats.routesTotal++;

    // Get budget status
    const budgetStatus = this.costLedger.getBudgetStatus();

    // Get Thompson Sampling recommendation from ModelIntelligence
    const modelSelection = this.modelIntelligence.selectModel(type || 'general', {
      budgetLevel: budgetStatus.level,
      needsReasoning: complexity === COMPLEXITY.COMPLEX,
    });

    // Decision logic
    let provider = PROVIDERS.anthropic;
    let reason = 'default';

    // 1. Budget exhausted → Force Ollama
    if (budgetStatus.level === 'exhausted' || budgetStatus.level === 'critical') {
      provider = PROVIDERS.ollama;
      reason = `budget_${budgetStatus.level}`;
    }
    // 2. Simple tasks → Ollama (no exploration needed)
    else if (complexity === COMPLEXITY.SIMPLE) {
      provider = PROVIDERS.ollama;
      reason = 'simple_task';
    }
    // 3. Complex tasks → Anthropic (quality needed)
    else if (complexity === COMPLEXITY.COMPLEX) {
      provider = PROVIDERS.anthropic;
      reason = 'complex_task';
    }
    // 4. Moderate tasks → Thompson Sampling decides
    else {
      // Thompson Sampling: ModelIntelligence selected model
      // If it selected Ollama/Haiku → explore, otherwise exploit
      const shouldExplore = (modelSelection.model === 'ollama' || modelSelection.model === 'haiku');

      if (shouldExplore && budgetStatus.consumedRatio < PHI_INV) {
        // Explore Ollama (under budget → can afford exploration)
        provider = PROVIDERS.ollama;
        reason = 'thompson_explore';
        this.stats.explorations++;
      } else if (budgetStatus.consumedRatio > 0.5) {
        // Budget concern → prefer Ollama
        provider = PROVIDERS.ollama;
        reason = 'budget_conscious';
      } else {
        // Exploit best known model (Anthropic if budget allows)
        provider = PROVIDERS.anthropic;
        reason = 'thompson_exploit';
      }
    }

    // Track provider stats
    if (provider.name === 'ollama') {
      this.stats.routesOllama++;
      // Cost saved = what we would've paid with Anthropic
      const savedCost = PROVIDERS.anthropic.cost * (estimatedTokens / 1_000_000);
      this.stats.costSaved += savedCost;
    } else {
      this.stats.routesAnthropic++;
    }

    const result = {
      provider: provider.name,
      model: provider.model,
      tier: provider.tier,
      reason,
      confidence: reason.includes('thompson') ? modelSelection.confidence : 0.8,
      estimatedCost: provider.cost * (estimatedTokens / 1_000_000),
      budgetLevel: budgetStatus.level,
    };

    // Log decision
    log.info('LLMRouter.route', {
      taskType: type,
      complexity,
      provider: provider.name,
      reason,
      budgetLevel: budgetStatus.level,
    });

    // Record to routing_accuracy table (Wiring Gap 2, G1.5 metric)
    try {
      if (!this.db) {
        const { getPool } = await import('@cynic/persistence');
        this.db = getPool();
      }

      await this.db.query(`
        INSERT INTO routing_accuracy (
          router_type, event_type, confidence, metadata
        ) VALUES ($1, $2, $3, $4)
      `, [
        'llm',
        type || 'general',
        result.confidence,
        JSON.stringify({
          provider: provider.name,
          model: provider.model,
          tier: provider.tier,
          reason,
          complexity,
          budgetLevel: budgetStatus.level,
          estimatedTokens,
        })
      ]);
    } catch (err) {
      // Non-blocking
      log.debug('Failed to record LLM routing', { error: err.message });
    }

    return result;
  }

  /**
   * Get routing stats
   */
  getStats() {
    return {
      ...this.stats,
      ollamaRatio: this.stats.routesTotal > 0
        ? this.stats.routesOllama / this.stats.routesTotal
        : 0,
      explorationRate: this.stats.routesTotal > 0
        ? this.stats.explorations / this.stats.routesTotal
        : 0,
    };
  }
}

// Singleton
let _instance = null;

export function getLLMRouter() {
  if (!_instance) {
    _instance = new LLMRouter();
  }
  return _instance;
}

export function resetLLMRouter() {
  _instance = null;
}

export default LLMRouter;
