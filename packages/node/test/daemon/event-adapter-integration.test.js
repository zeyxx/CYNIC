/**
 * EventAdapter Integration Test
 *
 * Verifies that EventAdapter correctly bridges old buses to unified bus.
 *
 * Test scenarios:
 * 1. Old bus → Unified bus routing
 * 2. Unified bus → Old bus routing (bidirectional)
 * 3. Translation accuracy
 * 4. Loop prevention
 */

'use strict';

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import { globalEventBus } from '@cynic/core';
import { getUnifiedEventBus, createEventAdapter } from '@cynic/core';
import { getEventBus } from '../../src/services/event-bus.js';
import { getGlobalEventBus as getAgentEventBus } from '../../src/agents/event-bus.js';

describe('EventAdapter Integration', () => {
  let adapter = null;
  let unifiedBus = null;
  let automationBus = null;
  let agentBus = null;

  before(() => {
    // Create fresh bus instances
    unifiedBus = getUnifiedEventBus();
    automationBus = getEventBus();
    agentBus = AgentEventBus.getInstance();

    // Create and start adapter
    adapter = createEventAdapter({
      unifiedBus,
      oldBuses: {
        core: globalEventBus,
        automation: automationBus,
        agent: agentBus,
      },
      bidirectional: true,
    });
  });

  after(() => {
    if (adapter) {
      adapter.stop();
    }
    if (unifiedBus) {
      unifiedBus.clear();
      unifiedBus.stop();
    }
  });

  it('should route events from old Core bus to unified bus', async () => {
    let receivedOnUnified = false;

    // Subscribe to unified bus
    const unsubscribe = unifiedBus.subscribe('judgment:created', (event) => {
      receivedOnUnified = true;
      assert.equal(event.payload.qScore, 88);
    });

    // Publish on old Core bus
    globalEventBus.publish('judgment:created', { qScore: 88 });

    // Wait for async routing
    await new Promise(resolve => setTimeout(resolve, 50));

    assert.ok(receivedOnUnified, 'Event should be routed to unified bus');
    unsubscribe();
  });

  it('should translate old event types to unified types', async () => {
    let translatedType = null;

    // Subscribe to unified bus
    const unsubscribe = unifiedBus.subscribe('learning:feedback:received', (event) => {
      translatedType = event.type;
    });

    // Publish old 'user:feedback' event (should translate to 'learning:feedback:received')
    globalEventBus.publish('user:feedback', { rating: 'good' });

    // Wait for async routing
    await new Promise(resolve => setTimeout(resolve, 50));

    assert.equal(translatedType, 'learning:feedback:received', 'Event type should be translated');
    unsubscribe();
  });

  it('should route events from unified bus back to old buses (bidirectional)', async () => {
    let receivedOnOldBus = false;

    // Subscribe to old Core bus
    const handler = (event) => {
      if (event.type === 'learning:feedback:received') {
        receivedOnOldBus = true;
      }
    };
    globalEventBus.on('user:feedback', handler);

    // Publish on unified bus
    unifiedBus.publish('learning:feedback:received', { rating: 'excellent' });

    // Wait for async routing
    await new Promise(resolve => setTimeout(resolve, 50));

    assert.ok(receivedOnOldBus, 'Event should be routed back to old bus');
    globalEventBus.off('user:feedback', handler);
  });

  it('should prevent infinite loops by skipping adapter events', async () => {
    let loopCount = 0;

    // Subscribe to unified bus and count events
    const unsubscribe = unifiedBus.subscribe('judgment:created', (event) => {
      loopCount++;
    });

    // Publish event
    globalEventBus.publish('judgment:created', { qScore: 88 });

    // Wait for routing to settle
    await new Promise(resolve => setTimeout(resolve, 100));

    // Should only receive event once (not in a loop)
    // Note: May be 1 or 2 depending on bidirectional routing, but NOT 10+
    assert.ok(loopCount < 5, `Should not loop infinitely (got ${loopCount} events)`);
    unsubscribe();
  });

  it('should track adapter statistics', () => {
    const stats = adapter.getStats();

    assert.ok(stats.oldToUnified >= 0, 'Should track old → unified routing');
    assert.ok(stats.unifiedToOld >= 0, 'Should track unified → old routing');
    assert.ok(stats.translationHits >= 0, 'Should track translation hits');
    assert.ok(stats.translationMisses >= 0, 'Should track translation misses');
    assert.ok(stats.translationHitRate, 'Should calculate translation hit rate');
  });

  it('should handle unknown event types gracefully', async () => {
    let receivedOnUnified = false;

    // Subscribe to unified bus
    const unsubscribe = unifiedBus.subscribe('unknown:custom:event', (event) => {
      receivedOnUnified = true;
      assert.equal(event.payload.custom, 'data');
    });

    // Publish custom event (not in translation map)
    globalEventBus.publish('unknown:custom:event', { custom: 'data' });

    // Wait for async routing
    await new Promise(resolve => setTimeout(resolve, 50));

    assert.ok(receivedOnUnified, 'Custom events should pass through untranslated');
    unsubscribe();
  });

  it('should route Agent bus events to unified bus', async () => {
    let receivedOnUnified = false;

    // Subscribe to unified bus (agent events translate to perception:agent:*)
    const unsubscribe = unifiedBus.subscribe('perception:agent:pattern', (event) => {
      receivedOnUnified = true;
    });

    // Publish on Agent bus
    agentBus.emit('agent:pattern:detected', {
      type: 'agent:pattern:detected',
      payload: { patternId: 'test-pattern' },
    });

    // Wait for async routing
    await new Promise(resolve => setTimeout(resolve, 50));

    assert.ok(receivedOnUnified, 'Agent events should be routed to unified bus');
    unsubscribe();
  });
});
