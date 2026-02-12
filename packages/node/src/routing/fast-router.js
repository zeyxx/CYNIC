/**
 * CYNIC Fast Router - A1 (Reflex Arc)
 *
 * High-frequency event router that bypasses UnifiedOrchestrator.
 * Provides <100ms response for critical perception events.
 *
 * "Le chien réagit avant de penser" - κυνικός
 *
 * Design:
 * - Perception events tagged with priority: 'critical' or 'normal'
 * - FastRouter handles 'critical' → Direct action (no dogs, no synthesis)
 * - UnifiedOrchestrator handles 'normal' → Full deliberation
 *
 * Use Cases:
 * - Price alerts (>φ threshold) → Immediate notification
 * - Solana tx anomalies → Instant block
 * - High-frequency market data → Fast aggregation
 *
 * @module @cynic/node/routing/fast-router
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2, PHI_INV_3, createLogger, globalEventBus, EventType } from '@cynic/core';

const log = createLogger('FastRouter');

/**
 * Action types for reflex responses
 */
export const ReflexActionType = {
  NOTIFY: 'notify',           // Send notification
  BLOCK: 'block',             // Block immediately
  AGGREGATE: 'aggregate',     // Aggregate data
  ESCALATE: 'escalate',       // Escalate to full orchestration
  LOG: 'log',                 // Just log (observation)
};

/**
 * Priority thresholds (φ-aligned)
 */
const PRIORITY_THRESHOLDS = {
  CRITICAL: PHI_INV,          // 61.8% - immediate action needed
  HIGH: PHI_INV_2,            // 38.2% - fast response
  NORMAL: PHI_INV_3,          // 23.6% - can wait
};

/**
 * FastRouter - Reflex arc for high-frequency events
 *
 * Bypasses full orchestration for critical events.
 * Provides <100ms response time.
 */
export class FastRouter extends EventEmitter {
  /**
   * @param {Object} options
   * @param {Object} [options.guardian] - Guardian for blocking actions
   * @param {Object} [options.notifier] - Notifier for alerts
   * @param {number} [options.maxLatency] - Max acceptable latency (ms)
   */
  constructor(options = {}) {
    super();

    this.guardian = options.guardian || null;
    this.notifier = options.notifier || null;
    this.maxLatency = options.maxLatency || 100; // 100ms default

    // Stats tracking
    this.stats = {
      totalEvents: 0,
      criticalEvents: 0,
      reflexActions: 0,
      escalations: 0,
      avgLatency: 0,
      maxLatencyViolations: 0,
      byEventType: {},
      byAction: {},
    };

    // Event history (ring buffer, F(11) = 89)
    this._history = [];
    this._maxHistory = 89;

    // Aggregation buffers (for high-frequency data)
    this._aggregationBuffers = new Map();
    this._aggregationTimers = new Map();

    // Wire event listeners
    this._wireEvents();

    log.info('FastRouter initialized', { maxLatency: this.maxLatency });
  }

  /**
   * Wire event listeners for critical perception events
   * @private
   */
  _wireEvents() {
    // A1: Market price alerts (critical)
    this._marketAlertSub = globalEventBus.subscribe(
      'perception:market:price_alert',
      (event) => this._handleEvent(event, 'market_alert')
    );

    // A1: Market price updates (high-frequency aggregation)
    this._marketPriceSub = globalEventBus.subscribe(
      'perception:market:price',
      (event) => this._handleEvent(event, 'market_price')
    );

    // A1: Solana tx anomalies (critical)
    this._solanaTxSub = globalEventBus.subscribe(
      'perception:solana:tx_anomaly',
      (event) => this._handleEvent(event, 'solana_anomaly')
    );

    // A1: Solana slot updates (high-frequency aggregation)
    this._solanaSlotSub = globalEventBus.subscribe(
      'perception:solana:slot',
      (event) => this._handleEvent(event, 'solana_slot')
    );

    log.debug('FastRouter event listeners wired');
  }

  /**
   * Handle incoming event
   * @private
   */
  _handleEvent(event, eventType) {
    const startTime = Date.now();

    try {
      this.stats.totalEvents++;
      this.stats.byEventType[eventType] = (this.stats.byEventType[eventType] || 0) + 1;

      const priority = this._assessPriority(event, eventType);

      if (priority === 'critical') {
        this.stats.criticalEvents++;
        const action = this._reflexAction(event, eventType);

        if (action) {
          this.stats.reflexActions++;
          this.stats.byAction[action.type] = (this.stats.byAction[action.type] || 0) + 1;

          // Record in history
          this._recordHistory({
            eventType,
            priority,
            action: action.type,
            latency: Date.now() - startTime,
            timestamp: Date.now(),
          });

          // Emit action event
          this.emit('reflex_action', action);
          globalEventBus.publish('fast_router:action', action, { source: 'fast-router' });

          log.debug('Reflex action executed', {
            eventType,
            action: action.type,
            latency: Date.now() - startTime,
          });
        }
      } else if (priority === 'high') {
        // High priority: aggregate or escalate
        this._handleHighPriority(event, eventType);
      } else {
        // Normal priority: let orchestrator handle it (no action from FastRouter)
        log.debug('Normal priority event, skipping fast path', { eventType });
      }

      // Track latency
      const latency = Date.now() - startTime;
      this._updateLatency(latency);

      if (latency > this.maxLatency) {
        this.stats.maxLatencyViolations++;
        log.warn('FastRouter latency exceeded', { latency, max: this.maxLatency, eventType });
      }
    } catch (err) {
      log.error('FastRouter event handling failed', { error: err.message, eventType });
    }
  }

  /**
   * Assess event priority
   * @private
   */
  _assessPriority(event, eventType) {
    const payload = event.payload || event;

    // Market price alerts: critical if severity high or extreme price change
    if (eventType === 'market_alert') {
      if (payload.severity === 'high' || payload.severity === 'critical') {
        return 'critical';
      }
      if (Math.abs(payload.priceChangePercent) >= PRIORITY_THRESHOLDS.CRITICAL * 100) {
        return 'critical';
      }
      return 'high';
    }

    // Solana tx anomalies: critical if flagged
    if (eventType === 'solana_anomaly') {
      if (payload.risk === 'critical' || payload.anomalyScore > PRIORITY_THRESHOLDS.CRITICAL) {
        return 'critical';
      }
      return 'high';
    }

    // High-frequency updates: aggregate
    if (eventType === 'market_price' || eventType === 'solana_slot') {
      return 'high';
    }

    return 'normal';
  }

  /**
   * Execute reflex action for critical event
   * @private
   */
  _reflexAction(event, eventType) {
    const payload = event.payload || event;

    // Market alert → Notify
    if (eventType === 'market_alert') {
      return {
        type: ReflexActionType.NOTIFY,
        message: `*ALERT* ${payload.direction === 'up' ? '📈' : '📉'} $asdfasdfa ${payload.priceChangePercent.toFixed(1)}%`,
        severity: payload.severity,
        data: payload,
        timestamp: Date.now(),
      };
    }

    // Solana anomaly → Block or escalate
    if (eventType === 'solana_anomaly') {
      if (payload.risk === 'critical') {
        return {
          type: ReflexActionType.BLOCK,
          target: payload.signature || payload.address,
          reason: payload.reason || 'Anomaly detected',
          data: payload,
          timestamp: Date.now(),
        };
      }
      return {
        type: ReflexActionType.ESCALATE,
        reason: 'Solana anomaly requires investigation',
        data: payload,
        timestamp: Date.now(),
      };
    }

    return null;
  }

  /**
   * Handle high-priority events (aggregation or escalation)
   * @private
   */
  _handleHighPriority(event, eventType) {
    // For high-frequency events, aggregate into buffers
    if (eventType === 'market_price' || eventType === 'solana_slot') {
      this._aggregateEvent(event, eventType);
    } else {
      // Escalate to orchestrator
      this.stats.escalations++;
      globalEventBus.publish('fast_router:escalate', {
        eventType,
        event,
        reason: 'High priority requires deliberation',
      }, { source: 'fast-router' });
    }
  }

  /**
   * Aggregate high-frequency events
   * @private
   */
  _aggregateEvent(event, eventType) {
    const buffer = this._aggregationBuffers.get(eventType) || [];
    buffer.push(event);
    this._aggregationBuffers.set(eventType, buffer);

    // Set aggregation timer (F(5) = 5 seconds)
    if (!this._aggregationTimers.has(eventType)) {
      const timer = setTimeout(() => {
        this._flushAggregation(eventType);
      }, 5000);
      timer.unref();
      this._aggregationTimers.set(eventType, timer);
    }
  }

  /**
   * Flush aggregation buffer
   * @private
   */
  _flushAggregation(eventType) {
    const buffer = this._aggregationBuffers.get(eventType);
    if (!buffer || buffer.length === 0) return;

    // Emit aggregated event
    const aggregated = {
      type: ReflexActionType.AGGREGATE,
      eventType,
      count: buffer.length,
      data: this._computeAggregateStats(buffer, eventType),
      timestamp: Date.now(),
    };

    this.emit('aggregation', aggregated);
    globalEventBus.publish('fast_router:aggregation', aggregated, { source: 'fast-router' });

    log.debug('Aggregation flushed', { eventType, count: buffer.length });

    // Clear buffer
    this._aggregationBuffers.delete(eventType);
    this._aggregationTimers.delete(eventType);
  }

  /**
   * Compute aggregate statistics
   * @private
   */
  _computeAggregateStats(buffer, eventType) {
    if (eventType === 'market_price') {
      const prices = buffer.map(e => (e.payload || e).price);
      return {
        min: Math.min(...prices),
        max: Math.max(...prices),
        avg: prices.reduce((a, b) => a + b, 0) / prices.length,
        count: prices.length,
      };
    }

    if (eventType === 'solana_slot') {
      const slots = buffer.map(e => (e.payload || e).slot);
      return {
        min: Math.min(...slots),
        max: Math.max(...slots),
        count: slots.length,
        throughput: slots.length / 5, // slots per second (over 5s window)
      };
    }

    return { count: buffer.length };
  }

  /**
   * Record event in history
   * @private
   */
  _recordHistory(entry) {
    this._history.push(entry);
    while (this._history.length > this._maxHistory) {
      this._history.shift();
    }
  }

  /**
   * Update average latency
   * @private
   */
  _updateLatency(latency) {
    const n = this.stats.totalEvents;
    this.stats.avgLatency = ((n - 1) * this.stats.avgLatency + latency) / n;
  }

  /**
   * Get router statistics
   */
  getStats() {
    return {
      ...this.stats,
      coverage: this.stats.totalEvents > 0
        ? (this.stats.criticalEvents / this.stats.totalEvents) * 100
        : 0,
      successRate: this.stats.totalEvents > 0
        ? ((this.stats.totalEvents - this.stats.maxLatencyViolations) / this.stats.totalEvents) * 100
        : 0,
    };
  }

  /**
   * Get recent history
   */
  getHistory(limit = 21) {
    return this._history.slice(-limit);
  }

  /**
   * Get health status
   */
  getHealth() {
    const successRate = this.stats.totalEvents > 0
      ? ((this.stats.totalEvents - this.stats.maxLatencyViolations) / this.stats.totalEvents)
      : 1;

    return {
      status: successRate >= PHI_INV ? 'healthy' : 'degraded',
      score: Math.min(PHI_INV, successRate),
      avgLatency: this.stats.avgLatency,
      maxLatency: this.maxLatency,
      violations: this.stats.maxLatencyViolations,
      coverage: this.stats.coverage || 0,
    };
  }

  /**
   * Cleanup and stop
   */
  stop() {
    // Unsubscribe from events
    if (this._marketAlertSub) this._marketAlertSub();
    if (this._marketPriceSub) this._marketPriceSub();
    if (this._solanaTxSub) this._solanaTxSub();
    if (this._solanaSlotSub) this._solanaSlotSub();

    // Clear aggregation timers
    for (const timer of this._aggregationTimers.values()) {
      clearTimeout(timer);
    }
    this._aggregationTimers.clear();
    this._aggregationBuffers.clear();

    log.info('FastRouter stopped', { stats: this.getStats() });
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════════

let _instance = null;

export function getFastRouter(options = {}) {
  if (!_instance) _instance = new FastRouter(options);
  return _instance;
}

export function resetFastRouter() {
  if (_instance) {
    _instance.stop();
    _instance.removeAllListeners();
    _instance = null;
  }
}

export default { FastRouter, ReflexActionType, getFastRouter, resetFastRouter };
