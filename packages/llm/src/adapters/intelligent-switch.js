/**
 * CYNIC Intelligent Switch
 *
 * Unifies all LLM adapters into one intelligent selection system.
 * Selects based on: cost, speed, privacy, quality.
 *
 * Based on research from:
 * - Vibe Companion (WebSocket protocol)
 * - Ollama Claude Code integration
 * - AirLLM disk offloading
 *
 * @module @cynic/llm/adapters/intelligent-switch
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { LLMAdapter } from './base.js';
import { ClaudeCodeAdapter } from './claude-code.js';
import { AnthropicAdapter, MODEL_MAP as ANTHROPIC_MODEL_MAP } from './anthropic.js';
import { OSSLLMAdapter, createOllamaValidator } from './oss-llm.js';
import { AirLLMAdapter } from './airllm.js';
import { WebSocketClaudeAdapter } from './websocket-claude-adapter.js';
import { getOracle, calculateCost } from '../pricing/oracle.js';

const log = createLogger('IntelligentSwitch');

/**
 * Selection strategy based on priorities
 */
export const Strategy = {
  /** Free first - local/websocket */
  FREE: 'free',
  /** Speed first - fastest response */
  SPEED: 'speed',
  /** Quality first - best model */
  QUALITY: 'quality',
  /** Balanced - trade-off */
  BALANCED: 'balanced',
};

/**
 * Priority levels for selection
 */
export const Priority = {
  COST: 'cost',       // Minimize cost
  SPEED: 'speed',    // Minimize latency
  PRIVACY: 'privacy', // Local processing
  QUALITY: 'max',    // Max quality
};

/**
 * Intelligent Switch - unifies all adapters
 */
export class IntelligentSwitch extends LLMAdapter {
  constructor(options = {}) {
    super({
      provider: 'intelligent-switch',
      model: 'unified',
      ...options,
    });

    // Initialize adapters
    this._adapters = {
      claudeCode: new ClaudeCodeAdapter(),
      anthropic: new AnthropicAdapter(),
      ollama: null, // Lazy init
      airllm: null, // Lazy init
      websocketClaude: null, // Lazy init - WebSocket connection
    };

    // Configuration
    this.strategy = options.strategy || Strategy.BALANCED;
    this.priority = options.priority || Priority.QUALITY;
    this.fallbackEnabled = options.fallbackEnabled !== false;
    this.streamingEnabled = options.streamingEnabled !== false;

    // Cache availability
    this._availabilityCache = new Map();
    this._cacheTimeout = options.cacheTimeout || 60000; // 1 min

    // Stats
    this.stats = {
      selections: 0,
      fallbacks: 0,
      failures: 0,
      byAdapter: {},
    };
  }

  /**
   * Detect available adapters and select best
   * Uses REAL costs from PricingOracle - NOTHING IS FREE
   * @private
   */
  async _detectAndSelect(prompt, options = {}) {
    const candidates = [];
    const oracle = getOracle();
    
    // Estimate token counts (will be refined after actual completion)
    const estimatedInputTokens = options.estimatedInputTokens || 1000;
    const estimatedOutputTokens = options.estimatedOutputTokens || 500;

    // 1. Claude Code (subscription cost: $20/month amortized)
    const ccAvailable = await this._adapters.claudeCode.isAvailable();
    if (ccAvailable) {
      const costResult = calculateCost('claudeCode', 'default', estimatedInputTokens, estimatedOutputTokens);
      candidates.push({
        adapter: 'claudeCode',
        adapterRef: this._adapters.claudeCode,
        cost: costResult.cost, // REAL cost (subscription amortized)
        costType: costResult.type,
        speed: 1, // Fastest - already inside
        privacy: 1, // Highest - no data leaves
        quality: PHI_INV, // 61.8%
        streaming: true,
      });
      log.debug('Claude Code cost', { cost: costResult.cost, type: costResult.type });
    }

    // 2. Ollama (GPU electricity + amortized hardware)
    try {
      if (!this._adapters.ollama) {
        this._adapters.ollama = createOllamaValidator();
      }
      const ollamaAvailable = await this._adapters.ollama.isAvailable();
      if (ollamaAvailable) {
        const costResult = calculateCost('ollama', 'default', estimatedInputTokens, estimatedOutputTokens);
        candidates.push({
          adapter: 'ollama',
          adapterRef: this._adapters.ollama,
          cost: costResult.cost, // REAL cost (GPU electricity)
          costType: costResult.type,
          speed: 0.7, // Depends on hardware
          privacy: 1,
          quality: 0.382, // φ⁻²
          streaming: true,
        });
        log.debug('Ollama cost', { cost: costResult.cost, type: costResult.type });
      }
    } catch (e) {
      log.debug('Ollama not available', { error: e.message });
    }

    // 3. Anthropic API (real per-token pricing)
    const anthropicAvailable = await this._adapters.anthropic.isAvailable();
    if (anthropicAvailable) {
      const model = options.model || 'claude-sonnet-4-5-20251101';
      const costResult = calculateCost('anthropic', model, estimatedInputTokens, estimatedOutputTokens);
      candidates.push({
        adapter: 'anthropic',
        adapterRef: this._adapters.anthropic,
        cost: costResult.cost, // REAL per-token cost
        costType: costResult.type,
        speed: 0.9,
        privacy: 0,
        quality: 1, // Best quality
        streaming: this.streamingEnabled,
      });
      log.debug('Anthropic cost', { model, cost: costResult.cost, type: costResult.type });
    }

    // 4. AirLLM (SSD I/O + CPU costs)
    if (process.env.CYNIC_AIRLLM === 'true') {
      try {
        if (!this._adapters.airllm) {
          this._adapters.airllm = new AirLLMAdapter();
        }
        const airllmAvailable = await this._adapters.airllm.isAvailable();
        if (airllmAvailable) {
          const costResult = calculateCost('airllm', 'default', estimatedInputTokens, estimatedOutputTokens);
          candidates.push({
            adapter: 'airllm',
            adapterRef: this._adapters.airllm,
            cost: costResult.cost, // REAL cost (SSD/CPU)
            costType: costResult.type,
            speed: 0.3, // Slow - disk I/O
            privacy: 1,
            quality: 0.618, // φ⁻¹ - decent for 70B
            streaming: false,
          });
          log.debug('AirLLM cost', { cost: costResult.cost, type: costResult.type });
        }
      } catch (e) {
        log.debug('AirLLM not available', { error: e.message });
      }
    }

    // 5. WebSocket Claude (subscription cost like Claude Code)
    if (process.env.CYNIC_WEBSOCKET_URL || process.env.CYNIC_WEBSOCKET_ENABLED === 'true') {
      try {
        if (!this._adapters.websocketClaude) {
          this._adapters.websocketClaude = new WebSocketClaudeAdapter({
            wsUrl: process.env.CYNIC_WEBSOCKET_URL || 'ws://localhost:3456',
          });
        }
        const wsAvailable = await this._adapters.websocketClaude.isAvailable();
        if (wsAvailable) {
          const costResult = calculateCost('websocketClaude', 'default', estimatedInputTokens, estimatedOutputTokens);
          candidates.push({
            adapter: 'websocketClaude',
            adapterRef: this._adapters.websocketClaude,
            cost: costResult.cost, // REAL cost (subscription amortized)
            costType: costResult.type,
            speed: 0.85, // Fast - local WebSocket
            privacy: 1, // High - data stays local
            quality: PHI_INV, // 61.8% - same as Claude Code
            streaming: true,
          });
          log.debug('WebSocket Claude cost', { cost: costResult.cost, type: costResult.type });
        }
      } catch (e) {
        log.debug('WebSocket Claude not available', { error: e.message });
      }
    }

    // Score and rank candidates
    const ranked = this._rankCandidates(candidates, options);

    return ranked;
  }

  /**
   * Rank candidates based on strategy
   * @private
   */
  _rankCandidates(candidates, options = {}) {
    if (candidates.length === 0) {
      throw new Error('No adapters available');
    }

    // Apply strategy weights
    const weights = this._getStrategyWeights(options.strategy || this.strategy);

    for (const candidate of candidates) {
      // Normalize scores (0-1)
      const normalized = {
        cost: 1 - candidate.cost, // Invert cost (lower is better)
        speed: candidate.speed,
        privacy: candidate.privacy,
        quality: candidate.quality,
      };

      // Calculate weighted score
      candidate.score =
        normalized.cost * weights.cost +
        normalized.speed * weights.speed +
        normalized.privacy * weights.privacy +
        normalized.quality * weights.quality;
    }

    // Sort by score descending
    candidates.sort((a, b) => b.score - a.score);

    return candidates;
  }

  /**
   * Get strategy weights
   * @private
   */
  _getStrategyWeights(strategy) {
    switch (strategy) {
      case Strategy.FREE:
        return { cost: 0.5, speed: 0.2, privacy: 0.2, quality: 0.1 };
      case Strategy.SPEED:
        return { cost: 0.1, speed: 0.5, privacy: 0.2, quality: 0.2 };
      case Strategy.QUALITY:
        return { cost: 0.1, speed: 0.2, privacy: 0.1, quality: 0.6 };
      case Strategy.BALANCED:
      default:
        return { cost: 0.25, speed: 0.25, privacy: 0.25, quality: 0.25 };
    }
  }

  /**
   * Complete prompt using best available adapter
   */
  async complete(prompt, options = {}) {
    this.stats.selections++;

    // Detect and select best adapter
    const candidates = await this._detectAndSelect(prompt, options);

    // Try candidates in order (with fallback)
    let lastError = null;
    for (let i = 0; i < candidates.length; i++) {
      const candidate = candidates[i];
      
      // Skip non-streaming if streaming required
      if (options.stream && !candidate.streaming) {
        continue;
      }

      try {
        log.info('Selected adapter', {
          adapter: candidate.adapter,
          score: candidate.score.toFixed(3),
          strategy: this.strategy,
        });

        // Track stats
        if (!this.stats.byAdapter[candidate.adapter]) {
          this.stats.byAdapter[candidate.adapter] = 0;
        }
        this.stats.byAdapter[candidate.adapter]++;

        // Execute with selected adapter
        const response = await candidate.adapterRef.complete(prompt, {
          ...options,
          // Override model if specified
          model: options.model || undefined,
        });

        // Add metadata about selection
        response.metadata.selectedAdapter = candidate.adapter;
        response.metadata.selectionScore = candidate.score;
        response.metadata.strategy = this.strategy;

        return response;

      } catch (error) {
        lastError = error;
        log.warn('Adapter failed, trying fallback', {
          adapter: candidate.adapter,
          error: error.message,
        });

        if (i > 0) {
          this.stats.fallbacks++;
        }
      }
    }

    // All adapters failed
    this.stats.failures++;
    throw lastError || new Error('All adapters failed');
  }

  /**
   * Complete with streaming
   */
  async completeStream(prompt, options = {}) {
    this.stats.selections++;

    // Detect and select - only streaming-capable
    const candidates = await this._detectAndSelect(prompt, { ...options, stream: true });

    for (const candidate of candidates) {
      if (!candidate.streaming) continue;

      try {
        log.info('Selected streaming adapter', { adapter: candidate.adapter });

        const stream = await candidate.adapterRef.complete(prompt, {
          ...options,
          stream: true,
        });

        return stream;
      } catch (error) {
        log.warn('Streaming adapter failed', { adapter: candidate.adapter, error: error.message });
      }
    }

    throw new Error('No streaming adapter available');
  }

  /**
   * Get availability of all adapters
   */
  async getAvailability() {
    const availability = {};

    for (const [name, adapter] of Object.entries(this._adapters)) {
      try {
        availability[name] = await adapter.isAvailable();
      } catch {
        availability[name] = false;
      }
    }

    return availability;
  }

  /**
   * Get switch stats
   */
  getStats() {
    return {
      ...this.stats,
      strategy: this.strategy,
      priority: this.priority,
    };
  }

  /**
   * Set strategy
   */
  setStrategy(strategy) {
    this.strategy = strategy;
    log.info('Strategy changed', { strategy });
  }

  /**
   * Force use specific adapter
   */
  async useAdapter(adapterName, prompt, options = {}) {
    const adapter = this._adapters[adapterName];
    if (!adapter) {
      throw new Error(`Unknown adapter: ${adapterName}`);
    }

    const available = await adapter.isAvailable();
    if (!available) {
      throw new Error(`Adapter not available: ${adapterName}`);
    }

    return adapter.complete(prompt, options);
  }
}

/**
 * Create Intelligent Switch instance
 */
export function createIntelligentSwitch(options = {}) {
  return new IntelligentSwitch(options);
}

/**
 * Factory for quick creation based on priority
 */
export function createForPriority(priority) {
  const strategyMap = {
    [Priority.COST]: Strategy.FREE,
    [Priority.SPEED]: Strategy.SPEED,
    [Priority.PRIVACY]: Strategy.FREE,
    [Priority.QUALITY]: Strategy.QUALITY,
  };

  return new IntelligentSwitch({
    strategy: strategyMap[priority] || Strategy.BALANCED,
    priority,
  });
}

export default IntelligentSwitch;
