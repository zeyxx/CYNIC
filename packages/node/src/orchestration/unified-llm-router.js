/**
 * UnifiedLLMRouter — One Router to Rule Them All
 *
 * Merges three fragmented LLM routing systems:
 * - llm-adapter.js: Consensus voting logic
 * - llm-router.js: Budget enforcement + cost routing
 * - llm-judgment-bridge.cjs: Ollama calling for hooks
 *
 * Provides unified API with composable strategies:
 * - cheapest: Route to cheapest capable model (budget-aware)
 * - consensus: Multi-LLM voting (semantic agreement ≥ φ⁻¹)
 * - best: Thompson Sampling (exploration + exploitation)
 * - fastest: Lowest latency provider
 *
 * φ-bounded confidence: max 61.8%
 * Budget circuit breaker: ABUNDANT → WARNING → CRITICAL → EXHAUSTED
 *
 * "Three routers merge. One truth emerges." — κυνικός
 *
 * @module @cynic/node/orchestration/unified-llm-router
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, PHI_INV_2 } from '@cynic/core';
import { calculateSemanticAgreement, SimilarityThresholds } from '@cynic/llm';
import { getModelIntelligence } from '../learning/model-intelligence.js';
import { getCostLedger, BudgetStatus } from '../accounting/cost-ledger.js';
import { getPool } from '@cynic/persistence';

const log = createLogger('UnifiedLLMRouter');

// ═══════════════════════════════════════════════════════════════════════════
// RESPONSE TYPES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Standardized LLM response
 */
export class LLMResponse {
  constructor(data = {}) {
    this.id = data.id || `resp-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    this.timestamp = data.timestamp || Date.now();
    this.provider = data.provider || 'unknown';
    this.model = data.model || 'unknown';
    this.content = data.content || '';
    this.confidence = Math.min(data.confidence || PHI_INV, PHI_INV); // φ⁻¹ cap
    this.tokens = data.tokens || { input: 0, output: 0 };
    this.duration = data.duration || 0;
    this.cost = data.cost || 0;
    this.metadata = data.metadata || {};
  }

  toJSON() {
    return {
      id: this.id,
      provider: this.provider,
      model: this.model,
      confidence: this.confidence,
      tokens: this.tokens,
      duration: this.duration,
      cost: this.cost,
    };
  }
}

/**
 * Consensus voting result
 */
export class ConsensusResult {
  constructor(data = {}) {
    this.id = data.id || `consensus-${Date.now()}`;
    this.timestamp = data.timestamp || Date.now();
    this.responses = data.responses || [];
    this.agreement = data.agreement || 0; // 0-1
    this.verdict = data.verdict || null;
    this.confidence = Math.min(data.confidence || 0, PHI_INV);
    this.dissent = data.dissent || [];
  }

  get hasConsensus() {
    return this.agreement >= PHI_INV; // φ⁻¹ = 61.8%
  }

  toJSON() {
    return {
      id: this.id,
      agreement: this.agreement,
      verdict: this.verdict,
      confidence: this.confidence,
      hasConsensus: this.hasConsensus,
      responseCount: this.responses.length,
      dissentCount: this.dissent.length,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// ENUMS
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Routing strategies
 */
export const Strategy = {
  CHEAPEST: 'cheapest',   // Minimize cost (budget-aware)
  CONSENSUS: 'consensus', // Multi-LLM voting (semantic agreement)
  BEST: 'best',           // Thompson Sampling (exploration/exploitation)
  FASTEST: 'fastest',     // Minimize latency
};

/**
 * Budget enforcement modes
 */
export const BudgetMode = {
  ENFORCE: 'enforce',  // Block expensive calls when budget critical
  WARN: 'warn',        // Log warning but proceed
  IGNORE: 'ignore',    // No budget checks
};

/**
 * Task priority levels (for circuit breaker)
 */
export const Priority = {
  CRITICAL: 'CRITICAL',  // Always allowed (errors, safety)
  HIGH: 'HIGH',          // Blocked at EXHAUSTED
  NORMAL: 'NORMAL',      // Blocked at CRITICAL
  LOW: 'LOW',            // Blocked at WARNING
};

/**
 * Task complexity tiers
 */
export const Complexity = {
  SIMPLE: 'simple',      // Ollama Haiku tier (free)
  MODERATE: 'moderate',  // Depends on budget
  COMPLEX: 'complex',    // Anthropic Sonnet tier (paid)
};

// ═══════════════════════════════════════════════════════════════════════════
// ERRORS
// ═══════════════════════════════════════════════════════════════════════════

export class BudgetExhaustedError extends Error {
  constructor(budgetStatus, suggestion) {
    super(`Budget ${budgetStatus.level}: ${suggestion}`);
    this.name = 'BudgetExhaustedError';
    this.budgetStatus = budgetStatus;
    this.suggestion = suggestion;
  }
}

export class ConsensusFailedError extends Error {
  constructor(agreement, quorum) {
    super(`Consensus failed: ${(agreement * 100).toFixed(1)}% < ${(quorum * 100).toFixed(1)}% required`);
    this.name = 'ConsensusFailedError';
    this.agreement = agreement;
    this.quorum = quorum;
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// PROVIDER ADAPTERS (reused from llm-adapter.js)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Abstract LLM provider adapter
 */
class LLMAdapter {
  constructor(options = {}) {
    this.provider = options.provider || 'unknown';
    this.enabled = options.enabled !== false;
    this.timeout = options.timeout || 10000;
  }

  /**
   * Complete a prompt (must be implemented by subclasses)
   * @param {string} prompt
   * @param {Object} options
   * @returns {Promise<LLMResponse>}
   */
  async complete(prompt, options = {}) {
    throw new Error('Not implemented');
  }

  /**
   * Health check
   * @returns {Promise<boolean>}
   */
  async health() {
    return this.enabled;
  }
}

/**
 * Claude Code adapter (via thin hooks → daemon → MCP/API)
 */
class ClaudeCodeAdapter extends LLMAdapter {
  constructor(options = {}) {
    super({ provider: 'anthropic', ...options });
    this.model = options.model || 'claude-sonnet-4-5-20250929';
  }

  async complete(prompt, options = {}) {
    // NOTE: This is a placeholder. Real implementation would:
    // 1. POST to daemon /llm endpoint
    // 2. Daemon routes to MCP or direct Anthropic API
    // 3. Returns standardized LLMResponse

    // For now, return mock response (to be implemented in daemon/llm-endpoints.js)
    return new LLMResponse({
      provider: this.provider,
      model: this.model,
      content: '[ClaudeCodeAdapter: Not yet wired to daemon LLM endpoint]',
      confidence: 0,
      tokens: { input: 0, output: 0 },
      duration: 0,
      cost: 0,
    });
  }
}

/**
 * Ollama adapter (local OSS LLMs)
 */
class OllamaAdapter extends LLMAdapter {
  constructor(options = {}) {
    super({ provider: 'ollama', ...options });
    this.model = options.model || 'llama3.2:latest';
    this.host = options.host || process.env.OLLAMA_HOST || 'http://localhost:11434';
  }

  async complete(prompt, options = {}) {
    try {
      const controller = new AbortController();
      const timer = setTimeout(() => controller.abort(), this.timeout);

      const res = await fetch(`${this.host}/api/generate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          model: this.model,
          prompt,
          stream: false,
        }),
        signal: controller.signal,
      });

      clearTimeout(timer);

      if (!res.ok) {
        throw new Error(`Ollama ${res.status}: ${res.statusText}`);
      }

      const data = await res.json();

      return new LLMResponse({
        provider: this.provider,
        model: this.model,
        content: data.response || '',
        confidence: PHI_INV, // φ⁻¹ default (no confidence from Ollama)
        tokens: { input: data.prompt_eval_count || 0, output: data.eval_count || 0 },
        duration: data.total_duration ? Math.round(data.total_duration / 1_000_000) : 0, // ns → ms
        cost: 0, // Free
      });
    } catch (err) {
      if (err.name === 'AbortError') {
        throw new Error(`Ollama timeout (${this.timeout}ms)`);
      }
      throw err;
    }
  }

  async health() {
    try {
      const res = await fetch(`${this.host}/api/tags`, {
        signal: AbortSignal.timeout(2000),
      });
      return res.ok;
    } catch {
      return false;
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// UNIFIED LLM ROUTER
// ═══════════════════════════════════════════════════════════════════════════

/**
 * UnifiedLLMRouter — merges three fragmented routers into one
 *
 * Features:
 * - Budget enforcement (from llm-router.js)
 * - Consensus voting (from llm-adapter.js)
 * - Thompson Sampling (from llm-router.js via ModelIntelligence)
 * - Multiple providers (Anthropic, Ollama, OpenAI, etc.)
 * - Composable strategies (cheapest, consensus, best, fastest)
 */
export class UnifiedLLMRouter extends EventEmitter {
  constructor(options = {}) {
    super();

    // Adapters
    this.adapters = {
      anthropic: new ClaudeCodeAdapter(options.anthropic),
      ollama: new OllamaAdapter(options.ollama),
    };

    // Dependencies
    this.costLedger = options.costLedger || getCostLedger();
    this.modelIntelligence = options.modelIntelligence || getModelIntelligence();
    this.pool = options.pool || (async () => getPool())(); // Lazy init

    // Consensus config
    this.consensusConfig = {
      quorum: options.quorum || PHI_INV,
      timeout: options.timeout || 10000,
      requirePrimary: options.requirePrimary !== false,
    };

    // Stats
    this.stats = {
      totalCalls: 0,
      budgetBlocks: 0,
      consensusCalls: 0,
      consensusReached: 0,
      consensusFailed: 0,
      byProvider: {},
      byStrategy: {},
    };

    log.info('UnifiedLLMRouter created', {
      adapters: Object.keys(this.adapters),
      quorum: this.consensusConfig.quorum,
    });
  }

  // ───────────────────────────────────────────────────────────────────────────
  // MAIN API
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Universal call interface
   *
   * @param {string} prompt
   * @param {Object} options
   * @param {string} [options.strategy='cheapest'] - Routing strategy
   * @param {string} [options.budget='enforce'] - Budget enforcement mode
   * @param {string} [options.priority='NORMAL'] - Task priority
   * @param {string} [options.complexity='moderate'] - Task complexity
   * @param {string[]} [options.models] - Specific models to use
   * @param {number} [options.timeout] - Request timeout
   * @param {number} [options.quorum] - Consensus threshold
   * @returns {Promise<LLMResponse|ConsensusResult>}
   */
  async call(prompt, options = {}) {
    this.stats.totalCalls++;

    const strategy = options.strategy || Strategy.CHEAPEST;
    const budgetMode = options.budget || BudgetMode.ENFORCE;
    const priority = options.priority || Priority.NORMAL;
    const complexity = options.complexity || Complexity.MODERATE;

    this.stats.byStrategy[strategy] = (this.stats.byStrategy[strategy] || 0) + 1;

    // 1. Budget check (if enforced)
    if (budgetMode === BudgetMode.ENFORCE) {
      const budgetStatus = this.costLedger.getStatus();
      const blocked = this._shouldBlockForBudget(budgetStatus, priority);

      if (blocked) {
        this.stats.budgetBlocks++;
        this.emit('budget:block', { budgetStatus, priority });

        // Degrade to free provider (Ollama)
        log.warn('Budget blocks paid LLM, degrading to Ollama', {
          budget: budgetStatus.level,
          priority,
        });

        return this._callProvider('ollama', prompt, options);
      }
    }

    // 2. Execute strategy
    switch (strategy) {
      case Strategy.CHEAPEST:
        return this._callCheapest(prompt, { complexity, ...options });

      case Strategy.CONSENSUS:
        return this._callConsensus(prompt, options);

      case Strategy.BEST:
        return this._callBest(prompt, { complexity, ...options });

      case Strategy.FASTEST:
        return this._callFastest(prompt, options);

      default:
        throw new Error(`Unknown strategy: ${strategy}`);
    }
  }

  // ───────────────────────────────────────────────────────────────────────────
  // LEGACY COMPATIBILITY
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Legacy: llm-router.js route() API
   */
  async route(task) {
    const { type, priority = Priority.NORMAL, throwOnBlock = false } = task;

    // Map task type → complexity
    const complexity = type === 'simple' ? Complexity.SIMPLE :
                      type === 'complex' ? Complexity.COMPLEX :
                      Complexity.MODERATE;

    try {
      const response = await this.call(task.prompt || '', {
        strategy: Strategy.CHEAPEST,
        budget: BudgetMode.ENFORCE,
        priority,
        complexity,
      });

      return {
        provider: response.provider,
        model: response.model,
        priority,
        cost: response.cost,
        rationale: `Routed to ${response.provider} (complexity=${complexity}, budget=${this.costLedger.getStatus().level})`,
      };
    } catch (err) {
      if (err instanceof BudgetExhaustedError && throwOnBlock) {
        throw err;
      }
      // Fallback to Ollama
      return {
        provider: 'ollama',
        model: this.adapters.ollama.model,
        priority,
        cost: 0,
        rationale: `Budget blocked, degraded to Ollama`,
      };
    }
  }

  /**
   * Legacy: llm-adapter.js consensus() API
   */
  async consensus(prompt, options = {}) {
    return this.call(prompt, {
      ...options,
      strategy: Strategy.CONSENSUS,
    });
  }

  // ───────────────────────────────────────────────────────────────────────────
  // STRATEGY IMPLEMENTATIONS
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Cheapest strategy: route to cheapest capable model
   */
  async _callCheapest(prompt, options) {
    const { complexity } = options;

    // SIMPLE tasks → Ollama (free)
    if (complexity === Complexity.SIMPLE) {
      return this._callProvider('ollama', prompt, options);
    }

    // COMPLEX tasks → Anthropic (paid, but necessary)
    if (complexity === Complexity.COMPLEX) {
      return this._callProvider('anthropic', prompt, options);
    }

    // MODERATE tasks → check budget, prefer Ollama
    const budgetStatus = this.costLedger.getStatus();
    const provider = budgetStatus.level === BudgetStatus.ABUNDANT ? 'anthropic' : 'ollama';

    return this._callProvider(provider, prompt, options);
  }

  /**
   * Best strategy: Thompson Sampling (exploration + exploitation)
   */
  async _callBest(prompt, options) {
    const { complexity, needsReasoning = false } = options;

    // Use ModelIntelligence Thompson Sampling
    const recommendation = this.modelIntelligence.selectModel(complexity, {
      needsReasoning,
    });

    const provider = recommendation.model.includes('claude') ? 'anthropic' : 'ollama';
    return this._callProvider(provider, prompt, options);
  }

  /**
   * Fastest strategy: lowest latency provider (TODO: track latencies)
   */
  async _callFastest(prompt, options) {
    // For now, Ollama is typically faster (local)
    // TODO: Track p50/p95 latencies per provider
    return this._callProvider('ollama', prompt, options);
  }

  /**
   * Consensus strategy: multi-LLM voting
   */
  async _callConsensus(prompt, options) {
    this.stats.consensusCalls++;

    const quorum = options.quorum || this.consensusConfig.quorum;
    const timeout = options.timeout || this.consensusConfig.timeout;

    // Get all enabled adapters
    const enabled = Object.entries(this.adapters)
      .filter(([_, adapter]) => adapter.enabled)
      .map(([name, adapter]) => ({ name, adapter }));

    if (enabled.length < 2) {
      log.warn('Not enough adapters for consensus, using single provider');
      const response = await this._callProvider(enabled[0]?.name || 'ollama', prompt, options);
      return new ConsensusResult({
        responses: [response],
        agreement: 1.0,
        verdict: response.content,
        confidence: response.confidence,
      });
    }

    // Request from all adapters in parallel
    const responsePromises = enabled.map(({ name, adapter }) =>
      Promise.race([
        adapter.complete(prompt, options).then(r => ({ ...r, provider: name })),
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error(`${name} timeout`)), timeout)
        ),
      ]).catch(err => {
        log.debug(`Adapter ${name} failed`, { error: err.message });
        return null;
      })
    );

    const results = await Promise.all(responsePromises);
    const responses = results.filter(r => r !== null);

    if (responses.length === 0) {
      throw new Error('All adapters failed');
    }

    // Calculate semantic agreement
    const agreement = this._calculateConsensusAgreement(responses);
    const verdict = this._selectConsensusVerdict(responses);

    const consensus = new ConsensusResult({
      responses,
      agreement,
      verdict,
      confidence: Math.min(agreement, PHI_INV),
    });

    if (consensus.hasConsensus) {
      this.stats.consensusReached++;
      this.emit('consensus:reached', consensus);
    } else {
      this.stats.consensusFailed++;
      this.emit('consensus:failed', consensus);
    }

    return consensus;
  }

  // ───────────────────────────────────────────────────────────────────────────
  // HELPERS
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Call a specific provider
   */
  async _callProvider(providerName, prompt, options) {
    const adapter = this.adapters[providerName];

    if (!adapter) {
      throw new Error(`Unknown provider: ${providerName}`);
    }

    if (!adapter.enabled) {
      throw new Error(`Provider ${providerName} is disabled`);
    }

    const start = Date.now();
    const response = await adapter.complete(prompt, options);
    const duration = Date.now() - start;

    // Track stats
    this.stats.byProvider[providerName] = (this.stats.byProvider[providerName] || 0) + 1;

    // Track cost
    if (response.cost > 0) {
      await this.costLedger.track({
        category: 'llm',
        operation: providerName,
        amount: response.cost,
        metadata: {
          model: response.model,
          tokens: response.tokens,
        },
      });
    }

    this.emit('call:complete', { provider: providerName, duration, response });

    return response;
  }

  /**
   * Should budget block this call?
   */
  _shouldBlockForBudget(budgetStatus, priority) {
    // CRITICAL always allowed
    if (priority === Priority.CRITICAL) {
      return false;
    }

    // EXHAUSTED blocks everything except CRITICAL
    if (budgetStatus.level === BudgetStatus.EXHAUSTED) {
      return true;
    }

    // CRITICAL blocks NORMAL and LOW
    if (budgetStatus.level === BudgetStatus.CRITICAL &&
        (priority === Priority.NORMAL || priority === Priority.LOW)) {
      return true;
    }

    // WARNING blocks LOW
    if (budgetStatus.level === BudgetStatus.WARNING && priority === Priority.LOW) {
      return true;
    }

    return false;
  }

  /**
   * Calculate consensus agreement between responses
   */
  _calculateConsensusAgreement(responses) {
    if (responses.length < 2) return 1.0;

    const contents = responses.map(r => r.content);
    let totalSimilarity = 0;
    let comparisons = 0;

    for (let i = 0; i < contents.length; i++) {
      for (let j = i + 1; j < contents.length; j++) {
        const similarity = calculateSemanticAgreement(contents[i], contents[j]);
        totalSimilarity += similarity;
        comparisons++;
      }
    }

    return comparisons > 0 ? totalSimilarity / comparisons : 0;
  }

  /**
   * Select consensus verdict (majority or highest confidence)
   */
  _selectConsensusVerdict(responses) {
    if (responses.length === 0) return null;
    if (responses.length === 1) return responses[0].content;

    // Sort by confidence, return highest
    const sorted = [...responses].sort((a, b) => b.confidence - a.confidence);
    return sorted[0].content;
  }

  /**
   * Get router statistics
   */
  getStats() {
    return {
      ...this.stats,
      adapters: Object.fromEntries(
        Object.entries(this.adapters).map(([name, adapter]) => [
          name,
          { enabled: adapter.enabled, model: adapter.model },
        ])
      ),
    };
  }

  /**
   * Health check all adapters
   */
  async health() {
    const results = {};

    for (const [name, adapter] of Object.entries(this.adapters)) {
      results[name] = await adapter.health().catch(() => false);
    }

    return {
      healthy: Object.values(results).some(v => v === true),
      adapters: results,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _singleton = null;

export function getUnifiedLLMRouter(options) {
  if (!_singleton) {
    _singleton = new UnifiedLLMRouter(options);
  }
  return _singleton;
}

export function _resetForTesting() {
  _singleton = null;
}
