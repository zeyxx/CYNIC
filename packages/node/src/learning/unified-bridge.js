/**
 * Unified Learning Bridge
 *
 * Connects CYNIC Judge output to UnifiedSignal pipeline.
 * "Tous les chemins mènent à l'apprentissage" - κυνικός
 *
 * This bridge:
 * 1. Listens to JUDGMENT_CREATED events from globalEventBus
 * 2. Creates UnifiedSignal from each judgment
 * 3. Stores in UnifiedSignalStore for learning pipeline
 *
 * The UnifiedSignal can then be:
 * - Converted to RLHF feedback when outcome is known
 * - Paired for DPO training
 * - Used for Q-learning episodes
 *
 * @module @cynic/node/learning/unified-bridge
 */

'use strict';

import { EventEmitter } from 'events';
import { globalEventBus, EventType, createLogger } from '@cynic/core';
import {
  UnifiedSignal,
  SignalSource,
  SignalOutcome,
  getUnifiedSignalStore,
} from './unified-signal.js';
import { getPool } from '@cynic/persistence';
import { getDBBatchWriter } from './db-batch-writer.js';

const log = createLogger('UnifiedBridge');

// ═══════════════════════════════════════════════════════════════════════════════
// Configuration
// ═══════════════════════════════════════════════════════════════════════════════

const BRIDGE_CONFIG = {
  // Auto-record judgments as signals
  autoRecordJudgments: true,

  // Auto-record tool executions
  autoRecordTools: true,

  // Buffer before storing (ms) - allows outcome attachment
  bufferTime: 100,

  // Max pending signals before force flush
  maxPending: 100,
};

// ═══════════════════════════════════════════════════════════════════════════════
// UnifiedBridge Class
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * UnifiedBridge - Connects Judge events to UnifiedSignal
 */
export class UnifiedBridge extends EventEmitter {
  /**
   * Create a new UnifiedBridge
   *
   * @param {Object} [options] - Configuration
   */
  constructor(options = {}) {
    super();

    this.config = { ...BRIDGE_CONFIG, ...options };
    this.store = options.store || getUnifiedSignalStore();

    // Pending signals awaiting outcome
    this._pending = new Map();

    // Subscribed events
    this._subscriptions = [];

    // Stats
    this._stats = {
      judgmentsReceived: 0,
      toolExecutionsReceived: 0,
      signalsCreated: 0,
      signalsWithOutcome: 0,
      errors: 0,
    };

    // DB batch writer for learning_events
    this.pool = options.pool || getPool();
    this.batchWriter = getDBBatchWriter(this.pool, {
      bufferLimit: 10,
      flushIntervalMs: 100,
    });

    this._isActive = false;
  }

  /**
   * Start the bridge (subscribe to events)
   */
  start() {
    if (this._isActive) return;

    log.info('Starting UnifiedBridge...');

    // Subscribe to JUDGMENT_CREATED
    if (this.config.autoRecordJudgments) {
      const judgmentHandler = (event) => this._handleJudgment(event);
      globalEventBus.on(EventType.JUDGMENT_CREATED, judgmentHandler);
      this._subscriptions.push([EventType.JUDGMENT_CREATED, judgmentHandler]);
    }

    // Subscribe to TOOL_COMPLETED (tool executions with outcome)
    if (this.config.autoRecordTools) {
      const toolHandler = (event) => this._handleToolExecution(event);
      globalEventBus.on(EventType.TOOL_COMPLETED, toolHandler);
      this._subscriptions.push([EventType.TOOL_COMPLETED, toolHandler]);
    }

    // Subscribe to FEEDBACK events (to attach outcomes)
    const feedbackHandler = (event) => this._handleFeedback(event);
    globalEventBus.on(EventType.USER_FEEDBACK, feedbackHandler);
    this._subscriptions.push([EventType.USER_FEEDBACK, feedbackHandler]);

    this._isActive = true;
    log.info('UnifiedBridge started');
    this.emit('started');
  }

  /**
   * Stop the bridge (unsubscribe from events)
   */
  stop() {
    if (!this._isActive) return;

    log.info('Stopping UnifiedBridge...');

    // Unsubscribe from all events
    for (const [eventType, handler] of this._subscriptions) {
      globalEventBus.off(eventType, handler);
    }
    this._subscriptions = [];

    // Flush pending signals
    this._flushPending();

    this._isActive = false;
    log.info('UnifiedBridge stopped');
    this.emit('stopped');
  }

  /**
   * Handle JUDGMENT_CREATED event
   * @private
   */
  async _handleJudgment(event) {
    this._stats.judgmentsReceived++;

    try {
      const { id, payload } = event;
      const { qScore, verdict, dimensions, itemType, confidence } = payload || {};

      // Create UnifiedSignal from judgment
      const signal = new UnifiedSignal({
        source: SignalSource.CYNIC_JUDGE,
        sessionId: event.metadata?.sessionId || null,

        // Input
        input: {
          itemType: itemType || 'unknown',
          itemHash: id,
          tool: 'judge',
          dog: 'Oracle', // Judge is Oracle's domain
          taskType: 'judgment',
          features: {
            dimensionCount: Object.keys(dimensions || {}).length,
          },
        },

        // Judgment (from the event)
        judgment: {
          qScore,
          confidence,
          verdict: verdict?.verdict || verdict,
          axiomScores: this._extractAxiomScores(dimensions),
          judgmentId: id,
        },

        // Outcome will be filled later by feedback
        outcome: {
          status: SignalOutcome.PENDING,
        },
      });

      // Store in pending (awaits outcome)
      this._pending.set(id, signal);

      // Buffer then store
      setTimeout(() => {
        if (this._pending.has(id)) {
          this._storeSignal(this._pending.get(id));
          this._pending.delete(id);
        }
      }, this.config.bufferTime);

      // Check pending limit
      if (this._pending.size > this.config.maxPending) {
        this._flushPending();
      }

      this._stats.signalsCreated++;
      this.emit('signal_created', { signalId: signal.id, judgmentId: id });

      // Record to learning_events for G1.2 metric (DPO pair candidate, batched, non-blocking)
      this.batchWriter.add(`
        INSERT INTO learning_events (loop_type, event_type, judgment_id, feedback_value, metadata)
        VALUES ($1, $2, $3, $4, $5)
      `, [
        'dpo',
        'pair-candidate',
        id,
        qScore || null,
        JSON.stringify({
          signalId: signal.id,
          source: SignalSource.CYNIC_JUDGE,
          verdict: verdict?.verdict || verdict,
          confidence: confidence || null
        })
      ]);

    } catch (err) {
      this._stats.errors++;
      log.error('Error handling judgment', { error: err.message });
    }
  }

  /**
   * Handle TOOL_EXECUTION event
   * @private
   */
  async _handleToolExecution(event) {
    this._stats.toolExecutionsReceived++;

    try {
      const { tool, success, duration, error } = event.payload || {};

      // Create UnifiedSignal from tool execution
      const signal = new UnifiedSignal({
        source: SignalSource.TOOL_EXECUTION,
        sessionId: event.metadata?.sessionId || null,

        input: {
          itemType: 'tool_call',
          itemHash: `tool_${Date.now()}`,
          tool: tool,
          dog: this._getDogForTool(tool),
          taskType: 'tool_execution',
        },

        // No judgment for direct tool calls
        judgment: null,

        // Outcome from execution result
        outcome: {
          status: success ? SignalOutcome.CORRECT : SignalOutcome.INCORRECT,
          reason: error || 'success',
        },

        // Telemetry
        telemetry: {
          latencyMs: duration,
        },
      });

      // Store immediately (tool executions have immediate outcome)
      await this._storeSignal(signal);
      this._stats.signalsCreated++;

    } catch (err) {
      this._stats.errors++;
      log.error('Error handling tool execution', { error: err.message });
    }
  }

  /**
   * Handle FEEDBACK_RECEIVED event
   * @private
   */
  async _handleFeedback(event) {
    try {
      const { judgmentId, isCorrect, correction, source: feedbackSource } = event.payload || {};

      // Find pending signal
      const signal = this._pending.get(judgmentId);
      if (!signal) {
        // Signal already stored, need to update in store
        await this._updateSignalOutcome(judgmentId, {
          status: isCorrect ? SignalOutcome.CORRECT : SignalOutcome.INCORRECT,
          reason: correction || feedbackSource,
        });
        return;
      }

      // Update pending signal with outcome
      signal.outcome = {
        status: isCorrect ? SignalOutcome.CORRECT : SignalOutcome.INCORRECT,
        actualScore: isCorrect ? signal.judgment?.qScore : Math.max(0, (signal.judgment?.qScore || 50) - 30),
        reason: correction || feedbackSource,
      };

      // Calculate learning values
      signal.learning = {
        reward: isCorrect ? 1.0 : -1.0,
        scoreDelta: signal.outcome.actualScore - (signal.judgment?.qScore || 50),
        feedbackType: isCorrect ? 'POSITIVE' : 'NEGATIVE',
        canPair: true,
        isChosen: isCorrect,
      };

      // Store immediately
      await this._storeSignal(signal);
      this._pending.delete(judgmentId);

      this._stats.signalsWithOutcome++;

    } catch (err) {
      this._stats.errors++;
      log.error('Error handling feedback', { error: err.message });
    }
  }

  /**
   * Store signal in UnifiedSignalStore
   * @private
   */
  async _storeSignal(signal) {
    try {
      await this.store.record(signal);
      log.debug('Signal stored', { id: signal.id, source: signal.source });
    } catch (err) {
      log.error('Failed to store signal', { error: err.message });
      throw err;
    }
  }

  /**
   * Update outcome for already-stored signal
   * @private
   */
  async _updateSignalOutcome(judgmentId, outcome) {
    const updated = await this.store.updateOutcome(judgmentId, outcome);
    if (updated) this._stats.signalsWithOutcome++;
  }

  /**
   * Flush all pending signals
   * @private
   */
  async _flushPending() {
    for (const [id, signal] of this._pending) {
      await this._storeSignal(signal);
    }
    this._pending.clear();
    log.debug('Flushed pending signals');
  }

  /**
   * Extract axiom scores from dimensions
   * @private
   */
  _extractAxiomScores(dimensions) {
    if (!dimensions) return {};

    // Simplified extraction - in production would aggregate properly
    const axioms = { PHI: [], VERIFY: [], CULTURE: [], BURN: [] };

    for (const [dim, score] of Object.entries(dimensions)) {
      // Determine axiom from dimension name patterns
      if (dim.includes('CONFIDENCE') || dim.includes('UNCERTAINTY')) {
        axioms.PHI.push(score);
      } else if (dim.includes('TEST') || dim.includes('PROOF')) {
        axioms.VERIFY.push(score);
      } else if (dim.includes('PATTERN') || dim.includes('CONVENTION')) {
        axioms.CULTURE.push(score);
      } else {
        axioms.BURN.push(score);
      }
    }

    // Calculate axiom averages
    const result = {};
    for (const [axiom, scores] of Object.entries(axioms)) {
      if (scores.length > 0) {
        result[axiom] = scores.reduce((a, b) => a + b, 0) / scores.length;
      }
    }

    return result;
  }

  /**
   * Get dog responsible for tool
   * @private
   */
  _getDogForTool(tool) {
    const toolDogMap = {
      'Read': 'Scout',
      'Write': 'Architect',
      'Edit': 'Architect',
      'Bash': 'Guardian',
      'Grep': 'Scout',
      'Glob': 'Scout',
      'Task': 'CYNIC',
      'WebFetch': 'Scholar',
      'WebSearch': 'Scholar',
    };
    return toolDogMap[tool] || 'CYNIC';
  }

  /**
   * Manually record a signal
   *
   * @param {Object} data - Signal data
   * @returns {UnifiedSignal} Created signal
   */
  recordManual(data) {
    const signal = new UnifiedSignal({
      source: SignalSource.MANUAL,
      ...data,
    });

    this._storeSignal(signal);
    this._stats.signalsCreated++;

    return signal;
  }

  /**
   * Get bridge statistics
   */
  getStats() {
    return {
      ...this._stats,
      pendingSignals: this._pending.size,
      isActive: this._isActive,
      storeStats: this.store.getStats(),
    };
  }

  /**
   * Get health assessment
   */
  getHealth() {
    const errorRate = this._stats.signalsCreated > 0
      ? this._stats.errors / this._stats.signalsCreated
      : 0;

    return {
      status: errorRate < 0.1 ? 'healthy' : errorRate < 0.3 ? 'degraded' : 'unhealthy',
      errorRate,
      isActive: this._isActive,
      pendingSignals: this._pending.size,
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create UnifiedBridge singleton
 *
 * @param {Object} [options] - Options
 * @returns {UnifiedBridge}
 */
export function getUnifiedBridge(options = {}) {
  if (!_instance) {
    _instance = new UnifiedBridge(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetUnifiedBridge() {
  if (_instance) {
    _instance.stop();
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  UnifiedBridge,
  getUnifiedBridge,
  resetUnifiedBridge,
};
