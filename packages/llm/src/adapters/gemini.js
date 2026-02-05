/**
 * CYNIC Gemini Adapter
 *
 * Adapter for Google Gemini API via @google/generative-ai SDK.
 *
 * Used for:
 * - Design/UI tasks (visual reasoning)
 * - Multi-modal analysis
 * - Consensus validation alongside Claude
 *
 * Env: GEMINI_API_KEY
 *
 * @module @cynic/llm/adapters/gemini
 */

'use strict';

import { createLogger, PHI_INV_2 } from '@cynic/core';
import { LLMAdapter } from './base.js';

const log = createLogger('GeminiAdapter');

/**
 * Gemini LLM Adapter
 *
 * Uses Google's Generative AI SDK for completions.
 * Confidence capped at φ⁻² (38.2%) — external models are validators, not primary.
 */
export class GeminiAdapter extends LLMAdapter {
  constructor(options = {}) {
    super({
      provider: 'gemini',
      model: options.model || 'gemini-2.0-flash',
      ...options,
    });

    this.apiKey = options.apiKey || process.env.GEMINI_API_KEY || null;
    this.timeout = options.timeout || 30000;
    this.enabled = !!this.apiKey;

    // SDK instance (lazy-loaded)
    this._client = null;
    this._genModel = null;
  }

  /**
   * Configure the adapter
   *
   * @param {Object} config
   * @param {string} [config.apiKey] - Gemini API key
   * @param {string} [config.model] - Model name
   */
  configure(config) {
    if (config.apiKey) this.apiKey = config.apiKey;
    if (config.model) this.model = config.model;
    this.enabled = !!this.apiKey;
    this._client = null;
    this._genModel = null;

    log.info('GeminiAdapter configured', {
      model: this.model,
      enabled: this.enabled,
    });
  }

  /**
   * Lazy-load the SDK and create client
   * @private
   */
  async _getModel() {
    if (this._genModel) return this._genModel;

    try {
      const { GoogleGenerativeAI } = await import('@google/generative-ai');
      this._client = new GoogleGenerativeAI(this.apiKey);
      this._genModel = this._client.getGenerativeModel({ model: this.model });
      return this._genModel;
    } catch (err) {
      log.error('Failed to load @google/generative-ai', { error: err.message });
      throw new Error(
        'Gemini SDK not available. Install: npm install @google/generative-ai'
      );
    }
  }

  /**
   * Complete a prompt using Gemini API
   *
   * @param {string} prompt
   * @param {Object} [options]
   * @param {number} [options.temperature=0.7]
   * @param {number} [options.maxTokens=1024]
   * @returns {Promise<LLMResponse>}
   */
  async complete(prompt, options = {}) {
    if (!this.enabled) {
      throw new Error('GeminiAdapter not configured: GEMINI_API_KEY missing');
    }

    this.stats.requests++;
    const startTime = Date.now();

    try {
      const model = await this._getModel();

      const generationConfig = {
        temperature: options.temperature ?? 0.7,
        maxOutputTokens: options.maxTokens ?? 1024,
      };

      const result = await Promise.race([
        model.generateContent({
          contents: [{ role: 'user', parts: [{ text: prompt }] }],
          generationConfig,
        }),
        new Promise((_, reject) =>
          setTimeout(() => reject(new Error('Gemini timeout')), this.timeout)
        ),
      ]);

      const responseText = result.response?.text?.() || '';
      const usage = result.response?.usageMetadata || {};

      const duration = Date.now() - startTime;
      this._updateLatency(duration);

      const tokens = {
        input: usage.promptTokenCount || this._estimateTokens(prompt),
        output: usage.candidatesTokenCount || this._estimateTokens(responseText),
      };

      const response = this._createResponse({
        model: this.model,
        content: responseText,
        confidence: PHI_INV_2, // Gemini gets φ⁻² = 38.2% max confidence
        tokens,
        duration,
        metadata: {
          type: 'gemini',
          finishReason: result.response?.candidates?.[0]?.finishReason,
        },
      });

      this.stats.successes++;
      this.stats.totalTokens += tokens.input + tokens.output;
      this.emit('complete', response);

      return response;
    } catch (err) {
      this.stats.failures++;
      log.error('GeminiAdapter request failed', {
        model: this.model,
        error: err.message,
      });
      throw err;
    }
  }

  /**
   * Check if Gemini API is available
   * @returns {Promise<boolean>}
   */
  async isAvailable() {
    if (!this.enabled) return false;

    try {
      const model = await this._getModel();
      // Quick check — list models or just verify SDK loaded
      return !!model;
    } catch {
      return false;
    }
  }
}

/**
 * Create a Gemini validator
 *
 * @param {Object} [options]
 * @param {string} [options.apiKey] - Gemini API key (defaults to GEMINI_API_KEY env)
 * @param {string} [options.model=gemini-2.0-flash] - Model name
 * @returns {GeminiAdapter}
 */
export function createGeminiValidator(options = {}) {
  return new GeminiAdapter({
    apiKey: options.apiKey || process.env.GEMINI_API_KEY,
    model: options.model || process.env.GEMINI_MODEL || 'gemini-2.0-flash',
    timeout: options.timeout || 30000,
  });
}

export default GeminiAdapter;
