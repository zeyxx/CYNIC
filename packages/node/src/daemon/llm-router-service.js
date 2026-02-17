/**
 * LLM Router Service — Intelligent routing between Claude and Ollama
 *
 * Routes LLM requests based on:
 * - Task complexity (simple/moderate/complex/critical)
 * - Risk signals (Solana, mainnet, auth, crypto)
 * - Budget constraints (weekly Claude limit)
 * - Model capabilities (Claude for complex, Ollama for simple/moderate)
 *
 * "Le chien choisit le bon outil" - κυνικός
 *
 * @module @cynic/node/daemon/llm-router-service
 */

'use strict';

import { createLogger } from '@cynic/core';
import { createSingleton } from '@cynic/core/axioms/phi-utils.js';

const log = createLogger('LLMRouterService');

const OLLAMA_BASE_URL = 'http://localhost:11434';
const ROUTING_STATS_INTERVAL = 5 * 60 * 1000; // 5min — φ-aligned with CYNIC heartbeat

/**
 * Classify task complexity from prompt
 */
function classifyTask(prompt) {
  const lower = prompt.toLowerCase();

  // Complexity signals
  const complexSignals = [
    'architecture', 'design', 'refactor', 'implement',
    'explain why', 'analyze', 'metathinking', 'strategy'
  ];

  const simpleSignals = [
    'fix typo', 'add comment', 'format', 'lint',
    'list', 'show', 'what is', 'simple'
  ];

  // Risk signals (always use Claude)
  const riskSignals = [
    'solana', 'mainnet', 'transaction', 'deploy',
    'security', 'auth', 'crypto', 'database migration'
  ];

  // Check risk first
  if (riskSignals.some(sig => lower.includes(sig))) {
    return {
      complexity: 'critical',
      reason: 'Risk-sensitive task detected'
    };
  }

  // Check complexity
  const complexCount = complexSignals.filter(sig => lower.includes(sig)).length;
  const simpleCount = simpleSignals.filter(sig => lower.includes(sig)).length;

  if (complexCount >= 2) {
    return {
      complexity: 'complex',
      reason: `${complexCount} complex signals`
    };
  }

  if (simpleCount >= 1 && complexCount === 0) {
    return {
      complexity: 'simple',
      reason: `${simpleCount} simple signals`
    };
  }

  // Code generation signals → moderate (even if short)
  const codeSignals = ['write', 'create', 'function', 'implement', 'code', 'build'];
  if (codeSignals.some(sig => lower.includes(sig))) {
    return {
      complexity: 'moderate',
      reason: 'Code generation task'
    };
  }

  // Length-based heuristic (fallback)
  if (prompt.length < 100) {
    return {
      complexity: 'simple',
      reason: 'Short prompt (< 100 chars)'
    };
  }

  if (prompt.length > 500) {
    return {
      complexity: 'complex',
      reason: 'Long prompt (> 500 chars)'
    };
  }

  return {
    complexity: 'moderate',
    reason: 'Default moderate complexity'
  };
}

/**
 * Choose model based on task classification
 */
function chooseModel(classification) {
  const { complexity } = classification;

  // Critical/Complex → Claude
  if (complexity === 'critical' || complexity === 'complex') {
    return {
      provider: 'claude',
      model: 'claude-sonnet-4-5',
      reason: `${complexity} task requires Claude`
    };
  }

  // Simple → Ollama (fast model)
  if (complexity === 'simple') {
    return {
      provider: 'ollama',
      model: 'qwen2.5:1.5b',
      reason: 'Simple task, fast Ollama model'
    };
  }

  // Moderate → Ollama (capable model)
  return {
    provider: 'ollama',
    model: 'mistral:7b-instruct-q4_0',
    reason: 'Moderate task, capable Ollama model'
  };
}

/**
 * Call Ollama API
 */
async function callOllama(model, prompt, options = {}) {
  const response = await fetch(`${OLLAMA_BASE_URL}/api/generate`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      model,
      prompt,
      stream: false,
      options: {
        temperature: options.temperature ?? 0.7,
        top_p: options.top_p ?? 0.9
      }
    })
  });

  if (!response.ok) {
    throw new Error(`Ollama API error: ${response.status}`);
  }

  const data = await response.json();
  return data.response;
}

/**
 * LLM Router Service
 */
class LLMRouterService {
  constructor() {
    this.stats = {
      totalRequests: 0,
      claudeRequests: 0,
      ollamaRequests: 0,
      ollamaFallbacks: 0,
      complexityBreakdown: {
        simple: 0,
        moderate: 0,
        complex: 0,
        critical: 0
      },
      avgLatency: {
        claude: 0,
        ollama: 0
      },
      errors: {
        ollama: 0,
        claude: 0
      }
    };

    this.decisions = []; // Ring buffer (last 100 decisions)
    this.maxDecisions = 100;

    // Start stats logging
    this._statsTimer = setInterval(() => {
      this._logStats();
    }, ROUTING_STATS_INTERVAL);
    this._statsTimer.unref();

    log.info('LLM Router Service initialized');
  }

  /**
   * Route a prompt to the best LLM provider
   *
   * @param {string} prompt - User prompt
   * @param {object} options - Routing options
   * @param {string} options.forceProvider - Force specific provider ('claude' or 'ollama')
   * @param {string} options.context - Additional context for classification
   * @returns {Promise<object>} { provider, model, response, latency, classification }
   */
  async route(prompt, options = {}) {
    const startTime = Date.now();
    this.stats.totalRequests++;

    try {
      // 1. Classify task
      const classification = classifyTask(prompt);
      this.stats.complexityBreakdown[classification.complexity]++;

      // 2. Choose model (unless forced)
      let choice;
      if (options.forceProvider) {
        choice = {
          provider: options.forceProvider,
          model: options.forceProvider === 'claude' ? 'claude-sonnet-4-5' : 'mistral:7b-instruct-q4_0',
          reason: 'Provider forced by caller'
        };
      } else {
        choice = chooseModel(classification);
      }

      // 3. Execute
      let response;
      let error = null;

      if (choice.provider === 'ollama') {
        try {
          response = await callOllama(choice.model, prompt, options);
          this.stats.ollamaRequests++;
        } catch (err) {
          log.warn(`Ollama failed: ${err.message}, falling back to Claude`);
          error = err.message;
          this.stats.errors.ollama++;
          this.stats.ollamaFallbacks++;

          // Fallback to Claude
          choice.provider = 'claude';
          choice.model = 'claude-sonnet-4-5';
          choice.reason = `Ollama failed: ${err.message}`;
          response = null; // Will be handled by caller
        }
      }

      if (choice.provider === 'claude') {
        this.stats.claudeRequests++;
        response = null; // Caller will handle Claude API call
      }

      const latency = Date.now() - startTime;

      // Update latency stats
      const providerKey = choice.provider === 'claude' ? 'claude' : 'ollama';
      if (this.stats.avgLatency[providerKey] === 0) {
        this.stats.avgLatency[providerKey] = latency;
      } else {
        // EMA with α = 0.2
        this.stats.avgLatency[providerKey] = 0.2 * latency + 0.8 * this.stats.avgLatency[providerKey];
      }

      // 4. Log decision
      const decision = {
        timestamp: new Date().toISOString(),
        classification,
        choice,
        latency,
        error,
        promptLength: prompt.length,
        responseLength: response?.length || 0
      };

      this.decisions.push(decision);
      if (this.decisions.length > this.maxDecisions) {
        this.decisions.shift(); // Keep last 100
      }

      return {
        provider: choice.provider,
        model: choice.model,
        response,
        latency,
        classification,
        reason: choice.reason,
        error
      };

    } catch (err) {
      log.error(`Routing failed: ${err.message}`);
      this.stats.errors.claude++;
      throw err;
    }
  }

  /**
   * Get routing statistics
   */
  getStats() {
    const ollamaRate = this.stats.totalRequests > 0
      ? (this.stats.ollamaRequests / this.stats.totalRequests * 100).toFixed(1)
      : 0;

    const claudeRate = this.stats.totalRequests > 0
      ? (this.stats.claudeRequests / this.stats.totalRequests * 100).toFixed(1)
      : 0;

    return {
      ...this.stats,
      distribution: {
        ollama: `${ollamaRate}%`,
        claude: `${claudeRate}%`
      },
      targetDistribution: {
        ollama: '61.8% (φ⁻¹)',
        claude: '38.2% (1 - φ⁻¹)'
      },
      recentDecisions: this.decisions.slice(-10) // Last 10 decisions
    };
  }

  /**
   * Reset statistics (for testing)
   */
  resetStats() {
    this.stats = {
      totalRequests: 0,
      claudeRequests: 0,
      ollamaRequests: 0,
      ollamaFallbacks: 0,
      complexityBreakdown: {
        simple: 0,
        moderate: 0,
        complex: 0,
        critical: 0
      },
      avgLatency: {
        claude: 0,
        ollama: 0
      },
      errors: {
        ollama: 0,
        claude: 0
      }
    };
    this.decisions = [];
    log.info('Stats reset');
  }

  /**
   * Log stats periodically
   */
  _logStats() {
    const stats = this.getStats();
    log.info('LLM routing stats', {
      total: stats.totalRequests,
      distribution: stats.distribution,
      avgLatency: {
        ollama: `${Math.round(stats.avgLatency.ollama)}ms`,
        claude: `${Math.round(stats.avgLatency.claude)}ms`
      },
      complexity: stats.complexityBreakdown,
      errors: stats.errors
    });
  }

  /**
   * Stop service
   */
  stop() {
    if (this._statsTimer) {
      clearInterval(this._statsTimer);
      this._statsTimer = null;
    }
    log.info('LLM Router Service stopped');
  }
}

/**
 * Create singleton instance
 */
const { getInstance, resetInstance } = createSingleton(LLMRouterService);

export { getInstance as getLLMRouterService, resetInstance as resetLLMRouterService };
