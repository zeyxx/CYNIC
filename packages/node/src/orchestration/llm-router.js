/**
 * LLMRouter - Cost-Aware Multi-Model Routing with Budget Enforcement
 *
 * Routes LLM requests to Ollama (free) or Anthropic (paid) based on:
 * - Task complexity
 * - Budget status
 * - Thompson Sampling exploration
 *
 * HARD ENFORCEMENT (Circuit Breaker):
 * - Budget exhausted → BLOCK all Anthropic calls
 * - Budget critical → BLOCK non-essential Anthropic calls
 * - Graceful degradation: ABUNDANT → WARNING → CRITICAL → EXHAUSTED
 *
 * "Don't pay for what you don't need" - κυνικός
 *
 * @module @cynic/node/orchestration/llm-router
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { getModelIntelligence } from '../learning/model-intelligence.js';
import { getCostLedger, BudgetStatus } from '../accounting/cost-ledger.js';
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
 * Task priority levels (for circuit breaker)
 */
export const PRIORITY = {
  CRITICAL: 'critical',   // Always allowed (errors, safety)
  HIGH: 'high',           // Allowed until EXHAUSTED
  NORMAL: 'normal',       // Blocked at CRITICAL
  LOW: 'low',             // Blocked at WARNING
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
 * Budget circuit breaker error
 */
export class BudgetExhaustedError extends Error {
  constructor(budgetStatus, suggestion) {
    super(`Budget ${budgetStatus.level}: ${suggestion}`);
    this.name = 'BudgetExhaustedError';
    this.budgetStatus = budgetStatus;
    this.suggestion = suggestion;
  }
}

/**
 * LLMRouter - Routes to cheapest capable model with budget enforcement
 */
export class LLMRouter {
  constructor(options = {}) {
    this.modelIntelligence = options.modelIntelligence || getModelIntelligence();
    this.costLedger = options.costLedger || getCostLedger();
    this.db = options.db || null;

    // Circuit breaker state
    this.circuitBreaker = {
      anthropicBlocked: false,
      blockedSince: null,
      blockReason: null,
    };

    // Stats
    this.stats = {
      routesTotal: 0,
      routesOllama: 0,
      routesAnthropic: 0,
      routesBlocked: 0,       // Blocked by circuit breaker
      routesDegraded: 0,      // Degraded from Anthropic to Ollama
      explorations: 0,        // Thompson Sampling explorations
      costSaved: 0,
    };
  }

  /**
   * Check if Anthropic calls are allowed based on budget status.
   *
   * @param {Object} budgetStatus - From costLedger.getBudgetStatus()
   * @param {string} priority - Task priority (CRITICAL/HIGH/NORMAL/LOW)
   * @returns {Object} { allowed: boolean, reason: string }
   * @private
   */
  _checkBudgetCircuitBreaker(budgetStatus, priority = PRIORITY.NORMAL) {
    const level = budgetStatus.level;

    // CRITICAL priority → always allow (safety, errors)
    if (priority === PRIORITY.CRITICAL) {
      return { allowed: true, reason: 'critical_priority' };
    }

    // EXHAUSTED → block everything except CRITICAL
    if (level === BudgetStatus.EXHAUSTED) {
      return {
        allowed: false,
        reason: 'budget_exhausted',
        suggestion: 'Increase budget or wait for reset. Using Ollama fallback.',
      };
    }

    // CRITICAL → block NORMAL and LOW priority
    if (level === BudgetStatus.CRITICAL) {
      if (priority === PRIORITY.HIGH) {
        return { allowed: true, reason: 'high_priority_allowed' };
      }
      return {
        allowed: false,
        reason: 'budget_critical',
        suggestion: 'Budget critical. Using Ollama for non-essential tasks.',
      };
    }

    // CAUTIOUS → block LOW priority only
    if (level === BudgetStatus.CAUTIOUS) {
      if (priority === PRIORITY.LOW) {
        return {
          allowed: false,
          reason: 'budget_cautious_low_priority',
          suggestion: 'Budget cautious. Using Ollama for low-priority tasks.',
        };
      }
      return { allowed: true, reason: 'cautious_but_allowed' };
    }

    // MODERATE/ABUNDANT → allow all
    return { allowed: true, reason: 'budget_ok' };
  }

  /**
   * Route LLM request to optimal provider with budget enforcement.
   *
   * @param {Object} task - Task to route
   * @param {string} task.type - Task type (code, chat, analysis, etc.)
   * @param {string} task.complexity - Complexity tier (simple/moderate/complex)
   * @param {string} [task.priority='normal'] - Priority level (critical/high/normal/low)
   * @param {number} [task.estimatedTokens=1000] - Estimated token count
   * @param {Object} [task.context={}] - Additional context
   * @param {boolean} [task.throwOnBlock=false] - Throw error if blocked (vs fallback to Ollama)
   * @returns {Promise<Object>} Routing decision
   * @throws {BudgetExhaustedError} If throwOnBlock=true and budget blocks Anthropic
   */
  async route(task) {
    const {
      type,
      complexity = COMPLEXITY.MODERATE,
      priority = PRIORITY.NORMAL,
      estimatedTokens = 1000,
      context = {},
      throwOnBlock = false,
    } = task;

    this.stats.routesTotal++;

    // Get budget status
    const budgetStatus = this.costLedger.getBudgetStatus();

    // Circuit breaker check
    const circuitCheck = this._checkBudgetCircuitBreaker(budgetStatus, priority);

    // Update circuit breaker state
    if (!circuitCheck.allowed && !this.circuitBreaker.anthropicBlocked) {
      this.circuitBreaker.anthropicBlocked = true;
      this.circuitBreaker.blockedSince = Date.now();
      this.circuitBreaker.blockReason = circuitCheck.reason;

      log.warn('Budget circuit breaker ENGAGED', {
        level: budgetStatus.level,
        reason: circuitCheck.reason,
        consumedRatio: budgetStatus.consumedRatio,
      });
    } else if (circuitCheck.allowed && this.circuitBreaker.anthropicBlocked) {
      // Circuit breaker reset
      const blockedDuration = Date.now() - this.circuitBreaker.blockedSince;
      log.info('Budget circuit breaker RESET', {
        level: budgetStatus.level,
        blockedDurationMs: blockedDuration,
      });
      this.circuitBreaker.anthropicBlocked = false;
      this.circuitBreaker.blockedSince = null;
      this.circuitBreaker.blockReason = null;
    }

    // Get Thompson Sampling recommendation from ModelIntelligence
    const modelSelection = this.modelIntelligence.selectModel(type || 'general', {
      budgetLevel: budgetStatus.level,
      needsReasoning: complexity === COMPLEXITY.COMPLEX,
    });

    // Decision logic
    let provider = PROVIDERS.anthropic;
    let reason = 'default';
    let degraded = false;

    // 1. Circuit breaker BLOCKS Anthropic → Force Ollama or throw
    if (!circuitCheck.allowed) {
      if (throwOnBlock) {
        this.stats.routesBlocked++;
        throw new BudgetExhaustedError(budgetStatus, circuitCheck.suggestion);
      }

      // Fallback to Ollama
      provider = PROVIDERS.ollama;
      reason = circuitCheck.reason;
      degraded = true;
      this.stats.routesDegraded++;
    }
    // 2. Simple tasks → Ollama (no exploration needed)
    else if (complexity === COMPLEXITY.SIMPLE) {
      provider = PROVIDERS.ollama;
      reason = 'simple_task';
    }
    // 3. Complex tasks → Anthropic (quality needed, if budget allows)
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
      degraded,                  // Was this downgraded due to budget?
      circuitBreakerActive: this.circuitBreaker.anthropicBlocked,
      confidence: reason.includes('thompson') ? modelSelection.confidence : 0.8,
      estimatedCost: provider.cost * (estimatedTokens / 1_000_000),
      budgetLevel: budgetStatus.level,
    };

    // Log decision
    log.info('LLMRouter.route', {
      taskType: type,
      complexity,
      priority,
      provider: provider.name,
      reason,
      degraded,
      budgetLevel: budgetStatus.level,
      circuitBreaker: this.circuitBreaker.anthropicBlocked,
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
          priority,
          degraded,
          budgetLevel: budgetStatus.level,
          circuitBreakerActive: this.circuitBreaker.anthropicBlocked,
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
      anthropicRatio: this.stats.routesTotal > 0
        ? this.stats.routesAnthropic / this.stats.routesTotal
        : 0,
      degradationRate: this.stats.routesTotal > 0
        ? this.stats.routesDegraded / this.stats.routesTotal
        : 0,
      blockRate: this.stats.routesTotal > 0
        ? this.stats.routesBlocked / this.stats.routesTotal
        : 0,
      explorationRate: this.stats.routesTotal > 0
        ? this.stats.explorations / this.stats.routesTotal
        : 0,
      circuitBreaker: { ...this.circuitBreaker },
    };
  }

  /**
   * Get circuit breaker state
   */
  getCircuitBreakerState() {
    return { ...this.circuitBreaker };
  }

  /**
   * Reset circuit breaker manually (for testing or manual override)
   */
  resetCircuitBreaker() {
    const wasBlocked = this.circuitBreaker.anthropicBlocked;
    this.circuitBreaker.anthropicBlocked = false;
    this.circuitBreaker.blockedSince = null;
    this.circuitBreaker.blockReason = null;

    if (wasBlocked) {
      log.info('Circuit breaker manually reset');
    }
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

export default { LLMRouter, getLLMRouter, resetLLMRouter, COMPLEXITY, PRIORITY, BudgetExhaustedError };
