/**
 * CYNIC LLM Types
 *
 * Core types for LLM integration across the CYNIC ecosystem.
 *
 * @module @cynic/llm/types
 */

'use strict';

import { PHI_INV, PHI_INV_2 } from '@cynic/core';

/**
 * LLM Response - Standardized response format
 *
 * All LLM responses are normalized to this format regardless of provider.
 */
export class LLMResponse {
  /**
   * @param {Object} data
   * @param {string} [data.id] - Unique response ID
   * @param {number} [data.timestamp] - Response timestamp
   * @param {string} [data.provider] - Provider name (ollama, claude-code, etc.)
   * @param {string} [data.model] - Model name
   * @param {string} [data.content] - Response content
   * @param {number} [data.confidence] - Confidence score (0-1, capped at φ⁻¹)
   * @param {Object} [data.tokens] - Token usage
   * @param {number} [data.duration] - Response time in ms
   * @param {Object} [data.metadata] - Additional metadata
   */
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
 * Consensus Result - Result of multi-LLM voting
 *
 * Represents the outcome of requesting the same prompt from multiple LLMs
 * and calculating agreement.
 */
export class ConsensusResult {
  /**
   * @param {Object} data
   * @param {string} [data.id] - Consensus request ID
   * @param {number} [data.timestamp] - Request timestamp
   * @param {LLMResponse[]} [data.responses] - Individual responses
   * @param {number} [data.agreement] - Agreement ratio (0-1)
   * @param {string} [data.verdict] - Majority verdict/response
   * @param {number} [data.confidence] - Overall confidence
   * @param {LLMResponse[]} [data.dissent] - Dissenting responses
   */
  constructor(data = {}) {
    this.id = data.id || `consensus-${Date.now()}`;
    this.timestamp = data.timestamp || Date.now();
    this.responses = data.responses || [];
    this.agreement = data.agreement || 0;
    this.verdict = data.verdict || null;
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
   * Threshold: 2 × φ⁻² ≈ 76.4% (φ-aligned, higher than consensus)
   */
  get isStrong() {
    return this.agreement >= (2 * PHI_INV_2);
  }

  /**
   * Get the consensus ratio as a percentage string
   */
  get agreementPercent() {
    return `${(this.agreement * 100).toFixed(1)}%`;
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
 * Execution Tiers for LLM routing
 *
 * Determines which class of LLM to use based on task complexity.
 */
export const ExecutionTier = Object.freeze({
  /** No LLM - pattern matching only */
  LOCAL: 'LOCAL',
  /** Small/fast models (gemma2:2b, qwen2:0.5b) */
  LIGHT: 'LIGHT',
  /** Medium models (mistral:7b, llama3:8b) */
  FULL: 'FULL',
  /** Large models via AirLLM (mistral:7b-q4, larger) */
  DEEP: 'DEEP',
});

/**
 * LLM Provider types
 */
export const LLMProvider = Object.freeze({
  OLLAMA: 'ollama',
  CLAUDE_CODE: 'claude-code',
  OPENAI: 'openai',
  LM_STUDIO: 'lm-studio',
  AIRLLM: 'airllm',
});

/**
 * Confidence thresholds (φ-aligned)
 */
export const ConfidenceThresholds = Object.freeze({
  /** Maximum confidence for any response */
  MAX: PHI_INV, // 61.8%
  /** Maximum confidence for OSS LLMs */
  OSS_MAX: PHI_INV_2, // 38.2%
  /** Minimum confidence to act on */
  MIN_ACTIONABLE: 0.3,
  /** Consensus quorum threshold */
  QUORUM: PHI_INV, // 61.8%
});

export default {
  LLMResponse,
  ConsensusResult,
  ExecutionTier,
  LLMProvider,
  ConfidenceThresholds,
};
