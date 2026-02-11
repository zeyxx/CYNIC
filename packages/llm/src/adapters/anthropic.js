/**
 * CYNIC Anthropic Adapter
 *
 * Direct adapter for Anthropic API via @anthropic-ai/sdk.
 * CYNIC's primary brain — calls Claude directly instead of being trapped inside one.
 *
 * Used for:
 * - Primary reasoning (code, architecture, security)
 * - Multi-model consensus alongside Ollama/Gemini
 * - Thompson-selected tier routing (opus/sonnet/haiku)
 *
 * Env: ANTHROPIC_API_KEY
 *
 * "Le chien pense par lui-même" — κυνικός
 *
 * @module @cynic/llm/adapters/anthropic
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { LLMAdapter } from './base.js';

const log = createLogger('AnthropicAdapter');

/**
 * Model tier → Anthropic model ID mapping
 */
export const MODEL_MAP = Object.freeze({
  opus: 'claude-opus-4-6',
  sonnet: 'claude-sonnet-4-5-20250929',
  haiku: 'claude-haiku-4-5-20251001',
});

/**
 * Anthropic LLM Adapter
 *
 * Uses Anthropic's SDK for direct API calls.
 * Confidence at φ⁻¹ (61.8%) — Anthropic is CYNIC's primary provider.
 */
export class AnthropicAdapter extends LLMAdapter {
  constructor(options = {}) {
    super({
      provider: 'anthropic',
      model: options.model || MODEL_MAP.sonnet,
      ...options,
    });

    this.apiKey = options.apiKey || process.env.ANTHROPIC_API_KEY || null;
    this.timeout = options.timeout || 60000;
    this.enabled = !!this.apiKey;

    // SDK instance (lazy-loaded)
    this._client = null;
  }

  /**
   * Configure the adapter
   *
   * @param {Object} config
   * @param {string} [config.apiKey] - Anthropic API key
   * @param {string} [config.model] - Model name or tier key (opus/sonnet/haiku)
   */
  configure(config) {
    if (config.apiKey) this.apiKey = config.apiKey;
    if (config.model) this.model = MODEL_MAP[config.model] || config.model;
    this.enabled = !!this.apiKey;
    this._client = null;

    log.info('AnthropicAdapter configured', {
      model: this.model,
      enabled: this.enabled,
    });
  }

  /**
   * Lazy-load the SDK and create client
   * @private
   */
  async _getClient() {
    if (this._client) return this._client;

    try {
      const { default: Anthropic } = await import('@anthropic-ai/sdk');
      this._client = new Anthropic({ apiKey: this.apiKey });
      return this._client;
    } catch (err) {
      log.error('Failed to load @anthropic-ai/sdk', { error: err.message });
      throw new Error(
        'Anthropic SDK not available. Install: npm install @anthropic-ai/sdk'
      );
    }
  }

  /**
   * Complete a prompt using Anthropic Messages API
   *
   * @param {string} prompt
   * @param {Object} [options]
   * @param {number} [options.temperature=0.7]
   * @param {number} [options.maxTokens=1024]
   * @param {string} [options.system] - System prompt
   * @param {string} [options.model] - Override model (tier key or full ID)
   * @returns {Promise<LLMResponse>}
   */
  async complete(prompt, options = {}) {
    if (!this.enabled) {
      throw new Error('AnthropicAdapter not configured: ANTHROPIC_API_KEY missing');
    }

    this.stats.requests++;
    const startTime = Date.now();

    try {
      const client = await this._getClient();

      const model = MODEL_MAP[options.model] || options.model || this.model;
      const params = {
        model,
        max_tokens: options.maxTokens ?? 1024,
        temperature: options.temperature ?? 0.7,
        messages: [{ role: 'user', content: prompt }],
      };

      if (options.system) {
        params.system = options.system;
      }

      const result = await Promise.race([
        client.messages.create(params),
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error('Anthropic timeout')), this.timeout)
        ),
      ]);

      const responseText = result.content?.[0]?.text || '';
      const duration = Date.now() - startTime;
      this._updateLatency(duration);

      const tokens = {
        input: result.usage?.input_tokens || this._estimateTokens(prompt),
        output: result.usage?.output_tokens || this._estimateTokens(responseText),
      };

      const response = this._createResponse({
        model,
        content: responseText,
        confidence: PHI_INV, // Anthropic = primary brain = φ⁻¹ max
        tokens,
        duration,
        metadata: {
          type: 'anthropic',
          stopReason: result.stop_reason,
          id: result.id,
        },
      });

      this.stats.successes++;
      this.stats.totalTokens += tokens.input + tokens.output;
      this.emit('complete', response);

      return response;
    } catch (err) {
      this.stats.failures++;
      log.error('AnthropicAdapter request failed', {
        model: this.model,
        error: err.message,
      });
      throw err;
    }
  }

  /**
   * Check if Anthropic API is available
   * @returns {Promise<boolean>}
   */
  async isAvailable() {
    if (!this.enabled) return false;

    try {
      await this._getClient();
      return true;
    } catch {
      return false;
    }
  }
}

/**
 * Create an Anthropic validator
 *
 * @param {Object} [options]
 * @param {string} [options.apiKey] - Anthropic API key (defaults to ANTHROPIC_API_KEY env)
 * @param {string} [options.model] - Model name or tier key (defaults to sonnet)
 * @returns {AnthropicAdapter}
 */
export function createAnthropicValidator(options = {}) {
  const model = options.model || process.env.ANTHROPIC_MODEL || 'sonnet';
  return new AnthropicAdapter({
    apiKey: options.apiKey || process.env.ANTHROPIC_API_KEY,
    model: MODEL_MAP[model] || model,
    timeout: options.timeout || 60000,
  });
}

export default AnthropicAdapter;
