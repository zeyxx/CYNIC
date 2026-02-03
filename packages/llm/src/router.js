/**
 * CYNIC LLM Router
 *
 * Routes requests to appropriate LLM adapters and provides consensus voting.
 *
 * Features:
 * - Single LLM completion via primary adapter
 * - Multi-LLM consensus with φ⁻¹ quorum threshold
 * - Hybrid routing (fast consensus + deep analysis fallback)
 *
 * @module @cynic/llm/router
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV } from '@cynic/core';
import { ConsensusResult, ExecutionTier } from './types.js';
import { ClaudeCodeAdapter, createOllamaValidator, createAirLLMValidator } from './adapters/index.js';
import { calculateSemanticAgreement, SimilarityThresholds } from './similarity.js';

const log = createLogger('LLMRouter');

// ═══════════════════════════════════════════════════════════════════════════
// COST OPTIMIZER (Integrated from @cynic/node/routing/cost-optimizer.js)
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Tier costs (relative units)
 */
export const TIER_COSTS = Object.freeze({
  [ExecutionTier.LOCAL]: 0,      // $0 - no API call
  [ExecutionTier.LIGHT]: 1,      // Baseline (small models)
  [ExecutionTier.FULL]: 15,      // ~15x (medium models)
  [ExecutionTier.DEEP]: 50,      // ~50x (large models via AirLLM)
});

/**
 * Tier latencies (approximate ms)
 */
export const TIER_LATENCIES = Object.freeze({
  [ExecutionTier.LOCAL]: 1,      // <1ms local
  [ExecutionTier.LIGHT]: 5000,   // ~5s small Ollama
  [ExecutionTier.FULL]: 15000,   // ~15s medium Ollama
  [ExecutionTier.DEEP]: 60000,   // ~60s large models
});

/**
 * Complexity classifier for tier selection
 */
export class ComplexityClassifier {
  constructor() {
    this.stats = { classifications: 0, byTier: {} };
    Object.values(ExecutionTier).forEach(t => this.stats.byTier[t] = 0);
  }

  /**
   * Classify request complexity
   */
  classify(request) {
    const content = (request.content || request.prompt || '').toLowerCase();
    this.stats.classifications++;

    // LOCAL: Simple patterns
    if (this._isLocalResolvable(content)) {
      this.stats.byTier[ExecutionTier.LOCAL]++;
      return { tier: ExecutionTier.LOCAL, reason: 'Pattern match' };
    }

    // DEEP: Complex analysis
    if (this._isDeepRequired(content, request.context)) {
      this.stats.byTier[ExecutionTier.DEEP]++;
      return { tier: ExecutionTier.DEEP, reason: 'Deep analysis required' };
    }

    // FULL: Medium complexity
    if (this._isComplex(content)) {
      this.stats.byTier[ExecutionTier.FULL]++;
      return { tier: ExecutionTier.FULL, reason: 'Complex reasoning' };
    }

    // DEFAULT: LIGHT
    this.stats.byTier[ExecutionTier.LIGHT]++;
    return { tier: ExecutionTier.LIGHT, reason: 'Standard processing' };
  }

  _isLocalResolvable(content) {
    // Exclude when followed by complex keywords
    const complexKeywords = /security|analyze|design|architect|refactor|optimize|review|vulnerability|audit/i;
    if (complexKeywords.test(content)) {
      return false;
    }
    return content.match(/^(list|show|get|check|status|format|lint)/i) ||
           content.match(/^(does|is) .+ exist/i);
  }

  _isComplex(content) {
    return content.length > 500 ||
           content.match(/analyze|design|architect|refactor|optimize|security|review/i);
  }

  _isDeepRequired(content, context) {
    return content.length > 2000 ||
           content.match(/comprehensive|thorough|detailed analysis|multi-step/i) ||
           context?.tier === ExecutionTier.DEEP;
  }
}

/**
 * LLM Router - Routes requests to appropriate adapters
 *
 * Manages multiple LLM adapters and provides:
 * - Single LLM completion
 * - Multi-LLM consensus voting
 * - Cost-optimized tier routing
 */
export class LLMRouter extends EventEmitter {
  constructor(options = {}) {
    super();

    // Primary adapter (Claude Code by default)
    this.primary = options.primary || new ClaudeCodeAdapter();

    // Validator adapters (OSS LLMs for consensus)
    this.validators = options.validators || [];

    // Tier adapters (for cost-optimized routing)
    this.tierAdapters = options.tierAdapters || {};

    // Complexity classifier for cost optimization
    this.classifier = options.classifier || new ComplexityClassifier();

    // Consensus configuration
    this.consensusConfig = {
      quorum: options.quorum || PHI_INV, // φ⁻¹ = 61.8% agreement needed
      timeout: options.timeout || 10000, // 10s timeout for validators
      requirePrimary: options.requirePrimary !== false, // Primary must agree
    };

    this.stats = {
      singleRequests: 0,
      consensusRequests: 0,
      consensusReached: 0,
      consensusFailed: 0,
      routedRequests: 0,
      byTier: {},
      totalCost: 0,
      costSaved: 0,
    };
    Object.values(ExecutionTier).forEach(t => this.stats.byTier[t] = 0);

    log.debug('LLMRouter created', {
      primary: this.primary.provider,
      validatorCount: this.validators.length,
    });
  }

  /**
   * Cost-optimized routing - chooses tier based on complexity
   *
   * @param {Object} request - Request to route
   * @param {string} request.content - Request content/prompt
   * @param {Object} [request.context] - Additional context
   * @param {string} [request.forceTier] - Force specific tier
   * @returns {Promise<Object>} Routed response with tier info
   */
  async route(request) {
    const startTime = Date.now();
    this.stats.routedRequests++;

    // Classify or use forced tier
    let tier, reason;
    if (request.forceTier && Object.values(ExecutionTier).includes(request.forceTier)) {
      tier = request.forceTier;
      reason = 'Forced tier';
    } else {
      const classification = this.classifier.classify(request);
      tier = classification.tier;
      reason = classification.reason;
    }

    this.stats.byTier[tier]++;

    // Handle LOCAL tier (no LLM)
    if (tier === ExecutionTier.LOCAL) {
      return {
        tier,
        content: '*sniff* Pattern match - no LLM needed.',
        confidence: PHI_INV,
        cost: 0,
        latency: Date.now() - startTime,
        reason,
      };
    }

    // Get adapter for tier
    const adapter = this._getAdapterForTier(tier);
    if (!adapter) {
      log.warn(`No adapter for tier ${tier}, falling back to validators`);
      // Fallback to first available validator
      const fallback = this.validators.find(v => v.enabled);
      if (!fallback) {
        return {
          tier,
          content: null,
          error: 'No adapter available',
          reason,
        };
      }
    }

    try {
      const response = await (adapter || this.validators[0]).complete(
        request.content || request.prompt,
        { timeout: TIER_LATENCIES[tier] * 1.5 }
      );

      const cost = TIER_COSTS[tier];
      const maxCost = TIER_COSTS[ExecutionTier.DEEP];
      this.stats.totalCost += cost;
      this.stats.costSaved += (maxCost - cost);

      return {
        tier,
        content: response.content,
        confidence: response.confidence,
        model: response.model,
        cost,
        latency: Date.now() - startTime,
        reason,
      };
    } catch (err) {
      log.error('Route failed', { tier, error: err.message });
      return {
        tier,
        content: null,
        error: err.message,
        cost: 0,
        latency: Date.now() - startTime,
        reason,
      };
    }
  }

  /**
   * Get adapter for specific tier
   * @private
   */
  _getAdapterForTier(tier) {
    // Check explicit tier adapters
    if (this.tierAdapters[tier]) {
      return this.tierAdapters[tier];
    }

    // Map tiers to validators by capability
    const validators = this.validators.filter(v => v.enabled);
    if (validators.length === 0) return null;

    switch (tier) {
      case ExecutionTier.LIGHT:
        // Prefer small models
        return validators.find(v => v.model?.includes('gemma') || v.model?.includes('qwen')) || validators[0];
      case ExecutionTier.FULL:
        // Prefer medium models
        return validators.find(v => v.model?.includes('mistral') || v.model?.includes('llama')) || validators[0];
      case ExecutionTier.DEEP:
        // Prefer AirLLM
        return validators.find(v => v.provider === 'airllm') || validators[validators.length - 1];
      default:
        return validators[0];
    }
  }

  /**
   * Single LLM completion (primary only)
   *
   * @param {string} prompt
   * @param {Object} [options]
   * @returns {Promise<LLMResponse>}
   */
  async complete(prompt, options = {}) {
    this.stats.singleRequests++;
    return this.primary.complete(prompt, options);
  }

  /**
   * Consensus completion (primary + validators)
   *
   * @param {string} prompt
   * @param {Object} [options]
   * @param {number} [options.quorum] - Override quorum threshold
   * @returns {Promise<ConsensusResult>}
   */
  async consensus(prompt, options = {}) {
    this.stats.consensusRequests++;

    const quorum = options.quorum || this.consensusConfig.quorum;
    const timeout = options.timeout || this.consensusConfig.timeout;

    // Get all available adapters
    const adapters = [this.primary, ...this.validators.filter(v => v.enabled)];

    if (adapters.length < 2) {
      // Not enough adapters for consensus, fall back to single
      log.debug('Not enough adapters for consensus, using primary only');
      const response = await this.primary.complete(prompt, options);
      return new ConsensusResult({
        responses: [response],
        agreement: 1.0,
        verdict: response.content,
        confidence: response.confidence,
      });
    }

    // Request from all adapters in parallel
    const responsePromises = adapters.map(adapter =>
      Promise.race([
        adapter.complete(prompt, options),
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error('Timeout')), timeout)
        ),
      ]).catch(err => {
        log.warn('Adapter failed in consensus', {
          provider: adapter.provider,
          error: err.message,
        });
        return null;
      })
    );

    const responses = (await Promise.all(responsePromises)).filter(r => r !== null);

    // Calculate agreement
    const { agreement, verdict, dissent } = this._calculateAgreement(responses);

    const result = new ConsensusResult({
      responses,
      agreement,
      verdict,
      confidence: agreement * PHI_INV, // Scale by max confidence
      dissent,
    });

    if (result.hasConsensus) {
      this.stats.consensusReached++;
    } else {
      this.stats.consensusFailed++;
    }

    this.emit('consensus', result);
    return result;
  }

  /**
   * Calculate agreement between responses
   * @private
   */
  _calculateAgreement(responses) {
    if (responses.length === 0) {
      return { agreement: 0, verdict: null, dissent: [] };
    }

    if (responses.length === 1) {
      return { agreement: 1.0, verdict: responses[0].content, dissent: [] };
    }

    // Use semantic similarity for consensus (replaces simple string matching)
    // Clusters similar responses together using φ⁻¹ threshold
    const result = calculateSemanticAgreement(responses, SimilarityThresholds.HIGH);

    log.debug('Semantic consensus calculated', {
      agreement: result.agreement,
      clusterCount: result.clusterCount,
      avgSimilarity: result.avgClusterSimilarity,
    });

    return {
      agreement: result.agreement,
      verdict: result.verdict,
      dissent: result.dissent,
      // Additional semantic metadata
      clusters: result.clusters,
      clusterCount: result.clusterCount,
      avgClusterSimilarity: result.avgClusterSimilarity,
    };
  }

  /**
   * Add a validator adapter
   *
   * @param {LLMAdapter} adapter
   */
  addValidator(adapter) {
    this.validators.push(adapter);
    log.info('Validator added', { provider: adapter.provider });
  }

  /**
   * Remove a validator adapter
   *
   * @param {string} provider - Provider name
   */
  removeValidator(provider) {
    this.validators = this.validators.filter(v => v.provider !== provider);
    log.info('Validator removed', { provider });
  }

  /**
   * Get router status
   */
  getStatus() {
    return {
      primary: this.primary.getInfo(),
      validators: this.validators.map(v => v.getInfo()),
      config: this.consensusConfig,
      stats: this.stats,
    };
  }
}

/**
 * Create an LLM Router
 *
 * @param {Object} [options]
 * @returns {LLMRouter}
 */
export function createLLMRouter(options = {}) {
  return new LLMRouter(options);
}

// Singleton
let _globalRouter = null;

/**
 * Get the global LLM Router
 *
 * @param {Object} [options]
 * @returns {LLMRouter}
 */
export function getLLMRouter(options) {
  if (!_globalRouter) {
    _globalRouter = new LLMRouter(options);
  }
  return _globalRouter;
}

/**
 * Reset global router (for testing)
 */
export function _resetLLMRouterForTesting() {
  _globalRouter = null;
}

/**
 * Create validators from environment configuration
 *
 * Reads from environment variables:
 * - CYNIC_VALIDATORS: Comma-separated list of validator types (ollama,lm-studio,airllm)
 * - OLLAMA_ENDPOINT: Ollama endpoint
 * - OLLAMA_MODEL: Ollama model
 * - LM_STUDIO_ENDPOINT: LM Studio endpoint
 * - LM_STUDIO_MODEL: LM Studio model
 * - CYNIC_AIRLLM: Enable AirLLM (true/false)
 * - CYNIC_AIRLLM_MODEL: AirLLM model
 *
 * @returns {LLMAdapter[]}
 */
export function createValidatorsFromEnv() {
  const validators = [];
  const validatorList = (process.env.CYNIC_VALIDATORS || '').split(',').filter(Boolean);

  for (const type of validatorList) {
    const normalized = type.trim().toLowerCase();
    try {
      switch (normalized) {
        case 'ollama':
          validators.push(createOllamaValidator());
          log.info('Created Ollama validator from env');
          break;
        case 'lm-studio':
        case 'lmstudio':
          // Import dynamically to avoid circular dependency
          const { createLMStudioValidator } = require('./adapters/oss-llm.js');
          validators.push(createLMStudioValidator());
          log.info('Created LM Studio validator from env');
          break;
        case 'airllm':
          if (process.env.CYNIC_AIRLLM === 'true') {
            validators.push(createAirLLMValidator());
            log.info('Created AirLLM validator from env');
          } else {
            log.warn('AirLLM in CYNIC_VALIDATORS but CYNIC_AIRLLM not set to true');
          }
          break;
        default:
          log.warn('Unknown validator type in CYNIC_VALIDATORS', { type: normalized });
      }
    } catch (err) {
      log.warn('Failed to create validator from env', { type: normalized, error: err.message });
    }
  }

  return validators;
}

/**
 * Get a router with validators from environment
 *
 * @param {Object} [options]
 * @returns {LLMRouter}
 */
export function getRouterWithValidators(options = {}) {
  const router = getLLMRouter(options);

  // Add validators from environment if not already added
  if (router.validators.length === 0) {
    const envValidators = createValidatorsFromEnv();
    for (const validator of envValidators) {
      router.addValidator(validator);
    }
  }

  return router;
}

/**
 * Create a hybrid router with both fast validators and AirLLM deep analysis
 *
 * Returns a router configured for:
 * - Fast consensus with small models (gemma2:2b, etc.)
 * - Deep analysis fallback with AirLLM (mistral:7b)
 *
 * @param {Object} [options]
 * @param {string[]} [options.fastModels=['gemma2:2b']] - Fast models for consensus
 * @param {string} [options.deepModel] - AirLLM model for deep analysis
 * @returns {LLMRouter}
 */
export function createHybridRouter(options = {}) {
  const fastModels = options.fastModels || ['gemma2:2b'];

  // Create fast validators
  const validators = fastModels.map(model => createOllamaValidator({ model }));

  // Add AirLLM if enabled
  if (process.env.CYNIC_AIRLLM === 'true') {
    const airllm = createAirLLMValidator({
      model: options.deepModel || process.env.CYNIC_AIRLLM_MODEL,
    });
    validators.push(airllm);
    log.info('Hybrid router created with AirLLM deep analysis');
  }

  return new LLMRouter({ validators });
}

export default LLMRouter;
