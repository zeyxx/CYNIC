/**
 * Engine Integration for CYNICJudge
 *
 * Connects the 73 philosophical engines to the judgment process.
 * Each engine provides a unique perspective based on its domain.
 *
 * "The wisdom of the pack" - κυνικός
 *
 * @module @cynic/node/judge/engine-integration
 */

'use strict';

import { PHI_INV, createLogger } from '@cynic/core';

const log = createLogger('EngineIntegration');

/**
 * Map judgment contexts to relevant engine domains
 */
export const CONTEXT_DOMAIN_MAP = {
  // Code judgments → logic, epistemology, decision
  code: ['logic', 'epistemology', 'decision', 'mathematics'],
  'code-quality': ['logic', 'epistemology', 'decision'],
  'code-review': ['logic', 'ethics', 'decision'],

  // Knowledge judgments → epistemology, science, metaphysics
  knowledge: ['epistemology', 'science', 'metaphysics'],
  fact: ['epistemology', 'science', 'logic'],
  claim: ['epistemology', 'logic', 'science'],

  // Ethical judgments → ethics, politics, law
  ethics: ['ethics', 'politics', 'law'],
  moral: ['ethics', 'mind', 'social'],
  decision: ['ethics', 'decision', 'logic'],

  // Creative judgments → aesthetics, language, mind
  creative: ['aesthetics', 'language', 'mind'],
  writing: ['language', 'aesthetics', 'epistemology'],
  design: ['aesthetics', 'mind', 'logic'],

  // Technical judgments → science, mathematics, logic
  technical: ['science', 'mathematics', 'logic'],
  architecture: ['logic', 'decision', 'integration'],
  security: ['ethics', 'logic', 'decision'],

  // Token/burn judgments → economics, ethics, decision
  token: ['economics', 'ethics', 'decision'],
  burn: ['economics', 'ethics', 'integration'],

  // Pattern judgments → logic, metaphysics, integration
  pattern: ['logic', 'metaphysics', 'integration'],
  anomaly: ['logic', 'metaphysics', 'epistemology'],

  // Default → broad coverage
  general: ['ethics', 'epistemology', 'logic', 'decision'],
};

/**
 * Engine consultation result
 */
export class EngineConsultation {
  constructor(data = {}) {
    this.engines = data.engines || [];
    this.consensus = data.consensus || null;
    this.insights = data.insights || [];
    this.confidence = data.confidence || 0;
    this.dissent = data.dissent || [];
    this.duration = data.duration || 0;
  }

  toJSON() {
    return {
      engineCount: this.engines.length,
      consensus: this.consensus,
      insights: this.insights.slice(0, 5), // Top 5 insights
      confidence: this.confidence,
      hasDisssent: this.dissent.length > 0,
      duration: this.duration,
    };
  }
}

/**
 * Engine Integration for CYNICJudge
 *
 * Provides methods to consult philosophy engines during judgment.
 */
export class EngineIntegration {
  /**
   * @param {Object} options
   * @param {import('@cynic/core/engines').EngineRegistry} options.registry - Engine registry
   * @param {import('@cynic/core/engines').EngineOrchestrator} [options.orchestrator] - Engine orchestrator
   */
  constructor(options = {}) {
    this.registry = options.registry || null;
    this.orchestrator = options.orchestrator || null;
    this.enabled = !!this.registry;

    // Stats
    this.stats = {
      consultations: 0,
      engineCalls: 0,
      avgDuration: 0,
    };
  }

  /**
   * Check if engine integration is available
   */
  isEnabled() {
    return this.enabled && this.registry && this.registry.size > 0;
  }

  /**
   * Get relevant engines for a judgment context
   *
   * @param {string} contextType - Context type (e.g., 'code', 'ethics')
   * @param {Object} [options] - Options
   * @param {number} [options.maxEngines=5] - Maximum engines to return
   * @returns {Object[]} Relevant engines
   */
  getRelevantEngines(contextType, options = {}) {
    const { maxEngines = 5 } = options;

    if (!this.isEnabled()) return [];

    // Get domains for this context
    const domains = CONTEXT_DOMAIN_MAP[contextType] || CONTEXT_DOMAIN_MAP.general;

    // Collect engines from relevant domains
    const engines = [];
    for (const domain of domains) {
      const domainEngines = this.registry.getByDomain(domain);
      for (const engine of domainEngines) {
        if (!engines.find(e => e.id === engine.id)) {
          engines.push(engine);
        }
        if (engines.length >= maxEngines) break;
      }
      if (engines.length >= maxEngines) break;
    }

    return engines;
  }

  /**
   * Consult engines for a judgment
   *
   * @param {Object} item - Item being judged
   * @param {Object} context - Judgment context
   * @param {Object} [options] - Consultation options
   * @returns {Promise<EngineConsultation>}
   */
  async consult(item, context = {}, options = {}) {
    const startTime = Date.now();
    const { maxEngines = 5, timeout = 5000 } = options;

    if (!this.isEnabled()) {
      return new EngineConsultation({ engines: [], confidence: 0 });
    }

    const contextType = context.type || 'general';
    const engines = this.getRelevantEngines(contextType, { maxEngines });

    if (engines.length === 0) {
      return new EngineConsultation({ engines: [], confidence: 0 });
    }

    // Prepare input for engines
    const input = {
      item,
      context,
      question: this._formulateQuestion(item, context),
    };

    // Consult each engine
    const results = [];
    const timeoutPromise = new Promise((_, reject) =>
      setTimeout(() => reject(new Error('Engine consultation timeout')), timeout)
    );

    for (const engine of engines) {
      try {
        const resultPromise = engine.evaluate(input, context);
        const result = await Promise.race([resultPromise, timeoutPromise]);

        if (result && result.insight) {
          results.push({
            engineId: engine.id,
            domain: engine.domain,
            insight: result.insight,
            confidence: result.confidence || PHI_INV,
            reasoning: result.reasoning || '',
          });
          this.stats.engineCalls++;
        }
      } catch (error) {
        // Engine failed - skip it
        log.warn('Engine failed', { engine: engine.id, error: error.message });
      }
    }

    // Calculate consensus
    const consultation = this._synthesizeResults(results, engines);
    consultation.duration = Date.now() - startTime;

    // Update stats
    this.stats.consultations++;
    this.stats.avgDuration =
      (this.stats.avgDuration * (this.stats.consultations - 1) + consultation.duration) /
      this.stats.consultations;

    return consultation;
  }

  /**
   * Formulate a question for engines based on item and context
   * @private
   */
  _formulateQuestion(item, context) {
    const type = item.type || context.type || 'item';

    if (item.question) return item.question;

    // Generate question based on item type
    switch (type) {
      case 'code':
        return `Evaluate this code for quality, correctness, and design: ${item.summary || item.content?.slice(0, 200)}`;
      case 'decision':
        return `Analyze this decision: ${item.description || item.content}`;
      case 'claim':
        return `Assess the validity of this claim: ${item.claim || item.content}`;
      case 'pattern':
        return `What does this pattern reveal? ${item.description || item.content}`;
      default:
        return `Provide your perspective on: ${item.summary || item.description || JSON.stringify(item).slice(0, 200)}`;
    }
  }

  /**
   * Synthesize results from multiple engines
   * @private
   */
  _synthesizeResults(results, engines) {
    if (results.length === 0) {
      return new EngineConsultation({
        engines: engines.map(e => e.id),
        consensus: null,
        insights: [],
        confidence: 0,
      });
    }

    // Sort by confidence
    results.sort((a, b) => (b.confidence || 0) - (a.confidence || 0));

    // Extract insights
    const insights = results.map(r => ({
      engine: r.engineId,
      domain: r.domain,
      insight: r.insight,
      confidence: r.confidence,
    }));

    // Calculate weighted consensus confidence
    const totalWeight = results.reduce((sum, r) => sum + (r.confidence || 0), 0);
    const avgConfidence = totalWeight / results.length;

    // Check for dissent (low confidence or contradictions)
    const dissent = results.filter(r => (r.confidence || 0) < PHI_INV * 0.5);

    // Build consensus summary
    const consensus = this._buildConsensus(results);

    return new EngineConsultation({
      engines: engines.map(e => e.id),
      consensus,
      insights,
      confidence: Math.min(avgConfidence, PHI_INV), // Cap at φ⁻¹
      dissent: dissent.map(d => ({ engine: d.engineId, reason: d.reasoning })),
    });
  }

  /**
   * Build consensus from engine results
   * @private
   */
  _buildConsensus(results) {
    if (results.length === 0) return null;
    if (results.length === 1) return results[0].insight;

    // Find common themes
    const topInsight = results[0].insight;
    const supportingCount = results.filter(r =>
      r.insight && r.insight.toLowerCase().includes(topInsight.split(' ')[0].toLowerCase())
    ).length;

    const consensusStrength = supportingCount / results.length;

    return {
      primary: topInsight,
      support: `${supportingCount}/${results.length} engines agree`,
      strength: consensusStrength,
    };
  }

  /**
   * Get integration stats
   */
  getStats() {
    return {
      ...this.stats,
      enabled: this.isEnabled(),
      registrySize: this.registry?.size || 0,
    };
  }
}

/**
 * Create engine integration from provided or global registry
 *
 * @param {Object} [options]
 * @param {import('@cynic/core/engines').EngineRegistry} [options.registry] - Engine registry
 * @param {import('@cynic/core/engines').EngineOrchestrator} [options.orchestrator] - Engine orchestrator
 * @returns {EngineIntegration}
 */
export function createEngineIntegration(options = {}) {
  const registry = options.registry || null;
  const orchestrator = options.orchestrator || null;

  return new EngineIntegration({ registry, orchestrator });
}
