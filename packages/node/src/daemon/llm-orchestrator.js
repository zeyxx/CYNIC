/**
 * LLM Orchestrator — Intelligent multi-LLM orchestration with inter-model communication
 *
 * Implements harmonious φ-balanced orchestration across 1-N models:
 * - Strategy-based routing (Single, Pipeline, Consensus, Hybrid)
 * - Inter-LLM protocol for metadata exchange
 * - Graceful degradation (works with 1-N models)
 * - Learning from disagreement (when agreement < φ⁻¹)
 * - Thompson Sampling for adaptive selection
 *
 * "Le chien choisit la meilleure stratégie" - κυνικός
 *
 * @module @cynic/node/daemon/llm-orchestrator
 */

'use strict';

import { createLogger } from '@cynic/core';
import { createSingleton, phiBound } from '@cynic/core/axioms/phi-utils.js';
import { PHI, PHI_INVERSE } from '@cynic/core/axioms/constants.js';

const log = createLogger('LLMOrchestrator');

// Inter-LLM Protocol Constants
const PROTOCOL_VERSION = '1.0.0';
const CONSENSUS_QUORUM = PHI_INVERSE; // 61.8% agreement required
const DISAGREEMENT_THRESHOLD = PHI_INVERSE; // Log when agreement < 61.8%

/**
 * Inter-LLM Metadata Envelope
 */
class LLMEnvelope {
  constructor(content, metadata = {}) {
    this.protocolVersion = PROTOCOL_VERSION;
    this.content = content;
    this.metadata = {
      timestamp: new Date().toISOString(),
      confidence: metadata.confidence ?? 0.5,
      uncertainty: metadata.uncertainty ?? [],
      refinementRequests: metadata.refinementRequests ?? [],
      modelId: metadata.modelId ?? 'unknown',
      strategyUsed: metadata.strategyUsed ?? 'single',
      ...metadata
    };
  }

  /**
   * Create envelope from LLM response
   */
  static fromResponse(response, modelId, strategyUsed) {
    return new LLMEnvelope(response, {
      modelId,
      strategyUsed,
      confidence: extractConfidence(response),
      uncertainty: extractUncertainty(response)
    });
  }

  /**
   * Check if this envelope requests refinement
   */
  needsRefinement() {
    return this.metadata.refinementRequests.length > 0 ||
           this.metadata.confidence < PHI_INVERSE;
  }
}

/**
 * Extract confidence from LLM response (heuristic)
 */
function extractConfidence(response) {
  const lower = response.toLowerCase();

  // High confidence signals
  if (lower.includes('certain') || lower.includes('definitely')) return 0.9;

  // Medium confidence signals
  if (lower.includes('likely') || lower.includes('probably')) return 0.7;

  // Low confidence signals
  if (lower.includes('uncertain') || lower.includes('maybe') || lower.includes('perhaps')) return 0.4;

  // Default: moderate confidence
  return 0.6;
}

/**
 * Extract uncertainty signals from LLM response
 */
function extractUncertainty(response) {
  const uncertainties = [];
  const lower = response.toLowerCase();

  if (lower.includes('not sure')) uncertainties.push('general_uncertainty');
  if (lower.includes('depends on')) uncertainties.push('conditional_uncertainty');
  if (lower.includes('need more context')) uncertainties.push('context_uncertainty');

  return uncertainties;
}

/**
 * Base Strategy Interface
 */
class OrchestrationStrategy {
  constructor(name, adapters) {
    this.name = name;
    this.adapters = adapters; // Array of LLM adapters
  }

  /**
   * Execute strategy
   * @param {string} prompt - User prompt
   * @param {object} context - Execution context
   * @returns {Promise<LLMEnvelope>}
   */
  async execute(prompt, context = {}) {
    throw new Error('Strategy.execute() must be implemented');
  }
}

/**
 * Single Strategy — Use best available model
 */
class SingleStrategy extends OrchestrationStrategy {
  constructor(adapters) {
    super('single', adapters);
  }

  async execute(prompt, context = {}) {
    const adapter = this.adapters[0]; // Use first adapter (already prioritized)

    if (!adapter) {
      throw new Error('No adapters available');
    }

    log.info(`SingleStrategy: using ${adapter.name}`);

    const response = await adapter.generate(prompt, context);
    return LLMEnvelope.fromResponse(response, adapter.name, 'single');
  }
}

/**
 * Pipeline Strategy — Sequential refinement (A → B → C)
 */
class PipelineStrategy extends OrchestrationStrategy {
  constructor(adapters) {
    super('pipeline', adapters);
  }

  async execute(prompt, context = {}) {
    if (this.adapters.length === 0) {
      throw new Error('No adapters available');
    }

    log.info(`PipelineStrategy: ${this.adapters.length} stages`);

    let currentEnvelope = null;
    let currentPrompt = prompt;

    // Stage 1: Draft (fast model)
    const draftAdapter = this.adapters.find(a => a.tier === 'fast') || this.adapters[0];
    const draft = await draftAdapter.generate(currentPrompt, context);
    currentEnvelope = LLMEnvelope.fromResponse(draft, draftAdapter.name, 'pipeline:draft');

    log.info(`Pipeline stage 1/3: draft from ${draftAdapter.name}`);

    // Stage 2: Refine (if available)
    if (this.adapters.length > 1) {
      const refineAdapter = this.adapters.find(a => a.tier === 'moderate') || this.adapters[1];
      const refinePrompt = `Refine this draft response:\n\n${draft}\n\nOriginal prompt: ${prompt}`;
      const refined = await refineAdapter.generate(refinePrompt, context);
      currentEnvelope = LLMEnvelope.fromResponse(refined, refineAdapter.name, 'pipeline:refine');

      log.info(`Pipeline stage 2/3: refine from ${refineAdapter.name}`);
    }

    // Stage 3: Finalize (if available and needed)
    if (this.adapters.length > 2 && currentEnvelope.needsRefinement()) {
      const finalAdapter = this.adapters.find(a => a.tier === 'premium') || this.adapters[this.adapters.length - 1];
      const finalPrompt = `Finalize this response:\n\n${currentEnvelope.content}\n\nOriginal prompt: ${prompt}`;
      const final = await finalAdapter.generate(finalPrompt, context);
      currentEnvelope = LLMEnvelope.fromResponse(final, finalAdapter.name, 'pipeline:final');

      log.info(`Pipeline stage 3/3: finalize from ${finalAdapter.name}`);
    }

    return currentEnvelope;
  }
}

/**
 * Consensus Strategy — Parallel voting with quorum
 */
class ConsensusStrategy extends OrchestrationStrategy {
  constructor(adapters, quorum = CONSENSUS_QUORUM) {
    super('consensus', adapters);
    this.quorum = quorum;
  }

  async execute(prompt, context = {}) {
    if (this.adapters.length < 2) {
      log.warn('ConsensusStrategy needs 2+ adapters, falling back to SingleStrategy');
      const singleStrategy = new SingleStrategy(this.adapters);
      return singleStrategy.execute(prompt, context);
    }

    log.info(`ConsensusStrategy: ${this.adapters.length} voters, quorum=${this.quorum}`);

    // Execute in parallel
    const responses = await Promise.all(
      this.adapters.map(async adapter => {
        const response = await adapter.generate(prompt, context);
        return LLMEnvelope.fromResponse(response, adapter.name, 'consensus');
      })
    );

    // Calculate agreement (simplified: exact match on normalized response)
    const normalized = responses.map(r => r.content.toLowerCase().trim());
    const agreement = this._calculateAgreement(normalized);

    log.info(`Consensus agreement: ${(agreement * 100).toFixed(1)}%`);

    // Check quorum
    if (agreement >= this.quorum) {
      log.info('Quorum reached - consensus achieved');
      // Return majority response
      return this._getMajorityResponse(responses);
    } else {
      log.warn(`Quorum not reached (${(agreement * 100).toFixed(1)}% < ${(this.quorum * 100).toFixed(1)}%)`);

      // Log disagreement for learning
      this._logDisagreement(prompt, responses, agreement);

      // Return aggregated response
      return this._aggregateResponses(responses, agreement);
    }
  }

  /**
   * Calculate agreement between responses (0-1)
   */
  _calculateAgreement(normalized) {
    if (normalized.length < 2) return 1.0;

    // Count unique responses
    const unique = new Set(normalized);
    const mostCommon = Array.from(unique).map(text => ({
      text,
      count: normalized.filter(n => n === text).length
    })).sort((a, b) => b.count - a.count)[0];

    return mostCommon.count / normalized.length;
  }

  /**
   * Get majority response
   */
  _getMajorityResponse(responses) {
    const counts = new Map();

    for (const envelope of responses) {
      const key = envelope.content.toLowerCase().trim();
      counts.set(key, (counts.get(key) || 0) + 1);
    }

    const majority = Array.from(counts.entries())
      .sort((a, b) => b[1] - a[1])[0][0];

    return responses.find(r => r.content.toLowerCase().trim() === majority);
  }

  /**
   * Aggregate multiple responses when no consensus
   */
  _aggregateResponses(responses, agreement) {
    // Combine responses with metadata about disagreement
    const combined = responses.map((r, i) => `**Model ${i + 1} (${r.metadata.modelId}):**\n${r.content}`).join('\n\n');

    const aggregated = `Multiple perspectives (${(agreement * 100).toFixed(1)}% agreement):\n\n${combined}`;

    return new LLMEnvelope(aggregated, {
      modelId: 'consensus-aggregate',
      strategyUsed: 'consensus',
      confidence: phiBound(agreement), // Use agreement as confidence
      uncertainty: ['low_consensus'],
      participatingModels: responses.map(r => r.metadata.modelId)
    });
  }

  /**
   * Log disagreement for learning
   */
  _logDisagreement(prompt, responses, agreement) {
    log.info('Logging disagreement for learning', {
      prompt: prompt.substring(0, 100),
      agreement: (agreement * 100).toFixed(1) + '%',
      models: responses.map(r => r.metadata.modelId)
    });

    // TODO: Emit to learning system
    // globalEventBus.emit('llm:disagreement', { prompt, responses, agreement });
  }
}

/**
 * LLM Orchestrator
 */
class LLMOrchestrator {
  constructor(adapters = []) {
    this.adapters = adapters;
    this.strategies = new Map();
    this.defaultStrategy = 'single';

    // Register built-in strategies
    this.registerStrategy('single', new SingleStrategy(adapters));
    this.registerStrategy('pipeline', new PipelineStrategy(adapters));
    this.registerStrategy('consensus', new ConsensusStrategy(adapters));

    // Statistics
    this.stats = {
      totalRequests: 0,
      strategyUsage: {
        single: 0,
        pipeline: 0,
        consensus: 0,
        custom: 0
      },
      avgAgreement: 0,
      disagreements: 0
    };

    log.info('LLMOrchestrator created', {
      adapters: adapters.length,
      strategies: Array.from(this.strategies.keys())
    });
  }

  /**
   * Register custom strategy
   */
  registerStrategy(name, strategy) {
    this.strategies.set(name, strategy);
    log.info(`Strategy registered: ${name}`);
  }

  /**
   * Orchestrate LLM request
   *
   * @param {string} prompt - User prompt
   * @param {object} options - Orchestration options
   * @param {string} options.strategy - Strategy to use ('single', 'pipeline', 'consensus', custom)
   * @param {object} options.context - Additional context
   * @returns {Promise<object>} Result with envelope and metadata
   */
  async orchestrate(prompt, options = {}) {
    this.stats.totalRequests++;

    const strategyName = options.strategy || this.defaultStrategy;
    const strategy = this.strategies.get(strategyName);

    if (!strategy) {
      throw new Error(`Unknown strategy: ${strategyName}`);
    }

    log.info(`Orchestrating with strategy: ${strategyName}`);

    const startTime = Date.now();
    const envelope = await strategy.execute(prompt, options.context || {});
    const latency = Date.now() - startTime;

    // Update stats
    this.stats.strategyUsage[strategyName] = (this.stats.strategyUsage[strategyName] || 0) + 1;

    return {
      envelope,
      latency,
      strategy: strategyName,
      adapters: this.adapters.map(a => a.name)
    };
  }

  /**
   * Get orchestrator statistics
   */
  getStats() {
    return {
      ...this.stats,
      adaptersAvailable: this.adapters.length,
      strategiesRegistered: Array.from(this.strategies.keys())
    };
  }

  /**
   * Reset statistics (for testing)
   */
  resetStats() {
    this.stats = {
      totalRequests: 0,
      strategyUsage: {
        single: 0,
        pipeline: 0,
        consensus: 0,
        custom: 0
      },
      avgAgreement: 0,
      disagreements: 0
    };
  }
}

/**
 * Create singleton instance
 */
const { getInstance, resetInstance } = createSingleton(LLMOrchestrator);

export {
  getInstance as getLLMOrchestrator,
  resetInstance as resetLLMOrchestrator,
  LLMOrchestrator,
  LLMEnvelope,
  OrchestrationStrategy,
  SingleStrategy,
  PipelineStrategy,
  ConsensusStrategy
};
