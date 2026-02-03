/**
 * CYNIC Brain - Consciousness Layer
 *
 * Encapsulates all "thinking" components:
 * - 155 philosophical engines (wisdom)
 * - 11 Dogs (judgment via Sefirot)
 * - Memory/patterns (recall)
 *
 * The Brain receives observations and outputs decisions.
 * It does NOT execute - it THINKS.
 *
 * "Le cerveau de CYNIC" - κυνικός
 *
 * Architecture:
 *   Brain (consciousness)
 *     ↓ decisions
 *   OS (orchestration) ← UnifiedOrchestrator
 *     ↓ tasks
 *   CPU (LLM layer) ← LLMAdapter
 *
 * @module @cynic/node/orchestration/brain
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, PHI_INV_2 } from '@cynic/core';

const log = createLogger('Brain');

/**
 * Brain state - consciousness snapshot
 */
export class BrainState {
  constructor(data = {}) {
    this.timestamp = data.timestamp || Date.now();
    this.consciousness = data.consciousness || PHI_INV; // Max 61.8%
    this.cognitiveLoad = data.cognitiveLoad || 0; // Miller's Law: 0-9
    this.entropy = data.entropy || 0;
    this.patterns = data.patterns || [];
    this.recentThoughts = data.recentThoughts || [];
  }

  toJSON() {
    return {
      timestamp: this.timestamp,
      consciousness: this.consciousness,
      cognitiveLoad: this.cognitiveLoad,
      entropy: this.entropy,
      patternCount: this.patterns.length,
      recentThoughtCount: this.recentThoughts.length,
    };
  }
}

/**
 * Thought result - output of brain thinking
 */
export class Thought {
  constructor(data = {}) {
    this.id = data.id || `thought-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    this.timestamp = data.timestamp || Date.now();
    this.input = data.input || null;
    this.judgment = data.judgment || null;
    this.synthesis = data.synthesis || null;
    this.patterns = data.patterns || [];
    this.confidence = Math.min(data.confidence || 0, PHI_INV); // Cap at φ⁻¹
    this.decision = data.decision || null;
    this.duration = data.duration || 0;
  }

  /**
   * Get verdict from judgment
   */
  get verdict() {
    return this.judgment?.verdict || null;
  }

  /**
   * Get score from judgment
   */
  get score() {
    return this.judgment?.score || 0;
  }

  /**
   * Is this thought confident enough to act on?
   * Threshold: φ⁻² = 38.2%
   */
  get isActionable() {
    return this.confidence >= PHI_INV_2;
  }

  toJSON() {
    return {
      id: this.id,
      timestamp: this.timestamp,
      verdict: this.verdict,
      score: this.score,
      confidence: this.confidence,
      isActionable: this.isActionable,
      duration: this.duration,
      patternCount: this.patterns.length,
    };
  }
}

/**
 * CYNIC Brain - The Consciousness Layer
 *
 * Wraps all cognitive components into a single interface.
 * The Brain thinks, the OS executes.
 */
export class Brain extends EventEmitter {
  /**
   * Create the Brain
   *
   * @param {Object} options
   * @param {Object} [options.dogOrchestrator] - Dog voting system
   * @param {Object} [options.engineOrchestrator] - Philosophical engines
   * @param {Object} [options.memoryStore] - Pattern/memory storage
   * @param {Object} [options.learningService] - Learning feedback
   * @param {Object} [options.llmRouter] - Multi-LLM router for consensus (Task #91)
   * @param {Object} [options.llmOrchestrator] - LLM execution orchestrator (Task #95)
   */
  constructor(options = {}) {
    super();

    // Core components
    this.dogOrchestrator = options.dogOrchestrator || null;
    this.engineOrchestrator = options.engineOrchestrator || null;
    this.memoryStore = options.memoryStore || null;
    this.learningService = options.learningService || null;
    // Task #91: Multi-LLM consensus router
    this.llmRouter = options.llmRouter || null;
    // Task #95: Da'at Bridge - LLM execution orchestrator
    this.llmOrchestrator = options.llmOrchestrator || null;

    // State
    this._state = new BrainState();
    this._thoughtHistory = [];
    this._maxThoughtHistory = 100;

    // Statistics
    this.stats = {
      thoughtsProcessed: 0,
      judgmentsRequested: 0,
      synthesisRequested: 0,
      patternsDetected: 0,
      avgThinkingTime: 0,
      executionsRequested: 0, // Task #95
    };

    log.debug('Brain initialized', {
      hasDogs: !!this.dogOrchestrator,
      hasEngines: !!this.engineOrchestrator,
      hasMemory: !!this.memoryStore,
      hasLLMRouter: !!this.llmRouter, // Task #91
      hasLLMOrchestrator: !!this.llmOrchestrator, // Task #95
    });
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // MAIN INTERFACE
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Think about something - main entry point
   *
   * Coordinates dogs, engines, and memory to produce a thought.
   *
   * @param {Object} input - What to think about
   * @param {string} input.content - Content to analyze
   * @param {string} [input.type] - Type of content (code, decision, claim, etc.)
   * @param {Object} [input.context] - Additional context
   * @param {Object} [options] - Thinking options
   * @param {boolean} [options.requestJudgment=true] - Request dog judgment
   * @param {boolean} [options.requestSynthesis=false] - Request engine synthesis
   * @param {boolean} [options.checkPatterns=true] - Check memory for patterns
   * @returns {Promise<Thought>}
   */
  async think(input, options = {}) {
    const startTime = Date.now();
    const {
      requestJudgment = true,
      requestSynthesis = false,
      checkPatterns = true,
    } = options;

    const thought = new Thought({ input });

    try {
      // 1. Check patterns first (fast, local)
      if (checkPatterns && this.memoryStore) {
        thought.patterns = await this._checkPatterns(input);
      }

      // 2. Request judgment from dogs (if needed)
      if (requestJudgment && this.dogOrchestrator) {
        thought.judgment = await this._requestJudgment(input);
        this.stats.judgmentsRequested++;
      }

      // 3. Request synthesis from engines (if needed)
      if (requestSynthesis && this.engineOrchestrator) {
        thought.synthesis = await this._requestSynthesis(input);
        this.stats.synthesisRequested++;
      }

      // 4. Combine into decision
      thought.decision = this._formDecision(thought);

      // 5. Calculate confidence
      thought.confidence = this._calculateConfidence(thought);

      // 6. Update state
      this._updateState(thought);

    } catch (err) {
      log.error('Brain thinking error', { error: err.message });
      thought.decision = { action: 'defer', reason: err.message };
      thought.confidence = 0;
    }

    // Record timing
    thought.duration = Date.now() - startTime;
    this.stats.thoughtsProcessed++;
    this._updateAvgThinkingTime(thought.duration);

    // Store in history
    this._recordThought(thought);

    // Emit thought event
    this.emit('thought', thought);

    return thought;
  }

  /**
   * Quick judgment - just dogs, no synthesis
   *
   * @param {Object} input - What to judge
   * @returns {Promise<Thought>}
   */
  async judge(input) {
    return this.think(input, {
      requestJudgment: true,
      requestSynthesis: false,
      checkPatterns: true,
    });
  }

  /**
   * Deep synthesis - dogs + engines
   *
   * @param {Object} input - What to synthesize
   * @returns {Promise<Thought>}
   */
  async synthesize(input) {
    return this.think(input, {
      requestJudgment: true,
      requestSynthesis: true,
      checkPatterns: true,
    });
  }

  /**
   * Pattern-only check - no LLM calls
   *
   * @param {Object} input - What to check
   * @returns {Promise<Thought>}
   */
  async recall(input) {
    return this.think(input, {
      requestJudgment: false,
      requestSynthesis: false,
      checkPatterns: true,
    });
  }

  /**
   * Generate a plan with alternatives before acting
   *
   * Uses deep synthesis to generate multiple approaches,
   * then judges each alternative.
   *
   * @param {Object} input - What to plan for
   * @param {string} input.content - The task/request
   * @param {string} [input.type] - Content type
   * @param {Object} [options] - Planning options
   * @param {number} [options.maxAlternatives=3] - Max alternatives to generate
   * @param {boolean} [options.judgeAlternatives=true] - Judge each alternative
   * @returns {Promise<Object>} Plan with alternatives
   */
  async plan(input, options = {}) {
    const startTime = Date.now();
    const {
      maxAlternatives = 3,
      judgeAlternatives = true,
    } = options;

    this.stats.plansGenerated = (this.stats.plansGenerated || 0) + 1;

    const result = {
      input,
      thought: null,
      alternatives: [],
      recommendation: null,
      confidence: 0,
      duration: 0,
    };

    try {
      // 1. Deep think with synthesis
      result.thought = await this.think({
        content: `Plan approaches for: ${input.content}`,
        type: 'planning',
        context: input.context || {},
      }, {
        requestJudgment: true,
        requestSynthesis: true,
        checkPatterns: true,
      });

      // 2. Generate alternatives from synthesis
      if (result.thought.synthesis?.consultations) {
        for (const consultation of result.thought.synthesis.consultations) {
          if (consultation.response?.suggestions) {
            result.alternatives.push(...consultation.response.suggestions);
          }
          if (consultation.response?.alternatives) {
            result.alternatives.push(...consultation.response.alternatives);
          }
        }
      }

      // 3. Add default alternatives if none generated
      if (result.alternatives.length === 0) {
        result.alternatives = this._generateDefaultAlternatives(input);
      }

      // Limit to maxAlternatives
      result.alternatives = result.alternatives.slice(0, maxAlternatives);

      // 4. Judge each alternative if requested
      if (judgeAlternatives && result.alternatives.length > 0) {
        for (const alt of result.alternatives) {
          try {
            const judgment = await this.judge({
              content: alt.description || alt.label || String(alt),
              type: 'alternative',
              context: { originalInput: input.content },
            });
            alt.judgment = {
              score: judgment.score,
              verdict: judgment.verdict,
              confidence: judgment.confidence,
            };
          } catch (err) {
            // DEFENSIVE: Judgment failure shouldn't break planning
            alt.judgment = { score: 50, verdict: 'UNKNOWN', error: err.message };
          }
        }

        // Sort by judgment score (highest first)
        result.alternatives.sort((a, b) =>
          (b.judgment?.score || 0) - (a.judgment?.score || 0)
        );
      }

      // 5. Set recommendation (highest-scored alternative)
      if (result.alternatives.length > 0) {
        result.recommendation = result.alternatives[0];
      }

      // 6. Calculate overall confidence
      result.confidence = result.thought.confidence;

    } catch (err) {
      log.error('Brain planning error', { error: err.message });
      result.error = err.message;
    }

    result.duration = Date.now() - startTime;
    this.emit('plan', result);

    return result;
  }

  /**
   * Generate default alternatives when synthesis fails
   * @private
   */
  _generateDefaultAlternatives(input) {
    return [
      {
        id: 'proceed',
        label: 'Proceed directly',
        description: `Execute: ${input.content}`,
        risk: 'medium',
      },
      {
        id: 'simplify',
        label: 'Simplify first',
        description: 'Break down into smaller steps',
        risk: 'low',
      },
      {
        id: 'research',
        label: 'Research first',
        description: 'Gather more information before acting',
        risk: 'low',
      },
    ];
  }

  /**
   * Execute with full Da'at flow (Task #95)
   *
   * Complete flow: Human → Brain.think() → LLM → Brain.judge() → Human
   *
   * @param {Object} input - What to process
   * @param {string} input.content - User's request
   * @param {string} [input.type] - Content type
   * @param {Object} [options] - Execution options
   * @param {boolean} [options.requestSynthesis=false] - Include philosophical synthesis
   * @param {Object} [options.context] - Additional context for LLM
   * @returns {Promise<Object>} Complete execution result
   */
  async execute(input, options = {}) {
    const startTime = Date.now();
    this.stats.executionsRequested++;

    // 1. Think first (local judgment + patterns + optional synthesis)
    const thought = await this.think(input, {
      requestJudgment: true,
      requestSynthesis: options.requestSynthesis || false,
      checkPatterns: true,
    });

    // 2. Check if thought blocks execution
    if (thought.decision?.action === 'reject' || thought.judgment?.blocked) {
      log.info('Execution blocked by thought', { reason: thought.decision?.reason });
      return {
        thought,
        response: null,
        judgment: null,
        blocked: true,
        reason: thought.decision?.reason || 'Blocked by judgment',
        duration: Date.now() - startTime,
      };
    }

    // 3. If no LLM orchestrator, return thought only
    if (!this.llmOrchestrator) {
      log.debug('No LLM orchestrator, returning thought only');
      return {
        thought,
        response: null,
        judgment: null,
        blocked: false,
        duration: Date.now() - startTime,
      };
    }

    // 4. Execute via LLM orchestrator
    let response;
    try {
      response = await this.llmOrchestrator.execute(
        thought,
        input.content,
        options.context || {}
      );
    } catch (err) {
      log.error('LLM execution failed', { error: err.message });
      return {
        thought,
        response: null,
        judgment: null,
        error: err.message,
        duration: Date.now() - startTime,
      };
    }

    // 5. If LLM was blocked or errored, return result
    if (response.blocked || response.error || !response.content) {
      return {
        thought,
        response,
        judgment: null,
        blocked: response.blocked || false,
        error: response.error,
        duration: Date.now() - startTime,
      };
    }

    // 6. Judge the LLM response
    let responseJudgment;
    try {
      responseJudgment = await this.judge({
        content: response.content,
        type: 'llm_response',
        context: {
          originalThought: thought.id,
          tier: response.tier,
          model: response.model,
        },
      });
    } catch (err) {
      log.warn('Response judgment failed', { error: err.message });
      responseJudgment = null;
    }

    // 7. Return complete result
    const result = {
      thought,
      response,
      judgment: responseJudgment,
      blocked: false,
      duration: Date.now() - startTime,
    };

    // Emit execution event
    this.emit('execution', result);

    return result;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // INTERNAL METHODS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Check memory for patterns
   * @private
   */
  async _checkPatterns(input) {
    if (!this.memoryStore) return [];

    try {
      const patterns = await this.memoryStore.findPatterns?.(input.content) || [];
      if (patterns.length > 0) {
        this.stats.patternsDetected += patterns.length;
      }
      return patterns;
    } catch (err) {
      log.warn('Pattern check failed', { error: err.message });
      return [];
    }
  }

  /**
   * Request judgment from dogs + optional multi-LLM consensus (Task #91)
   * @private
   */
  async _requestJudgment(input) {
    if (!this.dogOrchestrator) return null;

    try {
      const item = {
        content: input.content,
        itemType: input.type || 'general',
        context: input.context || {},
      };

      // 1. Get local dog judgment
      const result = await this.dogOrchestrator.judge(item);

      const judgment = {
        score: result.score,
        verdict: result.verdict,
        consensus: result.consensus,
        consensusRatio: result.consensusRatio,
        votes: result.votes,
        blocked: result.blocked,
        dimensions: result.dimensions,
        // Task #91: Mark as local-only initially
        source: 'dogs',
      };

      // ═══════════════════════════════════════════════════════════════════════
      // Task #91: Multi-LLM Consensus Validation
      // If validators are available, seek cross-LLM consensus
      // ═══════════════════════════════════════════════════════════════════════
      if (this.llmRouter?.validators?.length > 0) {
        try {
          const consensusResult = await this.llmRouter.consensus(
            `Judge this: ${input.content}\n\nLocal verdict: ${result.verdict} (score: ${result.score})`,
            { timeout: 5000 }
          );

          if (consensusResult) {
            // Merge multi-LLM consensus with dog judgment
            judgment.multiLLM = {
              hasConsensus: consensusResult.hasConsensus || false,
              consensusRatio: consensusResult.consensusRatio || 0,
              validators: consensusResult.validators?.length || 0,
              responses: consensusResult.responses?.length || 0,
            };

            // If LLM consensus disagrees significantly, flag for review
            if (consensusResult.hasConsensus && consensusResult.verdict !== result.verdict) {
              judgment.multiLLM.disagreement = true;
              judgment.multiLLM.llmVerdict = consensusResult.verdict;
              log.info('Multi-LLM disagreement', {
                dogVerdict: result.verdict,
                llmVerdict: consensusResult.verdict,
              });
            }

            // Update source to reflect multi-LLM validation
            judgment.source = 'dogs+llm';
          }
        } catch (llmErr) {
          // LLM consensus failed - continue with dog judgment only
          log.debug('Multi-LLM consensus unavailable', { error: llmErr.message });
          judgment.multiLLM = { error: llmErr.message, validators: 0 };
        }
      }

      return judgment;
    } catch (err) {
      log.warn('Dog judgment failed', { error: err.message });
      return null;
    }
  }

  /**
   * Request synthesis from engines
   * @private
   */
  async _requestSynthesis(input) {
    if (!this.engineOrchestrator) return null;

    try {
      const result = await this.engineOrchestrator.consult({
        query: input.content,
        domain: input.type,
        context: input.context || {},
      });

      return {
        insight: result.synthesis?.insight,
        confidence: result.confidence,
        strategy: result.strategy,
        consultations: result.consultations,
      };
    } catch (err) {
      log.warn('Engine synthesis failed', { error: err.message });
      return null;
    }
  }

  /**
   * Form a decision from thought components
   * @private
   */
  _formDecision(thought) {
    // Priority: patterns > judgment > synthesis

    // If we have strong patterns, use them
    if (thought.patterns.length > 0) {
      const strongPattern = thought.patterns.find(p => (p.confidence || 0) >= PHI_INV_2);
      if (strongPattern) {
        return {
          action: strongPattern.action || 'apply_pattern',
          reason: `Pattern: ${strongPattern.name || 'matched'}`,
          source: 'pattern',
        };
      }
    }

    // If we have judgment, use it
    if (thought.judgment) {
      const verdict = thought.judgment.verdict;
      const blocked = thought.judgment.blocked;

      if (blocked) {
        return { action: 'block', reason: 'Dogs blocked', source: 'judgment' };
      }

      switch (verdict) {
        case 'HOWL':
        case 'WAG':
          return { action: 'allow', reason: `Verdict: ${verdict}`, source: 'judgment' };
        case 'GROWL':
          return { action: 'transform', reason: 'Needs improvement', source: 'judgment' };
        case 'BARK':
          return { action: 'reject', reason: 'Failed judgment', source: 'judgment' };
        default:
          return { action: 'defer', reason: 'Unknown verdict', source: 'judgment' };
      }
    }

    // If we only have synthesis, use it
    if (thought.synthesis) {
      return {
        action: 'consider',
        reason: thought.synthesis.insight || 'See synthesis',
        source: 'synthesis',
      };
    }

    // No components, defer
    return { action: 'defer', reason: 'Insufficient data', source: 'none' };
  }

  /**
   * Calculate overall confidence
   * @private
   */
  _calculateConfidence(thought) {
    const scores = [];

    // Pattern confidence
    if (thought.patterns.length > 0) {
      const patternAvg = thought.patterns.reduce((sum, p) => sum + (p.confidence || 0.5), 0)
        / thought.patterns.length;
      scores.push(patternAvg);
    }

    // Judgment confidence (consensus ratio)
    if (thought.judgment) {
      scores.push(thought.judgment.consensusRatio || 0.5);
    }

    // Synthesis confidence
    if (thought.synthesis) {
      scores.push(thought.synthesis.confidence || 0.5);
    }

    // Average all scores
    if (scores.length === 0) return 0;
    const avg = scores.reduce((a, b) => a + b, 0) / scores.length;

    // Cap at φ⁻¹
    return Math.min(avg, PHI_INV);
  }

  /**
   * Update brain state after thinking
   * @private
   */
  _updateState(thought) {
    // Increase cognitive load based on thought complexity
    const complexity = (thought.patterns.length > 0 ? 1 : 0)
      + (thought.judgment ? 2 : 0)
      + (thought.synthesis ? 3 : 0);

    this._state.cognitiveLoad = Math.min(9, this._state.cognitiveLoad + complexity * 0.1);

    // Decay cognitive load over time (1% per second of thinking)
    const decayFactor = thought.duration / 1000 * 0.01;
    this._state.cognitiveLoad = Math.max(0, this._state.cognitiveLoad - decayFactor);

    // Update consciousness based on confidence
    this._state.consciousness = thought.confidence;

    // Add to recent thoughts
    this._state.recentThoughts.unshift(thought.id);
    if (this._state.recentThoughts.length > 10) {
      this._state.recentThoughts.pop();
    }

    this._state.timestamp = Date.now();
  }

  /**
   * Record thought in history
   * @private
   */
  _recordThought(thought) {
    this._thoughtHistory.unshift(thought);
    if (this._thoughtHistory.length > this._maxThoughtHistory) {
      this._thoughtHistory.pop();
    }
  }

  /**
   * Update average thinking time
   * @private
   */
  _updateAvgThinkingTime(duration) {
    const n = this.stats.thoughtsProcessed;
    this.stats.avgThinkingTime =
      (this.stats.avgThinkingTime * (n - 1) + duration) / n;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PUBLIC API
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get current brain state
   */
  getState() {
    return this._state;
  }

  /**
   * Get recent thoughts
   */
  getRecentThoughts(limit = 10) {
    return this._thoughtHistory.slice(0, limit);
  }

  /**
   * Get statistics
   */
  getStats() {
    return {
      ...this.stats,
      state: this._state.toJSON(),
      components: {
        hasDogs: !!this.dogOrchestrator,
        hasEngines: !!this.engineOrchestrator,
        hasMemory: !!this.memoryStore,
        hasLearning: !!this.learningService,
      },
    };
  }

  /**
   * Set dog orchestrator
   */
  setDogOrchestrator(dogOrchestrator) {
    this.dogOrchestrator = dogOrchestrator;
  }

  /**
   * Set engine orchestrator
   */
  setEngineOrchestrator(engineOrchestrator) {
    this.engineOrchestrator = engineOrchestrator;
  }

  /**
   * Set memory store
   */
  setMemoryStore(memoryStore) {
    this.memoryStore = memoryStore;
  }

  /**
   * Set learning service
   */
  setLearningService(learningService) {
    this.learningService = learningService;
  }

  /**
   * Provide feedback on a thought
   *
   * @param {string} thoughtId - Thought ID
   * @param {Object} feedback - Feedback data
   * @param {boolean} feedback.correct - Was the decision correct?
   * @param {string} [feedback.reason] - Why?
   */
  async provideFeedback(thoughtId, feedback) {
    const thought = this._thoughtHistory.find(t => t.id === thoughtId);
    if (!thought) {
      log.warn('Thought not found for feedback', { thoughtId });
      return;
    }

    // Forward to learning service
    if (this.learningService) {
      await this.learningService.recordFeedback?.({
        thoughtId,
        input: thought.input,
        decision: thought.decision,
        feedback,
      });
    }

    this.emit('feedback', { thoughtId, feedback });
  }
}

/**
 * Create a Brain instance
 *
 * @param {Object} options
 * @returns {Brain}
 */
export function createBrain(options = {}) {
  return new Brain(options);
}

// Singleton
let _globalBrain = null;

/**
 * Get the global Brain instance
 *
 * @param {Object} [options]
 * @returns {Brain}
 */
export function getBrain(options) {
  if (!_globalBrain) {
    _globalBrain = new Brain(options);
  }
  return _globalBrain;
}

/**
 * Reset global brain (for testing)
 */
export function _resetBrainForTesting() {
  _globalBrain = null;
}

export default Brain;
