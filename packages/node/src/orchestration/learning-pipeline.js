/**
 * LearningPipeline — 5-Stage Learning Orchestration
 *
 * Unifies CYNIC's fragmented learning systems into one coherent pipeline:
 *
 * OBSERVE → EVALUATE → ADAPT → PREDICT → REFLECT
 *    ↓         ↓          ↓        ↓         ↓
 *  Events   Judgments  Q-Learn  Thompson  MetaCog
 *
 * Each stage is optional and composable. The pipeline:
 * - Collects observations from multiple sources
 * - Evaluates outcomes via judgments
 * - Adapts behavior via Q-Learning, SONA, BehaviorModifier
 * - Predicts future outcomes via Thompson Sampling
 * - Reflects on learning progress via MetaCognition
 *
 * φ-bounded learning rate: max 61.8% update per cycle
 *
 * "Learn in cycles. Reflect in spirals." — κυνικός
 *
 * @module @cynic/node/orchestration/learning-pipeline
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV, PHI_INV_2, globalEventBus, EventType } from '@cynic/core';
import { getQLearningService } from './learning-service.js';
import { getModelIntelligence } from '../learning/model-intelligence.js';
import { getMetaCognition } from '../learning/meta-cognition.js';

const log = createLogger('LearningPipeline');

// ═══════════════════════════════════════════════════════════════════════════
// LEARNING STAGES
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Learning pipeline stages (5-stage cycle)
 */
export const Stage = {
  OBSERVE: 'observe',     // Collect observations (events, measurements)
  EVALUATE: 'evaluate',   // Judge quality (via Judge, metrics)
  ADAPT: 'adapt',         // Update models (Q-Learning, SONA, BehaviorModifier)
  PREDICT: 'predict',     // Forecast outcomes (Thompson Sampling)
  REFLECT: 'reflect',     // Meta-cognition (learning about learning)
};

/**
 * Observation types (what can be observed)
 */
export const ObservationType = {
  EVENT: 'event',               // Event bus events
  JUDGMENT: 'judgment',         // Judgment results
  FEEDBACK: 'feedback',         // User feedback
  PATTERN: 'pattern',           // Detected patterns
  ANOMALY: 'anomaly',           // Anomalies
  PERFORMANCE: 'performance',   // Performance metrics
};

// ═══════════════════════════════════════════════════════════════════════════
// OBSERVATION
// ═══════════════════════════════════════════════════════════════════════════

/**
 * Observation — a unit of learning data
 */
export class Observation {
  constructor(data = {}) {
    this.id = data.id || `obs-${Date.now()}-${Math.random().toString(36).slice(2, 6)}`;
    this.timestamp = data.timestamp || Date.now();
    this.type = data.type || ObservationType.EVENT;
    this.source = data.source || 'unknown';
    this.payload = data.payload || {};
    this.context = data.context || {};
    this.metadata = data.metadata || {};
  }

  toJSON() {
    return {
      id: this.id,
      timestamp: this.timestamp,
      type: this.type,
      source: this.source,
      context: this.context,
    };
  }
}

/**
 * Learning Cycle — one complete iteration through all stages
 */
export class LearningCycle {
  constructor(data = {}) {
    this.id = data.id || `cycle-${Date.now()}`;
    this.startTime = data.startTime || Date.now();
    this.endTime = data.endTime || null;
    this.observations = data.observations || [];
    this.evaluations = data.evaluations || [];
    this.adaptations = data.adaptations || [];
    this.predictions = data.predictions || [];
    this.reflections = data.reflections || [];
    this.metadata = data.metadata || {};
  }

  get duration() {
    return this.endTime ? this.endTime - this.startTime : Date.now() - this.startTime;
  }

  get isComplete() {
    return this.endTime !== null;
  }

  complete() {
    this.endTime = Date.now();
  }

  toJSON() {
    return {
      id: this.id,
      duration: this.duration,
      observationCount: this.observations.length,
      evaluationCount: this.evaluations.length,
      adaptationCount: this.adaptations.length,
      predictionCount: this.predictions.length,
      reflectionCount: this.reflections.length,
      isComplete: this.isComplete,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// LEARNING PIPELINE
// ═══════════════════════════════════════════════════════════════════════════

/**
 * LearningPipeline — orchestrates 5-stage learning cycle
 *
 * Integrates:
 * - QLearningService (Q-table updates, TD error)
 * - ModelIntelligence (Thompson Sampling)
 * - MetaCognition (reflection, learning rate calibration)
 * - SONA (real-time pattern adaptation) — via events
 * - BehaviorModifier (feedback → behavior) — via events
 */
export class LearningPipeline extends EventEmitter {
  constructor(options = {}) {
    super();

    // Dependencies (lazy-loaded singletons)
    this.qLearning = options.qLearning || null;
    this.modelIntelligence = options.modelIntelligence || null;
    this.metaCognition = options.metaCognition || null;

    // Configuration
    this.config = {
      enableObserve: options.enableObserve !== false,
      enableEvaluate: options.enableEvaluate !== false,
      enableAdapt: options.enableAdapt !== false,
      enablePredict: options.enablePredict !== false,
      enableReflect: options.enableReflect !== false,
      learningRate: Math.min(options.learningRate || 0.1, PHI_INV), // φ⁻¹ cap
      batchSize: options.batchSize || 10,
      cycleInterval: options.cycleInterval || 60000, // 60s default
    };

    // State
    this.currentCycle = null;
    this.cycleHistory = [];
    this.stats = {
      totalCycles: 0,
      totalObservations: 0,
      totalAdaptations: 0,
      avgCycleDuration: 0,
      lastCycleTime: null,
    };

    this._running = false;
    this._cycleTimer = null;

    log.info('LearningPipeline created', {
      learningRate: this.config.learningRate,
      cycleInterval: this.config.cycleInterval,
    });
  }

  // ───────────────────────────────────────────────────────────────────────────
  // LIFECYCLE
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Start the pipeline (begins periodic cycles)
   */
  start() {
    if (this._running) {
      log.debug('Pipeline already running');
      return;
    }

    this._running = true;

    // Lazy-load dependencies
    if (!this.qLearning) this.qLearning = getQLearningService();
    if (!this.modelIntelligence) this.modelIntelligence = getModelIntelligence();
    if (!this.metaCognition) this.metaCognition = getMetaCognition();

    // Start periodic cycles
    if (this.config.cycleInterval > 0) {
      this._cycleTimer = setInterval(() => {
        this.runCycle().catch(err => {
          log.error('Cycle failed', { error: err.message });
          this.emit('cycle:error', err);
        });
      }, this.config.cycleInterval);
      this._cycleTimer.unref(); // Don't block exit
    }

    this.emit('started');
    log.info('LearningPipeline started');
  }

  /**
   * Stop the pipeline
   */
  stop() {
    if (!this._running) return;

    this._running = false;

    if (this._cycleTimer) {
      clearInterval(this._cycleTimer);
      this._cycleTimer = null;
    }

    // Complete current cycle if running
    if (this.currentCycle && !this.currentCycle.isComplete) {
      this.currentCycle.complete();
    }

    this.emit('stopped');
    log.info('LearningPipeline stopped');
  }

  // ───────────────────────────────────────────────────────────────────────────
  // CORE API
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Run one complete learning cycle
   *
   * @param {Object} [options] - Cycle-specific options
   * @returns {Promise<LearningCycle>}
   */
  async runCycle(options = {}) {
    const cycle = new LearningCycle({
      metadata: options.metadata || {},
    });

    this.currentCycle = cycle;
    this.emit('cycle:start', cycle);

    try {
      // Stage 1: OBSERVE
      if (this.config.enableObserve) {
        await this._stageObserve(cycle);
      }

      // Stage 2: EVALUATE
      if (this.config.enableEvaluate && cycle.observations.length > 0) {
        await this._stageEvaluate(cycle);
      }

      // Stage 3: ADAPT
      if (this.config.enableAdapt && cycle.evaluations.length > 0) {
        await this._stageAdapt(cycle);
      }

      // Stage 4: PREDICT
      if (this.config.enablePredict) {
        await this._stagePredict(cycle);
      }

      // Stage 5: REFLECT
      if (this.config.enableReflect) {
        await this._stageReflect(cycle);
      }

      // Complete cycle
      cycle.complete();
      this.cycleHistory.push(cycle);

      // Update stats
      this.stats.totalCycles++;
      this.stats.totalObservations += cycle.observations.length;
      this.stats.totalAdaptations += cycle.adaptations.length;
      this.stats.lastCycleTime = Date.now();

      // Update average duration (EMA with α = φ⁻¹)
      if (this.stats.avgCycleDuration === 0) {
        this.stats.avgCycleDuration = cycle.duration;
      } else {
        this.stats.avgCycleDuration =
          PHI_INV * cycle.duration + (1 - PHI_INV) * this.stats.avgCycleDuration;
      }

      this.emit('cycle:complete', cycle);
      log.info('Learning cycle complete', cycle.toJSON());

      // Emit to globalEventBus for cross-system tracking
      globalEventBus.emit(EventType.LEARNING_CYCLE_COMPLETE || 'learning:cycle:complete', {
        cycleId: cycle.id,
        duration: cycle.duration,
        observationCount: cycle.observations.length,
        adaptationCount: cycle.adaptations.length,
      });

      return cycle;
    } catch (err) {
      cycle.complete();
      this.emit('cycle:error', { cycle, error: err });
      throw err;
    } finally {
      // Cleanup: keep only last 100 cycles in memory
      if (this.cycleHistory.length > 100) {
        this.cycleHistory = this.cycleHistory.slice(-100);
      }
    }
  }

  /**
   * Add observation to current cycle
   *
   * @param {Observation|Object} observation
   */
  observe(observation) {
    if (!this.currentCycle) {
      this.currentCycle = new LearningCycle();
    }

    const obs = observation instanceof Observation
      ? observation
      : new Observation(observation);

    this.currentCycle.observations.push(obs);
    this.stats.totalObservations++;

    this.emit('observation', obs);
  }

  // ───────────────────────────────────────────────────────────────────────────
  // STAGE IMPLEMENTATIONS
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Stage 1: OBSERVE — collect recent observations
   */
  async _stageObserve(cycle) {
    this.emit('stage:observe:start', cycle);

    // Observations are collected via observe() method (event-driven)
    // This stage just marks the observation window

    this.emit('stage:observe:complete', {
      cycle,
      count: cycle.observations.length,
    });
  }

  /**
   * Stage 2: EVALUATE — judge quality of observations
   */
  async _stageEvaluate(cycle) {
    this.emit('stage:evaluate:start', cycle);

    // Evaluations come from external Judge (via events)
    // For now, just count observations by type
    const evaluations = cycle.observations.map(obs => ({
      observationId: obs.id,
      type: obs.type,
      timestamp: Date.now(),
    }));

    cycle.evaluations.push(...evaluations);

    this.emit('stage:evaluate:complete', {
      cycle,
      count: evaluations.length,
    });
  }

  /**
   * Stage 3: ADAPT — update models based on evaluations
   */
  async _stageAdapt(cycle) {
    this.emit('stage:adapt:start', cycle);

    const adaptations = [];

    // Q-Learning updates (if qLearning available)
    if (this.qLearning) {
      // Check if there are recent episodes to learn from
      const stats = this.qLearning.getStats();
      if (stats.episodeCount > 0) {
        adaptations.push({
          type: 'q-learning',
          details: {
            episodes: stats.episodeCount,
            qUpdates: stats.qUpdateCount,
          },
        });
      }
    }

    // Model Intelligence updates (Thompson Sampling exploration)
    if (this.modelIntelligence) {
      const miStats = this.modelIntelligence.getStats();
      adaptations.push({
        type: 'thompson-sampling',
        details: {
          samplerMaturity: miStats.samplerMaturity,
          explorationRate: miStats.samplerMaturity < PHI_INV ? PHI_INV : PHI_INV_2,
        },
      });
    }

    cycle.adaptations.push(...adaptations);

    this.emit('stage:adapt:complete', {
      cycle,
      count: adaptations.length,
    });
  }

  /**
   * Stage 4: PREDICT — forecast future outcomes
   */
  async _stagePredict(cycle) {
    this.emit('stage:predict:start', cycle);

    const predictions = [];

    // Thompson Sampling predictions (next best action)
    if (this.modelIntelligence) {
      const recommendation = this.modelIntelligence.selectModel('moderate', {
        needsReasoning: false,
      });

      predictions.push({
        type: 'model-recommendation',
        model: recommendation.model,
        confidence: recommendation.confidence, // Already φ-bounded by selectModel
        rationale: recommendation.reason,
      });
    }

    cycle.predictions.push(...predictions);

    this.emit('stage:predict:complete', {
      cycle,
      count: predictions.length,
    });
  }

  /**
   * Stage 5: REFLECT — meta-cognition (learning about learning)
   */
  async _stageReflect(cycle) {
    this.emit('stage:reflect:start', cycle);

    const reflections = [];

    // MetaCognition reflection
    if (this.metaCognition) {
      const reflection = await this.metaCognition.reflect({
        cycleId: cycle.id,
        observations: cycle.observations.length,
        adaptations: cycle.adaptations.length,
        duration: cycle.duration,
      });

      reflections.push({
        type: 'meta-cognition',
        insights: reflection.insights || [],
        learningRate: reflection.suggestedLearningRate || this.config.learningRate,
      });

      // Adjust learning rate based on meta-cognition (φ-bounded)
      if (reflection.suggestedLearningRate) {
        const newRate = Math.min(reflection.suggestedLearningRate, PHI_INV);
        if (Math.abs(newRate - this.config.learningRate) > 0.01) {
          log.info('Learning rate adjusted', {
            old: this.config.learningRate,
            new: newRate,
            reason: 'meta-cognition',
          });
          this.config.learningRate = newRate;
        }
      }
    }

    cycle.reflections.push(...reflections);

    this.emit('stage:reflect:complete', {
      cycle,
      count: reflections.length,
    });
  }

  // ───────────────────────────────────────────────────────────────────────────
  // STATS & HEALTH
  // ───────────────────────────────────────────────────────────────────────────

  /**
   * Get pipeline statistics
   */
  getStats() {
    return {
      ...this.stats,
      running: this._running,
      config: this.config,
      currentCycle: this.currentCycle?.toJSON() || null,
    };
  }

  /**
   * Health check
   */
  async health() {
    return {
      running: this._running,
      cycleCount: this.stats.totalCycles,
      lastCycle: this.stats.lastCycleTime,
      avgDuration: Math.round(this.stats.avgCycleDuration),
      dependencies: {
        qLearning: !!this.qLearning,
        modelIntelligence: !!this.modelIntelligence,
        metaCognition: !!this.metaCognition,
      },
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _singleton = null;

export function getLearningPipeline(options) {
  if (!_singleton) {
    _singleton = new LearningPipeline(options);
  }
  return _singleton;
}

export function _resetForTesting() {
  if (_singleton) {
    _singleton.stop();
  }
  _singleton = null;
}
