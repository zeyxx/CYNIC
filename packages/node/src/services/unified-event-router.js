/**
 * UnifiedEventRouter — One API, three buses
 *
 * A3: Solves event bus fragmentation by providing a unified interface
 * over CYNIC's three nervous systems.
 *
 * Architecture:
 *   Components → UnifiedEventRouter → [globalEventBus, automation, AgentEventBus]
 *
 * Key features:
 * - Namespace-based routing (perception:* → core + agent)
 * - Automatic cross-bus forwarding (no manual rules)
 * - Wildcard subscriptions across all buses
 * - Loop prevention (via BRIDGED_TAG)
 * - Backward compatible (direct bus access still works)
 *
 * Routing rules:
 *   perception:* → globalEventBus + AgentEventBus (dogs need to hear)
 *   market:* → globalEventBus + AgentEventBus
 *   solana:* → globalEventBus + AgentEventBus
 *   dog:* → AgentEventBus only
 *   judgment:* → globalEventBus + automation
 *   learning:* → automation + globalEventBus
 *   qlearning:* → globalEventBus + automation
 *   automation:* → automation only
 *   agent:* → AgentEventBus only
 *   * (fallback) → globalEventBus
 *
 * "One nervous system. φ unifies." — κυνικός
 *
 * @module @cynic/node/services/unified-event-router
 */

'use strict';

import { globalEventBus, createLogger, EventType as CoreEventType } from '@cynic/core';
import { getEventBus, EventType as AutomationEventType } from './event-bus.js';
import { AgentEventMessage } from '../agents/events.js';
import { BRIDGED_TAG } from './event-bus-bridge.js';

const log = createLogger('UnifiedEventRouter');

/**
 * Router ID for AgentEventBus subscriptions
 */
const ROUTER_AGENT_ID = 'unified-router';

/**
 * Namespace routing configuration
 *
 * Maps event namespace prefixes to target buses.
 * Order matters: first match wins.
 */
const NAMESPACE_ROUTES = [
  // Perception events → core + agent (dogs need to hear)
  { pattern: /^perception:/, buses: ['core', 'agent'] },

  // Market events → core + agent (dogs trade)
  { pattern: /^market:/, buses: ['core', 'agent'] },

  // Solana events → core + agent (dogs monitor chain)
  { pattern: /^solana:/, buses: ['core', 'agent'] },

  // Dog events → agent only (internal dog communication)
  { pattern: /^dog:/, buses: ['agent'] },

  // Judgment events → core + automation (tracking)
  { pattern: /^judgment:/, buses: ['core', 'automation'] },

  // Learning events → automation + core (metathinking)
  { pattern: /^learning:/, buses: ['automation', 'core'] },

  // Q-Learning events → core + automation (hot-swap routing)
  { pattern: /^qlearning:/, buses: ['core', 'automation'] },

  // Automation events → automation only
  { pattern: /^automation:/, buses: ['automation'] },

  // Agent events → agent only
  { pattern: /^agent:/, buses: ['agent'] },

  // Fallback: everything else → core
  { pattern: /.*/, buses: ['core'] },
];

/**
 * UnifiedEventRouter
 *
 * Single API for publishing/subscribing across all three event buses.
 * Automatically routes events to appropriate buses based on namespace.
 */
export class UnifiedEventRouter {
  constructor(options = {}) {
    /** @type {import('../agents/event-bus.js').AgentEventBus | null} */
    this._agentBus = options.agentBus || null;
    this._agentBusRegistered = false;

    this._stats = {
      publishes: { core: 0, automation: 0, agent: 0, total: 0 },
      subscribes: { core: 0, automation: 0, agent: 0, total: 0 },
      duplicatesPrevented: 0,
      errors: 0,
      startedAt: Date.now(),
    };

    // Track active subscriptions for cleanup
    this._subscriptions = [];
  }

  /**
   * Publish event to appropriate bus(es) based on namespace.
   *
   * @param {string} type - Event type (e.g., "perception:market:price")
   * @param {any} payload - Event data
   * @param {Object} [options] - Event options (source, correlationId, metadata)
   * @returns {void}
   */
  publish(type, payload, options = {}) {
    // Prevent re-publishing bridged events (loop prevention)
    if (options.metadata?.[BRIDGED_TAG]) {
      this._stats.duplicatesPrevented++;
      return;
    }

    const buses = this._routeEvent(type);
    const enrichedOptions = {
      ...options,
      metadata: {
        ...options.metadata,
        [BRIDGED_TAG]: true, // Mark to prevent loops
        routedBy: 'unified-router',
        targetBuses: buses,
      },
    };

    for (const bus of buses) {
      try {
        switch (bus) {
          case 'core':
            globalEventBus.publish(type, payload, enrichedOptions);
            this._stats.publishes.core++;
            break;

          case 'automation':
            getEventBus().publish(type, payload, {
              source: enrichedOptions.source || 'unified-router',
              [BRIDGED_TAG]: true,
            });
            this._stats.publishes.automation++;
            break;

          case 'agent':
            if (this._agentBus) {
              this._ensureAgentBusRegistration();
              const agentMessage = new AgentEventMessage(
                type,
                enrichedOptions.source || 'unified-router',
                payload,
                {
                  priority: enrichedOptions.priority || 'medium',
                  correlationId: enrichedOptions.correlationId,
                  metadata: enrichedOptions.metadata,
                }
              );
              this._agentBus.publish(agentMessage);
              this._stats.publishes.agent++;
            }
            break;
        }
      } catch (err) {
        this._stats.errors++;
        log.error('Publish failed', { type, bus, error: err.message });
      }
    }

    this._stats.publishes.total++;
  }

  /**
   * Subscribe to events across all relevant buses.
   *
   * @param {string} pattern - Event type or wildcard pattern (e.g., "perception:*")
   * @param {Function} handler - Event handler (event) => void
   * @returns {Function} Unsubscribe function
   */
  subscribe(pattern, handler) {
    const buses = this._routeEvent(pattern);
    const unsubscribers = [];

    // Deduplicate: only call handler once per event (even if on multiple buses)
    const seenEventIds = new Set();
    const deduplicatedHandler = (event) => {
      const eventId = event.id || event.correlationId || `${event.type}-${event.timestamp || Date.now()}`;

      if (seenEventIds.has(eventId)) {
        this._stats.duplicatesPrevented++;
        return;
      }
      seenEventIds.add(eventId);

      // Clean up old IDs (keep last 100)
      if (seenEventIds.size > 100) {
        const oldestId = seenEventIds.values().next().value;
        seenEventIds.delete(oldestId);
      }

      handler(event);
    };

    for (const bus of buses) {
      try {
        switch (bus) {
          case 'core':
            unsubscribers.push(globalEventBus.subscribe(pattern, deduplicatedHandler));
            this._stats.subscribes.core++;
            break;

          case 'automation':
            unsubscribers.push(getEventBus().subscribe(pattern, deduplicatedHandler));
            this._stats.subscribes.automation++;
            break;

          case 'agent':
            if (this._agentBus) {
              this._ensureAgentBusRegistration();
              const subId = this._agentBus.subscribe(pattern, ROUTER_AGENT_ID, deduplicatedHandler);
              unsubscribers.push(() => this._agentBus.unsubscribe(subId));
              this._stats.subscribes.agent++;
            }
            break;
        }
      } catch (err) {
        this._stats.errors++;
        log.error('Subscribe failed', { pattern, bus, error: err.message });
      }
    }

    this._stats.subscribes.total++;

    // Return unified unsubscribe function
    const unsubscribe = () => {
      for (const unsub of unsubscribers) {
        try {
          unsub();
        } catch (err) {
          log.debug('Unsubscribe failed', { error: err.message });
        }
      }

      // Remove from tracking
      const idx = this._subscriptions.indexOf(unsubscribe);
      if (idx >= 0) this._subscriptions.splice(idx, 1);
    };

    this._subscriptions.push(unsubscribe);
    return unsubscribe;
  }

  /**
   * Route event type to target buses based on namespace.
   *
   * @param {string} type - Event type
   * @returns {string[]} Array of bus names ('core', 'automation', 'agent')
   * @private
   */
  _routeEvent(type) {
    for (const route of NAMESPACE_ROUTES) {
      if (route.pattern.test(type)) {
        return route.buses;
      }
    }

    // Should never reach here (fallback route catches all)
    return ['core'];
  }

  /**
   * Ensure router is registered on AgentEventBus.
   * @private
   */
  _ensureAgentBusRegistration() {
    if (!this._agentBusRegistered && this._agentBus) {
      if (!this._agentBus.isAgentRegistered(ROUTER_AGENT_ID)) {
        this._agentBus.registerAgent(ROUTER_AGENT_ID);
      }
      this._agentBusRegistered = true;
    }
  }

  /**
   * Late-bind AgentEventBus (when CollectivePack initializes after router).
   *
   * @param {import('../agents/event-bus.js').AgentEventBus} agentBus
   */
  setAgentBus(agentBus) {
    if (this._agentBus) {
      log.warn('AgentEventBus already set — skipping');
      return;
    }

    this._agentBus = agentBus;
    log.info('AgentEventBus late-bound to UnifiedEventRouter');
  }

  /**
   * Get router statistics.
   */
  getStats() {
    return {
      uptime: Date.now() - this._stats.startedAt,
      publishes: { ...this._stats.publishes },
      subscribes: { ...this._stats.subscribes },
      duplicatesPrevented: this._stats.duplicatesPrevented,
      errors: this._stats.errors,
      activeSubscriptions: this._subscriptions.length,
      routes: NAMESPACE_ROUTES.map(r => ({
        pattern: r.pattern.source,
        buses: r.buses,
      })),
      buses: {
        core: 'always available',
        automation: 'always available',
        agent: this._agentBus ? 'connected' : 'not connected',
      },
    };
  }

  /**
   * Clean up all subscriptions.
   */
  stop() {
    for (const unsubscribe of this._subscriptions) {
      try {
        unsubscribe();
      } catch (_) {
        // Ignore cleanup errors
      }
    }
    this._subscriptions = [];

    // Unregister from AgentEventBus
    if (this._agentBus?.isAgentRegistered?.(ROUTER_AGENT_ID)) {
      try {
        this._agentBus.unregisterAgent(ROUTER_AGENT_ID);
      } catch (_) {
        // Ignore
      }
    }

    this._agentBusRegistered = false;
    log.info('UnifiedEventRouter stopped', { stats: this._stats });
  }

  /**
   * Reset for testing.
   */
  _resetForTesting() {
    this.stop();
    this._stats = {
      publishes: { core: 0, automation: 0, agent: 0, total: 0 },
      subscribes: { core: 0, automation: 0, agent: 0, total: 0 },
      duplicatesPrevented: 0,
      errors: 0,
      startedAt: Date.now(),
    };
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SINGLETON
// ═══════════════════════════════════════════════════════════════════════════

let _instance = null;

/**
 * Get the global UnifiedEventRouter instance.
 *
 * @param {Object} [options] - Options (only used on first call)
 * @returns {UnifiedEventRouter}
 */
export function getUnifiedEventRouter(options = {}) {
  if (!_instance) {
    _instance = new UnifiedEventRouter(options);
    log.info('UnifiedEventRouter created');
  }
  return _instance;
}

/**
 * Reset singleton (for testing).
 */
export function resetUnifiedEventRouter() {
  if (_instance) {
    _instance.stop();
    _instance = null;
  }
}

export default { UnifiedEventRouter, getUnifiedEventRouter, resetUnifiedEventRouter, ROUTER_AGENT_ID };
