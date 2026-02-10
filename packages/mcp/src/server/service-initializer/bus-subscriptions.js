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
import fs from 'fs';
import path from 'path';
import os from 'os';

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

      // Feed to BehaviorModifier → actual behavior changes
      if (services.behaviorModifier) {
        services.behaviorModifier.processFeedback({
          type: source === 'explicit' ? 'explicit' : 'implicit_success',
          outcome: success && !blocked ? 'correct' : 'incorrect',
          dog: event.payload?.dog,
          dimension: event.payload?.dimension,
          taskType: source || 'tool_execution',
          score: event.payload?.qScore,
          timestamp: event.timestamp,
        });
      }
    })
  );

  // ═══════════════════════════════════════════════════════════════════════════
  // TOOL EVENTS + META-COGNITION
  // ═══════════════════════════════════════════════════════════════════════════

  subscriptions.push(
    globalEventBus.subscribe(EventType.TOOL_COMPLETED, (event) => {
      const { tool, duration, success, blocked, agentCount } = event.payload || {};
      services.metrics?.recordEvent('tool_completed', { tool, duration, success, blocked, agentCount });

      // Feed to MetaCognition → stuck/thrashing/flow detection
      if (services.metaCognition) {
        const analysis = services.metaCognition.recordAction({
          type: blocked ? 'blocked' : 'tool_execution',
          tool,
          success: !!success && !blocked,
          duration: duration || 0,
        });

        // Log strategy recommendations
        if (analysis?.recommendation) {
          log.info('MetaCognition recommendation', {
            strategy: analysis.recommendation.strategy,
            reason: analysis.recommendation.reason,
            state: analysis.state,
          });
        }
      }
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

  // ═══════════════════════════════════════════════════════════════════════════
  // SESSION END — Persist learning state
  // ═══════════════════════════════════════════════════════════════════════════

  subscriptions.push(
    globalEventBus.subscribe(EventType.SESSION_ENDED, () => {
      // Persist SONA state to disk (same pattern as Thompson Sampler)
      if (services.sona) {
        try {
          const sonaDir = path.join(os.homedir(), '.cynic', 'sona');
          fs.mkdirSync(sonaDir, { recursive: true });
          const state = {
            stats: services.sona.getStats(),
            insights: services.sona.getDimensionInsights(),
            savedAt: Date.now(),
          };
          fs.writeFileSync(
            path.join(sonaDir, 'state.json'),
            JSON.stringify(state, null, 2),
          );
          log.info('SONA state persisted', { observations: state.stats.totalObservations });
        } catch (e) {
          log.debug('SONA persist failed', { error: e.message });
        }
      }

      // Persist BehaviorModifier state
      if (services.behaviorModifier) {
        try {
          const bmDir = path.join(os.homedir(), '.cynic', 'behavior');
          fs.mkdirSync(bmDir, { recursive: true });
          fs.writeFileSync(
            path.join(bmDir, 'state.json'),
            JSON.stringify({
              stats: services.behaviorModifier.getStats(),
              context: services.behaviorModifier.getBehaviorContext(),
              savedAt: Date.now(),
            }, null, 2),
          );
          log.info('BehaviorModifier state persisted');
        } catch (e) {
          log.debug('BehaviorModifier persist failed', { error: e.message });
        }
      }

      // Persist MetaCognition state
      if (services.metaCognition) {
        try {
          const mcDir = path.join(os.homedir(), '.cynic', 'metacognition');
          fs.mkdirSync(mcDir, { recursive: true });
          fs.writeFileSync(
            path.join(mcDir, 'state.json'),
            JSON.stringify(services.metaCognition.exportState(), null, 2),
          );
          log.info('MetaCognition state persisted');
        } catch (e) {
          log.debug('MetaCognition persist failed', { error: e.message });
        }
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
