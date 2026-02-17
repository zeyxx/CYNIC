/**
 * CYNIC Event Bus
 *
 * Unified event system for inter-layer communication.
 * All components can emit and subscribe to typed events.
 *
 * "The pack communicates as one" - κυνικός
 *
 * @module @cynic/core/bus/event-bus
 */

'use strict';

import { EventEmitter } from 'node:events';
import { randomUUID } from 'node:crypto';
import { createLogger } from '../logger.js';

const log = createLogger('EventBus');

/**
 * Standard CYNIC event types
 */
export const EventType = {
  // Lifecycle events
  COMPONENT_READY: 'component:ready',
  COMPONENT_STOPPED: 'component:stopped',
  COMPONENT_ERROR: 'component:error',

  // Judgment events
  JUDGMENT_CREATED: 'judgment:created',

  // Pattern events
  PATTERN_DETECTED: 'pattern:detected',
  PATTERN_LEARNED: 'pattern:learned',
  ANOMALY_DETECTED: 'anomaly:detected',
  DIMENSION_CANDIDATE: 'dimension:candidate',
  CONSCIOUSNESS_CHANGED: 'consciousness:changed',

  // User events
  USER_ACTION: 'user:action',
  USER_FEEDBACK: 'user:feedback',
  SESSION_STARTED: 'session:started',
  SESSION_ENDED: 'session:ended',

  // Tool events (from hooks)
  TOOL_CALLED: 'tool:called',
  TOOL_COMPLETED: 'tool:completed',

  // Dog collective events
  CYNIC_STATE: 'cynic:state',
  DOG_EVENT: 'dog:event',
  CONSENSUS_COMPLETED: 'consensus:completed',

  // Social events (SOCIAL dimension)
  SOCIAL_CAPTURE: 'social:capture',
  SOCIAL_JUDGMENT: 'social:judgment',

  // Market events (MARKET dimension - C3.x)
  MARKET_PRICE_UPDATED: 'market:price:updated',
  MARKET_VOLUME_UPDATED: 'market:volume:updated',
  MARKET_LIQUIDITY_UPDATED: 'market:liquidity:updated',

  // Cynic self-judgment events (C6.2)
  CYNIC_JUDGMENT: 'cynic:judgment',

  // Cynic emergence events (C6.7)
  CYNIC_EMERGENCE: 'cynic:emergence',

  // Network/P2P events (PHASE 2: DECENTRALIZE)
  NODE_STARTED: 'node:started',
  NODE_STOPPED: 'node:stopped',
  BLOCK_FINALIZED: 'block:finalized',
  BLOCK_PROPOSED: 'block:proposed',
  BLOCK_ANCHORED: 'block:anchored',
  METRICS_REPORTED: 'metrics:reported',
  PEER_CONNECTED: 'peer:connected',
  PEER_DISCONNECTED: 'peer:disconnected',
  SYNC_COMPLETED: 'sync:completed',

  // Learning events
  EWC_CONSOLIDATION_COMPLETED: 'ewc:consolidation:completed',
  CALIBRATION_DRIFT_DETECTED: 'calibration:drift:detected',
  QLEARNING_WEIGHT_UPDATE: 'qlearning:weight:update',

  TD_ERROR_UPDATE: 'td_error:update',
  QLEARNING_CONVERGED: 'qlearning:converged',
  QLEARNING_DRIFT: 'qlearning:drift',
  // Orchestration events
  ORCHESTRATION_COMPLETED: 'orchestration:completed',

  // Decision events (RIGHT side — DECIDE/ACT/ACCOUNT)
  CODE_DECISION: 'code:decision',
  CODE_ACTION: 'code:action',
  HUMAN_ACTION: 'human:action',
  ACCOUNTING_UPDATE: 'accounting:update',

  // Topology events (self-awareness)
  TOPOLOGY_CHANGED: 'topology:changed',

  // Cost accounting events (token velocity)
  COST_UPDATE: 'cost:update',
};

/**
 * Event envelope with metadata
 */
export class CYNICEvent {
  constructor(type, payload, options = {}) {
    this.id = options.id || randomUUID();
    this.type = type;
    this.payload = payload;
    this.source = options.source || 'unknown';
    this.timestamp = options.timestamp || Date.now();
    this.correlationId = options.correlationId || null;
    this.metadata = options.metadata || {};
  }

  /**
   * Create a reply event correlated to this one
   */
  reply(type, payload, options = {}) {
    return new CYNICEvent(type, payload, {
      ...options,
      correlationId: this.id,
      source: options.source,
    });
  }

  toJSON() {
    return {
      id: this.id,
      type: this.type,
      payload: this.payload,
      source: this.source,
      timestamp: this.timestamp,
      correlationId: this.correlationId,
      metadata: this.metadata,
    };
  }
}

/**
 * CYNIC Event Bus
 *
 * Central hub for all inter-component communication.
 * Supports typed events, wildcards, and request/reply patterns.
 */
export class CYNICEventBus extends EventEmitter {
  #history = [];
  #historyLimit = 1000;
  #subscriptions = new Map();
  #middlewares = [];

  constructor(options = {}) {
    super();
    this.#historyLimit = options.historyLimit || 1000;
    this.setMaxListeners(100); // Allow many listeners
  }

  /**
   * Publish an event to all subscribers
   *
   * @param {string} type - Event type
   * @param {any} payload - Event data
   * @param {Object} [options] - Event options
   * @returns {CYNICEvent} The published event
   */
  publish(type, payload, options = {}) {
    const event = new CYNICEvent(type, payload, options);

    // Run through middlewares
    for (const middleware of this.#middlewares) {
      try {
        const result = middleware(event);
        if (result === false) {
          return event; // Middleware blocked the event
        }
      } catch (error) {
        log.error('Middleware error', { error: error.message });
      }
    }

    // Store in history
    this.#addToHistory(event);

    // Emit to specific type listeners
    this.emit(type, event);

    // Emit to wildcard listeners
    const namespace = type.split(':')[0];
    this.emit(`${namespace}:*`, event);
    this.emit('*', event);

    return event;
  }

  /**
   * Subscribe to events of a specific type
   *
   * @param {string} type - Event type (supports wildcards like "judgment:*")
   * @param {Function} handler - Event handler
   * @returns {Function} Unsubscribe function
   */
  subscribe(type, handler) {
    const wrappedHandler = (event) => {
      try {
        handler(event);
      } catch (error) {
        log.error('Handler error', { type, error: error.message });
        this.publish(EventType.COMPONENT_ERROR, {
          eventType: type,
          error: error.message,
        });
      }
    };

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
   */
  subscribeOnce(type, handler) {
    const wrappedHandler = (event) => {
      try {
        handler(event);
      } catch (error) {
        log.error('Handler error', { type, error: error.message });
      }
    };

    this.once(type, wrappedHandler);
  }

  /**
   * Request/Reply pattern - publish and wait for correlated response
   *
   * @param {string} type - Request event type
   * @param {any} payload - Request data
   * @param {Object} [options] - Options
   * @param {number} [options.timeout=5000] - Timeout in ms
   * @param {string} [options.replyType] - Expected reply event type
   * @returns {Promise<CYNICEvent>} Reply event
   */
  async request(type, payload, options = {}) {
    const { timeout = 5000, replyType } = options;

    const expectedReplyType = replyType || `${type}:reply`;

    // Create request event first to know its ID
    const requestEvent = new CYNICEvent(type, {}, options);
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

      // Register listener BEFORE publishing to catch synchronous replies
      this.on(expectedReplyType, handler);

      // Now publish the actual request (reusing the event ID)
      this.publish(type, payload, { ...options, id: requestId });
    });
  }

  /**
   * Add middleware that processes all events
   *
   * @param {Function} middleware - (event) => boolean|void
   */
  use(middleware) {
    this.#middlewares.push(middleware);
  }

  /**
   * Get recent event history
   *
   * @param {Object} [filter] - Filter options
   * @returns {CYNICEvent[]}
   */
  getHistory(filter = {}) {
    let events = [...this.#history];

    if (filter.type) {
      events = events.filter(e => e.type === filter.type);
    }

    if (filter.source) {
      events = events.filter(e => e.source === filter.source);
    }

    if (filter.since) {
      events = events.filter(e => e.timestamp >= filter.since);
    }

    if (filter.limit) {
      events = events.slice(-filter.limit);
    }

    return events;
  }

  /**
   * Get subscription statistics
   */
  getStats() {
    const stats = {
      totalSubscriptions: 0,
      byType: {},
      historySize: this.#history.length,
      middlewareCount: this.#middlewares.length,
    };

    for (const [type, handlers] of this.#subscriptions) {
      stats.byType[type] = handlers.size;
      stats.totalSubscriptions += handlers.size;
    }

    return stats;
  }

  /**
   * Clear all subscriptions and history
   */
  clear() {
    this.removeAllListeners();
    this.#subscriptions.clear();
    this.#history = [];
  }

  /**
   * Add event to history with size limit
   * @private
   */
  #addToHistory(event) {
    this.#history.push(event);
    if (this.#history.length > this.#historyLimit) {
      this.#history.shift();
    }
  }
}

/**
 * Global event bus instance
 */
export const globalEventBus = new CYNICEventBus();

/**
 * Convenience function to publish to global bus
 */
export function publish(type, payload, options = {}) {
  return globalEventBus.publish(type, payload, options);
}

/**
 * Convenience function to subscribe to global bus
 */
export function subscribe(type, handler) {
  return globalEventBus.subscribe(type, handler);
}
