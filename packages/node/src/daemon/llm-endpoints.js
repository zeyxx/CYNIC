/**
 * CYNIC Daemon — LLM Endpoints
 *
 * Phase 2: CYNIC calls LLMs directly instead of being trapped inside one.
 * ModelIntelligence (Thompson Sampling) selects the optimal model,
 * adapters execute, CostLedger tracks spending.
 *
 * "Le chien ne demande plus permission pour penser" — κυνικός
 *
 * @module @cynic/node/daemon/llm-endpoints
 */

'use strict';

import { createLogger, PHI_INV } from '@cynic/core';
import { getModelIntelligence } from '../learning/model-intelligence.js';
import { getCostLedger } from '../accounting/cost-ledger.js';

const log = createLogger('LLMEndpoints');

/**
 * Resolve an adapter for a given model tier.
 *
 * Priority: Anthropic (primary brain) → Ollama → Gemini
 * If no adapter available, returns null.
 *
 * @param {string} tier - Model tier (opus/sonnet/haiku/ollama)
 * @param {import('@cynic/llm').LLMAdapter[]} validators - Available adapters
 * @returns {import('@cynic/llm').LLMAdapter|null}
 */
function getAdapterForTier(tier, validators) {
  const enabled = validators.filter(v => v.enabled);

  // Anthropic tiers (opus/sonnet/haiku) → find anthropic adapter
  if (tier === 'opus' || tier === 'sonnet' || tier === 'haiku') {
    const anthropic = enabled.find(v => v.provider === 'anthropic');
    if (anthropic) {
      // Configure the model for the requested tier
      const { MODEL_MAP } = anthropic.constructor;
      if (MODEL_MAP && MODEL_MAP[tier]) {
        // Don't mutate — pass model override via complete() options
        return { adapter: anthropic, modelOverride: MODEL_MAP[tier] };
      }
      return { adapter: anthropic, modelOverride: null };
    }
  }

  // Ollama tier → find OSS adapter
  if (tier === 'ollama') {
    const ollama = enabled.find(v => v.provider === 'ollama');
    if (ollama) return { adapter: ollama, modelOverride: null };
  }

  // Fallback chain: Anthropic → Ollama → Gemini
  const fallback =
    enabled.find(v => v.provider === 'anthropic') ||
    enabled.find(v => v.provider === 'ollama') ||
    enabled.find(v => v.provider === 'gemini');

  if (fallback) return { adapter: fallback, modelOverride: null };

  return null;
}

/**
 * Setup LLM endpoints on the Express app.
 *
 * Replaces the Phase 1 stub (501) with live ModelIntelligence → adapter pipeline.
 *
 * @param {import('express').Express} app - Express app
 * @param {Object} [deps] - Injectable dependencies (for testing)
 * @param {Function} [deps.getValidators] - Returns array of LLM adapters
 */
export function setupLLMEndpoints(app, deps = {}) {
  /**
   * Get available validators (adapters).
   * Lazy-loads from LLMRouter if available, otherwise empty.
   */
  // Cache for lazy-loaded validators
  let _cachedValidators = null;

  const getValidators = deps.getValidators || (async () => {
    if (_cachedValidators) return _cachedValidators;
    try {
      const { getRouterWithValidators } = await import('@cynic/llm');
      const router = await getRouterWithValidators();
      _cachedValidators = router.validators || [];
      return _cachedValidators;
    } catch {
      // @cynic/llm may not be available in all contexts
      return [];
    }
  });

  // ===========================================================================
  // POST /llm/ask — Primary LLM completion endpoint
  // ===========================================================================

  app.post('/llm/ask', async (req, res) => {
    const { prompt, taskType, system, temperature, maxTokens, model } = req.body || {};

    if (!prompt) {
      return res.status(400).json({ error: 'Missing required field: prompt' });
    }

    const startTime = Date.now();

    try {
      const mi = getModelIntelligence();
      const costLedger = getCostLedger();
      const validators = await getValidators();

      // ModelIntelligence selects optimal tier via Thompson Sampling
      const budgetStatus = costLedger.getBudgetStatus();
      const selection = mi.selectModel(taskType || 'default', {
        budgetLevel: budgetStatus.level,
        tool: req.body.tool,
      });

      // Override tier if caller specified a model
      const targetTier = model || selection.model;

      // Resolve adapter for the selected tier
      const resolved = getAdapterForTier(targetTier, validators);

      if (!resolved) {
        return res.status(503).json({
          error: 'No adapter available',
          message: 'No LLM adapters configured. Set ANTHROPIC_API_KEY, configure Ollama, or add GEMINI_API_KEY.',
          selectedTier: targetTier,
          selection,
        });
      }

      const { adapter, modelOverride } = resolved;

      // Execute completion
      const response = await adapter.complete(prompt, {
        temperature,
        maxTokens,
        system,
        model: modelOverride,
      });

      // Record cost
      costLedger.record({
        type: 'llm_ask',
        model: response.model,
        inputTokens: response.tokens.input,
        outputTokens: response.tokens.output,
        durationMs: response.duration,
        source: 'daemon_llm_ask',
      });

      // Record outcome for Thompson learning
      mi.recordOutcome({
        taskType: taskType || 'default',
        model: targetTier,
        success: true,
        tool: req.body.tool,
        durationMs: response.duration,
      });

      const duration = Date.now() - startTime;

      res.json({
        content: response.content,
        model: response.model,
        tokens: response.tokens,
        confidence: response.confidence,
        tier: targetTier,
        duration,
        selection: {
          tier: selection.model,
          reason: selection.reason,
          confidence: selection.confidence,
        },
      });
    } catch (err) {
      const duration = Date.now() - startTime;
      log.error('LLM ask failed', { error: err.message, duration });

      // Record failure for Thompson learning
      try {
        const mi = getModelIntelligence();
        mi.recordOutcome({
          taskType: taskType || 'default',
          model: model || 'unknown',
          success: false,
        });
      } catch { /* best effort */ }

      res.status(500).json({
        error: err.message,
        duration,
      });
    }
  });

  // ===========================================================================
  // POST /llm/consensus — Multi-model consensus
  // ===========================================================================

  app.post('/llm/consensus', async (req, res) => {
    const { prompt, quorum, timeout } = req.body || {};

    if (!prompt) {
      return res.status(400).json({ error: 'Missing required field: prompt' });
    }

    try {
      // Dynamic import — consensus needs the full router
      const { getLLMRouter } = await import('@cynic/llm');
      const router = getLLMRouter();

      const result = await router.consensus(prompt, { quorum, timeout });

      res.json({
        verdict: result.verdict,
        agreement: result.agreement,
        hasConsensus: result.hasConsensus,
        responses: result.responses.map(r => ({
          provider: r.provider,
          model: r.model,
          content: r.content,
          confidence: r.confidence,
        })),
        confidence: result.confidence,
      });
    } catch (err) {
      log.error('LLM consensus failed', { error: err.message });
      res.status(500).json({ error: err.message });
    }
  });

  // ===========================================================================
  // GET /llm/models — Available models + Thompson stats
  // ===========================================================================

  app.get('/llm/models', async (req, res) => {
    try {
      const mi = getModelIntelligence();
      const validators = await getValidators();

      const models = validators
        .filter(v => v.enabled)
        .map(v => ({
          provider: v.provider,
          model: v.model,
          stats: v.stats,
        }));

      res.json({
        models,
        thompson: mi.getAffinityMatrix(),
        stats: mi.getStats(),
      });
    } catch (err) {
      log.error('LLM models query failed', { error: err.message });
      res.status(500).json({ error: err.message });
    }
  });

  // ===========================================================================
  // POST /llm/feedback — External quality signals for Thompson learning
  // ===========================================================================

  app.post('/llm/feedback', (req, res) => {
    const { taskType, model, success, qualityScore, tool, experimentId } = req.body || {};

    if (!taskType || !model) {
      return res.status(400).json({ error: 'Missing required fields: taskType, model' });
    }

    try {
      const mi = getModelIntelligence();

      mi.recordOutcome({
        taskType,
        model,
        success: success !== false,
        qualityScore,
        tool,
        experimentId,
      });

      res.json({
        recorded: true,
        stats: mi.getStats(),
      });
    } catch (err) {
      log.error('LLM feedback failed', { error: err.message });
      res.status(500).json({ error: err.message });
    }
  });

  log.info('LLM endpoints mounted: /llm/ask, /llm/consensus, /llm/models, /llm/feedback');
}

export default setupLLMEndpoints;
