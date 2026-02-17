/**
 * Tests for UnifiedEventBus
 *
 * Validates Phase 1, Day 1, Step 2 implementation.
 */

'use strict';

import { describe, it, before, after } from 'node:test';
import assert from 'node:assert/strict';
import {
  UnifiedEventBus,
  UnifiedEvent,
  EventCategory,
  EventPriority,
  getUnifiedEventBus,
  createUnifiedEventBus,
} from '../../src/bus/unified-event-bus.js';

describe('UnifiedEvent', () => {
  it('should create event with required fields', () => {
    const event = new UnifiedEvent('perception:human:state', { energy: 0.8 });

    assert.ok(event.id);
    assert.equal(event.type, 'perception:human:state');
    assert.equal(event.payload.energy, 0.8);
    assert.ok(event.timestamp);
    assert.equal(event.source, 'unknown');
    assert.equal(event.target, '*');
    assert.equal(event.priority, EventPriority.NORMAL);
  });

  it('should create event with options', () => {
    const event = new UnifiedEvent(
      'judgment:created',
      { qScore: 88 },
      {
        source: 'Judge',
        target: 'Router',
        priority: EventPriority.HIGH,
        correlationId: 'req-123',
        metadata: { domain: 'HUMAN' },
      }
    );

    assert.equal(event.source, 'Judge');
    assert.equal(event.target, 'Router');
    assert.equal(event.priority, EventPriority.HIGH);
    assert.equal(event.correlationId, 'req-123');
    assert.equal(event.metadata.domain, 'HUMAN');
  });

  it('should extract category from event type', () => {
    const event1 = new UnifiedEvent('perception:human:state', {});
    assert.equal(event1.metadata.category, 'perception');

    const event2 = new UnifiedEvent('judgment:created', {});
    assert.equal(event2.metadata.category, 'judgment');

    const event3 = new UnifiedEvent('system:component:ready', {});
    assert.equal(event3.metadata.category, 'system');
  });

  it('should create reply event', () => {
    const request = new UnifiedEvent('decision:consensus:request', {
      question: 'Approve deployment?',
    }, { source: 'Deployer' });

    const reply = request.reply('decision:consensus:response', {
      vote: 'approve',
    }, { source: 'Guardian' });

    assert.equal(reply.correlationId, request.id);
    assert.equal(reply.causationId, request.id);
    assert.equal(reply.target, 'Deployer'); // Reply to sender
  });

  it('should create caused-by event', () => {
    const trigger = new UnifiedEvent('perception:human:state', { energy: 0.2 });
    const judgment = trigger.causedBy('judgment:created', { qScore: 45 });

    assert.equal(judgment.causationId, trigger.id);
  });

  it('should check if event targets component', () => {
    const broadcast = new UnifiedEvent('perception:human:state', {});
    assert.ok(broadcast.targetsComponent('Judge'));
    assert.ok(broadcast.targetsComponent('Router'));

    const targeted = new UnifiedEvent('decision:code', {}, { target: 'CodeActor' });
    assert.ok(targeted.targetsComponent('CodeActor'));
    assert.ok(!targeted.targetsComponent('Judge'));
  });

  it('should check if event is high priority', () => {
    const normal = new UnifiedEvent('perception:human:state', {});
    assert.ok(!normal.isHighPriority());

    const high = new UnifiedEvent('system:component:error', {}, {
      priority: EventPriority.HIGH,
    });
    assert.ok(high.isHighPriority());

    const critical = new UnifiedEvent('system:component:error', {}, {
      priority: EventPriority.CRITICAL,
    });
    assert.ok(critical.isHighPriority());
  });

  it('should serialize to JSON', () => {
    const event = new UnifiedEvent('judgment:created', { qScore: 88 }, {
      source: 'Judge',
      metadata: { domain: 'HUMAN' },
    });

    const json = event.toJSON();

    assert.ok(json.id);
    assert.equal(json.type, 'judgment:created');
    assert.equal(json.payload.qScore, 88);
    assert.equal(json.source, 'Judge');
    assert.equal(json.metadata.domain, 'HUMAN');
  });

  it('should deserialize from JSON', () => {
    const original = new UnifiedEvent('learning:qlearning:weight-update', {
      weights: [0.5, 0.3],
    });

    const json = original.toJSON();
    const restored = UnifiedEvent.fromJSON(json);

    assert.equal(restored.id, original.id);
    assert.equal(restored.type, original.type);
    assert.equal(restored.timestamp, original.timestamp);
    assert.deepEqual(restored.payload, original.payload);
  });

  it('should calculate event age', async () => {
    const event = new UnifiedEvent('perception:human:state', {});
    await new Promise(resolve => setTimeout(resolve, 10));
    assert.ok(event.age() >= 10);
  });
});

describe('UnifiedEventBus', () => {
  let bus;

  before(() => {
    bus = createUnifiedEventBus();
  });

  after(() => {
    bus.stop();
  });

  it('should publish and subscribe to specific event', (t, done) => {
    const payload = { energy: 0.8 };

    bus.subscribe('perception:human:state', (event) => {
      assert.equal(event.type, 'perception:human:state');
      assert.equal(event.payload.energy, 0.8);
      done();
    });

    bus.publish('perception:human:state', payload, { source: 'HumanPerceiver' });
  });

  it('should support wildcard subscriptions (category:*)', (t, done) => {
    let eventsReceived = 0;

    bus.subscribe('perception:*', (event) => {
      eventsReceived++;
      assert.ok(event.type.startsWith('perception:'));

      if (eventsReceived === 2) {
        done();
      }
    });

    bus.publish('perception:human:state', { energy: 0.8 });
    bus.publish('perception:solana:block', { slot: 12345 });
  });

  it('should support subcategory wildcards (category:sub:*)', (t, done) => {
    bus.subscribe('perception:human:*', (event) => {
      assert.ok(event.type.startsWith('perception:human:'));
      done();
    });

    bus.publish('perception:human:state', { energy: 0.8 });
  });

  it('should support catch-all wildcard (*)', (t, done) => {
    bus.subscribe('*', (event) => {
      assert.ok(event.id);
      done();
    });

    bus.publish('judgment:created', { qScore: 88 });
  });

  it('should unsubscribe correctly', async () => {
    const bus2 = createUnifiedEventBus();
    let callCount = 0;

    const unsubscribe = bus2.subscribe('learning:qlearning:weight-update', () => {
      callCount++;
    });

    bus2.publish('learning:qlearning:weight-update', {});
    await new Promise(resolve => setTimeout(resolve, 10)); // Wait for async handlers
    assert.equal(callCount, 1);

    unsubscribe();

    bus2.publish('learning:qlearning:weight-update', {});
    await new Promise(resolve => setTimeout(resolve, 10));
    assert.equal(callCount, 1); // Should not increase

    bus2.stop();
  });

  it('should subscribe once', async () => {
    const bus2 = createUnifiedEventBus();
    let callCount = 0;

    bus2.subscribeOnce('action:deploy:started', () => {
      callCount++;
    });

    bus2.publish('action:deploy:started', {});
    await new Promise(resolve => setTimeout(resolve, 10));
    bus2.publish('action:deploy:started', {});
    await new Promise(resolve => setTimeout(resolve, 10));

    assert.equal(callCount, 1);
    bus2.stop();
  });

  it('should store events in history', () => {
    const bus2 = createUnifiedEventBus({ historyLimit: 10 });

    bus2.publish('perception:human:state', { energy: 0.8 });
    bus2.publish('judgment:created', { qScore: 88 });

    const history = bus2.getHistory();
    assert.ok(history.length >= 2);

    bus2.stop();
  });

  it('should filter history by type', () => {
    const bus2 = createUnifiedEventBus();

    bus2.publish('perception:human:state', { energy: 0.8 });
    bus2.publish('perception:solana:block', { slot: 12345 });
    bus2.publish('judgment:created', { qScore: 88 });

    const filtered = bus2.getHistory({ type: 'judgment:created' });
    assert.equal(filtered.length, 1);
    assert.equal(filtered[0].type, 'judgment:created');

    bus2.stop();
  });

  it('should filter history by category', () => {
    const bus2 = createUnifiedEventBus();

    bus2.publish('perception:human:state', { energy: 0.8 });
    bus2.publish('perception:solana:block', { slot: 12345 });
    bus2.publish('judgment:created', { qScore: 88 });

    const filtered = bus2.getHistory({ category: 'perception' });
    assert.equal(filtered.length, 2);

    bus2.stop();
  });

  it('should filter history by source', () => {
    const bus2 = createUnifiedEventBus();

    bus2.publish('perception:human:state', { energy: 0.8 }, { source: 'HumanPerceiver' });
    bus2.publish('perception:solana:block', { slot: 12345 }, { source: 'SolanaWatcher' });

    const filtered = bus2.getHistory({ source: 'HumanPerceiver' });
    assert.equal(filtered.length, 1);
    assert.equal(filtered[0].source, 'HumanPerceiver');

    bus2.stop();
  });

  it('should filter history by time', async () => {
    const bus2 = createUnifiedEventBus();

    const before = Date.now();
    await new Promise(resolve => setTimeout(resolve, 10));

    bus2.publish('perception:human:state', { energy: 0.8 });

    const filtered = bus2.getHistory({ since: before });
    assert.equal(filtered.length, 1);

    bus2.stop();
  });

  it('should limit history results', () => {
    const bus2 = createUnifiedEventBus();

    bus2.publish('perception:human:state', { energy: 0.8 });
    bus2.publish('judgment:created', { qScore: 88 });
    bus2.publish('decision:code', { approved: true });

    const limited = bus2.getHistory({ limit: 2 });
    assert.equal(limited.length, 2);

    bus2.stop();
  });

  it('should support request/reply pattern', async () => {
    const bus2 = createUnifiedEventBus();

    // Set up reply handler
    bus2.subscribe('decision:consensus:request', (event) => {
      const reply = event.reply('decision:consensus:response', {
        vote: 'approve',
        reason: 'Looks good',
      });

      bus2.publish(reply.type, reply.payload, {
        id: reply.id,
        correlationId: reply.correlationId,
        source: reply.source,
        target: reply.target,
      });
    });

    // Send request and wait for reply
    const reply = await bus2.request('decision:consensus:request', {
      question: 'Approve deployment?',
    }, {
      source: 'Deployer',
      replyType: 'decision:consensus:response',
    });

    assert.equal(reply.payload.vote, 'approve');

    bus2.stop();
  });

  it('should timeout on unanswered request', async () => {
    const bus2 = createUnifiedEventBus();

    await assert.rejects(
      async () => {
        await bus2.request('decision:consensus:request', {}, {
          timeout: 100,
        });
      },
      /Request timeout/
    );

    bus2.stop();
  });

  it('should run middlewares', (t, done) => {
    const bus2 = createUnifiedEventBus();
    let middlewareCalled = false;

    bus2.use((event) => {
      middlewareCalled = true;
      assert.equal(event.type, 'perception:human:state');
    });

    bus2.subscribe('perception:human:state', () => {
      assert.ok(middlewareCalled);
      bus2.stop();
      done();
    });

    bus2.publish('perception:human:state', { energy: 0.8 });
  });

  it('should block event if middleware returns false', () => {
    const bus2 = createUnifiedEventBus();
    let eventReceived = false;

    bus2.use((event) => {
      if (event.type === 'judgment:created') {
        return false; // Block this event
      }
    });

    bus2.subscribe('judgment:created', () => {
      eventReceived = true;
    });

    bus2.publish('judgment:created', { qScore: 88 });

    assert.ok(!eventReceived);
    bus2.stop();
  });

  it('should track subscription stats', () => {
    const bus2 = createUnifiedEventBus();

    bus2.subscribe('perception:human:state', () => {});
    bus2.subscribe('perception:human:state', () => {});
    bus2.subscribe('judgment:created', () => {});

    const stats = bus2.getSubscriptionStats();

    assert.equal(stats.totalSubscriptions, 3);
    assert.equal(stats.byType['perception:human:state'], 2);
    assert.equal(stats.byType['judgment:created'], 1);
    assert.equal(stats.byCategory['perception'], 2);
    assert.equal(stats.byCategory['judgment'], 1);

    bus2.stop();
  });

  it('should track health metrics', () => {
    const bus2 = createUnifiedEventBus();

    bus2.publish('perception:human:state', { energy: 0.8 });
    bus2.publish('judgment:created', { qScore: 88 });

    const health = bus2.getHealthMetrics();

    assert.ok(health.eventsPublished >= 2);
    assert.ok(health.historySize >= 2);
    assert.ok(health.p50Latency >= 0);
    assert.ok(health.p95Latency >= 0);

    bus2.stop();
  });

  it('should clear all subscriptions and history', () => {
    const bus2 = createUnifiedEventBus();

    bus2.subscribe('perception:human:state', () => {});
    bus2.publish('perception:human:state', { energy: 0.8 });

    bus2.clear();

    const stats = bus2.getSubscriptionStats();
    const history = bus2.getHistory();

    assert.equal(stats.totalSubscriptions, 0);
    assert.equal(history.length, 0);

    bus2.stop();
  });

  it('should handle subscriber errors gracefully', (t, done) => {
    const bus2 = createUnifiedEventBus();
    let errorEventReceived = false;

    // Subscribe to error events
    bus2.subscribe('system:component:error', (event) => {
      errorEventReceived = true;
      assert.equal(event.payload.eventType, 'perception:human:state');
    });

    // Subscribe with handler that throws
    bus2.subscribe('perception:human:state', () => {
      throw new Error('Handler error');
    });

    // Publish event
    bus2.publish('perception:human:state', { energy: 0.8 });

    // Give time for error event to emit
    setTimeout(() => {
      assert.ok(errorEventReceived);
      bus2.stop();
      done();
    }, 100);
  });
});

describe('UnifiedEventBus - Singleton', () => {
  it('should return same instance from getUnifiedEventBus()', () => {
    const bus1 = getUnifiedEventBus();
    const bus2 = getUnifiedEventBus();

    assert.strictEqual(bus1, bus2);

    bus1.stop();
  });

  it('should create new instance from createUnifiedEventBus()', () => {
    const bus1 = createUnifiedEventBus();
    const bus2 = createUnifiedEventBus();

    assert.notStrictEqual(bus1, bus2);

    bus1.stop();
    bus2.stop();
  });
});

describe('UnifiedEventBus - EventCategory', () => {
  it('should define 7 event categories (6 core + 1 transcendent)', () => {
    assert.equal(EventCategory.PERCEPTION, 'perception');
    assert.equal(EventCategory.JUDGMENT, 'judgment');
    assert.equal(EventCategory.DECISION, 'decision');
    assert.equal(EventCategory.ACTION, 'action');
    assert.equal(EventCategory.LEARNING, 'learning');
    assert.equal(EventCategory.SYSTEM, 'system');
    assert.equal(EventCategory.EMERGENCE, 'emergence'); // THE_UNNAMEABLE
  });
});

describe('UnifiedEventBus - EventPriority', () => {
  it('should define 4 priority levels', () => {
    assert.equal(EventPriority.CRITICAL, 'critical');
    assert.equal(EventPriority.HIGH, 'high');
    assert.equal(EventPriority.NORMAL, 'normal');
    assert.equal(EventPriority.LOW, 'low');
  });
});

describe('UnifiedEventBus - Parallel Execution', () => {
  it('should execute listeners in parallel (inherited from ParallelEventBus)', async () => {
    const bus = createUnifiedEventBus();
    const startTime = Date.now();
    let listener1Done = false;
    let listener2Done = false;

    // Listener 1: 50ms delay
    bus.subscribe('perception:human:state', async () => {
      await new Promise(resolve => setTimeout(resolve, 50));
      listener1Done = true;
    });

    // Listener 2: 50ms delay
    bus.subscribe('perception:human:state', async () => {
      await new Promise(resolve => setTimeout(resolve, 50));
      listener2Done = true;
    });

    bus.publish('perception:human:state', { energy: 0.8 });

    // Wait for both listeners to complete
    await new Promise(resolve => setTimeout(resolve, 100));

    const elapsed = Date.now() - startTime;
    assert.ok(listener1Done);
    assert.ok(listener2Done);
    // If sequential, would take 100ms. If parallel, ~50ms.
    assert.ok(elapsed < 80); // Allow some overhead

    bus.stop();
  });
});
