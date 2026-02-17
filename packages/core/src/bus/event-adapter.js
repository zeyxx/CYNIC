/**
 * CYNIC Event Adapter
 *
 * Migration layer that bridges old event buses (Core, Automation, Agent)
 * to the new UnifiedEventBus. Allows incremental migration with zero breakage.
 *
 * Strategy:
 * - Old events → translated → published to unified bus
 * - Unified events → translated → published to old buses (for old listeners)
 * - Both systems work simultaneously during migration
 *
 * "The bridge between past and future" - κυνικός
 *
 * @module @cynic/core/bus/event-adapter
 */

'use strict';

import { createLogger } from '../logger.js';
import { UnifiedEvent, EventPriority } from './unified-event-bus.js';

const log = createLogger('EventAdapter');

/**
 * Event translation mappings: Old event type → Unified event type
 *
 * Maps events from the 3 old buses to unified taxonomy.
 * Events not in this map are passed through unchanged (for custom events).
 */
const EVENT_TRANSLATION_MAP = {
  // ═══════════════════════════════════════════════════════════════════════════
  // CORE BUS → UNIFIED
  // ═══════════════════════════════════════════════════════════════════════════

  // Lifecycle
  'component:ready': 'system:component:ready',
  'component:stopped': 'system:component:stopped',
  'component:error': 'system:component:error',

  // Judgment
  'judgment:created': 'judgment:created',

  // Patterns
  'pattern:detected': 'emergence:pattern:detected',
  'pattern:learned': 'learning:pattern:learned',
  'anomaly:detected': 'emergence:pattern:detected',
  'dimension:candidate': 'judgment:dimension-candidate',
  'consciousness:changed': 'system:consciousness:changed',

  // User
  'user:action': 'perception:user:action',
  'user:feedback': 'learning:feedback:received',
  'session:started': 'perception:session:start',
  'session:ended': 'perception:session:end',

  // Tool
  'tool:called': 'perception:human:tool-use',
  'tool:completed': 'perception:human:tool-use',

  // Dog collective
  'cynic:state': 'perception:cynic:health',
  'dog:event': 'perception:agent:pattern',
  'consensus:completed': 'decision:consensus:completed',

  // Social
  'social:capture': 'perception:social:capture',
  'social:judgment': 'judgment:social',

  // Market
  'market:price:updated': 'perception:market:price',
  'market:volume:updated': 'perception:market:volume',
  'market:liquidity:updated': 'perception:market:liquidity',

  // Cynic
  'cynic:judgment': 'judgment:cynic',

  // Network
  'node:started': 'system:node:started',
  'node:stopped': 'system:node:stopped',
  'block:finalized': 'perception:solana:block',
  'block:proposed': 'perception:solana:block',
  'block:anchored': 'perception:solana:block',
  'metrics:reported': 'system:metrics:reported',
  'peer:connected': 'perception:cosmos:peer',
  'peer:disconnected': 'perception:cosmos:peer',
  'sync:completed': 'perception:cosmos:sync',

  // Learning
  'ewc:consolidation:completed': 'learning:ewc:consolidation',
  'calibration:drift:detected': 'judgment:calibration-drift',
  'qlearning:weight:update': 'learning:qlearning:weight-update',
  'td_error:update': 'learning:td-error:update',
  'qlearning:converged': 'learning:qlearning:converged',
  'qlearning:drift': 'learning:qlearning:drift',

  // Orchestration
  'orchestration:completed': 'system:orchestration:completed',

  // Decision
  'code:decision': 'decision:code',
  'code:action': 'action:code',
  'human:action': 'action:human',

  // Accounting
  'accounting:update': 'system:cost:update',
  'cost:update': 'system:cost:update',

  // Topology
  'topology:changed': 'system:topology:changed',

  // ═══════════════════════════════════════════════════════════════════════════
  // AUTOMATION BUS → UNIFIED
  // ═══════════════════════════════════════════════════════════════════════════

  'feedback:received': 'learning:feedback:received',
  'feedback:processed': 'learning:feedback:processed',
  'judgment:refined': 'judgment:refined',
  'session:start': 'perception:session:start',
  'session:end': 'perception:session:end',
  'learning:cycle:start': 'learning:cycle:start',
  'learning:cycle:complete': 'learning:cycle:complete',
  'learning:pattern:evolved': 'learning:pattern:evolved',
  'trigger:fired': 'action:trigger:fired',
  'trigger:evaluated': 'action:trigger:fired',
  'automation:tick': 'system:automation:tick',
  'automation:start': 'system:automation:start',
  'automation:stop': 'system:automation:stop',
  'goal:created': 'system:goal:created',
  'goal:progress': 'system:goal:progress',
  'goal:completed': 'action:goal:completed',
  'notification:created': 'system:notification:created',
  'notification:delivered': 'system:notification:delivered',
  'error': 'system:component:error',
  'hook:pre_tool': 'perception:human:tool-use',
  'hook:post_tool': 'perception:human:tool-use',
  'hook:session_start': 'perception:session:start',
  'hook:session_end': 'perception:session:end',

  // ═══════════════════════════════════════════════════════════════════════════
  // AGENT BUS → UNIFIED
  // ═══════════════════════════════════════════════════════════════════════════

  'agent:pattern:detected': 'perception:agent:pattern',
  'agent:anomaly:detected': 'perception:agent:anomaly',
  'agent:threat:blocked': 'perception:agent:threat',
  'agent:knowledge:extracted': 'learning:agent:knowledge',
  'agent:wisdom:shared': 'learning:agent:wisdom',
  'agent:consensus:request': 'decision:consensus:request',
  'agent:consensus:response': 'decision:consensus:response',
  'agent:profile:updated': 'system:agent:profile-updated',
  'cynic:decision': 'decision:cynic:decision',
  'cynic:override': 'decision:cynic:override',
  'cynic:guidance': 'emergence:cynic:guidance',
  'cynic:awakening': 'system:cynic:awakening',
  'cynic:introspection': 'system:cynic:introspection',
  'agent:introspection:response': 'system:agent:introspection-response',
  'agent:quality:report': 'judgment:agent:quality-report',
  'agent:autofix:applied': 'action:agent:autofix',
  'agent:deadcode:detected': 'judgment:agent:dead-code',
  'agent:discovery:found': 'emergence:agent:discovery',
  'agent:vulnerability:detected': 'judgment:agent:vulnerability',
  'agent:map:updated': 'emergence:map-updated',
  'agent:reality:drift': 'emergence:reality-drift',
  'agent:viz:generated': 'emergence:agent:visualization',
  'agent:prediction:made': 'emergence:agent:prediction',
  'agent:deploy:started': 'action:deploy:started',
  'agent:deploy:completed': 'action:deploy:completed',
  'agent:deploy:failed': 'action:deploy:failed',
  'hook:session:start': 'perception:session:start',
  'hook:prompt:submit': 'perception:human:tool-use',
  'hook:tool:pre': 'perception:human:tool-use',
  'hook:tool:post': 'perception:human:tool-use',
  'hook:session:stop': 'perception:session:end',
  'hook:pattern:detected': 'perception:agent:pattern',
};

/**
 * Reverse translation map: Unified event type → Old event type
 *
 * For translating unified events back to old format for legacy listeners.
 * Maps unified → first old event that maps to it (1:1 reverse mapping).
 */
const REVERSE_TRANSLATION_MAP = {};

// Build reverse map (unified → old)
for (const [oldType, unifiedType] of Object.entries(EVENT_TRANSLATION_MAP)) {
  if (!REVERSE_TRANSLATION_MAP[unifiedType]) {
    REVERSE_TRANSLATION_MAP[unifiedType] = oldType;
  }
}

/**
 * Event Adapter
 *
 * Bridges old event buses to unified event bus during migration.
 * Allows both systems to work simultaneously.
 *
 * Usage:
 * ```javascript
 * import { globalEventBus } from '@cynic/core'; // Old bus
 * import { getUnifiedEventBus } from '@cynic/core'; // New bus
 * import { getEventBus } from '@cynic/node/services/event-bus'; // Automation bus
 * import { AgentEventBus } from '@cynic/node/agents/event-bus'; // Agent bus
 *
 * const adapter = new EventAdapter({
 *   unifiedBus: getUnifiedEventBus(),
 *   oldBuses: {
 *     core: globalEventBus,
 *     automation: getEventBus(),
 *     agent: AgentEventBus.getInstance(),
 *   },
 * });
 *
 * adapter.start(); // Start bidirectional routing
 * ```
 */
export class EventAdapter {
  #unifiedBus = null;
  #oldBuses = {};
  #unsubscribers = [];
  #stats = {
    oldToUnified: 0,
    unifiedToOld: 0,
    translationHits: 0,
    translationMisses: 0,
  };

  /**
   * Create an event adapter
   *
   * @param {Object} options - Configuration
   * @param {UnifiedEventBus} options.unifiedBus - The unified event bus
   * @param {Object} options.oldBuses - Map of old buses
   * @param {EventBus} options.oldBuses.core - Core bus (globalEventBus)
   * @param {EventBus} [options.oldBuses.automation] - Automation bus (optional)
   * @param {EventBus} [options.oldBuses.agent] - Agent bus (optional)
   * @param {boolean} [options.bidirectional=true] - Enable unified → old routing
   */
  constructor(options = {}) {
    this.#unifiedBus = options.unifiedBus;
    this.#oldBuses = options.oldBuses || {};

    if (!this.#unifiedBus) {
      throw new Error('EventAdapter requires unifiedBus');
    }

    if (!this.#oldBuses.core) {
      throw new Error('EventAdapter requires at least oldBuses.core');
    }

    this.bidirectional = options.bidirectional !== false;
  }

  /**
   * Start the adapter (begin routing events)
   */
  start() {
    log.info('Starting EventAdapter', {
      bidirectional: this.bidirectional,
      oldBusCount: Object.keys(this.#oldBuses).length,
    });

    // Route: Old buses → Unified bus
    this._routeOldToUnified();

    // Route: Unified bus → Old buses (for legacy listeners)
    if (this.bidirectional) {
      this._routeUnifiedToOld();
    }

    log.info('EventAdapter started');
  }

  /**
   * Stop the adapter (stop routing events)
   */
  stop() {
    log.info('Stopping EventAdapter');

    // Unsubscribe from all buses
    for (const unsubscribe of this.#unsubscribers) {
      unsubscribe();
    }

    this.#unsubscribers = [];

    log.info('EventAdapter stopped');
  }

  /**
   * Get adapter statistics
   *
   * @returns {Object}
   */
  getStats() {
    return {
      ...this.#stats,
      translationHitRate: this.#stats.translationHits > 0
        ? ((this.#stats.translationHits / (this.#stats.translationHits + this.#stats.translationMisses)) * 100).toFixed(2) + '%'
        : '0%',
    };
  }

  /**
   * Route events from old buses to unified bus
   * @private
   */
  _routeOldToUnified() {
    for (const [busName, bus] of Object.entries(this.#oldBuses)) {
      // Subscribe to all events on this old bus
      const unsubscribe = this._subscribeToAllEvents(bus, (oldEvent) => {
        this.#stats.oldToUnified++;

        // Extract event type and data
        const { type: oldType, data, payload } = this._extractEventData(oldEvent);

        // Translate old event type to unified
        const unifiedType = this._translateOldToUnified(oldType);

        if (unifiedType !== oldType) {
          this.#stats.translationHits++;
        } else {
          this.#stats.translationMisses++;
        }

        // Publish to unified bus
        this.#unifiedBus.publish(unifiedType, payload || data || {}, {
          source: `adapter:${busName}`,
          metadata: {
            originalType: oldType,
            originalBus: busName,
            translated: unifiedType !== oldType,
          },
        });
      });

      this.#unsubscribers.push(unsubscribe);
    }

    log.debug('Routing old → unified established', {
      busCount: Object.keys(this.#oldBuses).length,
    });
  }

  /**
   * Route events from unified bus to old buses
   * @private
   */
  _routeUnifiedToOld() {
    // Subscribe to all events on unified bus
    const unsubscribe = this.#unifiedBus.subscribe('*', (unifiedEvent) => {
      this.#stats.unifiedToOld++;

      // Skip events that came from adapter (avoid loops)
      if (unifiedEvent.source.startsWith('adapter:')) {
        return;
      }

      // Translate unified event type back to old
      const oldType = this._translateUnifiedToOld(unifiedEvent.type);

      // Publish to old buses (for legacy listeners)
      for (const [busName, bus] of Object.entries(this.#oldBuses)) {
        this._publishToOldBus(bus, oldType, unifiedEvent.payload, {
          source: 'adapter:unified',
          originalType: unifiedEvent.type,
        });
      }
    });

    this.#unsubscribers.push(unsubscribe);

    log.debug('Routing unified → old established');
  }

  /**
   * Subscribe to all events on an old bus
   * @private
   */
  _subscribeToAllEvents(bus, handler) {
    // Different buses have different APIs
    if (bus.subscribe) {
      // Automation bus API
      return bus.subscribe('*', handler);
    } else if (bus.on) {
      // Core/Agent bus API (EventEmitter-based)
      const wrappedHandler = (event) => {
        handler(event);
      };

      // Subscribe to wildcard if supported
      if (bus.listeners('*').length >= 0) {
        bus.on('*', wrappedHandler);
      }

      return () => bus.off('*', wrappedHandler);
    }

    log.warn('Unknown bus API', { bus: bus.constructor.name });
    return () => {}; // No-op unsubscribe
  }

  /**
   * Publish event to old bus
   * @private
   */
  _publishToOldBus(bus, type, payload, metadata) {
    try {
      if (bus.publish) {
        // CYNICEventBus/Automation bus API
        bus.publish(type, payload, metadata);
      } else if (bus.emit) {
        // EventEmitter-based API
        bus.emit(type, { type, payload, metadata });
      } else {
        log.warn('Cannot publish to bus (unknown API)', {
          bus: bus.constructor.name,
        });
      }
    } catch (error) {
      log.error('Error publishing to old bus', {
        type,
        error: error.message,
      });
    }
  }

  /**
   * Extract event data from various old bus formats
   * @private
   */
  _extractEventData(event) {
    // Handle different event formats
    if (event.type && event.payload !== undefined) {
      // CYNICEvent format
      return { type: event.type, payload: event.payload, data: event.payload };
    } else if (event.type && event.data !== undefined) {
      // Automation bus format
      return { type: event.type, data: event.data, payload: event.data };
    } else if (typeof event === 'object' && event.constructor.name.includes('Event')) {
      // Agent bus format (specialized event classes)
      return {
        type: event.type,
        payload: event.payload,
        data: event.payload,
      };
    }

    // Unknown format - log warning and return as-is
    log.warn('Unknown event format', { event });
    return { type: 'unknown', payload: event, data: event };
  }

  /**
   * Translate old event type to unified event type
   * @private
   */
  _translateOldToUnified(oldType) {
    return EVENT_TRANSLATION_MAP[oldType] || oldType;
  }

  /**
   * Translate unified event type back to old event type
   * @private
   */
  _translateUnifiedToOld(unifiedType) {
    return REVERSE_TRANSLATION_MAP[unifiedType] || unifiedType;
  }
}

/**
 * Create and start an event adapter
 *
 * @param {Object} options - Configuration (see EventAdapter constructor)
 * @returns {EventAdapter}
 */
export function createEventAdapter(options) {
  const adapter = new EventAdapter(options);
  adapter.start();
  return adapter;
}

/**
 * Get translation mappings (for debugging/testing)
 *
 * @returns {Object}
 */
export function getTranslationMaps() {
  return {
    oldToUnified: EVENT_TRANSLATION_MAP,
    unifiedToOld: REVERSE_TRANSLATION_MAP,
  };
}

export default EventAdapter;
