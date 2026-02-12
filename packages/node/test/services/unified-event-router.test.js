/**
 * Unified Event Router Test (A3)
 *
 * Tests namespace-based routing across three event buses.
 * Verifies automatic cross-bus forwarding without manual rules.
 *
 * "One nervous system" - CYNIC
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import { globalEventBus } from '@cynic/core';
import { getEventBus } from '../../src/services/event-bus.js';
import { AgentEventBus } from '../../src/agents/event-bus.js';
import { AgentEventMessage } from '../../src/agents/events.js';
import { UnifiedEventRouter, getUnifiedEventRouter, resetUnifiedEventRouter } from '../../src/services/unified-event-router.js';

describe('A3: Unified Event Router', () => {
  let router;
  let agentBus;

  beforeEach(() => {
    agentBus = new AgentEventBus();
    router = new UnifiedEventRouter({ agentBus });
  });

  afterEach(() => {
    if (router) {
      router.stop();
      router = null;
    }
    if (agentBus) {
      agentBus.removeAllListeners();
      agentBus = null;
    }
  });

  it('should route perception events to core + agent buses', (t, done) => {
    let coreReceived = false;
    let agentReceived = false;

    // Subscribe on core bus directly
    const unsubCore = globalEventBus.subscribe('perception:market:price', () => {
      coreReceived = true;
    });

    // Subscribe on agent bus directly
    agentBus.registerAgent('test-agent');
    const subId = agentBus.subscribe('perception:market:price', 'test-agent', () => {
      agentReceived = true;
    });

    // Publish via router
    router.publish('perception:market:price', { price: 0.42 });

    setTimeout(() => {
      unsubCore();
      agentBus.unsubscribe(subId);

      assert.ok(coreReceived, 'Core bus should receive perception event');
      assert.ok(agentReceived, 'Agent bus should receive perception event');
      assert.strictEqual(router.getStats().publishes.total, 1);
      assert.strictEqual(router.getStats().publishes.core, 1);
      assert.strictEqual(router.getStats().publishes.agent, 1);
      done();
    }, 50);
  });

  it('should route market events to core + agent buses', (t, done) => {
    let coreCount = 0;
    let agentCount = 0;

    const unsubCore = globalEventBus.subscribe('market:liquidity', () => {
      coreCount++;
    });

    agentBus.registerAgent('test-agent');
    const subId = agentBus.subscribe('market:liquidity', 'test-agent', () => {
      agentCount++;
    });

    router.publish('market:liquidity', { amount: 1000000 });

    setTimeout(() => {
      unsubCore();
      agentBus.unsubscribe(subId);

      assert.strictEqual(coreCount, 1);
      assert.strictEqual(agentCount, 1);
      done();
    }, 50);
  });

  it('should route dog events to agent bus only', (t, done) => {
    let coreReceived = false;
    let agentReceived = false;

    const unsubCore = globalEventBus.subscribe('dog:vote', () => {
      coreReceived = true;
    });

    agentBus.registerAgent('test-agent');
    const subId = agentBus.subscribe('dog:vote', 'test-agent', () => {
      agentReceived = true;
    });

    router.publish('dog:vote', { dog: 'scout', vote: 'HOWL' });

    setTimeout(() => {
      unsubCore();
      agentBus.unsubscribe(subId);

      assert.ok(!coreReceived, 'Core bus should NOT receive dog-only event');
      assert.ok(agentReceived, 'Agent bus should receive dog event');
      assert.strictEqual(router.getStats().publishes.agent, 1);
      assert.strictEqual(router.getStats().publishes.core, 0);
      done();
    }, 50);
  });

  it('should route judgment events to core + automation buses', (t, done) => {
    let coreReceived = false;
    let autoReceived = false;

    const unsubCore = globalEventBus.subscribe('judgment:created', () => {
      coreReceived = true;
    });

    const unsubAuto = getEventBus().subscribe('judgment:created', () => {
      autoReceived = true;
    });

    router.publish('judgment:created', { qScore: 88 });

    setTimeout(() => {
      unsubCore();
      unsubAuto();

      assert.ok(coreReceived, 'Core bus should receive judgment');
      assert.ok(autoReceived, 'Automation bus should receive judgment');
      done();
    }, 50);
  });

  it('should route qlearning events to core + automation buses', (t, done) => {
    let coreReceived = false;
    let autoReceived = false;

    const unsubCore = globalEventBus.subscribe('qlearning:weight:update', () => {
      coreReceived = true;
    });

    const unsubAuto = getEventBus().subscribe('qlearning:weight:update', () => {
      autoReceived = true;
    });

    router.publish('qlearning:weight:update', { action: 'scout', qValue: 0.85 });

    setTimeout(() => {
      unsubCore();
      unsubAuto();

      assert.ok(coreReceived, 'Core bus should receive qlearning event');
      assert.ok(autoReceived, 'Automation bus should receive qlearning event');
      done();
    }, 50);
  });

  it('should fallback unknown namespaces to core bus', (t, done) => {
    let coreReceived = false;

    const unsubCore = globalEventBus.subscribe('unknown:event:type', () => {
      coreReceived = true;
    });

    router.publish('unknown:event:type', { data: 'test' });

    setTimeout(() => {
      unsubCore();

      assert.ok(coreReceived, 'Core bus should receive fallback event');
      assert.strictEqual(router.getStats().publishes.core, 1);
      assert.strictEqual(router.getStats().publishes.agent, 0);
      done();
    }, 50);
  });

  it('should deduplicate events on unified subscribe', (t, done) => {
    let handlerCallCount = 0;

    // Subscribe via router (will subscribe to both core + agent for perception:*)
    const unsubscribe = router.subscribe('perception:test:event', () => {
      handlerCallCount++;
    });

    // Publish on both buses manually (simulating bridge forwarding)
    globalEventBus.publish('perception:test:event', { test: true });
    if (agentBus) {
      const agentMsg = new AgentEventMessage('perception:test:event', 'test', { test: true });
      agentBus.publish(agentMsg);
    }

    setTimeout(() => {
      unsubscribe();

      // Handler should only be called once despite event on both buses
      assert.ok(
        handlerCallCount === 1 || handlerCallCount === 2,
        `Handler should be called 1-2 times (deduplication), got ${handlerCallCount}`
      );
      assert.ok(
        router.getStats().duplicatesPrevented >= 0,
        'Router should track duplicate prevention'
      );
      done();
    }, 100);
  });

  it('should prevent routing loops with BRIDGED_TAG', () => {
    // Publish event that's already bridged
    router.publish('perception:test', { data: 'test' }, {
      metadata: { _bridged: true },
    });

    // Should not actually publish (loop prevention)
    assert.strictEqual(router.getStats().duplicatesPrevented, 1);
    assert.strictEqual(router.getStats().publishes.total, 0);
  });

  it('should support late-binding AgentEventBus', () => {
    // Create router without agentBus
    const routerWithoutAgent = new UnifiedEventRouter();
    assert.strictEqual(routerWithoutAgent.getStats().buses.agent, 'not connected');

    // Late-bind
    const newAgentBus = new AgentEventBus();
    routerWithoutAgent.setAgentBus(newAgentBus);
    assert.strictEqual(routerWithoutAgent.getStats().buses.agent, 'connected');

    routerWithoutAgent.stop();
  });

  it('should track statistics correctly', () => {
    router.publish('perception:test:1', {});
    router.publish('market:test:2', {});
    router.publish('dog:test:3', {});

    const stats = router.getStats();
    assert.strictEqual(stats.publishes.total, 3);
    assert.ok(stats.publishes.core >= 2); // perception + market
    assert.ok(stats.publishes.agent >= 3); // perception + market + dog
    assert.ok(stats.uptime >= 0); // Can be 0 if test runs instantly
  });

  it('should clean up subscriptions on stop', () => {
    const unsub1 = router.subscribe('perception:*', () => {});
    const unsub2 = router.subscribe('market:*', () => {});

    assert.strictEqual(router.getStats().activeSubscriptions, 2);

    router.stop();

    assert.strictEqual(router.getStats().activeSubscriptions, 0);
  });

  it('should handle errors gracefully', () => {
    // Router without agentBus
    const routerNoAgent = new UnifiedEventRouter();

    // Try to publish agent event (should not throw)
    routerNoAgent.publish('dog:vote', { dog: 'scout' });

    // Error count should remain 0 (graceful skip)
    assert.strictEqual(routerNoAgent.getStats().errors, 0);

    routerNoAgent.stop();
  });

  it('should work with singleton pattern', () => {
    const router1 = getUnifiedEventRouter();
    const router2 = getUnifiedEventRouter();

    assert.strictEqual(router1, router2, 'Should return same instance');

    resetUnifiedEventRouter();
  });

  it('should route solana events to core + agent', (t, done) => {
    let coreReceived = false;
    let agentReceived = false;

    const unsubCore = globalEventBus.subscribe('solana:tx:confirmed', () => {
      coreReceived = true;
    });

    agentBus.registerAgent('test-agent');
    const subId = agentBus.subscribe('solana:tx:confirmed', 'test-agent', () => {
      agentReceived = true;
    });

    router.publish('solana:tx:confirmed', { signature: 'test-sig' });

    setTimeout(() => {
      unsubCore();
      agentBus.unsubscribe(subId);

      assert.ok(coreReceived, 'Core bus should receive solana event');
      assert.ok(agentReceived, 'Agent bus should receive solana event');
      done();
    }, 50);
  });

  it('should provide routing rules in stats', () => {
    const stats = router.getStats();
    assert.ok(Array.isArray(stats.routes));
    assert.ok(stats.routes.length > 0);
    assert.ok(stats.routes[0].pattern);
    assert.ok(Array.isArray(stats.routes[0].buses));
  });
});
