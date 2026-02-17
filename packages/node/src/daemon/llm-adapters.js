/**
 * LLM Adapters — Unified interface for API, Browser, SDK, and Local models
 *
 * Provides abstraction over different LLM access methods:
 * - API: REST APIs (Anthropic, OpenAI, etc.)
 * - Local: Ollama (unlimited, local)
 * - Browser: Playwright automation (Gemini, Claude web UIs)
 * - SDK: Claude Code --sdk-url integration
 *
 * "Le chien parle à tous les modèles" - κυνικός
 *
 * @module @cynic/node/daemon/llm-adapters
 */

'use strict';

import { createLogger } from '@cynic/core';
import { phiBound } from '@cynic/core/axioms/phi-utils.js';

const log = createLogger('LLMAdapters');

const OLLAMA_BASE_URL = 'http://localhost:11434';

/**
 * Base LLM Adapter Interface
 */
class LLMAdapter {
  constructor(name, tier = 'moderate', options = {}) {
    this.name = name;
    this.tier = tier; // 'fast', 'moderate', 'premium'
    this.options = options;
    this.stats = {
      requests: 0,
      successes: 0,
      failures: 0,
      avgLatency: 0
    };
  }

  /**
   * Generate response from LLM
   * @param {string} prompt - User prompt
   * @param {object} context - Generation context (temperature, max_tokens, etc.)
   * @returns {Promise<string>} - Generated response
   */
  async generate(prompt, context = {}) {
    throw new Error('LLMAdapter.generate() must be implemented');
  }

  /**
   * Check if adapter is available
   * @returns {Promise<boolean>}
   */
  async isAvailable() {
    return true; // Override in subclasses
  }

  /**
   * Get adapter cost estimate (in USD)
   * @param {number} inputTokens - Estimated input tokens
   * @param {number} outputTokens - Estimated output tokens
   * @returns {number} - Cost in USD
   */
  getCost(inputTokens, outputTokens) {
    return 0; // Override in subclasses
  }

  /**
   * Update statistics
   */
  _updateStats(latency, success) {
    this.stats.requests++;

    if (success) {
      this.stats.successes++;
    } else {
      this.stats.failures++;
    }

    // EMA for latency
    if (this.stats.avgLatency === 0) {
      this.stats.avgLatency = latency;
    } else {
      this.stats.avgLatency = 0.2 * latency + 0.8 * this.stats.avgLatency;
    }
  }
}

/**
 * Ollama Adapter (Local, unlimited)
 */
class OllamaAdapter extends LLMAdapter {
  constructor(model = 'mistral:7b-instruct-q4_0', tier = 'moderate') {
    super(`ollama:${model}`, tier, { baseUrl: OLLAMA_BASE_URL });
    this.model = model;
  }

  async generate(prompt, context = {}) {
    const startTime = Date.now();

    try {
      const response = await fetch(`${OLLAMA_BASE_URL}/api/generate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          model: this.model,
          prompt,
          stream: false,
          options: {
            temperature: context.temperature ?? 0.7,
            top_p: context.top_p ?? 0.9,
            num_predict: context.max_tokens ?? 512
          }
        })
      });

      if (!response.ok) {
        throw new Error(`Ollama API error: ${response.status}`);
      }

      const data = await response.json();
      const latency = Date.now() - startTime;

      this._updateStats(latency, true);

      log.info(`Ollama response`, {
        model: this.model,
        latency: `${latency}ms`,
        tokens: data.eval_count
      });

      return data.response;

    } catch (err) {
      const latency = Date.now() - startTime;
      this._updateStats(latency, false);

      log.error(`Ollama generation failed`, {
        model: this.model,
        error: err.message
      });

      throw err;
    }
  }

  async isAvailable() {
    try {
      const response = await fetch(`${OLLAMA_BASE_URL}/api/tags`);
      return response.ok;
    } catch {
      return false;
    }
  }

  getCost(inputTokens, outputTokens) {
    return 0; // Ollama is free (local)
  }
}

/**
 * API Adapter (Anthropic, OpenAI, etc.)
 */
class APIAdapter extends LLMAdapter {
  constructor(provider, model, apiKey, tier = 'premium') {
    super(`${provider}:${model}`, tier, { apiKey });
    this.provider = provider;
    this.model = model;
    this.apiKey = apiKey;
  }

  async generate(prompt, context = {}) {
    const startTime = Date.now();

    try {
      let response;

      if (this.provider === 'anthropic') {
        response = await this._callAnthropic(prompt, context);
      } else if (this.provider === 'openai') {
        response = await this._callOpenAI(prompt, context);
      } else {
        throw new Error(`Unsupported provider: ${this.provider}`);
      }

      const latency = Date.now() - startTime;
      this._updateStats(latency, true);

      return response;

    } catch (err) {
      const latency = Date.now() - startTime;
      this._updateStats(latency, false);

      log.error(`API generation failed`, {
        provider: this.provider,
        model: this.model,
        error: err.message
      });

      throw err;
    }
  }

  async _callAnthropic(prompt, context) {
    const response = await fetch('https://api.anthropic.com/v1/messages', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-api-key': this.apiKey,
        'anthropic-version': '2023-06-01'
      },
      body: JSON.stringify({
        model: this.model,
        max_tokens: context.max_tokens ?? 1024,
        messages: [{ role: 'user', content: prompt }],
        temperature: context.temperature ?? 0.7
      })
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`Anthropic API error: ${error.error?.message || response.statusText}`);
    }

    const data = await response.json();
    return data.content[0].text;
  }

  async _callOpenAI(prompt, context) {
    const response = await fetch('https://api.openai.com/v1/chat/completions', {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'Authorization': `Bearer ${this.apiKey}`
      },
      body: JSON.stringify({
        model: this.model,
        messages: [{ role: 'user', content: prompt }],
        temperature: context.temperature ?? 0.7,
        max_tokens: context.max_tokens ?? 1024
      })
    });

    if (!response.ok) {
      const error = await response.json();
      throw new Error(`OpenAI API error: ${error.error?.message || response.statusText}`);
    }

    const data = await response.json();
    return data.choices[0].message.content;
  }

  async isAvailable() {
    return !!this.apiKey;
  }

  getCost(inputTokens, outputTokens) {
    // Anthropic pricing (as of 2024)
    if (this.provider === 'anthropic') {
      if (this.model.includes('opus')) {
        return (inputTokens * 15 + outputTokens * 75) / 1_000_000;
      } else if (this.model.includes('sonnet')) {
        return (inputTokens * 3 + outputTokens * 15) / 1_000_000;
      } else if (this.model.includes('haiku')) {
        return (inputTokens * 0.25 + outputTokens * 1.25) / 1_000_000;
      }
    }

    // OpenAI pricing (approximate)
    if (this.provider === 'openai') {
      if (this.model.includes('gpt-4')) {
        return (inputTokens * 30 + outputTokens * 60) / 1_000_000;
      } else if (this.model.includes('gpt-3.5')) {
        return (inputTokens * 0.5 + outputTokens * 1.5) / 1_000_000;
      }
    }

    return 0;
  }
}

/**
 * Browser Adapter (Playwright automation for web UIs)
 *
 * NOTE: This is a STUB implementation - actual browser automation
 * requires Playwright setup and page interaction logic
 */
class BrowserAdapter extends LLMAdapter {
  constructor(service, tier = 'premium') {
    super(`browser:${service}`, tier);
    this.service = service; // 'gemini', 'claude'
    this.browser = null; // Playwright browser instance (to be initialized)
  }

  async generate(prompt, context = {}) {
    throw new Error('BrowserAdapter not yet implemented - requires Playwright setup');

    // Future implementation:
    // 1. Launch browser if not running
    // 2. Navigate to service URL
    // 3. Type prompt into input
    // 4. Wait for response
    // 5. Extract text from response element
    // 6. Return response
  }

  async isAvailable() {
    return false; // Not implemented yet
  }

  getCost(inputTokens, outputTokens) {
    // Browser access is "free" (uses subscription)
    return 0;
  }
}

/**
 * SDK Adapter (Claude Code --sdk-url integration)
 *
 * NOTE: This is a STUB implementation - requires Claude Code SDK server
 */
class SDKAdapter extends LLMAdapter {
  constructor(sdkUrl, tier = 'premium') {
    super('sdk:claude-code', tier, { sdkUrl });
    this.sdkUrl = sdkUrl;
  }

  async generate(prompt, context = {}) {
    throw new Error('SDKAdapter not yet implemented - requires Claude Code SDK server');

    // Future implementation:
    // 1. POST to SDK URL with prompt
    // 2. Receive streamed response
    // 3. Return aggregated text
  }

  async isAvailable() {
    return false; // Not implemented yet
  }

  getCost(inputTokens, outputTokens) {
    // SDK uses Claude API under the hood
    return (inputTokens * 3 + outputTokens * 15) / 1_000_000; // Sonnet pricing
  }
}

/**
 * Adapter Factory — Create adapters from configuration
 */
class AdapterFactory {
  /**
   * Create adapter from config
   *
   * @param {object} config - Adapter configuration
   * @param {string} config.type - Adapter type ('ollama', 'api', 'browser', 'sdk')
   * @param {string} config.model - Model name/identifier
   * @param {string} config.tier - Tier ('fast', 'moderate', 'premium')
   * @param {object} config.options - Adapter-specific options
   */
  static create(config) {
    const { type, model, tier = 'moderate', options = {} } = config;

    switch (type) {
      case 'ollama':
        return new OllamaAdapter(model || 'mistral:7b-instruct-q4_0', tier);

      case 'api':
        if (!options.provider || !options.apiKey) {
          throw new Error('API adapter requires provider and apiKey');
        }
        return new APIAdapter(options.provider, model, options.apiKey, tier);

      case 'browser':
        return new BrowserAdapter(options.service || 'gemini', tier);

      case 'sdk':
        if (!options.sdkUrl) {
          throw new Error('SDK adapter requires sdkUrl');
        }
        return new SDKAdapter(options.sdkUrl, tier);

      default:
        throw new Error(`Unknown adapter type: ${type}`);
    }
  }

  /**
   * Create adapters from user configuration
   *
   * Example config:
   * {
   *   adapters: [
   *     { type: 'ollama', model: 'qwen2.5:1.5b', tier: 'fast' },
   *     { type: 'ollama', model: 'mistral:7b-instruct-q4_0', tier: 'moderate' },
   *     { type: 'api', model: 'claude-sonnet-4-5', tier: 'premium', options: { provider: 'anthropic', apiKey: 'sk-...' } }
   *   ]
   * }
   */
  static createFromConfig(config) {
    if (!config.adapters || !Array.isArray(config.adapters)) {
      throw new Error('Config must have adapters array');
    }

    return config.adapters.map(adapterConfig => this.create(adapterConfig));
  }
}

export {
  LLMAdapter,
  OllamaAdapter,
  APIAdapter,
  BrowserAdapter,
  SDKAdapter,
  AdapterFactory
};
