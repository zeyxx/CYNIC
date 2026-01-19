/**
 * CYNIC Auto-Judgment Triggers
 *
 * "φ distrusts φ" - Automatic vigilance
 *
 * Trigger system for automatic judgments:
 * 1. Event-based triggers (on commit, error, decision)
 * 2. Time-based triggers (periodic review)
 * 3. Pattern-based triggers (anomaly detection)
 * 4. Threshold triggers (score changes)
 *
 * @module @cynic/core/triggers
 * @philosophy The watchdog that never sleeps
 */

'use strict';

import { PHI_INV, PHI_INV_2, THRESHOLDS } from '../axioms/constants.js';

// =============================================================================
// CONSTANTS
// =============================================================================

export const TRIGGER_CONSTANTS = {
  /** Max active triggers (Fib(8) = 21) */
  MAX_TRIGGERS: 21,

  /** Default debounce time in ms (Fib(6) = 8 * 1000 = 8s) */
  DEFAULT_DEBOUNCE_MS: 8000,

  /** Default cooldown time in ms (Fib(9) = 34 * 1000 = 34s) */
  DEFAULT_COOLDOWN_MS: 34000,

  /** Max triggers per minute (Fib(5) = 5) */
  MAX_TRIGGERS_PER_MINUTE: 5,

  /** Pattern detection window (Fib(7) = 13) */
  PATTERN_WINDOW: 13,
};

/**
 * Trigger types
 */
export const TriggerType = {
  EVENT: 'event',           // On specific events
  PERIODIC: 'periodic',     // Time-based
  PATTERN: 'pattern',       // Anomaly detection
  THRESHOLD: 'threshold',   // Score-based
  COMPOSITE: 'composite',   // Multiple conditions
};

/**
 * Event types that can trigger judgments
 */
export const TriggerEvent = {
  // Code events
  COMMIT: 'commit',
  PUSH: 'push',
  MERGE: 'merge',
  CODE_CHANGE: 'code_change',

  // Tool events
  TOOL_USE: 'tool_use',
  TOOL_ERROR: 'tool_error',

  // Session events
  SESSION_START: 'session_start',
  SESSION_END: 'session_end',

  // Decision events
  DECISION: 'decision',
  ARCHITECTURE_DECISION: 'architecture_decision',

  // Error events
  ERROR: 'error',
  EXCEPTION: 'exception',
  FAILURE: 'failure',

  // Custom
  CUSTOM: 'custom',
};

/**
 * Trigger actions
 */
export const TriggerAction = {
  JUDGE: 'judge',           // Run judgment
  LOG: 'log',               // Log only
  ALERT: 'alert',           // Send alert
  BLOCK: 'block',           // Block action
  REVIEW: 'review',         // Queue for review
  NOTIFY: 'notify',         // Send notification
};

// =============================================================================
// TRIGGER DEFINITION
// =============================================================================

/**
 * Trigger configuration
 */
export class Trigger {
  /**
   * @param {Object} options
   * @param {string} options.id - Unique trigger ID
   * @param {string} options.name - Human-readable name
   * @param {string} options.type - TriggerType
   * @param {Object} options.condition - Trigger condition
   * @param {string} options.action - TriggerAction
   * @param {Object} [options.config] - Additional configuration
   */
  constructor(options) {
    this.id = options.id || `trg_${Date.now().toString(36)}`;
    this.name = options.name;
    this.type = options.type || TriggerType.EVENT;
    this.condition = options.condition || {};
    this.action = options.action || TriggerAction.JUDGE;
    this.config = {
      enabled: true,
      debounceMs: TRIGGER_CONSTANTS.DEFAULT_DEBOUNCE_MS,
      cooldownMs: TRIGGER_CONSTANTS.DEFAULT_COOLDOWN_MS,
      priority: 'normal',
      ...options.config,
    };

    // Runtime state
    this.lastTriggered = null;
    this.triggerCount = 0;
    this.lastResult = null;
  }

  /**
   * Check if trigger is on cooldown
   */
  isOnCooldown() {
    if (!this.lastTriggered) return false;
    return Date.now() - this.lastTriggered < this.config.cooldownMs;
  }

  /**
   * Check if event matches trigger condition
   * @param {Object} event - Event data
   * @returns {boolean}
   */
  matches(event) {
    if (!this.config.enabled) return false;
    if (this.isOnCooldown()) return false;

    switch (this.type) {
      case TriggerType.EVENT:
        return this._matchesEvent(event);
      case TriggerType.THRESHOLD:
        return this._matchesThreshold(event);
      case TriggerType.PATTERN:
        return this._matchesPattern(event);
      case TriggerType.COMPOSITE:
        return this._matchesComposite(event);
      default:
        return false;
    }
  }

  /**
   * Check event-based condition
   * @private
   */
  _matchesEvent(event) {
    const { eventType, filter } = this.condition;

    // Check event type
    if (eventType && event.type !== eventType) {
      return false;
    }

    // Apply filters
    if (filter) {
      for (const [key, value] of Object.entries(filter)) {
        if (typeof value === 'function') {
          if (!value(event[key])) return false;
        } else if (value instanceof RegExp) {
          if (!value.test(event[key])) return false;
        } else if (event[key] !== value) {
          return false;
        }
      }
    }

    return true;
  }

  /**
   * Check threshold-based condition
   * @private
   */
  _matchesThreshold(event) {
    const { field, operator, value } = this.condition;
    const eventValue = event[field];

    if (eventValue === undefined) return false;

    switch (operator) {
      case 'gt': return eventValue > value;
      case 'gte': return eventValue >= value;
      case 'lt': return eventValue < value;
      case 'lte': return eventValue <= value;
      case 'eq': return eventValue === value;
      case 'neq': return eventValue !== value;
      case 'between':
        return eventValue >= value[0] && eventValue <= value[1];
      default:
        return false;
    }
  }

  /**
   * Check pattern-based condition
   * @private
   */
  _matchesPattern(event) {
    const { pattern, minOccurrences = 3 } = this.condition;

    // Pattern matching requires context from trigger manager
    // This is a placeholder - actual implementation in TriggerManager
    return event._patternMatches >= minOccurrences;
  }

  /**
   * Check composite condition (AND/OR)
   * @private
   */
  _matchesComposite(event) {
    const { operator = 'AND', conditions } = this.condition;

    if (!Array.isArray(conditions)) return false;

    if (operator === 'AND') {
      return conditions.every(cond => this._checkSubCondition(cond, event));
    } else {
      return conditions.some(cond => this._checkSubCondition(cond, event));
    }
  }

  /**
   * Check a sub-condition
   * @private
   */
  _checkSubCondition(condition, event) {
    const tempTrigger = new Trigger({
      type: condition.type || TriggerType.EVENT,
      condition,
    });
    return tempTrigger.matches(event);
  }

  /**
   * Mark as triggered
   */
  markTriggered(result = null) {
    this.lastTriggered = Date.now();
    this.triggerCount++;
    this.lastResult = result;
  }

  /**
   * To JSON
   */
  toJSON() {
    return {
      id: this.id,
      name: this.name,
      type: this.type,
      condition: this.condition,
      action: this.action,
      config: this.config,
      stats: {
        lastTriggered: this.lastTriggered,
        triggerCount: this.triggerCount,
        onCooldown: this.isOnCooldown(),
      },
    };
  }
}

// =============================================================================
// TRIGGER MANAGER
// =============================================================================

/**
 * Manages triggers and event processing
 */
export class TriggerManager {
  /**
   * @param {Object} options
   * @param {Function} [options.judgeCallback] - Callback to execute judgments
   * @param {Function} [options.alertCallback] - Callback for alerts
   * @param {Function} [options.logCallback] - Callback for logging
   */
  constructor(options = {}) {
    /** @type {Map<string, Trigger>} */
    this.triggers = new Map();

    /** Callbacks */
    this.judgeCallback = options.judgeCallback || null;
    this.alertCallback = options.alertCallback || null;
    this.logCallback = options.logCallback || console.log;

    /** Event history for pattern detection */
    this.eventHistory = [];

    /** Rate limiting */
    this.triggerTimestamps = [];

    /** Statistics */
    this.stats = {
      eventsProcessed: 0,
      triggersActivated: 0,
      judgmentsGenerated: 0,
      alertsSent: 0,
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // TRIGGER MANAGEMENT
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Register a trigger
   * @param {Object|Trigger} triggerConfig
   * @returns {Trigger}
   */
  register(triggerConfig) {
    if (this.triggers.size >= TRIGGER_CONSTANTS.MAX_TRIGGERS) {
      throw new Error(`Max triggers (${TRIGGER_CONSTANTS.MAX_TRIGGERS}) reached`);
    }

    const trigger = triggerConfig instanceof Trigger
      ? triggerConfig
      : new Trigger(triggerConfig);

    this.triggers.set(trigger.id, trigger);
    return trigger;
  }

  /**
   * Unregister a trigger
   * @param {string} triggerId
   */
  unregister(triggerId) {
    return this.triggers.delete(triggerId);
  }

  /**
   * Get trigger by ID
   * @param {string} triggerId
   */
  getTrigger(triggerId) {
    return this.triggers.get(triggerId);
  }

  /**
   * List all triggers
   */
  listTriggers() {
    return Array.from(this.triggers.values()).map(t => t.toJSON());
  }

  /**
   * Enable/disable a trigger
   */
  setEnabled(triggerId, enabled) {
    const trigger = this.triggers.get(triggerId);
    if (trigger) {
      trigger.config.enabled = enabled;
      return true;
    }
    return false;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // EVENT PROCESSING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Process an event and check triggers
   * @param {Object} event - Event data with at least { type }
   * @returns {Promise<Array<Object>>} Results from triggered actions
   */
  async processEvent(event) {
    this.stats.eventsProcessed++;

    // Add to history for pattern detection
    this._addToHistory(event);

    // Rate limiting check
    if (this._isRateLimited()) {
      return [{ skipped: true, reason: 'rate_limited' }];
    }

    // Enrich event with pattern data
    const enrichedEvent = this._enrichWithPatterns(event);

    // Find matching triggers
    const matchingTriggers = [];
    for (const trigger of this.triggers.values()) {
      if (trigger.matches(enrichedEvent)) {
        matchingTriggers.push(trigger);
      }
    }

    // Sort by priority
    matchingTriggers.sort((a, b) => {
      const priorityOrder = { critical: 0, high: 1, normal: 2, low: 3 };
      return (priorityOrder[a.config.priority] || 2) - (priorityOrder[b.config.priority] || 2);
    });

    // Execute actions
    const results = [];
    for (const trigger of matchingTriggers) {
      const result = await this._executeAction(trigger, enrichedEvent);
      results.push(result);
      trigger.markTriggered(result);
      this.stats.triggersActivated++;
      this._recordTriggerTimestamp();
    }

    return results;
  }

  /**
   * Add event to history
   * @private
   */
  _addToHistory(event) {
    this.eventHistory.push({
      ...event,
      _timestamp: Date.now(),
    });

    // Trim history
    while (this.eventHistory.length > 100) {
      this.eventHistory.shift();
    }
  }

  /**
   * Enrich event with pattern detection data
   * @private
   */
  _enrichWithPatterns(event) {
    // Count similar events in recent history
    const window = TRIGGER_CONSTANTS.PATTERN_WINDOW;
    const recent = this.eventHistory.slice(-window);

    let patternMatches = 0;
    for (const hist of recent) {
      if (hist.type === event.type) {
        patternMatches++;
      }
    }

    return {
      ...event,
      _patternMatches: patternMatches,
      _recentHistory: recent,
    };
  }

  /**
   * Check rate limiting
   * @private
   */
  _isRateLimited() {
    const now = Date.now();
    const oneMinuteAgo = now - 60000;

    // Clean old timestamps
    this.triggerTimestamps = this.triggerTimestamps.filter(t => t > oneMinuteAgo);

    return this.triggerTimestamps.length >= TRIGGER_CONSTANTS.MAX_TRIGGERS_PER_MINUTE;
  }

  /**
   * Record trigger timestamp for rate limiting
   * @private
   */
  _recordTriggerTimestamp() {
    this.triggerTimestamps.push(Date.now());
  }

  /**
   * Execute trigger action
   * @private
   */
  async _executeAction(trigger, event) {
    const result = {
      triggerId: trigger.id,
      triggerName: trigger.name,
      action: trigger.action,
      event: { type: event.type },
      timestamp: Date.now(),
    };

    try {
      switch (trigger.action) {
        case TriggerAction.JUDGE:
          if (this.judgeCallback) {
            result.judgment = await this.judgeCallback(this._buildJudgmentInput(trigger, event));
            this.stats.judgmentsGenerated++;
          } else {
            result.judgment = { skipped: true, reason: 'no_judge_callback' };
          }
          break;

        case TriggerAction.LOG:
          this.logCallback(`[Trigger:${trigger.name}] ${event.type}`, event);
          result.logged = true;
          break;

        case TriggerAction.ALERT:
          if (this.alertCallback) {
            await this.alertCallback({
              trigger: trigger.name,
              event,
              severity: trigger.config.priority,
            });
            this.stats.alertsSent++;
          }
          result.alerted = true;
          break;

        case TriggerAction.BLOCK:
          result.blocked = true;
          result.reason = trigger.condition.blockReason || 'Trigger condition met';
          break;

        case TriggerAction.REVIEW:
          result.queuedForReview = true;
          break;

        case TriggerAction.NOTIFY:
          result.notification = {
            title: trigger.name,
            body: `Event ${event.type} triggered`,
          };
          break;

        default:
          result.error = `Unknown action: ${trigger.action}`;
      }
    } catch (error) {
      result.error = error.message;
    }

    return result;
  }

  /**
   * Build judgment input from trigger and event
   * @private
   */
  _buildJudgmentInput(trigger, event) {
    return {
      type: trigger.condition.judgmentType || event.type,
      content: event.data || event.content || event,
      context: {
        triggeredBy: trigger.name,
        eventType: event.type,
        timestamp: Date.now(),
        ...event.context,
      },
    };
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // PREDEFINED TRIGGERS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Register common predefined triggers
   */
  registerDefaults() {
    // Error trigger
    this.register({
      id: 'trg_error',
      name: 'Error Handler',
      type: TriggerType.EVENT,
      condition: {
        eventType: TriggerEvent.ERROR,
      },
      action: TriggerAction.JUDGE,
      config: { priority: 'high' },
    });

    // Commit trigger
    this.register({
      id: 'trg_commit',
      name: 'Commit Review',
      type: TriggerType.EVENT,
      condition: {
        eventType: TriggerEvent.COMMIT,
      },
      action: TriggerAction.JUDGE,
      config: { priority: 'normal' },
    });

    // Decision trigger
    this.register({
      id: 'trg_decision',
      name: 'Decision Evaluator',
      type: TriggerType.EVENT,
      condition: {
        eventType: TriggerEvent.DECISION,
      },
      action: TriggerAction.JUDGE,
      config: { priority: 'normal' },
    });

    // Repeated error pattern
    this.register({
      id: 'trg_error_pattern',
      name: 'Repeated Error Alert',
      type: TriggerType.PATTERN,
      condition: {
        eventType: TriggerEvent.ERROR,
        minOccurrences: 3,
      },
      action: TriggerAction.ALERT,
      config: { priority: 'high' },
    });

    // Low score threshold
    this.register({
      id: 'trg_low_score',
      name: 'Low Score Alert',
      type: TriggerType.THRESHOLD,
      condition: {
        field: 'qScore',
        operator: 'lt',
        value: THRESHOLDS.GROWL,
      },
      action: TriggerAction.ALERT,
      config: { priority: 'high' },
    });

    return this;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // STATUS & EXPORT
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Get manager status
   */
  getStatus() {
    return {
      triggers: this.listTriggers(),
      stats: this.stats,
      rateLimitStatus: {
        triggersLastMinute: this.triggerTimestamps.length,
        limit: TRIGGER_CONSTANTS.MAX_TRIGGERS_PER_MINUTE,
      },
      eventHistorySize: this.eventHistory.length,
    };
  }

  /**
   * Export state
   */
  export() {
    return {
      triggers: Array.from(this.triggers.entries()).map(([id, t]) => [id, t.toJSON()]),
      stats: this.stats,
    };
  }

  /**
   * Import state
   */
  import(state) {
    if (state.triggers) {
      for (const [id, config] of state.triggers) {
        this.triggers.set(id, new Trigger(config));
      }
    }
    if (state.stats) {
      this.stats = state.stats;
    }
    return this;
  }

  /**
   * Reset manager
   */
  reset() {
    this.triggers.clear();
    this.eventHistory = [];
    this.triggerTimestamps = [];
    this.stats = {
      eventsProcessed: 0,
      triggersActivated: 0,
      judgmentsGenerated: 0,
      alertsSent: 0,
    };
  }
}

// =============================================================================
// EXPORTS
// =============================================================================

export default {
  // Constants
  TRIGGER_CONSTANTS,
  TriggerType,
  TriggerEvent,
  TriggerAction,

  // Classes
  Trigger,
  TriggerManager,
};
