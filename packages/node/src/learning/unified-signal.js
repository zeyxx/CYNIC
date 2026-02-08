/**
 * CYNIC Unified Signal - Single Type for Storage & Processing
 *
 * "Une seule vérité, un seul type" - κυνικός
 *
 * This type unifies:
 * - RLHF Feedback (outcome, actualScore)
 * - DPO Pairs (chosen/rejected)
 * - Q-Learning Episodes (state, action, reward)
 * - Telemetry Events (tool usage, latency)
 *
 * ONE TYPE flows through: PERCEIVE → JUDGE → DECIDE → ACT → LEARN
 * No format conversion, just progressive enrichment.
 *
 * @module @cynic/node/learning/unified-signal
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('UnifiedSignal');

/**
 * Signal types - what triggered this signal
 */
export const SignalSource = {
  TOOL_EXECUTION: 'tool_execution',     // MCP tool was called
  JUDGMENT: 'judgment',                  // Judge evaluated something
  CYNIC_JUDGE: 'cynic_judge',           // CYNIC Judge scored an item
  USER_FEEDBACK: 'user_feedback',        // Human correction
  TEST_RESULT: 'test_result',            // Automated test
  BUILD_RESULT: 'build_result',          // CI/CD outcome
  TRANSACTION: 'transaction',            // Solana tx
  PATTERN: 'pattern',                    // Emergent pattern
  DOG_ACTION: 'dog_action',              // Dog made decision
  MANUAL: 'manual',                      // Manually recorded signal
};

/**
 * Signal outcome - what happened
 */
export const SignalOutcome = {
  SUCCESS: 'success',
  FAILURE: 'failure',
  PARTIAL: 'partial',
  BLOCKED: 'blocked',
  TIMEOUT: 'timeout',
  UNKNOWN: 'unknown',
  CORRECT: 'correct',             // Ground truth: judgment was right
  INCORRECT: 'incorrect',         // Ground truth: judgment was wrong
  PENDING: 'pending',             // Outcome not yet known
};

/**
 * Unified Signal - The One Type
 *
 * Every learning event, feedback, metric, judgment flows through this.
 */
export class UnifiedSignal {
  /**
   * Create a unified signal
   *
   * @param {Object} params - Signal parameters
   */
  constructor(params = {}) {
    // ═══════════════════════════════════════════════════════════════════
    // IDENTITY (immutable after creation)
    // ═══════════════════════════════════════════════════════════════════
    this.id = params.id || `sig_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    this.timestamp = params.timestamp || Date.now();
    this.source = params.source || SignalSource.TOOL_EXECUTION;
    this.sessionId = params.sessionId || null;

    // ═══════════════════════════════════════════════════════════════════
    // RAW INPUT (what triggered the signal)
    // ═══════════════════════════════════════════════════════════════════
    this.input = {
      // What was evaluated/executed
      itemType: params.itemType || null,    // 'code', 'test', 'transaction', 'query'
      itemHash: params.itemHash || null,     // Content hash for dedup
      itemContent: params.itemContent || null, // Actual content (if small)

      // Context at time of signal
      tool: params.tool || null,             // 'Bash', 'Write', 'Edit', etc
      dog: params.dog || null,               // Which dog handled this
      taskType: params.taskType || null,     // 'security', 'code_change', 'query'

      // State features (for Q-learning)
      features: params.features || [],       // ['task:security', 'risk:high', ...]
    };

    // ═══════════════════════════════════════════════════════════════════
    // JUDGMENT (filled by Judge, optional)
    // ═══════════════════════════════════════════════════════════════════
    this.judgment = {
      qScore: params.qScore ?? null,         // 0-100
      confidence: params.confidence ?? null,  // 0-0.618 (φ max)
      verdict: params.verdict || null,       // 'HOWL', 'WAG', 'GROWL', 'BARK'
      axiomScores: params.axiomScores || {}, // { PHI: 60, VERIFY: 45, ... }
      judgmentId: params.judgmentId || null,
    };

    // ═══════════════════════════════════════════════════════════════════
    // OUTCOME (what actually happened - ground truth)
    // ═══════════════════════════════════════════════════════════════════
    this.outcome = {
      status: params.outcome || SignalOutcome.UNKNOWN,
      actualScore: params.actualScore ?? null,  // Ground truth score (0-100)
      reason: params.reason || null,
      errorCode: params.errorCode || null,
      errorMessage: params.errorMessage || null,
    };

    // ═══════════════════════════════════════════════════════════════════
    // TELEMETRY (performance metrics)
    // ═══════════════════════════════════════════════════════════════════
    this.telemetry = {
      latencyMs: params.latencyMs ?? null,
      tokensUsed: params.tokensUsed ?? null,
      computeUnits: params.computeUnits ?? null,  // For Solana
      fee: params.fee ?? null,                     // For Solana (lamports)
    };

    // ═══════════════════════════════════════════════════════════════════
    // LEARNING (computed from judgment vs outcome)
    // ═══════════════════════════════════════════════════════════════════
    this.learning = {
      // Reward signal for RL (computed)
      reward: null,

      // Score delta (actualScore - qScore)
      scoreDelta: null,

      // Feedback classification
      feedbackType: null,  // 'correct', 'incorrect', 'overscored', 'underscored'

      // For DPO pairing
      canPair: false,
      pairId: null,
      isChosen: null,  // true = chosen, false = rejected in pair
    };

    // ═══════════════════════════════════════════════════════════════════
    // METADATA
    // ═══════════════════════════════════════════════════════════════════
    this.metadata = params.metadata || {};
    this.processed = false;
    this.persistedAt = null;
  }

  // ═════════════════════════════════════════════════════════════════════
  // ENRICHMENT METHODS (progressive enhancement)
  // ═════════════════════════════════════════════════════════════════════

  /**
   * Add judgment data (from CYNICJudge)
   */
  addJudgment(judgment) {
    this.judgment.qScore = judgment.qScore;
    this.judgment.confidence = Math.min(PHI_INV, judgment.confidence);
    this.judgment.verdict = judgment.verdict;
    this.judgment.axiomScores = judgment.axiomScores || {};
    this.judgment.judgmentId = judgment.id;
    return this;
  }

  /**
   * Add outcome (ground truth from feedback/test/build)
   */
  addOutcome(outcome) {
    this.outcome.status = outcome.status || outcome;
    this.outcome.actualScore = outcome.actualScore ?? outcome.score ?? null;
    this.outcome.reason = outcome.reason || null;
    this.outcome.errorCode = outcome.errorCode || null;
    this.outcome.errorMessage = outcome.errorMessage || outcome.error || null;

    // Compute learning signals
    this._computeLearningSignals();
    return this;
  }

  /**
   * Add telemetry data
   */
  addTelemetry(telemetry) {
    if (telemetry.latencyMs !== undefined) this.telemetry.latencyMs = telemetry.latencyMs;
    if (telemetry.tokensUsed !== undefined) this.telemetry.tokensUsed = telemetry.tokensUsed;
    if (telemetry.computeUnits !== undefined) this.telemetry.computeUnits = telemetry.computeUnits;
    if (telemetry.fee !== undefined) this.telemetry.fee = telemetry.fee;
    return this;
  }

  /**
   * Compute learning signals from judgment vs outcome
   * @private
   */
  _computeLearningSignals() {
    // Reward calculation (RL signal)
    switch (this.outcome.status) {
      case SignalOutcome.SUCCESS:
      case SignalOutcome.CORRECT:
        this.learning.reward = 1.0;
        break;
      case SignalOutcome.PARTIAL:
        this.learning.reward = 0.5;
        break;
      case SignalOutcome.BLOCKED:
        this.learning.reward = 0.8;  // Blocking danger is good
        break;
      case SignalOutcome.FAILURE:
      case SignalOutcome.INCORRECT:
        this.learning.reward = -0.5;
        break;
      case SignalOutcome.TIMEOUT:
        this.learning.reward = -0.3;
        break;
      case SignalOutcome.PENDING:
        this.learning.reward = null;  // Not yet known
        break;
      default:
        this.learning.reward = 0;
    }

    // Score delta (if both judgment and actual exist)
    if (this.judgment.qScore !== null && this.outcome.actualScore !== null) {
      this.learning.scoreDelta = this.outcome.actualScore - this.judgment.qScore;

      // Classify feedback type
      const delta = this.learning.scoreDelta;
      if (Math.abs(delta) < 10) {
        this.learning.feedbackType = 'correct';
      } else if (delta > 0) {
        this.learning.feedbackType = 'underscored';  // We scored too low
      } else {
        this.learning.feedbackType = 'overscored';   // We scored too high
      }
    } else if (this.outcome.status === SignalOutcome.SUCCESS || this.outcome.status === SignalOutcome.CORRECT) {
      this.learning.feedbackType = 'correct';
    } else if (this.outcome.status === SignalOutcome.FAILURE || this.outcome.status === SignalOutcome.INCORRECT) {
      this.learning.feedbackType = 'incorrect';
    }

    // DPO pairing eligibility (PENDING signals can't pair — no ground truth yet)
    this.learning.canPair =
      this.judgment.qScore !== null &&
      this.outcome.status !== SignalOutcome.UNKNOWN &&
      this.outcome.status !== SignalOutcome.PENDING &&
      this.learning.feedbackType !== null;
  }

  // ═════════════════════════════════════════════════════════════════════
  // CONVERSION METHODS (for legacy systems)
  // ═════════════════════════════════════════════════════════════════════

  /**
   * Convert to RLHF Feedback format (legacy)
   */
  toRLHFFeedback() {
    return {
      judgmentId: this.judgment.judgmentId,
      outcome: this.learning.feedbackType === 'correct' ? 'correct'
        : this.learning.feedbackType === 'incorrect' ? 'incorrect'
          : 'partial',
      actualScore: this.outcome.actualScore,
      originalScore: this.judgment.qScore,
      itemType: this.input.itemType,
      source: this.source,
      sourceContext: this.metadata,
      scoreDelta: this.learning.scoreDelta,
      processedAt: Date.now(),
    };
  }

  /**
   * Convert to Q-Learning Episode format (legacy)
   */
  toQLearningEpisode() {
    return {
      episodeId: this.id,
      features: this.input.features,
      taskType: this.input.taskType,
      tool: this.input.tool,
      actions: this.input.dog ? [{ action: this.input.dog, timestamp: this.timestamp }] : [],
      outcome: {
        success: this.outcome.status === SignalOutcome.SUCCESS,
        score: this.judgment.qScore,
        blocked: this.outcome.status === SignalOutcome.BLOCKED,
        type: this.outcome.status,
      },
      reward: this.learning.reward,
      durationMs: this.telemetry.latencyMs,
    };
  }

  /**
   * Convert to DPO pair candidate format
   */
  toDPOCandidate() {
    return {
      signalId: this.id,
      judgmentId: this.judgment.judgmentId,
      qScore: this.judgment.qScore,
      verdict: this.judgment.verdict,
      itemType: this.input.itemType,
      outcome: this.learning.feedbackType,
      actualScore: this.outcome.actualScore,
      reason: this.outcome.reason,
      confidence: this.judgment.confidence,
      contextType: this.input.itemType,
      taskType: this.input.taskType,
    };
  }

  /**
   * Convert to telemetry record
   */
  toTelemetryRecord() {
    return {
      signalId: this.id,
      timestamp: this.timestamp,
      source: this.source,
      tool: this.input.tool,
      success: this.outcome.status === SignalOutcome.SUCCESS,
      latencyMs: this.telemetry.latencyMs,
      tokensUsed: this.telemetry.tokensUsed,
      errorCode: this.outcome.errorCode,
    };
  }

  /**
   * Convert to JSON for persistence
   */
  toJSON() {
    return {
      id: this.id,
      timestamp: this.timestamp,
      source: this.source,
      sessionId: this.sessionId,
      input: this.input,
      judgment: this.judgment,
      outcome: this.outcome,
      telemetry: this.telemetry,
      learning: this.learning,
      metadata: this.metadata,
      processed: this.processed,
      persistedAt: this.persistedAt,
    };
  }

  /**
   * Create from JSON
   */
  static fromJSON(json) {
    const signal = new UnifiedSignal({
      id: json.id,
      timestamp: json.timestamp,
      source: json.source,
      sessionId: json.sessionId,
      ...json.input,
      ...json.judgment,
      ...json.outcome,
      ...json.telemetry,
      metadata: json.metadata,
    });
    signal.learning = json.learning;
    signal.processed = json.processed;
    signal.persistedAt = json.persistedAt;
    return signal;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// UNIFIED SIGNAL STORE
// ═══════════════════════════════════════════════════════════════════════════════

/**
 * Unified Signal Store - Single place for all signals
 */
export class UnifiedSignalStore extends EventEmitter {
  constructor(options = {}) {
    super();

    this._signals = new Map();
    this._maxSignals = options.maxSignals || 10000;
    this._persistencePool = options.pool || null;

    // Indexes for fast lookup
    this._bySource = new Map();
    this._byItemType = new Map();
    this._byOutcome = new Map();
    this._unpaired = [];  // Signals waiting for DPO pairing

    // Stats
    this._stats = {
      totalReceived: 0,
      totalPersisted: 0,
      bySource: {},
      byOutcome: {},
    };
  }

  /**
   * Record a signal
   */
  async record(signal) {
    // Ensure it's a UnifiedSignal
    if (!(signal instanceof UnifiedSignal)) {
      signal = new UnifiedSignal(signal);
    }

    // Store in memory
    this._signals.set(signal.id, signal);
    this._stats.totalReceived++;

    // Update indexes
    this._indexSignal(signal);

    // Check for DPO pairing
    if (signal.learning.canPair) {
      this._checkDPOPairing(signal);
    }

    // Emit to event bus
    this.emit('signal', signal);
    globalEventBus.emit('learning:signal', signal.toJSON());

    // Persist if pool available
    if (this._persistencePool) {
      await this._persist(signal);
    }

    // Cleanup old signals
    this._cleanup();

    return signal;
  }

  /**
   * Index signal for fast lookup
   * @private
   */
  _indexSignal(signal) {
    // By source
    if (!this._bySource.has(signal.source)) {
      this._bySource.set(signal.source, []);
    }
    this._bySource.get(signal.source).push(signal.id);
    this._stats.bySource[signal.source] = (this._stats.bySource[signal.source] || 0) + 1;

    // By item type
    if (signal.input.itemType) {
      if (!this._byItemType.has(signal.input.itemType)) {
        this._byItemType.set(signal.input.itemType, []);
      }
      this._byItemType.get(signal.input.itemType).push(signal.id);
    }

    // By outcome
    if (signal.outcome.status !== SignalOutcome.UNKNOWN) {
      if (!this._byOutcome.has(signal.outcome.status)) {
        this._byOutcome.set(signal.outcome.status, []);
      }
      this._byOutcome.get(signal.outcome.status).push(signal.id);
      this._stats.byOutcome[signal.outcome.status] = (this._stats.byOutcome[signal.outcome.status] || 0) + 1;
    }
  }

  /**
   * Check for DPO pairing opportunities
   * @private
   */
  _checkDPOPairing(signal) {
    // Look for opposite outcome in same context
    const contextKey = `${signal.input.itemType}:${signal.input.taskType}`;

    for (let i = this._unpaired.length - 1; i >= 0; i--) {
      const candidate = this._signals.get(this._unpaired[i]);
      if (!candidate) {
        this._unpaired.splice(i, 1);
        continue;
      }

      const candContextKey = `${candidate.input.itemType}:${candidate.input.taskType}`;

      // Same context, opposite outcome
      if (contextKey === candContextKey &&
          signal.learning.feedbackType !== candidate.learning.feedbackType) {

        // Determine chosen/rejected
        let chosen, rejected;
        if (signal.learning.feedbackType === 'correct' ||
            (signal.learning.feedbackType === 'partial' && candidate.learning.feedbackType === 'incorrect')) {
          chosen = signal;
          rejected = candidate;
        } else {
          chosen = candidate;
          rejected = signal;
        }

        // Create pair
        const pairId = `pair_${Date.now()}`;
        chosen.learning.pairId = pairId;
        chosen.learning.isChosen = true;
        rejected.learning.pairId = pairId;
        rejected.learning.isChosen = false;

        // Emit pair event
        this.emit('dpo_pair', {
          pairId,
          chosen: chosen.toDPOCandidate(),
          rejected: rejected.toDPOCandidate(),
          context: {
            contextType: signal.input.itemType,
            taskType: signal.input.taskType,
          },
        });

        globalEventBus.emit('learning:dpo_pair', {
          pairId,
          chosenId: chosen.id,
          rejectedId: rejected.id,
        });

        // Remove from unpaired
        this._unpaired.splice(i, 1);
        log.info('DPO pair created', { pairId, chosen: chosen.id, rejected: rejected.id });
        return;
      }
    }

    // No pair found, add to unpaired list
    this._unpaired.push(signal.id);
  }

  /**
   * Persist signal to database
   * @private
   */
  async _persist(signal) {
    if (!this._persistencePool) return;

    try {
      await this._persistencePool.query(`
        INSERT INTO unified_signals (
          id, timestamp, source, session_id, input, judgment, outcome, telemetry, learning, metadata
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        ON CONFLICT (id) DO UPDATE SET
          judgment = $6, outcome = $7, telemetry = $8, learning = $9
      `, [
        signal.id,
        new Date(signal.timestamp),
        signal.source,
        signal.sessionId,
        JSON.stringify(signal.input),
        JSON.stringify(signal.judgment),
        JSON.stringify(signal.outcome),
        JSON.stringify(signal.telemetry),
        JSON.stringify(signal.learning),
        JSON.stringify(signal.metadata),
      ]);

      signal.persistedAt = Date.now();
      this._stats.totalPersisted++;
    } catch (error) {
      log.error('Failed to persist signal', { id: signal.id, error: error.message });
    }
  }

  /**
   * Cleanup old signals
   * @private
   */
  _cleanup() {
    if (this._signals.size <= this._maxSignals) return;

    // Remove oldest signals
    const toRemove = this._signals.size - this._maxSignals;
    const ids = Array.from(this._signals.keys()).slice(0, toRemove);

    for (const id of ids) {
      this._signals.delete(id);
    }
  }

  /**
   * Get signal by ID
   */
  get(id) {
    return this._signals.get(id);
  }

  /**
   * Get signals by source
   */
  getBySource(source, limit = 100) {
    const ids = this._bySource.get(source) || [];
    return ids.slice(-limit).map(id => this._signals.get(id)).filter(Boolean);
  }

  /**
   * Get signals by outcome
   */
  getByOutcome(outcome, limit = 100) {
    const ids = this._byOutcome.get(outcome) || [];
    return ids.slice(-limit).map(id => this._signals.get(id)).filter(Boolean);
  }

  /**
   * Get unpaired signals (for DPO)
   */
  getUnpaired() {
    return this._unpaired.map(id => this._signals.get(id)).filter(Boolean);
  }

  /**
   * Get stats
   */
  getStats() {
    return {
      ...this._stats,
      signalsInMemory: this._signals.size,
      unpairedCount: this._unpaired.length,
    };
  }

  /**
   * Clear all data
   */
  clear() {
    this._signals.clear();
    this._bySource.clear();
    this._byItemType.clear();
    this._byOutcome.clear();
    this._unpaired = [];
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _store = null;

/**
 * Get or create UnifiedSignalStore singleton
 */
export function getUnifiedSignalStore(options = {}) {
  if (!_store) {
    _store = new UnifiedSignalStore(options);
  }
  return _store;
}

/**
 * Reset singleton (for testing)
 */
export function resetUnifiedSignalStore() {
  if (_store) {
    _store.removeAllListeners();
    _store.clear();
  }
  _store = null;
}

export default {
  UnifiedSignal,
  UnifiedSignalStore,
  SignalSource,
  SignalOutcome,
  getUnifiedSignalStore,
  resetUnifiedSignalStore,
};
