/**
 * CYNIC Event Bus - Centralized Event System
 *
 * A publish-subscribe event bus for CYNIC's automation layer.
 * Enables loose coupling between components while maintaining
 * the φ principle of simplicity.
 *
 * Events:
 * - feedback:received - When feedback is submitted
 * - judgment:created - When a judgment is made
 * - session:start - When a session begins
 * - session:end - When a session ends
 * - learning:cycle - When a learning cycle completes
 * - trigger:fired - When a trigger activates
 * - automation:tick - Periodic automation heartbeat
 *
 * "φ connects, φ observes" - κυνικός
 *
 * @module @cynic/node/services/event-bus
 */

'use strict';

import { EventEmitter } from 'events';
import { createLogger, PHI_INV } from '@cynic/core';

const log = createLogger('EventBus');

/**
 * CYNIC Event Types
 * @readonly
 * @enum {string}
 */
export const EventType = {
  // Feedback events
  FEEDBACK_RECEIVED: 'feedback:received',
  FEEDBACK_PROCESSED: 'feedback:processed', // @unused — 0 publishers, 0 subscribers

  // Judgment events
  JUDGMENT_CREATED: 'judgment:created',
  JUDGMENT_REFINED: 'judgment:refined', // @unused — 0 publishers, 0 subscribers

  // Session events (subscribers in AutomationExecutor, but published on CORE bus not here)
  SESSION_START: 'session:start',
  SESSION_END: 'session:end',

  // Learning events
  LEARNING_CYCLE_START: 'learning:cycle:start',
  LEARNING_CYCLE_COMPLETE: 'learning:cycle:complete',
  LEARNING_PATTERN_EVOLVED: 'learning:pattern:evolved', // @unused — 0 publishers, 0 subscribers

  // Trigger events
  TRIGGER_FIRED: 'trigger:fired',
  TRIGGER_EVALUATED: 'trigger:evaluated',

  // Automation events
  AUTOMATION_TICK: 'automation:tick',
  AUTOMATION_START: 'automation:start',
  AUTOMATION_STOP: 'automation:stop',

  // Goal events
  GOAL_CREATED: 'goal:created', // @unused — GoalsRepo uses MCP tools, not bus events
  GOAL_PROGRESS: 'goal:progress', // @unused — GoalsRepo uses MCP tools, not bus events
  GOAL_COMPLETED: 'goal:completed',

  // Notification events
  NOTIFICATION_CREATED: 'notification:created', // @unused — NotificationsRepo uses MCP tools, not bus events
  NOTIFICATION_DELIVERED: 'notification:delivered', // @unused — NotificationsRepo uses MCP tools, not bus events

  // Error events
  ERROR: 'error', // @unused — errors flow through ErrorHandler → ConsciousnessBridge, not bus

  // Hook events (from Claude Code hooks → internal system)
  HOOK_PRE_TOOL: 'hook:pre_tool',
  HOOK_POST_TOOL: 'hook:post_tool',
  HOOK_SESSION_START: 'hook:session_start', // @unused — hooks use core globalEventBus, not automation bus
  HOOK_SESSION_END: 'hook:session_end', // @unused — hooks use core globalEventBus, not automation bus
};

/**
 * Event metadata for tracking
 * @typedef {Object} EventMeta
 * @property {string} eventId - Unique event ID
 * @property {number} timestamp - Event timestamp
 * @property {string} source - Event source (component name)
 * @property {string} [userId] - Optional user ID
 * @property {string} [sessionId] - Optional session ID
 */

/**
 * Generate a short event ID
 * @returns {string}
 */
function generateEventId() {
  return 'evt_' + Date.now().toString(36) + Math.random().toString(36).slice(2, 8);
}

/**
 * CYNIC Event Bus
 *
 * Centralized pub-sub for automation events.
 * Supports wildcards, priorities, and event history.
 */
export class EventBus extends EventEmitter {
  /**
   * Create the event bus
   * @param {Object} [options] - Options
   * @param {number} [options.maxHistory=100] - Max events to keep in history
   * @param {boolean} [options.debug=false] - Enable debug logging
   */
  constructor(options = {}) {
    super();

    this.maxHistory = options.maxHistory ?? 100;
    this.debug = options.debug ?? false;

    // Event history (circular buffer)
    this._history = [];
    this._historyIndex = 0;

    // Subscriber counts by event
    this._subscriberCounts = new Map();

    // Statistics
    this._stats = {
      eventsEmitted: 0,
      eventsFailed: 0,
      subscribersActive: 0,
    };

    // Set high max listeners (many components may subscribe)
    this.setMaxListeners(50);

    log.debug('Event bus initialized');
  }

  /**
   * Emit an event with metadata
   *
   * @param {string} eventType - Event type from EventType enum
   * @param {Object} data - Event data
   * @param {Object} [meta] - Optional metadata override
   * @returns {boolean} True if event had listeners
   */
  publish(eventType, data, meta = {}) {
    const eventMeta = {
      eventId: generateEventId(),
      timestamp: Date.now(),
      source: meta.source || 'unknown',
      userId: meta.userId,
      sessionId: meta.sessionId,
      ...meta,
    };

    const event = {
      type: eventType,
      data,
      meta: eventMeta,
    };

    // Add to history
    this._addToHistory(event);

    // Log if debug
    if (this.debug) {
      log.debug('Event published', { type: eventType, eventId: eventMeta.eventId });
    }

    // Emit to specific listeners
    const hadListeners = this.emit(eventType, event);

    // Also emit to wildcard listeners
    if (eventType.includes(':')) {
      const category = eventType.split(':')[0];
      this.emit(`${category}:*`, event);
    }

    // Always emit to catch-all
    this.emit('*', event);

    this._stats.eventsEmitted++;

    return hadListeners;
  }

  /**
   * Subscribe to an event
   *
   * @param {string} eventType - Event type or pattern (e.g., 'feedback:*')
   * @param {Function} handler - Event handler (event) => void
   * @returns {Function} Unsubscribe function
   */
  subscribe(eventType, handler) {
    this.on(eventType, handler);

    // Track subscriber count
    const count = this._subscriberCounts.get(eventType) || 0;
    this._subscriberCounts.set(eventType, count + 1);
    this._stats.subscribersActive++;

    // Return unsubscribe function
    return () => {
      this.off(eventType, handler);
      const newCount = (this._subscriberCounts.get(eventType) || 1) - 1;
      this._subscriberCounts.set(eventType, newCount);
      this._stats.subscribersActive--;
    };
  }

  /**
   * Subscribe to an event once
   *
   * @param {string} eventType - Event type
   * @param {Function} handler - Event handler
   * @returns {Function} Unsubscribe function
   */
  subscribeOnce(eventType, handler) {
    const unsubscribe = this.subscribe(eventType, (event) => {
      unsubscribe();
      handler(event);
    });
    return unsubscribe;
  }

  /**
   * Wait for an event (promise-based)
   *
   * @param {string} eventType - Event type to wait for
   * @param {number} [timeout=30000] - Timeout in ms (φ × 50000 ≈ 30000)
   * @returns {Promise<Object>} The event
   */
  waitFor(eventType, timeout = 30000) {
    return new Promise((resolve, reject) => {
      const timeoutId = setTimeout(() => {
        unsubscribe();
        reject(new Error(`Timeout waiting for event: ${eventType}`));
      }, timeout);

      const unsubscribe = this.subscribeOnce(eventType, (event) => {
        clearTimeout(timeoutId);
        resolve(event);
      });
    });
  }

  /**
   * Get event history
   *
   * @param {Object} [filter] - Filter options
   * @param {string} [filter.type] - Filter by event type
   * @param {string} [filter.source] - Filter by source
   * @param {number} [filter.since] - Filter by timestamp
   * @param {number} [filter.limit] - Max events to return
   * @returns {Object[]} Matching events
   */
  getHistory(filter = {}) {
    let events = [...this._history].filter(Boolean);

    if (filter.type) {
      events = events.filter((e) => e.type === filter.type);
    }
    if (filter.source) {
      events = events.filter((e) => e.meta.source === filter.source);
    }
    if (filter.since) {
      events = events.filter((e) => e.meta.timestamp >= filter.since);
    }

    // Sort by timestamp descending
    events.sort((a, b) => b.meta.timestamp - a.meta.timestamp);

    if (filter.limit) {
      events = events.slice(0, filter.limit);
    }

    return events;
  }

  /**
   * Get statistics
   * @returns {Object} Stats
   */
  getStats() {
    return {
      ...this._stats,
      historySize: this._history.filter(Boolean).length,
      eventTypes: Array.from(this._subscriberCounts.entries())
        .filter(([_, count]) => count > 0)
        .map(([type, count]) => ({ type, subscribers: count })),
    };
  }

  /**
   * Clear event history
   */
  clearHistory() {
    this._history = [];
    this._historyIndex = 0;
  }

  /**
   * Add event to circular history buffer
   * @private
   */
  _addToHistory(event) {
    this._history[this._historyIndex] = event;
    this._historyIndex = (this._historyIndex + 1) % this.maxHistory;
  }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SINGLETON INSTANCE
// ═══════════════════════════════════════════════════════════════════════════════

let _globalBus = null;

/**
 * Get the global event bus instance
 * @returns {EventBus}
 */
export function getEventBus() {
  if (!_globalBus) {
    _globalBus = new EventBus();
  }
  return _globalBus;
}

/**
 * Create a new event bus instance
 * @param {Object} [options] - Options
 * @returns {EventBus}
 */
export function createEventBus(options) {
  return new EventBus(options);
}

/**
 * Convenience: Publish to global bus
 * @param {string} eventType - Event type
 * @param {Object} data - Event data
 * @param {Object} [meta] - Optional metadata
 */
export function publish(eventType, data, meta) {
  return getEventBus().publish(eventType, data, meta);
}

/**
 * Convenience: Subscribe to global bus
 * @param {string} eventType - Event type
 * @param {Function} handler - Handler
 * @returns {Function} Unsubscribe function
 */
export function subscribe(eventType, handler) {
  return getEventBus().subscribe(eventType, handler);
}

export default EventBus;
