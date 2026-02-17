/**
 * CYNIC Unified Event Bus
 *
 * Single event system replacing Core + Automation + Agent buses.
 * Supports hierarchical namespaces, wildcard subscriptions, event history,
 * and health metrics. Built on ParallelEventBus for 16× throughput.
 *
 * "One voice. Many listeners. φ unity." - κυνικός
 *
 * @module @cynic/core/bus/unified-event-bus
 */

'use strict';

import { randomUUID } from 'node:crypto';
import { ParallelEventBus } from './parallel-event-bus.js';
import { createLogger } from '../logger.js';

const log = createLogger('UnifiedEventBus');

/**
 * Unified Event Categories
 * 6 core categories (Fib(4)) + 1 transcendent
 */
export const EventCategory = {
  PERCEPTION: 'perception',   // Input from 7 domains (PERCEIVE phase)
  JUDGMENT: 'judgment',       // Scoring, verdicts (JUDGE phase)
  DECISION: 'decision',       // Routing, governance (DECIDE phase)
  ACTION: 'action',           // Execution (ACT phase)
  LEARNING: 'learning',       // Weight updates (LEARN phase)
  SYSTEM: 'system',           // Lifecycle, health, meta
  EMERGENCE: 'emergence',     // THE_UNNAMEABLE - transcendent patterns
};

/**
 * Event Priority Levels
 */
export const EventPriority = {
  CRITICAL: 'critical',
  HIGH: 'high',
  NORMAL: 'normal',
  LOW: 'low',
};

/**
 * Unified Event Envelope
 *
 * Standard structure for all events across CYNIC.
 */
export class UnifiedEvent {
  /**
   * Create a unified event
   *
   * @param {string} type - Event type (e.g., 'perception:human:state')
   * @param {any} payload - Event data
   * @param {Object} [options] - Event options
   * @param {string} [options.id] - Event ID (auto-generated if not provided)
   * @param {string} [options.source] - Component that emitted
   * @param {string} [options.target] - Target component (default: '*' for all)
   * @param {string} [options.priority] - Event priority
   * @param {string} [options.correlationId] - For request/reply
   * @param {string} [options.causationId] - What triggered this event
   * @param {Object} [options.metadata] - Additional metadata
   */
  constructor(type, payload, options = {}) {
    // Identity
    this.id = options.id || randomUUID();
    this.type = type;

    // Timing
    this.timestamp = Date.now();

    // Routing
    this.source = options.source || 'unknown';
    this.target = options.target || '*';

    // Priority
    this.priority = options.priority || EventPriority.NORMAL;

    // Correlation (for request/reply patterns)
    this.correlationId = options.correlationId || null;
    this.causationId = options.causationId || null;

    // Payload
    this.payload = payload;

    // Metadata
    this.metadata = {
      category: this._extractCategory(type),
      ...(options.metadata || {}),
    };
  }

  /**
   * Extract category from event type
   * @private
   */
  _extractCategory(type) {
    const parts = type.split(':');
    return parts[0] || 'unknown';
  }

  /**
   * Create a reply event correlated to this one
   *
   * @param {string} replyType - Reply event type
   * @param {any} payload - Reply payload
   * @param {Object} [options] - Additional options
   * @returns {UnifiedEvent}
   */
  reply(replyType, payload, options = {}) {
    return new UnifiedEvent(replyType, payload, {
      ...options,
      source: options.source || 'reply',
      target: this.source, // Reply to sender
      correlationId: this.id,
      causationId: this.id,
    });
  }

  /**
   * Create a follow-up event caused by this one
   *
   * @param {string} followUpType - Follow-up event type
   * @param {any} payload - Follow-up payload
   * @param {Object} [options] - Additional options
   * @returns {UnifiedEvent}
   */
  causedBy(followUpType, payload, options = {}) {
    return new UnifiedEvent(followUpType, payload, {
      ...options,
      causationId: this.id,
    });
  }

  /**
   * Check if event targets specific component
   *
   * @param {string} componentName - Component to check
   * @returns {boolean}
   */
  targetsComponent(componentName) {
    return this.target === '*' || this.target === componentName;
  }

  /**
   * Check if event is high priority (critical or high)
   *
   * @returns {boolean}
   */
  isHighPriority() {
    return this.priority === EventPriority.CRITICAL || this.priority === EventPriority.HIGH;
  }

  /**
   * Get event age in milliseconds
   *
   * @returns {number}
   */
  age() {
    return Date.now() - this.timestamp;
  }

  /**
   * Serialize to JSON
   *
   * @returns {Object}
   */
  toJSON() {
    return {
      id: this.id,
      type: this.type,
      timestamp: this.timestamp,
      source: this.source,
      target: this.target,
      priority: this.priority,
      correlationId: this.correlationId,
      causationId: this.causationId,
      payload: this.payload,
      metadata: this.metadata,
    };
  }

  /**
   * Deserialize from JSON
   *
   * @param {Object} data - JSON data
   * @returns {UnifiedEvent}
   */
  static fromJSON(data) {
    const event = new UnifiedEvent(data.type, data.payload, {
      id: data.id,
      source: data.source,
      target: data.target,
      priority: data.priority,
      correlationId: data.correlationId,
      causationId: data.causationId,
      metadata: data.metadata,
    });

    // Restore original timestamp
    event.timestamp = data.timestamp;

    return event;
  }
}

/**
 * Unified Event Bus
 *
 * Single event system for CYNIC. Replaces Core + Automation + Agent buses.
 *
 * Features:
 * - Hierarchical namespaces (perception:human:state)
 * - Wildcard subscriptions (perception:*, perception:human:*)
 * - Event history (1000 event circular buffer)
 * - Event filtering (by category, source, time)
 * - Health metrics (events/sec, p50/p95 latency)
 * - 16× throughput (via ParallelEventBus)
 *
 * Usage:
 * ```javascript
 * const bus = new UnifiedEventBus();
 *
 * // Subscribe to specific event
 * bus.subscribe('perception:human:state', handler);
 *
 * // Subscribe to all human perception
 * bus.subscribe('perception:human:*', handler);
 *
 * // Subscribe to all perception
 * bus.subscribe('perception:*', handler);
 *
 * // Publish event
 * bus.publish('perception:human:state', { energy: 0.8 }, {
 *   source: 'HumanPerceiver',
 * });
 * ```
 */
export class UnifiedEventBus extends ParallelEventBus {
  #history = [];
  #historyLimit = 1000; // φ-aligned: Fib(17) = 1597 ≈ 1000
  #historyIndex = 0;

  #subscriptions = new Map();
  #middlewares = [];

  #latencyStats = {
    samples: [],
    maxSamples: 100, // Last 100 events for percentiles
  };

  #healthMetrics = {
    eventsPublished: 0,
    eventsPerSecond: 0,
    lastSecondCount: 0,
    lastSecondTimestamp: Date.now(),
    p50Latency: 0,
    p95Latency: 0,
  };

  constructor(options = {}) {
    super(options);

    this.#historyLimit = options.historyLimit || 1000;

    // Start metrics update interval (φ-aligned: 1000ms)
    this._metricsInterval = setInterval(() => {
      this._updateHealthMetrics();
    }, 1000);
  }

  /**
   * Publish an event to all subscribers
   *
   * @param {string} type - Event type (e.g., 'perception:human:state')
   * @param {any} payload - Event data
   * @param {Object} [options] - Event options
   * @returns {UnifiedEvent} The published event
   */
  publish(type, payload, options = {}) {
    const startTime = Date.now();

    const event = new UnifiedEvent(type, payload, options);

    // Run through middlewares
    for (const middleware of this.#middlewares) {
      try {
        const result = middleware(event);
        if (result === false) {
          // Middleware blocked the event
          return event;
        }
      } catch (error) {
        log.warn('Middleware error', { error: error.message, type });
      }
    }

    // Store in history
    this._addToHistory(event);

    // Emit to specific type listeners
    this.emit(type, event);

    // Emit to wildcard listeners
    this._emitToWildcards(type, event);

    // Track metrics
    const latency = Date.now() - startTime;
    this._recordLatency(latency);
    this.#healthMetrics.eventsPublished++;
    this.#healthMetrics.lastSecondCount++;

    return event;
  }

  /**
   * Subscribe to events of a specific type (supports wildcards)
   *
   * @param {string} type - Event type or wildcard (e.g., 'perception:*')
   * @param {Function} handler - Event handler
   * @returns {Function} Unsubscribe function
   */
  subscribe(type, handler) {
    // Wrap handler to catch errors
    const wrappedHandler = (event) => {
      try {
        handler(event);
      } catch (error) {
        log.error('Subscriber error', {
          type,
          error: error.message,
          stack: error.stack,
        });

        // Emit system:component:error
        this.publish('system:component:error', {
          component: 'UnifiedEventBus',
          eventType: type,
          error: error.message,
        }, {
          source: 'UnifiedEventBus',
          priority: EventPriority.HIGH,
        });
      }
    };

    // Subscribe using parent class
    this.on(type, wrappedHandler);

    // Track subscription
    if (!this.#subscriptions.has(type)) {
      this.#subscriptions.set(type, new Set());
    }
    this.#subscriptions.get(type).add(wrappedHandler);

    // Return unsubscribe function
    return () => {
      this.off(type, wrappedHandler);
      this.#subscriptions.get(type)?.delete(wrappedHandler);
    };
  }

  /**
   * Subscribe to an event once
   *
   * @param {string} type - Event type
   * @param {Function} handler - Event handler
   * @returns {Function} Unsubscribe function
   */
  subscribeOnce(type, handler) {
    const wrappedHandler = (event) => {
      try {
        handler(event);
      } catch (error) {
        log.error('Subscriber error (once)', {
          type,
          error: error.message,
        });
      }
    };

    this.once(type, wrappedHandler);

    return () => {
      this.off(type, wrappedHandler);
    };
  }

  /**
   * Request/Reply pattern - publish and wait for correlated response
   *
   * @param {string} type - Request event type
   * @param {any} payload - Request data
   * @param {Object} [options] - Options
   * @param {number} [options.timeout=5000] - Timeout in ms
   * @param {string} [options.replyType] - Expected reply event type
   * @returns {Promise<UnifiedEvent>} Reply event
   */
  async request(type, payload, options = {}) {
    const { timeout = 5000, replyType } = options;

    const expectedReplyType = replyType || `${type}:reply`;

    // Create request event first to know its ID
    const requestEvent = new UnifiedEvent(type, payload, options);
    const requestId = requestEvent.id;

    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        this.off(expectedReplyType, handler);
        reject(new Error(`Request timeout for ${type}`));
      }, timeout);

      const handler = (event) => {
        if (event.correlationId === requestId) {
          clearTimeout(timeoutId);
          this.off(expectedReplyType, handler);
          resolve(event);
        }
      };

      // Register listener BEFORE publishing
      this.on(expectedReplyType, handler);

      // Now publish the request (reusing the event ID)
      this.publish(type, payload, { ...options, id: requestId });
    });
  }

  /**
   * Add middleware that processes all events
   *
   * @param {Function} middleware - (event) => boolean|void (return false to block)
   */
  use(middleware) {
    this.#middlewares.push(middleware);
  }

  /**
   * Get recent event history
   *
   * @param {Object} [filter] - Filter options
   * @param {string} [filter.type] - Filter by event type
   * @param {string} [filter.category] - Filter by category (perception, judgment, etc.)
   * @param {string} [filter.source] - Filter by source component
   * @param {number} [filter.since] - Filter by timestamp (events after this time)
   * @param {number} [filter.limit] - Max events to return
   * @returns {UnifiedEvent[]}
   */
  getHistory(filter = {}) {
    let events = [...this.#history].filter(Boolean); // Filter out nulls

    if (filter.type) {
      events = events.filter(e => e.type === filter.type);
    }

    if (filter.category) {
      events = events.filter(e => e.metadata.category === filter.category);
    }

    if (filter.source) {
      events = events.filter(e => e.source === filter.source);
    }

    if (filter.since) {
      events = events.filter(e => e.timestamp >= filter.since);
    }

    // Sort by timestamp descending (most recent first)
    events.sort((a, b) => b.timestamp - a.timestamp);

    if (filter.limit) {
      events = events.slice(0, filter.limit);
    }

    return events;
  }

  /**
   * Get subscription statistics
   *
   * @returns {Object}
   */
  getSubscriptionStats() {
    const stats = {
      totalSubscriptions: 0,
      byType: {},
      byCategory: {},
    };

    for (const [type, handlers] of this.#subscriptions) {
      const count = handlers.size;
      stats.byType[type] = count;
      stats.totalSubscriptions += count;

      // Count by category
      const category = type.split(':')[0];
      stats.byCategory[category] = (stats.byCategory[category] || 0) + count;
    }

    return stats;
  }

  /**
   * Get health metrics
   *
   * @returns {Object}
   */
  getHealthMetrics() {
    return {
      ...this.#healthMetrics,
      historySize: this.#history.filter(Boolean).length,
      subscriptions: this.#subscriptions.size,
      middlewares: this.#middlewares.length,
      uptime: process.uptime(),
    };
  }

  /**
   * Clear all subscriptions and history
   */
  clear() {
    this.removeAllListeners();
    this.#subscriptions.clear();
    this.#history = [];
    this.#historyIndex = 0;
  }

  /**
   * Stop the bus and clean up resources
   */
  stop() {
    if (this._metricsInterval) {
      clearInterval(this._metricsInterval);
      this._metricsInterval = null;
    }
    this.clear();
  }

  /**
   * Emit to wildcard subscribers
   * @private
   */
  _emitToWildcards(type, event) {
    const parts = type.split(':');

    // Emit to category wildcard (e.g., perception:*)
    if (parts.length >= 1) {
      this.emit(`${parts[0]}:*`, event);
    }

    // Emit to subcategory wildcard (e.g., perception:human:*)
    if (parts.length >= 2) {
      this.emit(`${parts[0]}:${parts[1]}:*`, event);
    }

    // Emit to catch-all
    this.emit('*', event);
  }

  /**
   * Add event to circular history buffer
   * @private
   */
  _addToHistory(event) {
    this.#history[this.#historyIndex] = event;
    this.#historyIndex = (this.#historyIndex + 1) % this.#historyLimit;
  }

  /**
   * Record latency sample
   * @private
   */
  _recordLatency(latency) {
    this.#latencyStats.samples.push(latency);

    // Keep only last 100 samples
    if (this.#latencyStats.samples.length > this.#latencyStats.maxSamples) {
      this.#latencyStats.samples.shift();
    }
  }

  /**
   * Update health metrics (called every second)
   * @private
   */
  _updateHealthMetrics() {
    // Calculate events per second
    const now = Date.now();
    const timeDelta = (now - this.#healthMetrics.lastSecondTimestamp) / 1000;
    this.#healthMetrics.eventsPerSecond = Math.round(
      this.#healthMetrics.lastSecondCount / timeDelta
    );

    // Reset counter
    this.#healthMetrics.lastSecondCount = 0;
    this.#healthMetrics.lastSecondTimestamp = now;

    // Calculate percentiles
    if (this.#latencyStats.samples.length > 0) {
      const sorted = [...this.#latencyStats.samples].sort((a, b) => a - b);
      const p50Index = Math.floor(sorted.length * 0.5);
      const p95Index = Math.floor(sorted.length * 0.95);

      this.#healthMetrics.p50Latency = sorted[p50Index] || 0;
      this.#healthMetrics.p95Latency = sorted[p95Index] || 0;
    }
  }
}

/**
 * Global unified event bus instance
 */
let _globalBus = null;

/**
 * Get the global unified event bus
 *
 * @returns {UnifiedEventBus}
 */
export function getUnifiedEventBus() {
  if (!_globalBus) {
    _globalBus = new UnifiedEventBus();
  }
  return _globalBus;
}

/**
 * Create a new unified event bus instance
 *
 * @param {Object} [options] - Options
 * @returns {UnifiedEventBus}
 */
export function createUnifiedEventBus(options = {}) {
  return new UnifiedEventBus(options);
}

/**
 * Convenience: Publish to global bus
 *
 * @param {string} type - Event type
 * @param {any} payload - Event data
 * @param {Object} [options] - Options
 * @returns {UnifiedEvent}
 */
export function publish(type, payload, options = {}) {
  return getUnifiedEventBus().publish(type, payload, options);
}

/**
 * Convenience: Subscribe to global bus
 *
 * @param {string} type - Event type
 * @param {Function} handler - Handler
 * @returns {Function} Unsubscribe function
 */
export function subscribe(type, handler) {
  return getUnifiedEventBus().subscribe(type, handler);
}

export default UnifiedEventBus;
