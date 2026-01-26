/**
 * @cynic/core - Event Bus System Tests
 *
 * Tests the nervous system of CYNIC:
 * - Event Bus (pub/sub, wildcards, request/reply)
 * - Connector (forwarding, namespaces)
 * - Service Registry (DI, layers, interfaces)
 * - Interfaces (validation, layer enforcement)
 *
 * "The pack communicates as one" - κυνικός
 *
 * @module @cynic/core/test/bus
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';

// Event Bus
import {
  CYNICEventBus,
  CYNICEvent,
  EventType,
  globalEventBus,
  publish,
  subscribe,
} from '../src/bus/event-bus.js';

// Connector
import {
  connectToBus,
  withBusConnectivity,
  subscribeToComponent,
  subscribeToAllFromComponent,
  EVENT_NAMESPACES,
} from '../src/bus/connector.js';

// Service Registry
import {
  ServiceRegistry,
  globalServiceRegistry,
  registerService,
  getService,
} from '../src/bus/service-registry.js';

// Interfaces
import {
  Layer,
  IRepository,
  ICache,
  IEventBus,
  implements_,
  assertImplements,
  isLayerCallAllowed,
} from '../src/bus/interfaces.js';

// =============================================================================
// CYNIC EVENT TESTS
// =============================================================================

describe('CYNICEvent', () => {
  describe('Construction', () => {
    it('should create event with type and payload', () => {
      const event = new CYNICEvent('test:event', { data: 'hello' });

      assert.strictEqual(event.type, 'test:event');
      assert.deepStrictEqual(event.payload, { data: 'hello' });
      assert.ok(event.id, 'should have an id');
      assert.ok(event.timestamp, 'should have a timestamp');
      assert.strictEqual(event.source, 'unknown');
      assert.strictEqual(event.correlationId, null);
    });

    it('should accept custom options', () => {
      const event = new CYNICEvent('test:event', {}, {
        id: 'custom-id',
        source: 'test-source',
        correlationId: 'corr-123',
        metadata: { custom: true },
      });

      assert.strictEqual(event.id, 'custom-id');
      assert.strictEqual(event.source, 'test-source');
      assert.strictEqual(event.correlationId, 'corr-123');
      assert.deepStrictEqual(event.metadata, { custom: true });
    });
  });

  describe('reply()', () => {
    it('should create correlated reply event', () => {
      const original = new CYNICEvent('request', { query: 'test' });
      const reply = original.reply('response', { answer: 42 });

      assert.strictEqual(reply.type, 'response');
      assert.deepStrictEqual(reply.payload, { answer: 42 });
      assert.strictEqual(reply.correlationId, original.id);
    });
  });

  describe('toJSON()', () => {
    it('should serialize to JSON', () => {
      const event = new CYNICEvent('test', { data: 1 }, { source: 'test' });
      const json = event.toJSON();

      assert.strictEqual(json.type, 'test');
      assert.deepStrictEqual(json.payload, { data: 1 });
      assert.strictEqual(json.source, 'test');
      assert.ok(json.id);
      assert.ok(json.timestamp);
    });
  });
});

// =============================================================================
// EVENT BUS TESTS
// =============================================================================

describe('CYNICEventBus', () => {
  let bus;

  beforeEach(() => {
    bus = new CYNICEventBus();
  });

  afterEach(() => {
    bus.clear();
  });

  describe('publish()', () => {
    it('should publish event and return it', () => {
      const event = bus.publish('test:event', { value: 42 });

      assert.strictEqual(event.type, 'test:event');
      assert.deepStrictEqual(event.payload, { value: 42 });
    });

    it('should emit to specific type listeners', (t, done) => {
      bus.subscribe('test:specific', (event) => {
        assert.strictEqual(event.payload.data, 'hello');
        done();
      });

      bus.publish('test:specific', { data: 'hello' });
    });

    it('should add event to history', () => {
      bus.publish('test:history', { num: 1 });
      bus.publish('test:history', { num: 2 });

      const history = bus.getHistory();
      assert.strictEqual(history.length, 2);
      assert.strictEqual(history[0].payload.num, 1);
      assert.strictEqual(history[1].payload.num, 2);
    });
  });

  describe('subscribe()', () => {
    it('should receive published events', (t, done) => {
      bus.subscribe('test:sub', (event) => {
        assert.strictEqual(event.payload.msg, 'hello');
        done();
      });

      bus.publish('test:sub', { msg: 'hello' });
    });

    it('should return unsubscribe function', () => {
      let callCount = 0;
      const unsub = bus.subscribe('test:unsub', () => {
        callCount++;
      });

      bus.publish('test:unsub', {});
      assert.strictEqual(callCount, 1);

      unsub();
      bus.publish('test:unsub', {});
      assert.strictEqual(callCount, 1, 'should not receive after unsubscribe');
    });

    it('should handle multiple subscribers', () => {
      let count1 = 0, count2 = 0;

      bus.subscribe('test:multi', () => { count1++; });
      bus.subscribe('test:multi', () => { count2++; });

      bus.publish('test:multi', {});

      assert.strictEqual(count1, 1);
      assert.strictEqual(count2, 1);
    });

    it('should catch handler errors and continue', () => {
      let secondCalled = false;

      bus.subscribe('test:error', () => {
        throw new Error('Handler error');
      });
      bus.subscribe('test:error', () => {
        secondCalled = true;
      });

      // Should not throw
      bus.publish('test:error', {});
      assert.strictEqual(secondCalled, true);
    });
  });

  describe('Wildcards', () => {
    it('should emit to namespace wildcard listeners', (t, done) => {
      bus.subscribe('test:*', (event) => {
        assert.strictEqual(event.type, 'test:specific');
        done();
      });

      bus.publish('test:specific', {});
    });

    it('should emit to global wildcard listeners', (t, done) => {
      bus.subscribe('*', (event) => {
        assert.strictEqual(event.type, 'any:event');
        done();
      });

      bus.publish('any:event', {});
    });

    it('should not cross-emit to different namespaces', () => {
      let called = false;
      bus.subscribe('other:*', () => {
        called = true;
      });

      bus.publish('test:event', {});
      assert.strictEqual(called, false);
    });
  });

  describe('subscribeOnce()', () => {
    it('should only receive one event', () => {
      let count = 0;
      bus.subscribeOnce('test:once', () => {
        count++;
      });

      bus.publish('test:once', {});
      bus.publish('test:once', {});

      assert.strictEqual(count, 1);
    });
  });

  describe('request()', () => {
    it('should resolve with correlated reply', async () => {
      // Setup responder
      bus.subscribe('test:request', (event) => {
        const reply = event.reply('test:request:reply', { answer: event.payload.question * 2 });
        bus.publish(reply.type, reply.payload, {
          correlationId: event.id,
        });
      });

      const reply = await bus.request('test:request', { question: 21 }, { timeout: 1000 });
      assert.strictEqual(reply.payload.answer, 42);
    });

    it('should timeout if no reply', async () => {
      await assert.rejects(
        () => bus.request('test:no-reply', {}, { timeout: 100 }),
        /Request timeout/
      );
    });
  });

  describe('Middleware', () => {
    it('should run middleware before publish', () => {
      const log = [];

      bus.use((event) => {
        log.push(`middleware:${event.type}`);
      });

      bus.subscribe('test:mw', (event) => {
        log.push(`handler:${event.type}`);
      });

      bus.publish('test:mw', {});

      assert.deepStrictEqual(log, ['middleware:test:mw', 'handler:test:mw']);
    });

    it('should block event if middleware returns false', () => {
      let handlerCalled = false;

      bus.use(() => false); // Block all events

      bus.subscribe('test:blocked', () => {
        handlerCalled = true;
      });

      bus.publish('test:blocked', {});
      assert.strictEqual(handlerCalled, false);
    });

    it('should continue if middleware throws', () => {
      let handlerCalled = false;

      bus.use(() => {
        throw new Error('Middleware error');
      });

      bus.subscribe('test:mw-error', () => {
        handlerCalled = true;
      });

      bus.publish('test:mw-error', {});
      assert.strictEqual(handlerCalled, true);
    });
  });

  describe('History', () => {
    it('should filter history by type', () => {
      bus.publish('type:a', { n: 1 });
      bus.publish('type:b', { n: 2 });
      bus.publish('type:a', { n: 3 });

      const history = bus.getHistory({ type: 'type:a' });
      assert.strictEqual(history.length, 2);
      assert.strictEqual(history[0].payload.n, 1);
      assert.strictEqual(history[1].payload.n, 3);
    });

    it('should filter history by source', () => {
      bus.publish('test', {}, { source: 'src-a' });
      bus.publish('test', {}, { source: 'src-b' });

      const history = bus.getHistory({ source: 'src-a' });
      assert.strictEqual(history.length, 1);
    });

    it('should limit history size', () => {
      const smallBus = new CYNICEventBus({ historyLimit: 3 });

      for (let i = 0; i < 5; i++) {
        smallBus.publish('test', { n: i });
      }

      const history = smallBus.getHistory();
      assert.strictEqual(history.length, 3);
      assert.strictEqual(history[0].payload.n, 2); // Oldest kept
      assert.strictEqual(history[2].payload.n, 4); // Newest
    });

    it('should filter history by since timestamp', () => {
      const t1 = Date.now();
      bus.publish('test', { n: 1 }, { timestamp: t1 - 1000 });
      bus.publish('test', { n: 2 }, { timestamp: t1 });
      bus.publish('test', { n: 3 }, { timestamp: t1 + 1000 });

      const history = bus.getHistory({ since: t1 });
      assert.strictEqual(history.length, 2);
    });

    it('should limit history results', () => {
      for (let i = 0; i < 10; i++) {
        bus.publish('test', { n: i });
      }

      const history = bus.getHistory({ limit: 3 });
      assert.strictEqual(history.length, 3);
      assert.strictEqual(history[0].payload.n, 7); // Last 3
    });
  });

  describe('getStats()', () => {
    it('should return subscription statistics', () => {
      bus.subscribe('type:a', () => {});
      bus.subscribe('type:a', () => {});
      bus.subscribe('type:b', () => {});
      bus.use(() => {});

      const stats = bus.getStats();

      assert.strictEqual(stats.totalSubscriptions, 3);
      assert.strictEqual(stats.byType['type:a'], 2);
      assert.strictEqual(stats.byType['type:b'], 1);
      assert.strictEqual(stats.middlewareCount, 1);
    });
  });

  describe('clear()', () => {
    it('should remove all subscriptions and history', () => {
      bus.subscribe('test', () => {});
      bus.publish('test', {});

      bus.clear();

      const stats = bus.getStats();
      assert.strictEqual(stats.totalSubscriptions, 0);
      assert.strictEqual(stats.historySize, 0);
    });
  });
});

// =============================================================================
// EVENT TYPE CONSTANTS
// =============================================================================

describe('EventType', () => {
  it('should have lifecycle events', () => {
    assert.ok(EventType.COMPONENT_READY);
    assert.ok(EventType.COMPONENT_STOPPED);
    assert.ok(EventType.COMPONENT_ERROR);
  });

  it('should have judgment events', () => {
    assert.ok(EventType.JUDGMENT_REQUESTED);
    assert.ok(EventType.JUDGMENT_CREATED);
    assert.ok(EventType.JUDGMENT_UPDATED);
  });

  it('should have pattern events', () => {
    assert.ok(EventType.PATTERN_DETECTED);
    assert.ok(EventType.PATTERN_LEARNED);
    assert.ok(EventType.ANOMALY_DETECTED);
  });

  it('should follow namespace:action format', () => {
    for (const [key, value] of Object.entries(EventType)) {
      assert.ok(value.includes(':'), `${key} should have namespace:action format`);
    }
  });
});

// =============================================================================
// CONNECTOR TESTS
// =============================================================================

describe('Connector', () => {
  let bus;

  beforeEach(() => {
    bus = new CYNICEventBus();
  });

  afterEach(() => {
    bus.clear();
  });

  describe('connectToBus()', () => {
    it('should forward local events to bus', (t, done) => {
      const emitter = new EventEmitter();

      connectToBus(emitter, { namespace: 'test', bus });

      bus.subscribe('test:local-event', (event) => {
        assert.strictEqual(event.payload.value, 42);
        done();
      });

      emitter.emit('local-event', { value: 42 });
    });

    it('should preserve local emit behavior', (t, done) => {
      const emitter = new EventEmitter();

      connectToBus(emitter, { namespace: 'test', bus });

      emitter.on('local-event', (data) => {
        assert.strictEqual(data.value, 42);
        done();
      });

      emitter.emit('local-event', { value: 42 });
    });

    it('should return disconnect function', () => {
      const emitter = new EventEmitter();
      let busReceived = false;

      const disconnect = connectToBus(emitter, { namespace: 'test', bus });

      bus.subscribe('test:*', () => {
        busReceived = true;
      });

      disconnect();
      emitter.emit('event', {});

      assert.strictEqual(busReceived, false, 'should not forward after disconnect');
    });

    it('should respect events allow list', () => {
      const emitter = new EventEmitter();
      const received = [];

      connectToBus(emitter, {
        namespace: 'test',
        bus,
        events: ['allowed'],
      });

      bus.subscribe('test:*', (event) => {
        received.push(event.type);
      });

      emitter.emit('allowed', {});
      emitter.emit('not-allowed', {});

      assert.deepStrictEqual(received, ['test:allowed']);
    });

    it('should respect exclude list', () => {
      const emitter = new EventEmitter();
      const received = [];

      connectToBus(emitter, {
        namespace: 'test',
        bus,
        exclude: ['excluded'],
      });

      bus.subscribe('test:*', (event) => {
        received.push(event.type);
      });

      emitter.emit('normal', {});
      emitter.emit('excluded', {});

      assert.deepStrictEqual(received, ['test:normal']);
    });

    it('should skip internal Node events', () => {
      const emitter = new EventEmitter();
      let busReceived = false;

      connectToBus(emitter, { namespace: 'test', bus });

      bus.subscribe('test:*', () => {
        busReceived = true;
      });

      emitter.emit('newListener', () => {});
      emitter.emit('removeListener', () => {});

      assert.strictEqual(busReceived, false);
    });
  });

  describe('subscribeToComponent()', () => {
    it('should subscribe to component-namespaced events', (t, done) => {
      subscribeToComponent('mycomp', 'action', (event) => {
        assert.strictEqual(event.payload.data, 'test');
        done();
      }, bus);

      bus.publish('mycomp:action', { data: 'test' });
    });
  });

  describe('subscribeToAllFromComponent()', () => {
    it('should subscribe to all events from component', () => {
      const received = [];

      subscribeToAllFromComponent('mycomp', (event) => {
        received.push(event.type);
      }, bus);

      bus.publish('mycomp:event1', {});
      bus.publish('mycomp:event2', {});
      bus.publish('other:event', {});

      assert.strictEqual(received.length, 2);
      assert.ok(received.includes('mycomp:event1'));
      assert.ok(received.includes('mycomp:event2'));
    });
  });

  describe('EVENT_NAMESPACES', () => {
    it('should have persistence layer mappings', () => {
      assert.strictEqual(EVENT_NAMESPACES.GraphOverlay, 'graph');
      assert.strictEqual(EVENT_NAMESPACES.PoJChain, 'poj');
      assert.strictEqual(EVENT_NAMESPACES.MerkleDAG, 'dag');
    });

    it('should have MCP layer mappings', () => {
      assert.strictEqual(EVENT_NAMESPACES.MetricsService, 'metrics');
      assert.strictEqual(EVENT_NAMESPACES.DiscoveryService, 'discovery');
    });

    it('should have judge layer mappings', () => {
      assert.strictEqual(EVENT_NAMESPACES.LearningService, 'learning');
      assert.strictEqual(EVENT_NAMESPACES.SelfSkeptic, 'skeptic');
    });
  });
});

// =============================================================================
// SERVICE REGISTRY TESTS
// =============================================================================

describe('ServiceRegistry', () => {
  let registry;

  beforeEach(() => {
    registry = new ServiceRegistry();
  });

  describe('register()', () => {
    it('should register a service', () => {
      const service = { name: 'test' };
      registry.register('myService', service);

      assert.ok(registry.has('myService'));
    });

    it('should throw on duplicate registration', () => {
      registry.register('dup', {});

      assert.throws(
        () => registry.register('dup', {}),
        /already registered/
      );
    });

    it('should support chaining', () => {
      const result = registry
        .register('a', {})
        .register('b', {});

      assert.ok(registry.has('a'));
      assert.ok(registry.has('b'));
    });
  });

  describe('get()', () => {
    it('should return registered service', () => {
      const service = { value: 42 };
      registry.register('myService', service);

      const retrieved = registry.get('myService');
      assert.strictEqual(retrieved.value, 42);
    });

    it('should throw for unknown service', () => {
      assert.throws(
        () => registry.get('unknown'),
        /not found/
      );
    });

    it('should support factory functions', () => {
      let callCount = 0;
      registry.register('factory', () => {
        callCount++;
        return { instance: callCount };
      }, { singleton: false });

      const first = registry.get('factory');
      const second = registry.get('factory');

      assert.strictEqual(first.instance, 1);
      assert.strictEqual(second.instance, 2);
    });

    it('should cache singleton factory results', () => {
      let callCount = 0;
      registry.register('singleton-factory', () => {
        callCount++;
        return { instance: callCount };
      }, { singleton: true });

      const first = registry.get('singleton-factory');
      const second = registry.get('singleton-factory');

      assert.strictEqual(first.instance, 1);
      assert.strictEqual(second.instance, 1); // Same instance
      assert.strictEqual(callCount, 1);
    });
  });

  describe('getByInterface()', () => {
    it('should find service by interface', () => {
      const mockRepo = {
        findById: () => {},
        findAll: () => {},
        save: () => {},
        delete: () => {},
      };

      registry.register('userRepo', mockRepo, {
        interfaces: [IRepository],
      });

      const repo = registry.getByInterface(IRepository);
      assert.strictEqual(repo, mockRepo);
    });

    it('should throw if no implementation found', () => {
      assert.throws(
        () => registry.getByInterface(ICache),
        /No service implements/
      );
    });
  });

  describe('getAllByInterface()', () => {
    it('should return all implementations', () => {
      const repo1 = { findById: () => {}, findAll: () => {}, save: () => {}, delete: () => {} };
      const repo2 = { findById: () => {}, findAll: () => {}, save: () => {}, delete: () => {} };

      registry.register('repo1', repo1, { interfaces: [IRepository] });
      registry.register('repo2', repo2, { interfaces: [IRepository] });

      const all = registry.getAllByInterface(IRepository);
      assert.strictEqual(all.length, 2);
    });

    it('should return empty array if none found', () => {
      const all = registry.getAllByInterface(ICache);
      assert.deepStrictEqual(all, []);
    });
  });

  describe('getByLayer()', () => {
    it('should return services in layer', () => {
      registry.register('svc1', {}, { layer: Layer.DOMAIN });
      registry.register('svc2', {}, { layer: Layer.DOMAIN });
      registry.register('svc3', {}, { layer: Layer.APPLICATION });

      const domain = registry.getByLayer(Layer.DOMAIN);
      assert.strictEqual(domain.length, 2);
    });
  });

  describe('getByTag()', () => {
    it('should return services with tag', () => {
      registry.register('svc1', {}, { tags: ['core', 'essential'] });
      registry.register('svc2', {}, { tags: ['core'] });
      registry.register('svc3', {}, { tags: ['optional'] });

      const core = registry.getByTag('core');
      assert.strictEqual(core.length, 2);
    });
  });

  describe('Layer Enforcement', () => {
    it('should enforce layer boundaries in strict mode', () => {
      const strictRegistry = new ServiceRegistry({ strictLayers: true });

      strictRegistry.register('infraService', {}, {
        layer: Layer.INFRASTRUCTURE,
      });

      // APPLICATION can access INFRASTRUCTURE
      assert.ok(strictRegistry.get('infraService', {
        callerLayer: Layer.APPLICATION,
      }));

      // PRESENTATION cannot access INFRASTRUCTURE directly
      assert.throws(
        () => strictRegistry.get('infraService', {
          callerLayer: Layer.PRESENTATION,
        }),
        /Layer violation/
      );
    });

    it('should not enforce in non-strict mode', () => {
      registry.register('infraService', {}, {
        layer: Layer.INFRASTRUCTURE,
      });

      // No error even with mismatched layers
      assert.ok(registry.get('infraService', {
        callerLayer: Layer.PRESENTATION,
      }));
    });
  });

  describe('unregister()', () => {
    it('should remove registered service', () => {
      registry.register('toRemove', {});
      assert.ok(registry.has('toRemove'));

      const removed = registry.unregister('toRemove');
      assert.strictEqual(removed, true);
      assert.strictEqual(registry.has('toRemove'), false);
    });

    it('should return false for unknown service', () => {
      const removed = registry.unregister('unknown');
      assert.strictEqual(removed, false);
    });
  });

  describe('getStats()', () => {
    it('should return registry statistics', () => {
      registry.register('svc1', {}, { layer: Layer.DOMAIN, tags: ['a'] });
      registry.register('svc2', {}, { layer: Layer.APPLICATION, tags: ['a', 'b'] });

      const stats = registry.getStats();

      assert.strictEqual(stats.totalServices, 2);
      assert.strictEqual(stats.byLayer[Layer.DOMAIN], 1);
      assert.strictEqual(stats.byLayer[Layer.APPLICATION], 1);
      assert.strictEqual(stats.tags, 2);
    });
  });

  describe('list()', () => {
    it('should list all services', () => {
      registry.register('svc1', {}, { layer: Layer.DOMAIN, tags: ['core'] });

      const list = registry.list();

      assert.strictEqual(list.length, 1);
      assert.strictEqual(list[0].name, 'svc1');
      assert.strictEqual(list[0].layer, Layer.DOMAIN);
      assert.deepStrictEqual(list[0].tags, ['core']);
    });
  });

  describe('clear()', () => {
    it('should remove all services', () => {
      registry.register('a', {});
      registry.register('b', {});

      registry.clear();

      assert.strictEqual(registry.getStats().totalServices, 0);
    });
  });
});

// =============================================================================
// INTERFACE TESTS
// =============================================================================

describe('Interfaces', () => {
  describe('implements_()', () => {
    it('should return true if object implements interface', () => {
      const obj = {
        get: () => {},
        set: () => {},
        delete: () => {},
        has: () => {},
        clear: () => {},
      };

      assert.strictEqual(implements_(obj, ICache), true);
    });

    it('should return false if missing methods', () => {
      const obj = {
        get: () => {},
        set: () => {},
        // missing delete, has, clear
      };

      assert.strictEqual(implements_(obj, ICache), false);
    });

    it('should return false for null/undefined', () => {
      assert.strictEqual(implements_(null, ICache), false);
      assert.strictEqual(implements_(undefined, ICache), false);
    });
  });

  describe('assertImplements()', () => {
    it('should not throw if implements', () => {
      const obj = {
        publish: () => {},
        subscribe: () => {},
        unsubscribe: () => {},
      };

      assert.doesNotThrow(() => {
        assertImplements(obj, IEventBus);
      });
    });

    it('should throw with missing methods listed', () => {
      const obj = {
        publish: () => {},
        // missing subscribe, unsubscribe
      };

      assert.throws(
        () => assertImplements(obj, IEventBus, 'IEventBus'),
        /Missing: subscribe, unsubscribe/
      );
    });
  });

  describe('Layer', () => {
    it('should have 4 layers', () => {
      assert.strictEqual(Object.keys(Layer).length, 4);
      assert.ok(Layer.PRESENTATION);
      assert.ok(Layer.APPLICATION);
      assert.ok(Layer.DOMAIN);
      assert.ok(Layer.INFRASTRUCTURE);
    });
  });

  describe('isLayerCallAllowed()', () => {
    it('should allow PRESENTATION -> APPLICATION', () => {
      assert.strictEqual(
        isLayerCallAllowed(Layer.PRESENTATION, Layer.APPLICATION),
        true
      );
    });

    it('should allow APPLICATION -> DOMAIN', () => {
      assert.strictEqual(
        isLayerCallAllowed(Layer.APPLICATION, Layer.DOMAIN),
        true
      );
    });

    it('should allow APPLICATION -> INFRASTRUCTURE', () => {
      assert.strictEqual(
        isLayerCallAllowed(Layer.APPLICATION, Layer.INFRASTRUCTURE),
        true
      );
    });

    it('should allow DOMAIN -> INFRASTRUCTURE', () => {
      assert.strictEqual(
        isLayerCallAllowed(Layer.DOMAIN, Layer.INFRASTRUCTURE),
        true
      );
    });

    it('should NOT allow PRESENTATION -> DOMAIN (skip layer)', () => {
      assert.strictEqual(
        isLayerCallAllowed(Layer.PRESENTATION, Layer.DOMAIN),
        false
      );
    });

    it('should NOT allow INFRASTRUCTURE -> anything', () => {
      assert.strictEqual(
        isLayerCallAllowed(Layer.INFRASTRUCTURE, Layer.DOMAIN),
        false
      );
      assert.strictEqual(
        isLayerCallAllowed(Layer.INFRASTRUCTURE, Layer.APPLICATION),
        false
      );
    });

    it('should NOT allow upward calls', () => {
      assert.strictEqual(
        isLayerCallAllowed(Layer.DOMAIN, Layer.APPLICATION),
        false
      );
      assert.strictEqual(
        isLayerCallAllowed(Layer.APPLICATION, Layer.PRESENTATION),
        false
      );
    });
  });
});

// =============================================================================
// GLOBAL INSTANCES
// =============================================================================

describe('Global Instances', () => {
  afterEach(() => {
    globalEventBus.clear();
    globalServiceRegistry.clear();
  });

  describe('globalEventBus', () => {
    it('should be a CYNICEventBus instance', () => {
      assert.ok(globalEventBus instanceof CYNICEventBus);
    });

    it('should work with convenience functions', (t, done) => {
      subscribe('global:test', (event) => {
        assert.strictEqual(event.payload.value, 123);
        done();
      });

      publish('global:test', { value: 123 });
    });
  });

  describe('globalServiceRegistry', () => {
    it('should be a ServiceRegistry instance', () => {
      assert.ok(globalServiceRegistry instanceof ServiceRegistry);
    });

    it('should work with convenience functions', () => {
      registerService('globalTest', { data: 'test' });

      const svc = getService('globalTest');
      assert.strictEqual(svc.data, 'test');
    });
  });
});

// =============================================================================
// INTEGRATION TESTS
// =============================================================================

describe('Bus System Integration', () => {
  let bus;
  let registry;

  beforeEach(() => {
    bus = new CYNICEventBus();
    registry = new ServiceRegistry();
  });

  afterEach(() => {
    bus.clear();
    registry.clear();
  });

  it('should support full request/reply cycle with services', async () => {
    // Register a "responder" service
    const responder = {
      handle(event) {
        return event.payload.x * 2;
      },
    };
    registry.register('responder', responder);

    // Setup request handler
    bus.subscribe('math:double', (event) => {
      const svc = registry.get('responder');
      const result = svc.handle(event);

      bus.publish('math:double:reply', { result }, {
        correlationId: event.id,
      });
    });

    // Make request
    const reply = await bus.request('math:double', { x: 21 }, { timeout: 1000 });

    assert.strictEqual(reply.payload.result, 42);
  });

  it('should support component event forwarding with registry', () => {
    const received = [];

    // Create component that emits events
    const component = new EventEmitter();
    registry.register('myComponent', component, {
      layer: Layer.DOMAIN,
      tags: ['emitter'],
    });

    // Connect to bus
    connectToBus(component, { namespace: 'mycomp', bus });

    // Subscribe to component events via bus
    bus.subscribe('mycomp:*', (event) => {
      received.push(event.type);
    });

    // Component emits locally
    component.emit('action:completed', { id: 1 });
    component.emit('action:failed', { id: 2 });

    assert.strictEqual(received.length, 2);
    assert.ok(received.includes('mycomp:action:completed'));
    assert.ok(received.includes('mycomp:action:failed'));
  });
});
