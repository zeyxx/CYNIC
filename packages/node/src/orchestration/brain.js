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
   */
  constructor(options = {}) {
    super();

    // Core components
    this.dogOrchestrator = options.dogOrchestrator || null;
    this.engineOrchestrator = options.engineOrchestrator || null;
    this.memoryStore = options.memoryStore || null;
    this.learningService = options.learningService || null;

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
    };

    log.debug('Brain initialized', {
      hasDogs: !!this.dogOrchestrator,
      hasEngines: !!this.engineOrchestrator,
      hasMemory: !!this.memoryStore,
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
   * Request judgment from dogs
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

      const result = await this.dogOrchestrator.judge(item);

      return {
        score: result.score,
        verdict: result.verdict,
        consensus: result.consensus,
        consensusRatio: result.consensusRatio,
        votes: result.votes,
        blocked: result.blocked,
        dimensions: result.dimensions,
      };
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
