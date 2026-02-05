/**
 * CYNIC Solana Decider - C2.3 (SOLANA × DECIDE)
 *
 * Routes Solana transaction decisions using φ-aligned logic.
 * Part of the 7×7 Fractal Matrix decision layer.
 *
 * "Act when φ permits, wait when φ doubts" - κυνικός
 *
 * Decides:
 * - Transaction priority (send now vs queue)
 * - Retry strategy (when/how to retry)
 * - Fee optimization (priority vs cost)
 * - RPC endpoint selection (load balancing)
 *
 * @module @cynic/node/solana/solana-decider
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger, globalEventBus } from '@cynic/core';

const log = createLogger('SolanaDecider');

/**
 * Decision types
 */
export const SolanaDecisionType = {
  SEND_NOW: 'send_now',
  QUEUE: 'queue',
  RETRY: 'retry',
  ABORT: 'abort',
  WAIT: 'wait',
};

/**
 * Transaction priority levels
 */
export const PriorityLevel = {
  CRITICAL: 'critical',   // Send immediately, max fee
  HIGH: 'high',           // Send soon, elevated fee
  NORMAL: 'normal',       // Standard processing
  LOW: 'low',             // Wait for low congestion
  BATCH: 'batch',         // Batch with others
};

/**
 * φ-aligned decision thresholds
 */
const DECISION_THRESHOLDS = {
  // Confidence needed to send immediately
  sendNowConfidence: PHI_INV,        // 61.8%

  // Confidence needed to queue
  queueConfidence: PHI_INV_2,        // 38.2%

  // Max retries before abort
  maxRetries: 8,                      // Fib(6)

  // Retry delay multiplier (exponential backoff)
  retryBackoffBase: 1.618,           // φ

  // Fee thresholds (in lamports)
  maxFeeNormal: 5000,
  maxFeeHigh: 25000,
  maxFeeCritical: 100000,
};

/**
 * SolanaDecider - Routes Solana transaction decisions
 */
export class SolanaDecider extends EventEmitter {
  /**
   * Create a new SolanaDecider
   *
   * @param {Object} [options] - Configuration options
   * @param {Object} [options.endpoints] - RPC endpoint config
   * @param {number} [options.maxRetries] - Max retry attempts
   */
  constructor(options = {}) {
    super();

    this._maxRetries = options.maxRetries || DECISION_THRESHOLDS.maxRetries;

    // RPC endpoints with health tracking
    this._endpoints = options.endpoints || [];
    this._endpointHealth = new Map();

    // Decision queue
    this._queue = [];
    this._maxQueueSize = 100;

    // Stats
    this._stats = {
      decisionsTotal: 0,
      byType: {},
      retriesTotal: 0,
      abortsTotal: 0,
      avgDecisionTime: 0,
      lastDecision: null,
    };

    // Initialize decision type counters
    for (const type of Object.values(SolanaDecisionType)) {
      this._stats.byType[type] = 0;
    }
  }

  /**
   * Decide what to do with a transaction
   *
   * @param {Object} tx - Transaction context
   * @param {Object} [context] - Additional context (network state, etc.)
   * @returns {Object} Decision result
   */
  decide(tx, context = {}) {
    const startTime = Date.now();

    // Extract relevant factors
    const factors = this._extractFactors(tx, context);

    // Calculate confidence
    const confidence = this._calculateConfidence(factors);

    // Make decision based on confidence and priority
    const decision = this._makeDecision(factors, confidence);

    // Build result
    const result = {
      type: decision.type,
      priority: factors.priority,
      confidence,
      cell: 'C2.3',
      dimension: 'SOLANA',
      analysis: 'DECIDE',
      reasoning: decision.reasoning,
      parameters: decision.parameters,
      timestamp: Date.now(),
      decisionTimeMs: Date.now() - startTime,
    };

    // Update stats
    this._updateStats(result);

    // Emit events
    this.emit('decision', result);
    globalEventBus.emit('solana:decision', result);

    log.debug('Solana decision', { type: result.type, confidence: confidence.toFixed(2) });

    return result;
  }

  /**
   * Decide on retry strategy
   *
   * @param {Object} failedTx - Failed transaction info
   * @param {number} attemptNumber - Current attempt number
   * @returns {Object} Retry decision
   */
  decideRetry(failedTx, attemptNumber) {
    const shouldRetry = attemptNumber < this._maxRetries;

    if (!shouldRetry) {
      return {
        type: SolanaDecisionType.ABORT,
        reason: `Max retries (${this._maxRetries}) exceeded`,
        attemptNumber,
      };
    }

    // Calculate backoff delay using φ
    const delay = Math.pow(DECISION_THRESHOLDS.retryBackoffBase, attemptNumber) * 1000;

    // Adjust fee if needed
    const feeMultiplier = 1 + (attemptNumber * 0.1);

    return {
      type: SolanaDecisionType.RETRY,
      delay: Math.round(delay),
      feeMultiplier,
      attemptNumber: attemptNumber + 1,
      maxAttempts: this._maxRetries,
    };
  }

  /**
   * Decide optimal fee
   *
   * @param {Object} options - Fee decision options
   * @returns {Object} Fee recommendation
   */
  decideFee(options = {}) {
    const { priority = PriorityLevel.NORMAL, networkCongestion = 0.5 } = options;

    let baseFee;
    switch (priority) {
      case PriorityLevel.CRITICAL:
        baseFee = DECISION_THRESHOLDS.maxFeeCritical;
        break;
      case PriorityLevel.HIGH:
        baseFee = DECISION_THRESHOLDS.maxFeeHigh;
        break;
      case PriorityLevel.LOW:
      case PriorityLevel.BATCH:
        baseFee = DECISION_THRESHOLDS.maxFeeNormal * PHI_INV_2;
        break;
      default:
        baseFee = DECISION_THRESHOLDS.maxFeeNormal;
    }

    // Adjust for congestion (φ-scaled)
    const congestionMultiplier = 1 + (networkCongestion * PHI_INV);
    const recommendedFee = Math.round(baseFee * congestionMultiplier);

    return {
      recommendedFee,
      priority,
      congestionMultiplier,
      maxFee: baseFee * 2,
    };
  }

  /**
   * Select best RPC endpoint
   *
   * @returns {Object|null} Selected endpoint or null
   */
  selectEndpoint() {
    if (this._endpoints.length === 0) {
      return null;
    }

    // Score endpoints by health
    let bestEndpoint = null;
    let bestScore = -1;

    for (const endpoint of this._endpoints) {
      const health = this._endpointHealth.get(endpoint.url) || { latency: 1000, errors: 0 };

      // Score: lower latency and fewer errors = better
      const score = 1000 / (health.latency + 1) - health.errors * 10;

      if (score > bestScore) {
        bestScore = score;
        bestEndpoint = endpoint;
      }
    }

    return bestEndpoint;
  }

  /**
   * Record endpoint health
   *
   * @param {string} url - Endpoint URL
   * @param {Object} metrics - Health metrics
   */
  recordEndpointHealth(url, metrics) {
    const existing = this._endpointHealth.get(url) || { latency: 1000, errors: 0, successes: 0 };

    // Rolling average for latency
    if (metrics.latency !== undefined) {
      existing.latency = (existing.latency * 0.9) + (metrics.latency * 0.1);
    }

    if (metrics.error) {
      existing.errors++;
    }

    if (metrics.success) {
      existing.successes++;
      // Decay error count on success
      existing.errors = Math.max(0, existing.errors - 0.1);
    }

    this._endpointHealth.set(url, existing);
  }

  /**
   * Extract decision factors
   * @private
   */
  _extractFactors(tx, context) {
    return {
      priority: tx.priority || PriorityLevel.NORMAL,
      value: tx.value || 0,
      deadline: tx.deadline || null,
      retryCount: tx.retryCount || 0,
      networkCongestion: context.congestion || 0.5,
      recentFailures: context.recentFailures || 0,
      queueLength: this._queue.length,
    };
  }

  /**
   * Calculate decision confidence
   * @private
   */
  _calculateConfidence(factors) {
    let confidence = PHI_INV; // Start at max

    // Reduce confidence for congested network
    if (factors.networkCongestion > 0.8) {
      confidence *= 0.7;
    } else if (factors.networkCongestion > 0.5) {
      confidence *= 0.9;
    }

    // Reduce confidence for many retries
    if (factors.retryCount > 3) {
      confidence *= 0.6;
    } else if (factors.retryCount > 0) {
      confidence *= 0.8;
    }

    // Reduce confidence for recent failures
    if (factors.recentFailures > 5) {
      confidence *= 0.5;
    }

    // Cap at φ⁻¹
    return Math.min(PHI_INV, Math.max(0, confidence));
  }

  /**
   * Make decision based on factors and confidence
   * @private
   */
  _makeDecision(factors, confidence) {
    // Critical priority always sends now
    if (factors.priority === PriorityLevel.CRITICAL) {
      return {
        type: SolanaDecisionType.SEND_NOW,
        reasoning: 'Critical priority - immediate send',
        parameters: { maxFee: DECISION_THRESHOLDS.maxFeeCritical },
      };
    }

    // High confidence + high/normal priority = send now
    if (confidence >= DECISION_THRESHOLDS.sendNowConfidence) {
      return {
        type: SolanaDecisionType.SEND_NOW,
        reasoning: `High confidence (${(confidence * 100).toFixed(1)}%) - sending now`,
        parameters: {},
      };
    }

    // Medium confidence = queue
    if (confidence >= DECISION_THRESHOLDS.queueConfidence) {
      return {
        type: SolanaDecisionType.QUEUE,
        reasoning: `Medium confidence (${(confidence * 100).toFixed(1)}%) - queueing`,
        parameters: { queuePosition: this._queue.length },
      };
    }

    // Low confidence with deadline = wait then retry
    if (factors.deadline) {
      const timeToDeadline = factors.deadline - Date.now();
      if (timeToDeadline > 60000) {
        return {
          type: SolanaDecisionType.WAIT,
          reasoning: 'Low confidence but deadline allows waiting',
          parameters: { waitMs: 30000 },
        };
      }
    }

    // Low priority = batch
    if (factors.priority === PriorityLevel.LOW || factors.priority === PriorityLevel.BATCH) {
      return {
        type: SolanaDecisionType.QUEUE,
        reasoning: 'Low priority - batching',
        parameters: { batchEligible: true },
      };
    }

    // Default: queue
    return {
      type: SolanaDecisionType.QUEUE,
      reasoning: 'Default decision - queueing for better conditions',
      parameters: {},
    };
  }

  /**
   * Update statistics
   * @private
   */
  _updateStats(result) {
    this._stats.decisionsTotal++;
    this._stats.byType[result.type] = (this._stats.byType[result.type] || 0) + 1;
    this._stats.lastDecision = Date.now();

    if (result.type === SolanaDecisionType.RETRY) {
      this._stats.retriesTotal++;
    }
    if (result.type === SolanaDecisionType.ABORT) {
      this._stats.abortsTotal++;
    }

    // Update rolling average decision time
    const n = this._stats.decisionsTotal;
    this._stats.avgDecisionTime = ((n - 1) * this._stats.avgDecisionTime + result.decisionTimeMs) / n;
  }

  /**
   * Get statistics
   *
   * @returns {Object} Statistics
   */
  getStats() {
    return { ...this._stats };
  }

  /**
   * Get health assessment
   *
   * @returns {Object} Health assessment
   */
  getHealth() {
    const queueHealth = this._queue.length < this._maxQueueSize * PHI_INV ? 'healthy' : 'congested';

    return {
      status: queueHealth,
      score: queueHealth === 'healthy' ? PHI_INV : PHI_INV_2,
      queueLength: this._queue.length,
      maxQueueSize: this._maxQueueSize,
      decisionsTotal: this._stats.decisionsTotal,
      avgDecisionTimeMs: this._stats.avgDecisionTime,
      endpointsTracked: this._endpointHealth.size,
    };
  }

  /**
   * Clear state
   */
  clear() {
    this._queue = [];
    this._endpointHealth.clear();
    this._stats = {
      decisionsTotal: 0,
      byType: {},
      retriesTotal: 0,
      abortsTotal: 0,
      avgDecisionTime: 0,
      lastDecision: null,
    };
    for (const type of Object.values(SolanaDecisionType)) {
      this._stats.byType[type] = 0;
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get or create SolanaDecider singleton
 *
 * @param {Object} [options] - Options (only used on first call)
 * @returns {SolanaDecider}
 */
export function getSolanaDecider(options = {}) {
  if (!_instance) {
    _instance = new SolanaDecider(options);
  }
  return _instance;
}

/**
 * Reset singleton (for testing)
 */
export function resetSolanaDecider() {
  if (_instance) {
    _instance.removeAllListeners();
  }
  _instance = null;
}

export default {
  SolanaDecider,
  SolanaDecisionType,
  PriorityLevel,
  getSolanaDecider,
  resetSolanaDecider,
};
