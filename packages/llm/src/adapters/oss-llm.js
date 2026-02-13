/**
 * CYNIC OSS LLM Adapter
 *
 * Adapter for Ollama and OpenAI-compatible endpoints.
 *
 * Supports:
 * - Ollama API (default, http://localhost:11434)
 * - OpenAI-compatible APIs (LM Studio, vLLM, etc.)
 *
 * @module @cynic/llm/adapters/oss-llm
 */

'use strict';

import { createLogger, PHI_INV_2 } from '@cynic/core';
import { LLMAdapter } from './base.js';

const log = createLogger('OSSLLMAdapter');

/**
 * OSS LLM Adapter for Ollama and OpenAI-compatible endpoints
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
    this.enabled = options.autoEnable ? !!this.endpoint : false;
  }

  /**
   * Configure the adapter
   *
   * @param {Object} config
   * @param {string} config.endpoint - API endpoint
   * @param {string} [config.apiKey] - API key (for OpenAI-compatible)
   * @param {string} [config.model] - Model name
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

      const response = this._createResponse({
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
 * Create Llama adapter (Ollama)
 *
 * @param {Object} [options]
 * @param {string} [options.model=llama3.2] - Llama model version
 * @param {string} [options.endpoint] - Ollama endpoint
 * @returns {OSSLLMAdapter}
 */
export function createLlamaValidator(options = {}) {
  return createOllamaValidator({
    ...options,
    model: options.model || 'llama3.2',
  });
}

/**
 * Create Mistral adapter (Ollama)
 *
 * @param {Object} [options]
 * @param {string} [options.model=mistral] - Mistral model version
 * @param {string} [options.endpoint] - Ollama endpoint
 * @returns {OSSLLMAdapter}
 */
export function createMistralValidator(options = {}) {
  return createOllamaValidator({
    ...options,
    model: options.model || 'mistral',
    provider: 'mistral',
  });
}

/**
 * Create DeepSeek adapter (Ollama)
 *
 * @param {Object} [options]
 * @param {string} [options.model=deepseek-coder] - DeepSeek model version
 * @param {string} [options.endpoint] - Ollama endpoint
 * @returns {OSSLLMAdapter}
 */
export function createDeepSeekValidator(options = {}) {
  return createOllamaValidator({
    ...options,
    model: options.model || 'deepseek-coder',
    provider: 'deepseek',
  });
}

/**
 * Create Qwen adapter (Ollama)
 *
 * @param {Object} [options]
 * @param {string} [options.model=qwen2.5] - Qwen model version
 * @param {string} [options.endpoint] - Ollama endpoint
 * @returns {OSSLLMAdapter}
 */
export function createQwenValidator(options = {}) {
  return createOllamaValidator({
    ...options,
    model: options.model || 'qwen2.5',
    provider: 'qwen',
  });
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

export default OSSLLMAdapter;
