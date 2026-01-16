/**
 * @cynic/node - Agent Event Bus
 *
 * φ-aligned communication backbone for the Five Dogs collective.
 * Handles event routing, consensus coordination, and history management.
 *
 * "φ distrusts φ" - κυνικός
 *
 * @module @cynic/node/agents/event-bus
 */

'use strict';

import { EventEmitter } from 'events';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';
import {
  EVENT_CONSTANTS,
  AgentEvent,
  EventPriority,
  AgentId,
  ConsensusVote,
  AgentEventMessage,
  ConsensusRequestEvent,
  ConsensusResponseEvent,
} from './events.js';

/**
 * φ-aligned constants for event bus
 */
export const BUS_CONSTANTS = {
  /** Max concurrent event processing (Fib(8) = 21) */
  MAX_CONCURRENT: 21,

  /** Event batch size (Fib(7) = 13) */
  BATCH_SIZE: 13,

  /** Cleanup interval in ms (Fib(9) = 34 × 1000 = 34s) */
  CLEANUP_INTERVAL_MS: 34000,

  /** Max listeners per event type (Fib(6) = 8) */
  MAX_LISTENERS_PER_TYPE: 8,

  /** Priority queue levels (Fib(4) = 3) */
  PRIORITY_LEVELS: 3,
};

/**
 * Event subscription
 */
class Subscription {
  /**
   * @param {string} eventType
   * @param {string} agentId
   * @param {Function} handler
   * @param {object} [options]
   */
  constructor(eventType, agentId, handler, options = {}) {
    this.id = `sub_${Date.now().toString(36)}_${Math.random().toString(36).slice(2, 8)}`;
    this.eventType = eventType;
    this.agentId = agentId;
    this.handler = handler;
    this.filter = options.filter || null;
    this.priority = options.priority || EventPriority.NORMAL;
    this.once = options.once || false;
    this.createdAt = Date.now();
    this.invocationCount = 0;
  }

  /**
   * Check if event matches subscription filter
   * @param {AgentEventMessage} event
   */
  matches(event) {
    // Check event type
    if (this.eventType !== '*' && this.eventType !== event.type) {
      return false;
    }

    // Check if event targets this agent
    if (!event.targetsAgent(this.agentId)) {
      return false;
    }

    // Apply custom filter
    if (this.filter && !this.filter(event)) {
      return false;
    }

    return true;
  }

  /**
   * Invoke handler
   * @param {AgentEventMessage} event
   */
  async invoke(event) {
    this.invocationCount++;
    await this.handler(event);
  }
}

/**
 * Agent Event Bus
 *
 * Central hub for inter-agent communication in the collective.
 */
export class AgentEventBus extends EventEmitter {
  constructor() {
    super();

    // Set max listeners (Fib(10) = 55)
    this.setMaxListeners(55);

    /** @type {Map<string, Subscription[]>} Event type → subscriptions */
    this.subscriptions = new Map();

    /** @type {AgentEventMessage[]} Event history */
    this.history = [];

    /** @type {Map<string, ConsensusRequestEvent>} Pending consensus requests */
    this.pendingConsensus = new Map();

    /** @type {Set<string>} Registered agent IDs */
    this.registeredAgents = new Set();

    // Statistics
    this.stats = {
      eventsPublished: 0,
      eventsDelivered: 0,
      eventsDropped: 0,
      consensusRequests: 0,
      consensusResolved: 0,
    };

    // Start cleanup timer
    this._cleanupInterval = setInterval(
      () => this._cleanup(),
      BUS_CONSTANTS.CLEANUP_INTERVAL_MS
    );
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // AGENT REGISTRATION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Register an agent with the bus
   * @param {string} agentId - Agent identifier
   */
  registerAgent(agentId) {
    this.registeredAgents.add(agentId);
    this.emit('agent:registered', { agentId, timestamp: Date.now() });
  }

  /**
   * Unregister an agent
   * @param {string} agentId
   */
  unregisterAgent(agentId) {
    this.registeredAgents.delete(agentId);

    // Remove all subscriptions for this agent
    for (const [eventType, subs] of this.subscriptions) {
      this.subscriptions.set(
        eventType,
        subs.filter(s => s.agentId !== agentId)
      );
    }

    this.emit('agent:unregistered', { agentId, timestamp: Date.now() });
  }

  /**
   * Check if agent is registered
   * @param {string} agentId
   */
  isAgentRegistered(agentId) {
    return this.registeredAgents.has(agentId);
  }

  /**
   * Get all registered agents
   */
  getRegisteredAgents() {
    return Array.from(this.registeredAgents);
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // SUBSCRIPTION
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Subscribe to events
   *
   * @param {string} eventType - Event type to subscribe to (or '*' for all)
   * @param {string} agentId - Subscribing agent ID
   * @param {Function} handler - Event handler
   * @param {object} [options] - Subscription options
   * @returns {string} Subscription ID
   */
  subscribe(eventType, agentId, handler, options = {}) {
    if (!this.isAgentRegistered(agentId)) {
      throw new Error(`Agent ${agentId} not registered`);
    }

    const subscription = new Subscription(eventType, agentId, handler, options);

    if (!this.subscriptions.has(eventType)) {
      this.subscriptions.set(eventType, []);
    }

    const subs = this.subscriptions.get(eventType);

    // Check max listeners
    if (subs.length >= BUS_CONSTANTS.MAX_LISTENERS_PER_TYPE) {
      throw new Error(`Max listeners (${BUS_CONSTANTS.MAX_LISTENERS_PER_TYPE}) reached for ${eventType}`);
    }

    subs.push(subscription);

    return subscription.id;
  }

  /**
   * Subscribe to event once
   *
   * @param {string} eventType
   * @param {string} agentId
   * @param {Function} handler
   * @param {object} [options]
   */
  subscribeOnce(eventType, agentId, handler, options = {}) {
    return this.subscribe(eventType, agentId, handler, { ...options, once: true });
  }

  /**
   * Unsubscribe by subscription ID
   *
   * @param {string} subscriptionId
   */
  unsubscribe(subscriptionId) {
    for (const [eventType, subs] of this.subscriptions) {
      const index = subs.findIndex(s => s.id === subscriptionId);
      if (index !== -1) {
        subs.splice(index, 1);
        return true;
      }
    }
    return false;
  }

  /**
   * Unsubscribe all for an agent
   *
   * @param {string} agentId
   */
  unsubscribeAll(agentId) {
    let removed = 0;
    for (const [eventType, subs] of this.subscriptions) {
      const before = subs.length;
      this.subscriptions.set(
        eventType,
        subs.filter(s => s.agentId !== agentId)
      );
      removed += before - this.subscriptions.get(eventType).length;
    }
    return removed;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // EVENT PUBLISHING
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Publish an event
   *
   * @param {AgentEventMessage} event - Event to publish
   * @returns {Promise<{ delivered: number, errors: Error[] }>}
   */
  async publish(event) {
    // Validate event
    if (!(event instanceof AgentEventMessage)) {
      throw new Error('Event must be an AgentEventMessage');
    }

    // Check payload size
    const payloadSize = JSON.stringify(event.payload).length;
    if (payloadSize > EVENT_CONSTANTS.MAX_PAYLOAD_SIZE) {
      throw new Error(`Event payload exceeds max size (${EVENT_CONSTANTS.MAX_PAYLOAD_SIZE} bytes)`);
    }

    // Add to history
    this._addToHistory(event);

    // Get matching subscriptions
    const matchingSubscriptions = this._getMatchingSubscriptions(event);

    // Sort by priority
    matchingSubscriptions.sort((a, b) => {
      const priorityOrder = {
        [EventPriority.CRITICAL]: 0,
        [EventPriority.HIGH]: 1,
        [EventPriority.NORMAL]: 2,
        [EventPriority.LOW]: 3,
      };
      return priorityOrder[a.priority] - priorityOrder[b.priority];
    });

    // Deliver to subscribers
    const errors = [];
    let delivered = 0;

    for (const subscription of matchingSubscriptions) {
      try {
        await subscription.invoke(event);
        delivered++;

        // Remove one-time subscriptions
        if (subscription.once) {
          this.unsubscribe(subscription.id);
        }
      } catch (error) {
        errors.push(error);
        this.emit('error', { event, subscription, error });
      }
    }

    // Update stats
    this.stats.eventsPublished++;
    this.stats.eventsDelivered += delivered;

    // Emit internal event
    this.emit('event:published', {
      eventId: event.id,
      type: event.type,
      delivered,
      errors: errors.length,
    });

    return { delivered, errors };
  }

  /**
   * Publish multiple events
   *
   * @param {AgentEventMessage[]} events
   */
  async publishBatch(events) {
    const results = [];
    for (const event of events) {
      results.push(await this.publish(event));
    }
    return results;
  }

  /**
   * Get matching subscriptions for an event
   * @private
   */
  _getMatchingSubscriptions(event) {
    const matching = [];

    // Check specific event type subscriptions
    const typeSubs = this.subscriptions.get(event.type) || [];
    matching.push(...typeSubs.filter(s => s.matches(event)));

    // Check wildcard subscriptions
    const wildcardSubs = this.subscriptions.get('*') || [];
    matching.push(...wildcardSubs.filter(s => s.matches(event)));

    return matching;
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // CONSENSUS
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Request consensus from the collective
   *
   * @param {string} requestingAgent - Agent requesting consensus
   * @param {object} request - Consensus request details
   * @returns {Promise<object>} Consensus result
   */
  async requestConsensus(requestingAgent, request) {
    if (!this.isAgentRegistered(requestingAgent)) {
      throw new Error(`Agent ${requestingAgent} not registered`);
    }

    // Check pending consensus limit
    if (this.pendingConsensus.size >= EVENT_CONSTANTS.MAX_PENDING_CONSENSUS) {
      throw new Error('Too many pending consensus requests');
    }

    // Create consensus event
    const consensusEvent = new ConsensusRequestEvent(requestingAgent, request);

    // Store pending request
    this.pendingConsensus.set(consensusEvent.id, consensusEvent);
    this.stats.consensusRequests++;

    // Publish to all agents
    await this.publish(consensusEvent);

    // Wait for consensus or timeout
    return new Promise((resolve, reject) => {
      const checkInterval = setInterval(() => {
        // Check timeout
        if (consensusEvent.isTimedOut()) {
          clearInterval(checkInterval);
          this.pendingConsensus.delete(consensusEvent.id);

          resolve({
            approved: false,
            reason: 'timeout',
            votes: Array.from(consensusEvent.votes.values()),
          });
          return;
        }

        // Check consensus
        const result = consensusEvent.checkConsensus();
        if (result) {
          clearInterval(checkInterval);
          this.pendingConsensus.delete(consensusEvent.id);
          this.stats.consensusResolved++;

          resolve(result);
        }
      }, 100); // Check every 100ms
    });
  }

  /**
   * Vote on a consensus request
   *
   * @param {string} votingAgent - Agent casting vote
   * @param {string} requestId - Consensus request ID
   * @param {string} vote - ConsensusVote value
   * @param {string} [reason] - Optional reason
   */
  async vote(votingAgent, requestId, vote, reason = null) {
    if (!this.isAgentRegistered(votingAgent)) {
      throw new Error(`Agent ${votingAgent} not registered`);
    }

    const consensusRequest = this.pendingConsensus.get(requestId);
    if (!consensusRequest) {
      throw new Error(`Consensus request ${requestId} not found`);
    }

    // Record vote
    consensusRequest.recordVote(votingAgent, vote, reason);

    // Publish response
    const response = new ConsensusResponseEvent(votingAgent, requestId, vote, reason);
    await this.publish(response);

    return consensusRequest.checkConsensus();
  }

  /**
   * Get pending consensus requests
   */
  getPendingConsensus() {
    return Array.from(this.pendingConsensus.values());
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // HISTORY
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Add event to history
   * @private
   */
  _addToHistory(event) {
    this.history.push(event);

    // Trim history if needed
    while (this.history.length > EVENT_CONSTANTS.MAX_HISTORY) {
      this.history.shift();
    }
  }

  /**
   * Get event history
   *
   * @param {object} [options] - Query options
   * @returns {AgentEventMessage[]}
   */
  getHistory(options = {}) {
    let events = [...this.history];

    // Filter by type
    if (options.type) {
      events = events.filter(e => e.type === options.type);
    }

    // Filter by source
    if (options.source) {
      events = events.filter(e => e.source === options.source);
    }

    // Filter by time range
    if (options.since) {
      events = events.filter(e => e.timestamp >= options.since);
    }
    if (options.until) {
      events = events.filter(e => e.timestamp <= options.until);
    }

    // Filter non-expired
    if (options.excludeExpired) {
      events = events.filter(e => !e.isExpired());
    }

    // Sort (newest first by default)
    if (options.sortOrder === 'asc') {
      events.sort((a, b) => a.timestamp - b.timestamp);
    } else {
      events.sort((a, b) => b.timestamp - a.timestamp);
    }

    // Limit
    if (options.limit) {
      events = events.slice(0, options.limit);
    }

    return events;
  }

  /**
   * Get event by ID
   * @param {string} eventId
   */
  getEvent(eventId) {
    return this.history.find(e => e.id === eventId) || null;
  }

  /**
   * Get correlated events
   * @param {string} correlationId
   */
  getCorrelatedEvents(correlationId) {
    return this.history.filter(
      e => e.correlationId === correlationId || e.id === correlationId
    );
  }

  // ═══════════════════════════════════════════════════════════════════════════
  // UTILITIES
  // ═══════════════════════════════════════════════════════════════════════════

  /**
   * Clean up expired events and timed-out consensus requests
   * @private
   */
  _cleanup() {
    // Clean up expired events from history
    const now = Date.now();
    const before = this.history.length;
    this.history = this.history.filter(e => !e.isExpired());
    const removed = before - this.history.length;

    // Clean up timed-out consensus requests
    for (const [id, request] of this.pendingConsensus) {
      if (request.isTimedOut()) {
        this.pendingConsensus.delete(id);
      }
    }

    if (removed > 0) {
      this.emit('cleanup', { eventsRemoved: removed });
    }
  }

  /**
   * Get bus statistics
   */
  getStats() {
    const subscriptionCounts = {};
    for (const [type, subs] of this.subscriptions) {
      subscriptionCounts[type] = subs.length;
    }

    return {
      registeredAgents: this.registeredAgents.size,
      totalSubscriptions: Array.from(this.subscriptions.values())
        .reduce((sum, subs) => sum + subs.length, 0),
      subscriptionsByType: subscriptionCounts,
      historySize: this.history.length,
      maxHistory: EVENT_CONSTANTS.MAX_HISTORY,
      pendingConsensus: this.pendingConsensus.size,
      ...this.stats,
      phi: {
        consensusThreshold: EVENT_CONSTANTS.CONSENSUS_THRESHOLD,
        vetoThreshold: EVENT_CONSTANTS.VETO_THRESHOLD,
        maxConfidence: PHI_INV,
      },
    };
  }

  /**
   * Reset the bus (for testing)
   */
  reset() {
    this.subscriptions.clear();
    this.history = [];
    this.pendingConsensus.clear();
    this.registeredAgents.clear();
    this.stats = {
      eventsPublished: 0,
      eventsDelivered: 0,
      eventsDropped: 0,
      consensusRequests: 0,
      consensusResolved: 0,
    };
  }

  /**
   * Destroy the bus
   */
  destroy() {
    if (this._cleanupInterval) {
      clearInterval(this._cleanupInterval);
      this._cleanupInterval = null;
    }
    this.reset();
    this.removeAllListeners();
  }

  /**
   * Shutdown the bus (alias for destroy)
   * @returns {Promise<void>}
   */
  async shutdown() {
    this.destroy();
  }
}

/**
 * Create a singleton event bus
 */
let globalBus = null;

export function getGlobalEventBus() {
  if (!globalBus) {
    globalBus = new AgentEventBus();
  }
  return globalBus;
}

export function resetGlobalEventBus() {
  if (globalBus) {
    globalBus.destroy();
    globalBus = null;
  }
}

export default {
  BUS_CONSTANTS,
  Subscription,
  AgentEventBus,
  getGlobalEventBus,
  resetGlobalEventBus,
};
