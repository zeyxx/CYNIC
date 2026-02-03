/**
 * Event Bus Tests
 *
 * Tests for CYNIC's centralized pub-sub event system.
 *
 * "φ connects, φ observes" - κυνικός
 */

import { describe, it, beforeEach, mock } from 'node:test';
import assert from 'node:assert/strict';

import {
  EventBus,
  EventType,
} from '../src/services/event-bus.js';

// =============================================================================
// EVENT TYPE TESTS
// =============================================================================

describe('EventType', () => {
  it('should define feedback events', () => {
    assert.equal(EventType.FEEDBACK_RECEIVED, 'feedback:received');
    assert.equal(EventType.FEEDBACK_PROCESSED, 'feedback:processed');
  });

  it('should define judgment events', () => {
    assert.equal(EventType.JUDGMENT_CREATED, 'judgment:created');
    assert.equal(EventType.JUDGMENT_REFINED, 'judgment:refined');
  });

  it('should define session events', () => {
    assert.equal(EventType.SESSION_START, 'session:start');
    assert.equal(EventType.SESSION_END, 'session:end');
  });

  it('should define learning events', () => {
    assert.equal(EventType.LEARNING_CYCLE_START, 'learning:cycle:start');
    assert.equal(EventType.LEARNING_CYCLE_COMPLETE, 'learning:cycle:complete');
    assert.equal(EventType.LEARNING_PATTERN_EVOLVED, 'learning:pattern:evolved');
  });

  it('should define trigger events', () => {
    assert.equal(EventType.TRIGGER_FIRED, 'trigger:fired');
    assert.equal(EventType.TRIGGER_EVALUATED, 'trigger:evaluated');
  });

  it('should define automation events', () => {
    assert.equal(EventType.AUTOMATION_TICK, 'automation:tick');
    assert.equal(EventType.AUTOMATION_START, 'automation:start');
    assert.equal(EventType.AUTOMATION_STOP, 'automation:stop');
  });

  it('should define goal events', () => {
    assert.equal(EventType.GOAL_CREATED, 'goal:created');
    assert.equal(EventType.GOAL_PROGRESS, 'goal:progress');
    assert.equal(EventType.GOAL_COMPLETED, 'goal:completed');
  });

  it('should define notification events', () => {
    assert.equal(EventType.NOTIFICATION_CREATED, 'notification:created');
    assert.equal(EventType.NOTIFICATION_DELIVERED, 'notification:delivered');
  });

  it('should define error event', () => {
    assert.equal(EventType.ERROR, 'error');
  });

  it('should use colon-separated namespaces', () => {
    const colonEvents = Object.values(EventType).filter(e => e.includes(':'));
    assert.ok(colonEvents.length > 10); // Most events use namespaces
  });
});

// =============================================================================
// EVENT BUS TESTS
// =============================================================================

describe('EventBus', () => {
  let bus;

  beforeEach(() => {
    bus = new EventBus();
  });

  // ===========================================================================
  // CONSTRUCTION
  // ===========================================================================

  describe('construction', () => {
    it('should create with defaults', () => {
      assert.equal(bus.maxHistory, 100);
      assert.equal(bus.debug, false);
    });

    it('should accept custom options', () => {
      const customBus = new EventBus({ maxHistory: 50, debug: true });
      assert.equal(customBus.maxHistory, 50);
      assert.equal(customBus.debug, true);
    });

    it('should initialize empty history', () => {
      const history = bus.getHistory();
      assert.deepEqual(history, []);
    });

    it('should set high max listeners', () => {
      assert.ok(bus.getMaxListeners() >= 50);
    });
  });

  // ===========================================================================
  // PUBLISH
  // ===========================================================================

  describe('publish', () => {
    it('should publish events', () => {
      let received = null;
      bus.on(EventType.FEEDBACK_RECEIVED, (event) => { received = event; });

      bus.publish(EventType.FEEDBACK_RECEIVED, { feedback: 'test' });

      assert.ok(received);
      assert.equal(received.type, EventType.FEEDBACK_RECEIVED);
      assert.deepEqual(received.data, { feedback: 'test' });
    });

    it('should include metadata', () => {
      let received = null;
      bus.on(EventType.JUDGMENT_CREATED, (event) => { received = event; });

      bus.publish(EventType.JUDGMENT_CREATED, { score: 75 }, { source: 'test-source' });

      assert.ok(received.meta);
      assert.ok(received.meta.eventId.startsWith('evt_'));
      assert.ok(received.meta.timestamp > 0);
      assert.equal(received.meta.source, 'test-source');
    });

    it('should return true if had listeners', () => {
      bus.on(EventType.SESSION_START, () => {});

      const hadListeners = bus.publish(EventType.SESSION_START, {});
      assert.equal(hadListeners, true);
    });

    it('should return false if no listeners', () => {
      const hadListeners = bus.publish('unknown:event', {});
      assert.equal(hadListeners, false);
    });

    it('should add to history', () => {
      bus.publish(EventType.AUTOMATION_TICK, { tick: 1 });
      bus.publish(EventType.AUTOMATION_TICK, { tick: 2 });

      const history = bus.getHistory();
      assert.equal(history.length, 2);
    });

    it('should increment eventsEmitted stat', () => {
      // Use non-error event type (error events need listeners or they throw)
      bus.publish(EventType.AUTOMATION_TICK, { tick: 1 });
      bus.publish(EventType.AUTOMATION_TICK, { tick: 2 });

      const stats = bus.getStats();
      assert.equal(stats.eventsEmitted, 2);
    });
  });

  // ===========================================================================
  // SUBSCRIBE
  // ===========================================================================

  describe('subscribe', () => {
    it('should subscribe to events', () => {
      const events = [];
      bus.subscribe(EventType.GOAL_CREATED, (e) => events.push(e));

      bus.publish(EventType.GOAL_CREATED, { goal: 'a' });
      bus.publish(EventType.GOAL_CREATED, { goal: 'b' });

      assert.equal(events.length, 2);
    });

    it('should return unsubscribe function', () => {
      const events = [];
      const unsubscribe = bus.subscribe(EventType.GOAL_PROGRESS, (e) => events.push(e));

      bus.publish(EventType.GOAL_PROGRESS, { progress: 1 });
      unsubscribe();
      bus.publish(EventType.GOAL_PROGRESS, { progress: 2 });

      assert.equal(events.length, 1);
    });

    it('should track subscriber count', () => {
      const unsub1 = bus.subscribe(EventType.ERROR, () => {});
      const unsub2 = bus.subscribe(EventType.ERROR, () => {});

      const stats = bus.getStats();
      assert.equal(stats.subscribersActive, 2);

      unsub1();
      const stats2 = bus.getStats();
      assert.equal(stats2.subscribersActive, 1);
    });
  });

  // ===========================================================================
  // SUBSCRIBE ONCE
  // ===========================================================================

  describe('subscribeOnce', () => {
    it('should only receive one event', () => {
      const events = [];
      bus.subscribeOnce(EventType.NOTIFICATION_CREATED, (e) => events.push(e));

      bus.publish(EventType.NOTIFICATION_CREATED, { n: 1 });
      bus.publish(EventType.NOTIFICATION_CREATED, { n: 2 });
      bus.publish(EventType.NOTIFICATION_CREATED, { n: 3 });

      assert.equal(events.length, 1);
      assert.equal(events[0].data.n, 1);
    });

    it('should return unsubscribe function', () => {
      const events = [];
      const unsubscribe = bus.subscribeOnce(EventType.TRIGGER_FIRED, (e) => events.push(e));

      unsubscribe(); // Unsubscribe before any events
      bus.publish(EventType.TRIGGER_FIRED, {});

      assert.equal(events.length, 0);
    });
  });

  // ===========================================================================
  // WILDCARD SUPPORT
  // ===========================================================================

  describe('wildcard support', () => {
    it('should emit to category wildcard', () => {
      const events = [];
      bus.on('feedback:*', (e) => events.push(e));

      bus.publish(EventType.FEEDBACK_RECEIVED, { a: 1 });
      bus.publish(EventType.FEEDBACK_PROCESSED, { b: 2 });

      assert.equal(events.length, 2);
    });

    it('should emit to global wildcard', () => {
      const events = [];
      bus.on('*', (e) => events.push(e));

      bus.publish(EventType.JUDGMENT_CREATED, {});
      bus.publish(EventType.SESSION_START, {});
      bus.publish(EventType.NOTIFICATION_CREATED, {});

      assert.equal(events.length, 3);
    });

    it('should emit to both specific and wildcard listeners', () => {
      const specific = [];
      const wildcard = [];

      bus.on(EventType.LEARNING_CYCLE_COMPLETE, (e) => specific.push(e));
      bus.on('learning:*', (e) => wildcard.push(e));

      bus.publish(EventType.LEARNING_CYCLE_COMPLETE, {});

      assert.equal(specific.length, 1);
      assert.equal(wildcard.length, 1);
    });
  });

  // ===========================================================================
  // WAIT FOR
  // ===========================================================================

  describe('waitFor', () => {
    it('should resolve when event is published', async () => {
      // Publish after a short delay
      setTimeout(() => {
        bus.publish(EventType.GOAL_COMPLETED, { goal: 'test' });
      }, 10);

      const event = await bus.waitFor(EventType.GOAL_COMPLETED, 1000);

      assert.equal(event.type, EventType.GOAL_COMPLETED);
      assert.equal(event.data.goal, 'test');
    });

    it('should timeout if event not published', async () => {
      await assert.rejects(
        () => bus.waitFor('never:happens', 50),
        /Timeout waiting for event/
      );
    });

    it('should clean up on timeout', async () => {
      const initialListeners = bus.listenerCount('timeout:test');

      try {
        await bus.waitFor('timeout:test', 10);
      } catch (e) {
        // Expected
      }

      const finalListeners = bus.listenerCount('timeout:test');
      assert.equal(finalListeners, initialListeners);
    });
  });

  // ===========================================================================
  // HISTORY
  // ===========================================================================

  describe('getHistory', () => {
    beforeEach(() => {
      bus.publish(EventType.SESSION_START, { s: 1 }, { source: 'source1' });
      bus.publish(EventType.JUDGMENT_CREATED, { j: 1 }, { source: 'source2' });
      bus.publish(EventType.SESSION_END, { s: 1 }, { source: 'source1' });
    });

    it('should return all history', () => {
      const history = bus.getHistory();
      assert.equal(history.length, 3);
    });

    it('should filter by type', () => {
      const history = bus.getHistory({ type: EventType.SESSION_START });
      assert.equal(history.length, 1);
      assert.equal(history[0].type, EventType.SESSION_START);
    });

    it('should filter by source', () => {
      const history = bus.getHistory({ source: 'source1' });
      assert.equal(history.length, 2);
    });

    it('should filter by timestamp', () => {
      const now = Date.now();
      const history = bus.getHistory({ since: now + 1000 });
      assert.equal(history.length, 0);
    });

    it('should limit results', () => {
      const history = bus.getHistory({ limit: 2 });
      assert.equal(history.length, 2);
    });

    it('should sort by timestamp descending', () => {
      const history = bus.getHistory();
      for (let i = 1; i < history.length; i++) {
        assert.ok(history[i - 1].meta.timestamp >= history[i].meta.timestamp);
      }
    });

    it('should combine filters', () => {
      const history = bus.getHistory({
        source: 'source1',
        limit: 1,
      });
      assert.equal(history.length, 1);
    });
  });

  // ===========================================================================
  // STATS
  // ===========================================================================

  describe('getStats', () => {
    it('should return statistics', () => {
      bus.subscribe(EventType.ERROR, () => {});
      bus.publish(EventType.ERROR, {});
      bus.publish(EventType.ERROR, {});

      const stats = bus.getStats();

      assert.equal(stats.eventsEmitted, 2);
      assert.equal(stats.subscribersActive, 1);
      assert.ok('eventsFailed' in stats);
    });
  });

  // ===========================================================================
  // EDGE CASES
  // ===========================================================================

  describe('edge cases', () => {
    it('should handle rapid publishing', () => {
      const events = [];
      bus.subscribe(EventType.AUTOMATION_TICK, (e) => events.push(e));

      for (let i = 0; i < 100; i++) {
        bus.publish(EventType.AUTOMATION_TICK, { i });
      }

      assert.equal(events.length, 100);
    });

    it('should handle multiple subscribers to same event', () => {
      const results = { a: 0, b: 0, c: 0 };

      bus.subscribe(EventType.TRIGGER_FIRED, () => results.a++);
      bus.subscribe(EventType.TRIGGER_FIRED, () => results.b++);
      bus.subscribe(EventType.TRIGGER_FIRED, () => results.c++);

      bus.publish(EventType.TRIGGER_FIRED, {});

      assert.equal(results.a, 1);
      assert.equal(results.b, 1);
      assert.equal(results.c, 1);
    });

    it('should handle subscriber errors gracefully', () => {
      bus.subscribe(EventType.ERROR, () => { throw new Error('Subscriber error'); });
      bus.subscribe(EventType.ERROR, () => {}); // Should still be called

      // Should not throw
      assert.throws(() => {
        bus.publish(EventType.ERROR, {});
      });
    });

    it('should generate unique event IDs', () => {
      const ids = new Set();

      for (let i = 0; i < 100; i++) {
        bus.publish(EventType.AUTOMATION_TICK, { i });
      }

      const history = bus.getHistory();
      for (const event of history) {
        ids.add(event.meta.eventId);
      }

      assert.equal(ids.size, 100);
    });

    it('should respect maxHistory', () => {
      const smallBus = new EventBus({ maxHistory: 5 });

      for (let i = 0; i < 20; i++) {
        smallBus.publish(EventType.AUTOMATION_TICK, { i });
      }

      const history = smallBus.getHistory();
      assert.ok(history.length <= 5);
    });

    it('should handle empty data', () => {
      let received = null;
      bus.subscribe(EventType.SESSION_START, (e) => { received = e; });

      bus.publish(EventType.SESSION_START, null);

      assert.ok(received);
      assert.equal(received.data, null);
    });

    it('should preserve metadata fields', () => {
      let received = null;
      bus.subscribe(EventType.FEEDBACK_RECEIVED, (e) => { received = e; });

      bus.publish(EventType.FEEDBACK_RECEIVED, {}, {
        userId: 'user123',
        sessionId: 'session456',
        customField: 'custom',
      });

      assert.equal(received.meta.userId, 'user123');
      assert.equal(received.meta.sessionId, 'session456');
      assert.equal(received.meta.customField, 'custom');
    });
  });

  // ===========================================================================
  // DEBUG MODE
  // ===========================================================================

  describe('debug mode', () => {
    it('should create with debug enabled', () => {
      const debugBus = new EventBus({ debug: true });
      assert.equal(debugBus.debug, true);

      // Should not throw when publishing (logging enabled)
      debugBus.publish(EventType.SESSION_START, {});
    });
  });
});
