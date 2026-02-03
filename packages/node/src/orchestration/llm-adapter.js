/**
 * CYNIC LLM Adapter - CPU Layer
 *
 * Abstract interface for LLM interactions.
 * Enables multi-LLM consensus without changing upper layers.
 *
 * Current: Claude only (via Claude Code)
 * Future: Claude + OSS LLMs for consensus voting
 *
 * "Le processeur de CYNIC" - κυνικός
 *
 * Architecture:
 *   Brain (consciousness) ← brain.js
 *     ↓ decisions
 *   OS (orchestration) ← unified-orchestrator.js
 *     ↓ tasks
 *   CPU (LLM layer) ← THIS FILE
 *
 * @module @cynic/node/orchestration/llm-adapter
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, PHI_INV_2 } from '@cynic/core';
import { calculateSemanticAgreement, SimilarityThresholds } from '@cynic/llm';

const log = createLogger('LLMAdapter');

/**
 * LLM Response - standardized response format
 */
export class LLMResponse {
  constructor(data = {}) {
    this.id = data.id || `resp-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    this.timestamp = data.timestamp || Date.now();
    this.provider = data.provider || 'unknown';
    this.model = data.model || 'unknown';
    this.content = data.content || '';
    this.confidence = Math.min(data.confidence || PHI_INV, PHI_INV); // Cap at φ⁻¹
    this.tokens = data.tokens || { input: 0, output: 0 };
    this.duration = data.duration || 0;
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
    };
  }
}

/**
 * Consensus Result - result of multi-LLM voting
 */
export class ConsensusResult {
  constructor(data = {}) {
    this.id = data.id || `consensus-${Date.now()}`;
    this.timestamp = data.timestamp || Date.now();
    this.responses = data.responses || [];
    this.agreement = data.agreement || 0; // 0-1
    this.verdict = data.verdict || null; // Majority verdict
    this.confidence = Math.min(data.confidence || 0, PHI_INV);
    this.dissent = data.dissent || [];
  }

  /**
   * Is consensus reached?
   * Threshold: φ⁻¹ = 61.8% agreement
   */
  get hasConsensus() {
    return this.agreement >= PHI_INV;
  }

  /**
   * Is consensus strong?
   * Threshold: 100% - φ⁻² = 61.8% + some margin
   */
  get isStrong() {
    return this.agreement >= (1 - PHI_INV_2); // ~81.8%
  }

  toJSON() {
    return {
      id: this.id,
      responseCount: this.responses.length,
      agreement: this.agreement,
      hasConsensus: this.hasConsensus,
      isStrong: this.isStrong,
      verdict: this.verdict,
      confidence: this.confidence,
      dissentCount: this.dissent.length,
    };
  }
}

/**
 * Abstract LLM Adapter Interface
 *
 * All LLM adapters must implement this interface.
 */
export class LLMAdapter extends EventEmitter {
  /**
   * @param {Object} options
   * @param {string} options.provider - Provider name
   * @param {string} [options.model] - Model name
   */
  constructor(options = {}) {
    super();
    this.provider = options.provider || 'abstract';
    this.model = options.model || 'unknown';
    this.enabled = false;

    this.stats = {
      requests: 0,
      successes: 0,
      failures: 0,
      totalTokens: 0,
      avgLatency: 0,
    };
  }

  /**
   * Complete a prompt
   *
   * @param {string} prompt - Prompt to complete
   * @param {Object} [options] - Options
   * @returns {Promise<LLMResponse>}
   */
  async complete(prompt, options = {}) {
    throw new Error('complete() must be implemented by subclass');
  }

  /**
   * Check if adapter is available
   *
   * @returns {Promise<boolean>}
   */
  async isAvailable() {
    return this.enabled;
  }

  /**
   * Get adapter info
   */
  getInfo() {
    return {
      provider: this.provider,
      model: this.model,
      enabled: this.enabled,
      stats: this.stats,
    };
  }
}

/**
 * Claude Code Adapter
 *
 * Adapter for Claude when running inside Claude Code.
 * Note: Does NOT make API calls - just returns structured prompts
 * that the outer Claude Code will execute.
 *
 * This is a "pass-through" adapter since Claude Code IS the LLM.
 */
export class ClaudeCodeAdapter extends LLMAdapter {
  constructor(options = {}) {
    super({
      provider: 'claude-code',
      model: options.model || 'claude-opus-4-5-20251101',
      ...options,
    });

    this.enabled = true; // Always enabled inside Claude Code
  }

  /**
   * Complete a prompt
   *
   * Since we're INSIDE Claude Code, this is a no-op that returns
   * the prompt formatted for the outer Claude to process.
   *
   * @param {string} prompt
   * @param {Object} [options]
   * @returns {Promise<LLMResponse>}
   */
  async complete(prompt, options = {}) {
    const startTime = Date.now();
    this.stats.requests++;

    // Inside Claude Code, we don't make API calls
    // We return a structured response indicating this should be processed
    // by the outer Claude

    const response = new LLMResponse({
      provider: this.provider,
      model: this.model,
      content: prompt, // Return prompt as content for outer processing
      confidence: PHI_INV, // Claude Code is trusted
      tokens: { input: this._estimateTokens(prompt), output: 0 },
      duration: Date.now() - startTime,
      metadata: {
        type: 'pass-through',
        note: 'Prompt for outer Claude Code to process',
      },
    });

    this.stats.successes++;
    this.emit('complete', response);

    return response;
  }

  /**
   * Estimate tokens (rough approximation)
   * @private
   */
  _estimateTokens(text) {
    return Math.ceil(text.length / 4);
  }

  async isAvailable() {
    return true;
  }
}

/**
 * OSS LLM Adapter for Ollama and OpenAI-compatible endpoints
 *
 * Supports:
 * - Ollama API (default, http://localhost:11434)
 * - OpenAI-compatible APIs (LM Studio, vLLM, etc.)
 *
 * Task #59: Enable OSS LLM validators for consensus
 */
export class OSSLLMAdapter extends LLMAdapter {
  constructor(options = {}) {
    super({
      provider: options.provider || 'ollama',
      model: options.model || 'llama3.2',
      ...options,
    });

    this.endpoint = options.endpoint || null;
    this.apiKey = options.apiKey || null;
    this.apiFormat = options.apiFormat || 'ollama'; // 'ollama' or 'openai'
    this.timeout = options.timeout || 30000; // 30s default
    this.enabled = false; // Disabled by default until configured
  }

  /**
   * Configure the adapter
   *
   * @param {Object} config
   * @param {string} config.endpoint - API endpoint (e.g., http://localhost:11434)
   * @param {string} [config.apiKey] - API key (for OpenAI-compatible)
   * @param {string} [config.model] - Model name (e.g., llama3.2, mistral)
   * @param {string} [config.apiFormat] - 'ollama' or 'openai'
   */
  configure(config) {
    this.endpoint = config.endpoint || this.endpoint;
    this.apiKey = config.apiKey || this.apiKey;
    this.model = config.model || this.model;
    this.apiFormat = config.apiFormat || this.apiFormat;
    this.enabled = !!this.endpoint;

    log.info('OSSLLMAdapter configured', {
      provider: this.provider,
      model: this.model,
      apiFormat: this.apiFormat,
      enabled: this.enabled,
    });
  }

  /**
   * Complete a prompt using Ollama or OpenAI-compatible API
   *
   * @param {string} prompt
   * @param {Object} [options]
   * @param {number} [options.temperature=0.7]
   * @param {number} [options.maxTokens=512]
   * @returns {Promise<LLMResponse>}
   */
  async complete(prompt, options = {}) {
    if (!this.enabled) {
      throw new Error(`OSSLLMAdapter not configured: ${this.provider}`);
    }

    this.stats.requests++;
    const startTime = Date.now();

    try {
      let result;
      if (this.apiFormat === 'openai') {
        result = await this._callOpenAICompatible(prompt, options);
      } else {
        result = await this._callOllama(prompt, options);
      }

      const duration = Date.now() - startTime;
      this._updateLatency(duration);

      const response = new LLMResponse({
        provider: this.provider,
        model: result.model || this.model,
        content: result.content,
        confidence: PHI_INV_2, // OSS LLMs get φ⁻² = 38.2% max confidence
        tokens: result.tokens,
        duration,
        metadata: {
          type: 'oss-llm',
          apiFormat: this.apiFormat,
          endpoint: this.endpoint,
        },
      });

      this.stats.successes++;
      this.stats.totalTokens += (result.tokens.input + result.tokens.output);
      this.emit('complete', response);

      return response;
    } catch (err) {
      this.stats.failures++;
      log.error('OSSLLMAdapter request failed', {
        provider: this.provider,
        model: this.model,
        error: err.message,
      });
      throw err;
    }
  }

  /**
   * Call Ollama API
   * @private
   */
  async _callOllama(prompt, options = {}) {
    const url = `${this.endpoint}/api/generate`;

    const body = {
      model: this.model,
      prompt,
      stream: false,
      options: {
        temperature: options.temperature ?? 0.7,
        num_predict: options.maxTokens ?? 512,
      },
    };

    const response = await this._fetch(url, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`Ollama API error: ${response.status} - ${text}`);
    }

    const data = await response.json();

    return {
      content: data.response || '',
      model: data.model,
      tokens: {
        input: data.prompt_eval_count || this._estimateTokens(prompt),
        output: data.eval_count || this._estimateTokens(data.response || ''),
      },
    };
  }

  /**
   * Call OpenAI-compatible API (LM Studio, vLLM, etc.)
   * @private
   */
  async _callOpenAICompatible(prompt, options = {}) {
    const url = `${this.endpoint}/v1/chat/completions`;

    const body = {
      model: this.model,
      messages: [{ role: 'user', content: prompt }],
      temperature: options.temperature ?? 0.7,
      max_tokens: options.maxTokens ?? 512,
    };

    const headers = {
      'Content-Type': 'application/json',
    };

    if (this.apiKey) {
      headers['Authorization'] = `Bearer ${this.apiKey}`;
    }

    const response = await this._fetch(url, {
      method: 'POST',
      headers,
      body: JSON.stringify(body),
    });

    if (!response.ok) {
      const text = await response.text();
      throw new Error(`OpenAI API error: ${response.status} - ${text}`);
    }

    const data = await response.json();
    const choice = data.choices?.[0];

    return {
      content: choice?.message?.content || '',
      model: data.model,
      tokens: {
        input: data.usage?.prompt_tokens || this._estimateTokens(prompt),
        output: data.usage?.completion_tokens || this._estimateTokens(choice?.message?.content || ''),
      },
    };
  }

  /**
   * Fetch wrapper with timeout
   * @private
   */
  async _fetch(url, options) {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), this.timeout);

    try {
      const response = await fetch(url, {
        ...options,
        signal: controller.signal,
      });
      return response;
    } finally {
      clearTimeout(timeoutId);
    }
  }

  /**
   * Estimate tokens (rough approximation)
   * @private
   */
  _estimateTokens(text) {
    return Math.ceil((text || '').length / 4);
  }

  /**
   * Update average latency
   * @private
   */
  _updateLatency(duration) {
    const count = this.stats.requests;
    this.stats.avgLatency = ((this.stats.avgLatency * (count - 1)) + duration) / count;
  }

  /**
   * Check if Ollama/endpoint is available
   * @returns {Promise<boolean>}
   */
  async isAvailable() {
    if (!this.enabled) return false;

    try {
      // Ollama health check
      if (this.apiFormat === 'ollama') {
        const response = await this._fetch(`${this.endpoint}/api/tags`, { method: 'GET' });
        return response.ok;
      }
      // OpenAI-compatible - check models endpoint
      const headers = this.apiKey ? { 'Authorization': `Bearer ${this.apiKey}` } : {};
      const response = await this._fetch(`${this.endpoint}/v1/models`, {
        method: 'GET',
        headers,
      });
      return response.ok;
    } catch {
      return false;
    }
  }
}

/**
 * LLM Router - Routes requests to appropriate adapters
 *
 * Manages multiple LLM adapters and provides consensus voting.
 */
export class LLMRouter extends EventEmitter {
  constructor(options = {}) {
    super();

    // Primary adapter (Claude Code)
    this.primary = options.primary || new ClaudeCodeAdapter();

    // Validator adapters (OSS LLMs for consensus)
    this.validators = options.validators || [];

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
    };

    log.debug('LLMRouter created', {
      primary: this.primary.provider,
      validatorCount: this.validators.length,
    });
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

    // Use semantic similarity for consensus (from @cynic/llm)
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

// ============================================================================
// OSS LLM Factory Functions (Task #59: Enable OSS validators for consensus)
// ============================================================================

/**
 * Create an Ollama validator
 *
 * @param {Object} [options]
 * @param {string} [options.endpoint=http://localhost:11434] - Ollama endpoint
 * @param {string} [options.model=llama3.2] - Model name
 * @returns {OSSLLMAdapter}
 */
export function createOllamaValidator(options = {}) {
  const adapter = new OSSLLMAdapter({
    provider: 'ollama',
    model: options.model || process.env.OLLAMA_MODEL || 'llama3.2',
    endpoint: options.endpoint || process.env.OLLAMA_ENDPOINT || 'http://localhost:11434',
    apiFormat: 'ollama',
    timeout: options.timeout || 30000,
  });
  adapter.enabled = true;
  return adapter;
}

/**
 * Create an LM Studio validator (OpenAI-compatible)
 *
 * @param {Object} [options]
 * @param {string} [options.endpoint=http://localhost:1234] - LM Studio endpoint
 * @param {string} [options.model=local-model] - Model name
 * @returns {OSSLLMAdapter}
 */
export function createLMStudioValidator(options = {}) {
  const adapter = new OSSLLMAdapter({
    provider: 'lm-studio',
    model: options.model || process.env.LM_STUDIO_MODEL || 'local-model',
    endpoint: options.endpoint || process.env.LM_STUDIO_ENDPOINT || 'http://localhost:1234',
    apiFormat: 'openai',
    timeout: options.timeout || 30000,
  });
  adapter.enabled = true;
  return adapter;
}

/**
 * Create a generic OpenAI-compatible validator
 *
 * @param {Object} options
 * @param {string} options.endpoint - API endpoint
 * @param {string} [options.model] - Model name
 * @param {string} [options.apiKey] - API key
 * @param {string} [options.provider=openai-compat] - Provider name
 * @returns {OSSLLMAdapter}
 */
export function createOpenAIValidator(options = {}) {
  if (!options.endpoint) {
    throw new Error('endpoint is required for OpenAI-compatible validator');
  }
  const adapter = new OSSLLMAdapter({
    provider: options.provider || 'openai-compat',
    model: options.model || 'gpt-3.5-turbo',
    endpoint: options.endpoint,
    apiKey: options.apiKey,
    apiFormat: 'openai',
    timeout: options.timeout || 30000,
  });
  adapter.enabled = true;
  return adapter;
}

/**
 * Create validators from environment configuration
 *
 * Reads from environment variables:
 * - CYNIC_VALIDATORS: Comma-separated list of validator types (ollama,lm-studio)
 * - OLLAMA_ENDPOINT: Ollama endpoint
 * - OLLAMA_MODEL: Ollama model
 * - LM_STUDIO_ENDPOINT: LM Studio endpoint
 * - LM_STUDIO_MODEL: LM Studio model
 *
 * @returns {OSSLLMAdapter[]}
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
          validators.push(createLMStudioValidator());
          log.info('Created LM Studio validator from env');
          break;
        case 'airllm':
          // AirLLM requires explicit enable via CYNIC_AIRLLM=true
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

// ============================================================================
// AirLLM Integration (Task #98: Connect AirLLM to Da'at architecture)
// "Mistral 7B avec 28GB RAM" - Large models via disk offloading
// ============================================================================

/**
 * AirLLM Adapter - Large models via disk offloading
 *
 * Extends OSSLLMAdapter with:
 * - Longer timeout (120s default)
 * - Lower temperature (0.2) for consistency
 * - More tokens (1000) for deeper reasoning
 * - Deep analysis mode
 *
 * Uses Ollama with quantized models (q4_0) for memory efficiency.
 */
export class AirLLMAdapter extends OSSLLMAdapter {
  constructor(options = {}) {
    super({
      provider: 'airllm',
      model: options.model || process.env.CYNIC_AIRLLM_MODEL || 'mistral:7b-instruct-q4_0',
      endpoint: options.endpoint || process.env.OLLAMA_ENDPOINT || 'http://localhost:11434',
      apiFormat: 'ollama',
      timeout: options.timeout || 120000, // 2 minutes for large models
      ...options,
    });

    this.deepAnalysis = options.deepAnalysis !== false;
  }

  /**
   * Complete with deep analysis settings
   *
   * @param {string} prompt
   * @param {Object} [options]
   * @returns {Promise<LLMResponse>}
   */
  async complete(prompt, options = {}) {
    // Override defaults for deep analysis
    const deepOptions = {
      temperature: options.temperature ?? 0.2,  // Lower for consistency
      maxTokens: options.maxTokens ?? 1000,     // More tokens for reasoning
      ...options,
    };

    const response = await super.complete(prompt, deepOptions);

    // Mark as deep analysis
    response.metadata.deepAnalysis = this.deepAnalysis;
    response.metadata.type = 'airllm';

    return response;
  }

  /**
   * Check if the AirLLM model is available
   *
   * @returns {Promise<{available: boolean, reason?: string}>}
   */
  async checkAvailability() {
    if (!this.enabled) {
      return { available: false, reason: 'AirLLM adapter not enabled' };
    }

    try {
      const response = await fetch(`${this.endpoint}/api/tags`);
      if (!response.ok) {
        return { available: false, reason: 'Cannot list models' };
      }

      const data = await response.json();
      const models = data.models?.map(m => m.name) || [];
      const modelBase = this.model.split(':')[0];

      if (models.some(m => m.includes(modelBase))) {
        return { available: true, model: this.model };
      }

      return {
        available: false,
        reason: `Model ${this.model} not found. Run: ollama pull ${this.model}`,
        availableModels: models,
      };
    } catch (err) {
      return { available: false, reason: err.message };
    }
  }
}

/**
 * Create an AirLLM validator for deep analysis
 *
 * @param {Object} [options]
 * @param {string} [options.model=mistral:7b-instruct-q4_0] - Large quantized model
 * @param {boolean} [options.deepAnalysis=true] - Enable deep analysis mode
 * @returns {AirLLMAdapter}
 */
export function createAirLLMValidator(options = {}) {
  const adapter = new AirLLMAdapter({
    model: options.model || process.env.CYNIC_AIRLLM_MODEL || 'mistral:7b-instruct-q4_0',
    endpoint: options.endpoint || process.env.OLLAMA_ENDPOINT || 'http://localhost:11434',
    timeout: options.timeout || 120000,
    deepAnalysis: options.deepAnalysis !== false,
  });
  adapter.enabled = true;
  return adapter;
}

/**
 * Check if AirLLM is available and configured
 *
 * @returns {Promise<{available: boolean, reason?: string, model?: string}>}
 */
export async function checkAirLLMAvailability() {
  const enabled = process.env.CYNIC_AIRLLM === 'true';
  if (!enabled) {
    return { available: false, reason: 'AirLLM disabled (set CYNIC_AIRLLM=true)' };
  }

  const adapter = createAirLLMValidator();
  return adapter.checkAvailability();
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
