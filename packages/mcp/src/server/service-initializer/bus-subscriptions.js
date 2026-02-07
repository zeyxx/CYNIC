/**
 * Event Bus Subscriptions
 *
 * Sets up globalEventBus subscriptions for cross-layer communication.
 * Bridges PoJ/Graph/Judgment events to metrics and learning.
 *
 * @module @cynic/mcp/server/service-initializer/bus-subscriptions
 */

'use strict';

import { globalEventBus, EventType, createLogger } from '@cynic/core';

const log = createLogger('BusSubscriptions');

/**
 * Setup all event bus subscriptions
 * @param {Object} services - Initialized services
 * @returns {Function[]} Array of unsubscribe functions
 */
export function setupBusSubscriptions(services) {
  const subscriptions = [];

  // ═══════════════════════════════════════════════════════════════════════════
  // POJ CHAIN EVENTS
  // ═══════════════════════════════════════════════════════════════════════════

  subscriptions.push(
    globalEventBus.subscribe('poj:block:created', (event) => {
      const { slot, judgmentCount } = event.payload || {};
      log.info('PoJ block created', { slot, judgmentCount });
      services.metrics?.recordEvent('poj_block_created', { slot, judgmentCount });
    })
  );

  subscriptions.push(
    globalEventBus.subscribe('poj:block:finalized', (event) => {
      const { slot } = event.payload || {};
      log.info('PoJ block finalized', { slot });
      services.metrics?.recordEvent('poj_block_finalized', { slot });
    })
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // GRAPH EVENTS
  // ═══════════════════════════════════════════════════════════════════════════

  subscriptions.push(
    globalEventBus.subscribe('graph:node:added', (event) => {
      const { nodeType } = event.payload || {};
      services.metrics?.recordEvent('graph_node_added', { nodeType });
    })
  );

  subscriptions.push(
    globalEventBus.subscribe('graph:edge:added', (event) => {
      const { edgeType } = event.payload || {};
      services.metrics?.recordEvent('graph_edge_added', { edgeType });
    })
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // JUDGMENT EVENTS
  // ═══════════════════════════════════════════════════════════════════════════

  // Network nodes to forward judgments to (POST /judgment)
  const networkNodes = (process.env.CYNIC_NETWORK_NODES || '')
    .split(',')
    .map(s => s.trim())
    .filter(Boolean);

  if (networkNodes.length > 0) {
    log.info('Judgment forwarding enabled', { nodes: networkNodes });
  } else {
    log.warn('Judgment forwarding DISABLED — CYNIC_NETWORK_NODES not set');
  }

  subscriptions.push(
    globalEventBus.subscribe(EventType.JUDGMENT_CREATED, (event) => {
      const { qScore, verdict, dimensions } = event.payload || {};
      services.metrics?.recordEvent('judgment_created', { qScore, verdict });

      // Feed SONA with judgment data for adaptive learning
      if (services.sona && dimensions) {
        services.sona.observe({
          patternId: event.id,
          dimensionScores: dimensions,
        });
      }

      // Forward judgment to P2P network nodes for blockchain inclusion.
      // Skip forwarded judgments (source starts with 'peer:' or 'http:') to avoid loops.
      const source = event.source || '';
      if (networkNodes.length > 0 && !source.startsWith('peer:') && !source.startsWith('http:')) {
        const judgment = {
          id: event.id,
          qScore: qScore ?? 50,
          verdict: verdict || 'BARK',
          timestamp: event.timestamp || Date.now(),
        };
        const body = JSON.stringify(judgment);

        // Fire-and-forget POST to first reachable node (others get it via gossip)
        for (const nodeUrl of networkNodes) {
          const url = `${nodeUrl.replace(/\/$/, '')}/judgment`;
          fetch(url, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body,
            signal: AbortSignal.timeout(5000),
          })
            .then(r => {
              if (r.ok) log.info('Judgment forwarded to network', { node: nodeUrl, id: event.id, qScore });
              else log.warn('Judgment forward failed', { node: nodeUrl, status: r.status });
            })
            .catch(err => {
              log.warn('Judgment forward unreachable', { node: nodeUrl, error: err.message });
            });
          break; // One node is enough — gossip propagates to the rest
        }
      }
    })
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // ENGINE EVENTS
  // ═══════════════════════════════════════════════════════════════════════════

  subscriptions.push(
    globalEventBus.subscribe(EventType.ENGINE_CONSULTED, (event) => {
      const { engineId, domain } = event.payload || {};
      services.metrics?.recordEvent('engine_consulted', { engineId, domain });
    })
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // ANOMALY EVENTS
  // ═══════════════════════════════════════════════════════════════════════════

  subscriptions.push(
    globalEventBus.subscribe(EventType.ANOMALY_DETECTED, (event) => {
      const { type, severity } = event.payload || {};
      log.warn('Anomaly detected', { type, severity });
      services.metrics?.recordEvent('anomaly_detected', { type, severity });
    })
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // USER FEEDBACK EVENTS (Learning Integration)
  // ═══════════════════════════════════════════════════════════════════════════

  subscriptions.push(
    globalEventBus.subscribe(EventType.USER_FEEDBACK, async (event) => {
      const { source, tool, success, blocked, duration, userId, itemId, impact } = event.payload || {};
      services.metrics?.recordEvent('user_feedback', { source, success, blocked });

      // Feed to LearningService
      if (services.learningService) {
        try {
          const feedback = {
            source: source || 'tool_execution',
            itemType: 'tool',
            itemId: tool,
            positive: success && !blocked,
            context: { duration, userId, blocked },
            timestamp: event.timestamp,
          };
          await services.learningService.recordFeedback?.(feedback);
          log.trace('Learning feedback recorded', { tool, success });
        } catch (err) {
          log.warn('Learning feedback error', { error: err.message });
        }
      }

      // Feed to SONA for correlation
      if (services.sona && itemId) {
        services.sona.processFeedback({
          patternId: itemId,
          success: !!success,
          impact: impact || (success ? 0.7 : 0.3),
        });
      }
    })
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // TOOL EVENTS
  // ═══════════════════════════════════════════════════════════════════════════

  subscriptions.push(
    globalEventBus.subscribe(EventType.TOOL_COMPLETED, (event) => {
      const { tool, duration, success, blocked, agentCount } = event.payload || {};
      services.metrics?.recordEvent('tool_completed', { tool, duration, success, blocked, agentCount });
    })
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // CLAUDE FLOW EVENTS
  // ═══════════════════════════════════════════════════════════════════════════

  subscriptions.push(
    globalEventBus.subscribe('request:classify', (event) => {
      const { content } = event.payload || {};
      if (services.tieredRouter && content) {
        const tier = services.complexityClassifier?.classify({ content });
        services.metrics?.recordEvent('request_routed', { tier: tier?.tier });
      }
    })
  );

  log.debug('Bus subscriptions active', { count: subscriptions.length });
  return subscriptions;
}

/**
 * Cleanup bus subscriptions
 * @param {Function[]} subscriptions - Array of unsubscribe functions
 */
export function cleanupBusSubscriptions(subscriptions) {
  if (subscriptions) {
    for (const unsubscribe of subscriptions) {
      if (typeof unsubscribe === 'function') {
        unsubscribe();
      }
    }
    log.debug('Bus subscriptions cleaned up');
  }
}
