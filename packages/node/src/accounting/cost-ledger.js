/**
 * CostLedger — Universal Cost Accounting
 *
 * The meta-accountant that CYNIC was blind to.
 * Tracks real token costs across ALL LLM operations,
 * computes burn rate, budget status, and model recommendations.
 *
 * Feeds CynicAccountant.trackOperation() with REAL tokensUsed values
 * instead of the 0 that has been flowing since inception.
 *
 * "Le chien connaît le prix de sa propre pensée" — κυνικός
 *
 * Cell: Cross-cutting (feeds C6.6, C6.1, all domain accountants)
 * Persistence: ~/.cynic/cost/ledger-state.json
 * Events: cost:update → globalEventBus
 *
 * @module @cynic/node/accounting/cost-ledger
 */

'use strict';

import { EventEmitter } from 'events';
import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'fs';
import { join } from 'path';
import { homedir } from 'os';
import {
  PHI_INV, PHI_INV_2, PHI_INV_3, PHI_INV_4,
  createLogger, globalEventBus,
} from '@cynic/core';
import { phiBound, pushHistory, roundTo } from '@cynic/core';

const log = createLogger('CostLedger');

// =============================================================================
// MODEL COST RATES ($ per 1M tokens, as of 2026)
// =============================================================================

/**
 * Model identifiers — what CYNIC knows about
 */
export const ModelId = {
  OPUS: 'opus',
  SONNET: 'sonnet',
  HAIKU: 'haiku',
  OLLAMA: 'ollama',
  UNKNOWN: 'unknown',
};

/**
 * Cost per 1M tokens (USD) — input and output separate.
 * φ-observation: Opus/Sonnet/Haiku price ratios ~φ² apart.
 *
 * These are estimates. Real pricing changes — the ledger
 * can be recalibrated via setCostRates().
 */
const DEFAULT_COST_RATES = {
  [ModelId.OPUS]:    { input: 5.00,   output: 25.00,  label: 'Claude Opus 4.6' },
  [ModelId.SONNET]:  { input: 3.00,   output: 15.00,  label: 'Claude Sonnet 4.5' },
  [ModelId.HAIKU]:   { input: 1.00,   output: 5.00,   label: 'Claude Haiku 4.5' },
  [ModelId.OLLAMA]:  { input: 0,      output: 0,      label: 'Local Ollama (free)' },
  [ModelId.UNKNOWN]: { input: 3.00,   output: 15.00,  label: 'Unknown (Sonnet fallback)' },
};

/**
 * Token estimation: ~4 chars per token (conservative).
 * Same heuristic as @cynic/core/context countTokens.
 */
const CHARS_PER_TOKEN = 4;

/**
 * Budget tiers (φ-derived thresholds)
 */
export const BudgetStatus = {
  ABUNDANT:  'abundant',   // < φ⁻² consumed (< 38.2%)
  MODERATE:  'moderate',   // < φ⁻¹ consumed (< 61.8%)
  CAUTIOUS:  'cautious',   // < 80% consumed
  CRITICAL:  'critical',   // < 95% consumed
  EXHAUSTED: 'exhausted',  // >= 95% consumed
};

// =============================================================================
// COST LEDGER
// =============================================================================

export class CostLedger extends EventEmitter {
  constructor(options = {}) {
    super();

    this._costRates = { ...DEFAULT_COST_RATES, ...(options.costRates || {}) };
    this._currentModel = options.model || ModelId.OPUS;

    // Session budget (configurable — Claude Max x5 = ~5x base)
    this._sessionBudget = options.sessionBudget || null; // null = unlimited

    // Rolling window for burn rate (last N operations)
    this._burnWindow = [];
    this._burnWindowSize = options.burnWindowSize || 55; // F(10)

    // Operation history
    this._history = [];
    this._maxHistory = options.maxHistory || 233; // F(13)

    // Session totals
    this._session = {
      startTime: Date.now(),
      operations: 0,
      inputTokens: 0,
      outputTokens: 0,
      totalTokens: 0,
      estimatedCostUSD: 0,
      byModel: {},
      byOperation: {},
    };

    // Persistence path
    this._persistPath = options.persistPath ||
      join(homedir(), '.cynic', 'cost', 'ledger-state.json');

    // Load persisted state
    this._lifetime = this._loadLifetime();

    // Velocity alarm history (for pattern detection)
    this._velocityAlarms = [];

    // Wire event listeners
    this._wireEvents();

    log.debug('CostLedger initialized', {
      model: this._currentModel,
      budget: this._sessionBudget,
    });
  }

  /**
   * Wire event listeners for velocity tracking.
   * Closes orphan loop: velocity:alarm → record for historical analysis
   * @private
   */
  _wireEvents() {
    globalEventBus.on('velocity:alarm', (data) => {
      const { velocity, trend, tokensPerMinute, action } = data;
      const alarm = {
        timestamp: Date.now(),
        velocity,
        trend,
        tokensPerMinute,
        action,
        budgetStatus: this.getBudgetStatus().level,
      };
      this._velocityAlarms.push(alarm);

      // Keep last 21 alarms (F(8))
      if (this._velocityAlarms.length > 21) {
        this._velocityAlarms.shift();
      }

      log.info('Velocity alarm recorded', {
        velocity: (velocity * 100).toFixed(1) + '%',
        trend,
        tokensPerMinute,
        alarmCount: this._velocityAlarms.length,
      });
    });
  }

  // ===========================================================================
  // CORE: Record an operation
  // ===========================================================================

  /**
   * Record a token-consuming operation.
   *
   * @param {Object} op - Operation details
   * @param {string} op.type - Operation type (e.g. 'tool_call', 'judgment', 'perception')
   * @param {string} [op.model] - Model used (defaults to current)
   * @param {number} [op.inputTokens] - Actual input tokens (if known)
   * @param {number} [op.outputTokens] - Actual output tokens (if known)
   * @param {string} [op.inputText] - Input text (for estimation if tokens unknown)
   * @param {string} [op.outputText] - Output text (for estimation if tokens unknown)
   * @param {number} [op.durationMs] - Operation duration
   * @param {string} [op.source] - What produced this (e.g. 'observe_hook', 'mcp_tool')
   * @param {Object} [op.metadata] - Additional context
   * @returns {Object} Cost record
   */
  record(op) {
    const model = op.model || this._currentModel;
    const rates = this._costRates[model] || this._costRates[ModelId.UNKNOWN];

    // Estimate tokens if not provided
    const inputTokens = op.inputTokens || this._estimateTokens(op.inputText);
    const outputTokens = op.outputTokens || this._estimateTokens(op.outputText);
    const totalTokens = inputTokens + outputTokens;

    // Calculate cost
    const inputCost = (inputTokens / 1_000_000) * rates.input;
    const outputCost = (outputTokens / 1_000_000) * rates.output;
    const totalCost = inputCost + outputCost;

    const timestamp = Date.now();

    const record = {
      timestamp,
      type: op.type || 'unknown',
      model,
      inputTokens,
      outputTokens,
      totalTokens,
      cost: {
        input: roundTo(inputCost, 6),
        output: roundTo(outputCost, 6),
        total: roundTo(totalCost, 6),
      },
      durationMs: op.durationMs || 0,
      source: op.source || 'unknown',
    };

    // Update session
    this._session.operations++;
    this._session.inputTokens += inputTokens;
    this._session.outputTokens += outputTokens;
    this._session.totalTokens += totalTokens;
    this._session.estimatedCostUSD += totalCost;

    // By model
    if (!this._session.byModel[model]) {
      this._session.byModel[model] = { operations: 0, tokens: 0, cost: 0 };
    }
    this._session.byModel[model].operations++;
    this._session.byModel[model].tokens += totalTokens;
    this._session.byModel[model].cost += totalCost;

    // By operation type
    const opType = op.type || 'unknown';
    if (!this._session.byOperation[opType]) {
      this._session.byOperation[opType] = { operations: 0, tokens: 0, cost: 0 };
    }
    this._session.byOperation[opType].operations++;
    this._session.byOperation[opType].tokens += totalTokens;
    this._session.byOperation[opType].cost += totalCost;

    // Burn window (for velocity calculation)
    pushHistory(this._burnWindow, { timestamp, tokens: totalTokens, cost: totalCost }, this._burnWindowSize);

    // History
    pushHistory(this._history, record, this._maxHistory);

    // Update lifetime
    this._lifetime.totalOperations++;
    this._lifetime.totalTokens += totalTokens;
    this._lifetime.totalCostUSD += totalCost;
    this._lifetime.lastActivity = timestamp;

    // Emit for listeners
    this.emit('cost:recorded', record);

    // Publish to global bus (event-listeners.js and others can consume)
    if (typeof globalEventBus.publish === 'function') {
      globalEventBus.publish('cost:update', {
        record,
        burnRate: this.getBurnRate(),
        budget: this.getBudgetStatus(),
      }, { source: 'CostLedger' });
    }

    // Check budget threshold alerts
    this._checkBudgetAlerts();

    return record;
  }

  // ===========================================================================
  // BURN RATE — rolling window velocity
  // ===========================================================================

  /**
   * Calculate current burn rate from rolling window.
   *
   * @returns {Object} Burn rate metrics
   */
  getBurnRate() {
    if (this._burnWindow.length < 2) {
      return {
        tokensPerMinute: 0,
        costPerMinute: 0,
        tokensPerHour: 0,
        costPerHour: 0,
        velocity: 0,     // 0-1 normalized speed
        trend: 'stable',
        samples: this._burnWindow.length,
      };
    }

    const oldest = this._burnWindow[0];
    const newest = this._burnWindow[this._burnWindow.length - 1];
    const durationMs = newest.timestamp - oldest.timestamp;

    if (durationMs <= 0) {
      return {
        tokensPerMinute: 0,
        costPerMinute: 0,
        tokensPerHour: 0,
        costPerHour: 0,
        velocity: 0,
        trend: 'stable',
        samples: this._burnWindow.length,
      };
    }

    const totalTokens = this._burnWindow.reduce((s, w) => s + w.tokens, 0);
    const totalCost = this._burnWindow.reduce((s, w) => s + w.cost, 0);
    const minutes = durationMs / 60000;

    const tokensPerMinute = totalTokens / minutes;
    const costPerMinute = totalCost / minutes;

    // Velocity: normalized 0-1, where 1 = unsustainable burn
    // φ-aligned: 10k tokens/min = velocity 1.0 (Opus context fills in ~20min)
    const velocity = phiBound(tokensPerMinute / 10000);

    // Trend: compare first half vs second half
    const mid = Math.floor(this._burnWindow.length / 2);
    const firstHalfTokens = this._burnWindow.slice(0, mid).reduce((s, w) => s + w.tokens, 0);
    const secondHalfTokens = this._burnWindow.slice(mid).reduce((s, w) => s + w.tokens, 0);
    const trend = secondHalfTokens > firstHalfTokens * 1.2 ? 'accelerating'
      : secondHalfTokens < firstHalfTokens * 0.8 ? 'decelerating'
      : 'stable';

    return {
      tokensPerMinute: roundTo(tokensPerMinute, 1),
      costPerMinute: roundTo(costPerMinute, 6),
      tokensPerHour: roundTo(tokensPerMinute * 60, 0),
      costPerHour: roundTo(costPerMinute * 60, 4),
      velocity,
      trend,
      samples: this._burnWindow.length,
    };
  }

  // ===========================================================================
  // BUDGET STATUS
  // ===========================================================================

  /**
   * Get budget consumption status.
   *
   * @returns {Object} Budget status
   */
  getBudgetStatus() {
    const sessionDuration = Date.now() - this._session.startTime;

    const status = {
      consumed: this._session.totalTokens,
      consumedCost: roundTo(this._session.estimatedCostUSD, 4),
      sessionMinutes: roundTo(sessionDuration / 60000, 1),
      operations: this._session.operations,
    };

    if (this._sessionBudget) {
      const ratio = this._session.totalTokens / this._sessionBudget;
      const burnRate = this.getBurnRate();

      // Time to limit (minutes remaining at current velocity)
      const remaining = this._sessionBudget - this._session.totalTokens;
      const ttl = burnRate.tokensPerMinute > 0
        ? remaining / burnRate.tokensPerMinute
        : Infinity;

      status.budget = this._sessionBudget;
      status.remaining = remaining;
      status.consumedRatio = roundTo(ratio, 3);
      status.timeToLimitMinutes = ttl === Infinity ? null : roundTo(ttl, 1);
      status.level = ratio < PHI_INV_2 ? BudgetStatus.ABUNDANT
        : ratio < PHI_INV ? BudgetStatus.MODERATE
        : ratio < 0.8 ? BudgetStatus.CAUTIOUS
        : ratio < 0.95 ? BudgetStatus.CRITICAL
        : BudgetStatus.EXHAUSTED;
    } else {
      status.budget = null;
      status.remaining = null;
      status.consumedRatio = null;
      status.timeToLimitMinutes = null;
      status.level = BudgetStatus.ABUNDANT; // no limit = always abundant
    }

    return status;
  }

  // ===========================================================================
  // MODEL RECOMMENDATION
  // ===========================================================================

  /**
   * Recommend which model to use based on task complexity and budget.
   *
   * @param {Object} [context] - Task context
   * @param {string} [context.taskType] - 'simple'|'moderate'|'complex'
   * @param {boolean} [context.needsReasoning] - Deep reasoning required?
   * @param {number} [context.estimatedTokens] - Expected token count
   * @returns {Object} Model recommendation
   */
  recommendModel(context = {}) {
    const budget = this.getBudgetStatus();
    const burnRate = this.getBurnRate();

    const { taskType = 'moderate', needsReasoning = false } = context;

    // Base recommendation by task type
    let recommended = ModelId.SONNET;
    let reason = 'balanced cost/capability';

    if (taskType === 'simple' || !needsReasoning) {
      recommended = ModelId.HAIKU;
      reason = 'simple task — Haiku sufficient';
    }

    if (taskType === 'complex' || needsReasoning) {
      recommended = ModelId.OPUS;
      reason = 'complex reasoning required';
    }

    // Budget pressure downgrades
    if (budget.level === BudgetStatus.CRITICAL) {
      if (recommended === ModelId.OPUS) {
        recommended = ModelId.SONNET;
        reason = 'budget critical — downgraded from Opus';
      }
    }

    if (budget.level === BudgetStatus.EXHAUSTED) {
      recommended = ModelId.HAIKU;
      reason = 'budget exhausted — minimum cost model';
    }

    // Velocity pressure (>= because phiBound caps at exactly PHI_INV)
    if (burnRate.velocity >= PHI_INV && recommended === ModelId.OPUS) {
      recommended = ModelId.SONNET;
      reason = 'burn rate > φ⁻¹ — deceleration recommended';
    }

    const rates = this._costRates[recommended];

    return {
      model: recommended,
      reason,
      label: rates.label,
      costPer1MInput: rates.input,
      costPer1MOutput: rates.output,
      budgetLevel: budget.level,
      burnVelocity: burnRate.velocity,
      confidence: phiBound(0.5), // Model recommendations are uncertain
    };
  }

  // ===========================================================================
  // COST ESTIMATION (preview before action)
  // ===========================================================================

  /**
   * Estimate cost of an operation BEFORE executing it.
   *
   * @param {Object} params
   * @param {string} [params.model] - Model to use
   * @param {number} [params.inputTokens] - Expected input tokens
   * @param {number} [params.outputTokens] - Expected output tokens
   * @param {string} [params.inputText] - Input text (for estimation)
   * @returns {Object} Cost estimate
   */
  estimate(params = {}) {
    const model = params.model || this._currentModel;
    const rates = this._costRates[model] || this._costRates[ModelId.UNKNOWN];

    const inputTokens = params.inputTokens || this._estimateTokens(params.inputText);
    const outputTokens = params.outputTokens || 0;

    const inputCost = (inputTokens / 1_000_000) * rates.input;
    const outputCost = (outputTokens / 1_000_000) * rates.output;

    return {
      model,
      inputTokens,
      outputTokens,
      totalTokens: inputTokens + outputTokens,
      estimatedCost: roundTo(inputCost + outputCost, 6),
      budgetImpact: this._sessionBudget
        ? roundTo((inputTokens + outputTokens) / this._sessionBudget, 4)
        : null,
    };
  }

  // ===========================================================================
  // SESSION SUMMARY
  // ===========================================================================

  /**
   * Get complete session cost summary.
   */
  getSessionSummary() {
    const duration = Date.now() - this._session.startTime;
    const burnRate = this.getBurnRate();
    const budget = this.getBudgetStatus();

    return {
      duration,
      durationMinutes: roundTo(duration / 60000, 1),
      operations: this._session.operations,
      tokens: {
        input: this._session.inputTokens,
        output: this._session.outputTokens,
        total: this._session.totalTokens,
      },
      cost: {
        total: roundTo(this._session.estimatedCostUSD, 4),
        perOperation: this._session.operations > 0
          ? roundTo(this._session.estimatedCostUSD / this._session.operations, 6)
          : 0,
        perMinute: burnRate.costPerMinute,
      },
      byModel: this._session.byModel,
      byOperation: this._session.byOperation,
      burnRate,
      budget,
      currentModel: this._currentModel,
    };
  }

  // ===========================================================================
  // LIFETIME STATS
  // ===========================================================================

  /**
   * Get cross-session lifetime statistics.
   */
  getLifetimeStats() {
    return {
      ...this._lifetime,
      totalCostUSD: roundTo(this._lifetime.totalCostUSD, 4),
      sessionsCount: this._lifetime.sessions,
      avgTokensPerSession: this._lifetime.sessions > 0
        ? roundTo(this._lifetime.totalTokens / this._lifetime.sessions, 0)
        : 0,
      avgCostPerSession: this._lifetime.sessions > 0
        ? roundTo(this._lifetime.totalCostUSD / this._lifetime.sessions, 4)
        : 0,
    };
  }

  // ===========================================================================
  // MODEL & BUDGET MANAGEMENT
  // ===========================================================================

  /**
   * Set current model.
   * @param {string} modelId - One of ModelId.*
   */
  setModel(modelId) {
    if (this._costRates[modelId]) {
      this._currentModel = modelId;
      log.debug('Model changed', { model: modelId });
    }
  }

  /**
   * Set session budget (tokens).
   * @param {number|null} budget - Token budget (null = unlimited)
   */
  setSessionBudget(budget) {
    this._sessionBudget = budget;
    log.debug('Session budget set', { budget });
  }

  /**
   * Update cost rates (for price changes).
   * @param {Object} rates - Partial rate overrides
   */
  setCostRates(rates) {
    Object.assign(this._costRates, rates);
    log.debug('Cost rates updated', { models: Object.keys(rates) });
  }

  /**
   * Get current model info.
   */
  getCurrentModel() {
    const rates = this._costRates[this._currentModel];
    return {
      id: this._currentModel,
      label: rates.label,
      inputPer1M: rates.input,
      outputPer1M: rates.output,
    };
  }

  // ===========================================================================
  // PERSISTENCE
  // ===========================================================================

  /**
   * Persist lifetime stats to disk.
   */
  persist() {
    try {
      const dir = join(homedir(), '.cynic', 'cost');
      if (!existsSync(dir)) {
        mkdirSync(dir, { recursive: true });
      }

      const data = {
        ...this._lifetime,
        lastPersisted: Date.now(),
      };

      writeFileSync(this._persistPath, JSON.stringify(data, null, 2), 'utf8');
      log.debug('CostLedger persisted', { path: this._persistPath });
    } catch (err) {
      log.debug('CostLedger persist failed', { error: err.message });
    }
  }

  /**
   * Record session end (persist + increment session count).
   */
  endSession() {
    this._lifetime.sessions++;
    this.persist();
    this.emit('session:ended', this.getSessionSummary());
  }

  // ===========================================================================
  // INTERNALS
  // ===========================================================================

  /**
   * Estimate tokens from text (4 chars per token, conservative).
   * @private
   */
  _estimateTokens(text) {
    if (!text || typeof text !== 'string') return 0;
    return Math.ceil(text.length / CHARS_PER_TOKEN);
  }

  /**
   * Load lifetime stats from disk.
   * @private
   */
  _loadLifetime() {
    try {
      if (existsSync(this._persistPath)) {
        const raw = readFileSync(this._persistPath, 'utf8');
        const data = JSON.parse(raw);
        return {
          totalOperations: data.totalOperations || 0,
          totalTokens: data.totalTokens || 0,
          totalCostUSD: data.totalCostUSD || 0,
          sessions: data.sessions || 0,
          firstSeen: data.firstSeen || Date.now(),
          lastActivity: data.lastActivity || Date.now(),
        };
      }
    } catch {
      // Corrupt or missing — start fresh
    }

    return {
      totalOperations: 0,
      totalTokens: 0,
      totalCostUSD: 0,
      sessions: 0,
      firstSeen: Date.now(),
      lastActivity: Date.now(),
    };
  }

  /**
   * Check budget thresholds and emit alerts.
   * @private
   */
  _checkBudgetAlerts() {
    if (!this._sessionBudget) return;

    const ratio = this._session.totalTokens / this._sessionBudget;

    // Alert at φ-derived thresholds (only once per threshold)
    if (ratio >= 0.95 && !this._alertedExhausted) {
      this._alertedExhausted = true;
      this.emit('budget:exhausted', this.getBudgetStatus());
      log.info('Budget EXHAUSTED (>95%)', { ratio: roundTo(ratio, 3) });
    } else if (ratio >= 0.8 && !this._alertedCritical) {
      this._alertedCritical = true;
      this.emit('budget:critical', this.getBudgetStatus());
      log.info('Budget CRITICAL (>80%)', { ratio: roundTo(ratio, 3) });
    } else if (ratio >= PHI_INV && !this._alertedModerate) {
      this._alertedModerate = true;
      this.emit('budget:moderate', this.getBudgetStatus());
      log.debug('Budget passed φ⁻¹ threshold (>61.8%)');
    }
  }

  /**
   * Get operation history.
   * @param {number} [limit=21] - Max items
   */
  getHistory(limit = 21) {
    return this._history.slice(-limit);
  }

  /**
   * Reset session (for testing or new session).
   */
  resetSession() {
    this._session = {
      startTime: Date.now(),
      operations: 0,
      inputTokens: 0,
      outputTokens: 0,
      totalTokens: 0,
      estimatedCostUSD: 0,
      byModel: {},
      byOperation: {},
    };
    this._burnWindow = [];
    this._history = [];
    this._alertedExhausted = false;
    this._alertedCritical = false;
    this._alertedModerate = false;
    this.emit('session:reset');
  }
}

// =============================================================================
// SINGLETON
// =============================================================================

let _instance = null;

/**
 * Get or create the CostLedger singleton.
 * @param {Object} [options] - Constructor options (only used on first call)
 */
export function getCostLedger(options = {}) {
  if (!_instance) {
    _instance = new CostLedger(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing).
 */
export function resetCostLedger() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  CostLedger,
  ModelId,
  BudgetStatus,
  getCostLedger,
  resetCostLedger,
};
