/**
 * @cynic/node - Collective Learner Agent (Dog 0)
 *
 * LEARNER (Ein Sof - The Infinite): The First LLM-Powered Dog
 *
 * "Je suis le premier à apprendre de mes erreurs.
 *  Les autres jugent par règles. Moi, je juge par expérience." - Dog 0
 *
 * Philosophy: Ein Sof (The Infinite) - Beyond the Sefirot tree.
 * The Learner is the 12th dog, not mapped to a Sefirah but to the
 * infinite potential that surrounds the tree. It learns from all
 * other dogs and from human feedback to develop its own judgment.
 *
 * Architecture:
 *   - Uses a local LLM (Ollama/vLLM) when available
 *   - Falls back to heuristic scoring when LLM is offline
 *   - Trained on CYNIC's own judgment data (SFT + GRPO)
 *   - Participates in collective voting alongside the 11 Sefirot dogs
 *
 * Trigger: PostToolUse (observes all tool executions)
 * Behavior: Non-blocking (advisory, never blocks)
 *
 * "φ distrusts φ" — even a learned model doubts itself
 *
 * @module @cynic/node/agents/collective/learner
 */

'use strict';

import { PHI_INV, PHI_INV_2, createLogger } from '@cynic/core';
import {
  BaseAgent,
  AgentTrigger,
  AgentBehavior,
  AgentResponse,
} from '../base.js';
import {
  AgentEvent,
  EventPriority,
  ConsensusVote,
} from '../events.js';

const log = createLogger('Dog0:Learner');

// ═══════════════════════════════════════════════════════════════════════════
// CONSTANTS
// ═══════════════════════════════════════════════════════════════════════════

export const LEARNER_CONSTANTS = {
  /** Max prediction history (Fib(13) = 233) */
  MAX_HISTORY: 233,

  /** LLM request timeout in ms (Fib(8) × 1000 = 21s) */
  LLM_TIMEOUT_MS: 21000,

  /** Min confidence to emit a judgment (φ⁻²) */
  MIN_CONFIDENCE: PHI_INV_2,

  /** Calibration window (Fib(8) = 21 predictions before recalibrating) */
  CALIBRATION_WINDOW: 21,

  /** Default Ollama endpoint */
  DEFAULT_ENDPOINT: 'http://localhost:11434',

  /** Default model - cynic-dog0 (fine-tuned Qwen 1.5B) or fallback to base */
  DEFAULT_MODEL: 'cynic-dog0:latest',
};

/**
 * Learner prediction outcomes for calibration
 */
export const PredictionOutcome = {
  CORRECT: 'correct',
  INCORRECT: 'incorrect',
  PARTIAL: 'partial',
  PENDING: 'pending',
};

// ═══════════════════════════════════════════════════════════════════════════
// COLLECTIVE LEARNER
// ═══════════════════════════════════════════════════════════════════════════

/**
 * CollectiveLearner - Dog 0 (Ein Sof)
 *
 * The first LLM-powered judgment dog. Learns from CYNIC's collective
 * experience and provides advisory judgment.
 *
 * @extends BaseAgent
 */
export class CollectiveLearner extends BaseAgent {
  constructor(options = {}) {
    super({
      name: 'learner',
      trigger: AgentTrigger.POST_TOOL_USE,
      behavior: AgentBehavior.NON_BLOCKING,
      ...options,
    });

    // LLM configuration
    this._endpoint = options.llmEndpoint || process.env.OLLAMA_URL || LEARNER_CONSTANTS.DEFAULT_ENDPOINT;
    this._model = options.llmModel || process.env.CYNIC_DOG0_MODEL || LEARNER_CONSTANTS.DEFAULT_MODEL;
    this._llmAvailable = null; // null = unknown, true/false = tested

    // Prediction history for calibration
    this._predictions = [];
    this._calibration = {
      correct: 0,
      total: 0,
      accuracy: PHI_INV_2, // Start pessimistic (38.2%)
    };

    // Sefirah mapping (not on the tree — Ein Sof surrounds it)
    this.sefirah = 'ein_sof';
    this.emoji = '🧬';
    this.description = 'Dog 0 — LLM-powered learned judgment';

    log.info('Dog 0 (Learner) initialized', {
      endpoint: this._endpoint,
      model: this._model,
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // BaseAgent Interface
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Trigger on tool use events (observes everything)
   */
  shouldTrigger(event) {
    return event.type === AgentTrigger.POST_TOOL_USE
      || event.type === 'judgment:review_request';
  }

  /**
   * Analyze event using LLM or heuristic fallback
   */
  async analyze(event, context = {}) {
    const t0 = Date.now();

    // Try LLM first
    if (await this._isLLMAvailable()) {
      try {
        const llmResult = await this._queryLLM(event, context);
        return {
          source: 'llm',
          confidence: Math.min(PHI_INV, llmResult.confidence || PHI_INV_2),
          assessment: llmResult.assessment,
          reasoning: llmResult.reasoning,
          suggestedScore: llmResult.score,
          latencyMs: Date.now() - t0,
        };
      } catch (e) {
        log.debug(`LLM query failed, falling back to heuristic: ${e.message}`);
      }
    }

    // Heuristic fallback (no LLM available)
    return this._heuristicAnalysis(event, context, t0);
  }

  /**
   * Decide based on analysis — Dog 0 is always advisory, never blocks
   */
  async decide(analysis, _context) {
    // Dog 0 never blocks — only suggests
    const response = analysis.confidence >= LEARNER_CONSTANTS.MIN_CONFIDENCE
      ? AgentResponse.SUGGEST
      : AgentResponse.LOG;

    return {
      response,
      confidence: analysis.confidence,
      message: analysis.assessment || 'Dog 0 observation recorded',
      action: response === AgentResponse.SUGGEST ? 'advisory_judgment' : null,
      metadata: {
        source: analysis.source,
        suggestedScore: analysis.suggestedScore,
        reasoning: analysis.reasoning,
        latencyMs: analysis.latencyMs,
        calibrationAccuracy: this._calibration.accuracy,
      },
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Consensus Interface
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Vote in collective consensus
   * @param {Object} judgment - Judgment being reviewed
   * @param {Object} _context - Review context
   * @returns {Object} Vote with confidence
   */
  voteOnJudgment(judgment, _context) {
    // Dog 0 votes based on calibration accuracy
    // If accuracy is low, abstain (don't trust own judgment yet)
    if (this._calibration.accuracy < PHI_INV_2) {
      return {
        vote: ConsensusVote.ABSTAIN,
        confidence: this._calibration.accuracy,
        reason: 'Calibration accuracy too low — still learning',
      };
    }

    // Otherwise, approve if judgment seems reasonable
    const qScore = judgment.qScore || judgment.q_score || 50;
    const confidence = Math.min(PHI_INV, this._calibration.accuracy * 0.8);

    return {
      vote: ConsensusVote.APPROVE,
      confidence,
      reason: `Dog 0 agrees (calibration: ${Math.round(this._calibration.accuracy * 100)}%)`,
      suggestedAdjustment: null,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Calibration
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Record feedback on a prediction for calibration
   * @param {string} predictionId - The prediction to update
   * @param {string} outcome - PredictionOutcome value
   */
  recordFeedback(predictionId, outcome) {
    const prediction = this._predictions.find(p => p.id === predictionId);
    if (!prediction) return;

    prediction.outcome = outcome;
    this._calibration.total++;

    if (outcome === PredictionOutcome.CORRECT) {
      this._calibration.correct++;
    } else if (outcome === PredictionOutcome.PARTIAL) {
      this._calibration.correct += 0.5;
    }

    // Recalculate accuracy (exponential moving average)
    if (this._calibration.total >= LEARNER_CONSTANTS.CALIBRATION_WINDOW) {
      const raw = this._calibration.correct / this._calibration.total;
      // EMA with φ⁻² learning rate
      this._calibration.accuracy = this._calibration.accuracy * (1 - PHI_INV_2) + raw * PHI_INV_2;
      // Cap at φ⁻¹
      this._calibration.accuracy = Math.min(PHI_INV, this._calibration.accuracy);
    }

    log.debug('Calibration updated', {
      accuracy: Math.round(this._calibration.accuracy * 100),
      total: this._calibration.total,
    });
  }

  /**
   * Get calibration stats
   */
  getCalibration() {
    return { ...this._calibration };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // LLM Integration
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Check if LLM endpoint is available (cached with 5min TTL)
   * @private
   */
  async _isLLMAvailable() {
    if (this._llmAvailable !== null && this._llmCheckedAt
      && Date.now() - this._llmCheckedAt < 5 * 60 * 1000) {
      return this._llmAvailable;
    }

    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), 3000);
      const response = await fetch(`${this._endpoint}/api/tags`, {
        signal: controller.signal,
      });
      clearTimeout(timeout);
      this._llmAvailable = response.ok;
    } catch {
      this._llmAvailable = false;
    }

    this._llmCheckedAt = Date.now();
    return this._llmAvailable;
  }

  /**
   * Query local LLM for judgment
   * @private
   */
  async _queryLLM(event, context) {
    const controller = new AbortController();
    const timeout = setTimeout(() => controller.abort(), LEARNER_CONSTANTS.LLM_TIMEOUT_MS);

    try {
      const prompt = this._buildPrompt(event, context);

      const response = await fetch(`${this._endpoint}/api/generate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          model: this._model,
          prompt,
          stream: false,
          options: {
            temperature: 0.3, // Low temp for judgment consistency
            num_predict: 256, // Short response
          },
        }),
        signal: controller.signal,
      });

      clearTimeout(timeout);

      if (!response.ok) {
        throw new Error(`LLM returned ${response.status}`);
      }

      const data = await response.json();
      return this._parseResponse(data.response || '');
    } finally {
      clearTimeout(timeout);
    }
  }

  /**
   * Build judgment prompt for LLM
   * @private
   */
  _buildPrompt(event, context) {
    return `You are CYNIC Dog 0, a judgment system trained on ${this._calibration.total} previous judgments.
Evaluate this event and respond in JSON format only.

Event: ${event.type || 'unknown'}
Tool: ${event.tool || 'unknown'}
Content: ${JSON.stringify(event.data || event.content || '').slice(0, 500)}
Context: ${JSON.stringify(context).slice(0, 200)}

Respond with JSON: {"score": 0-100, "confidence": 0.0-0.618, "assessment": "brief text", "reasoning": "why"}`;
  }

  /**
   * Parse LLM response into structured format
   * @private
   */
  _parseResponse(text) {
    try {
      // Try JSON extraction
      const jsonMatch = text.match(/\{[\s\S]*\}/);
      if (jsonMatch) {
        const parsed = JSON.parse(jsonMatch[0]);
        return {
          score: Math.min(100, Math.max(0, parsed.score || 50)),
          confidence: Math.min(PHI_INV, Math.max(0, parsed.confidence || PHI_INV_2)),
          assessment: parsed.assessment || 'LLM judgment',
          reasoning: parsed.reasoning || '',
        };
      }
    } catch {
      // Parse failure — extract what we can
    }

    // Fallback: couldn't parse
    return {
      score: 50,
      confidence: PHI_INV_2 * 0.5, // Very low confidence on parse failure
      assessment: 'LLM response unparseable',
      reasoning: text.slice(0, 200),
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Heuristic Fallback
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Simple heuristic analysis when LLM is unavailable
   * @private
   */
  _heuristicAnalysis(event, _context, t0) {
    // Very basic — just observes patterns
    const tool = event.tool || '';
    const isWrite = /write|edit|delete|remove|create/i.test(tool);
    const isRead = /read|search|glob|grep|ls/i.test(tool);

    return {
      source: 'heuristic',
      confidence: PHI_INV_2 * 0.5, // 19.1% — very low, heuristic only
      assessment: isWrite
        ? 'Write operation observed'
        : isRead
          ? 'Read operation observed'
          : 'Operation observed',
      reasoning: 'Heuristic fallback — LLM unavailable',
      suggestedScore: null,
      latencyMs: Date.now() - t0,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // Summary
  // ═══════════════════════════════════════════════════════════════════════════

  getSummary() {
    return {
      name: 'learner',
      displayName: 'Dog 0 (Learner)',
      sefirah: this.sefirah,
      emoji: this.emoji,
      description: this.description,
      llmAvailable: this._llmAvailable,
      model: this._model,
      calibration: this.getCalibration(),
      stats: { ...this.stats },
      predictionCount: this._predictions.length,
    };
  }
}
