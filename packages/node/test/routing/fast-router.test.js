/**
 * Fast Router Test (A1)
 *
 * Tests reflex arc for <100ms response to critical perception events.
 * Bypasses UnifiedOrchestrator for immediate action.
 *
 * "Le chien réagit avant de penser" - CYNIC
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import { globalEventBus, PHI_INV, PHI_INV_2 } from '@cynic/core';
import { FastRouter, ReflexActionType } from '../../src/routing/fast-router.js';

describe('A1: Fast Router (Reflex Arc)', () => {
  let router;

  beforeEach(() => {
    router = new FastRouter({ maxLatency: 100 });
  });

  afterEach(() => {
    if (router) {
      router.stop();
      router.removeAllListeners();
      router = null;
    }
  });

  it('should initialize with default 100ms max latency', () => {
    assert.strictEqual(router.maxLatency, 100);
    assert.ok(router.stats);
    assert.strictEqual(router.stats.totalEvents, 0);
  });

  it('should handle critical market alert with NOTIFY action', (t, done) => {
    let actionReceived = false;

    router.on('reflex_action', (action) => {
      assert.strictEqual(action.type, ReflexActionType.NOTIFY);
      assert.ok(action.message.includes('$asdfasdfa'));
      assert.ok(action.severity);
      actionReceived = true;
    });

    // Emit critical market alert
    globalEventBus.publish('perception:market:price_alert', {
      direction: 'up',
      priceChangePercent: 65, // >61.8% = critical
      severity: 'high',
      price: 0.42,
    }, { source: 'test' });

    setTimeout(() => {
      assert.ok(actionReceived, 'Reflex action should be executed');
      assert.strictEqual(router.stats.criticalEvents, 1);
      assert.strictEqual(router.stats.reflexActions, 1);
      done();
    }, 50);
  });

  it('should handle Solana anomaly with BLOCK action', (t, done) => {
    let blockReceived = false;

    router.on('reflex_action', (action) => {
      assert.strictEqual(action.type, ReflexActionType.BLOCK);
      assert.ok(action.target);
      assert.ok(action.reason);
      blockReceived = true;
    });

    // Emit critical Solana anomaly
    globalEventBus.publish('perception:solana:tx_anomaly', {
      risk: 'critical',
      signature: 'test-sig-123',
      reason: 'Suspicious drain pattern',
      anomalyScore: 0.95,
    }, { source: 'test' });

    setTimeout(() => {
      assert.ok(blockReceived, 'Block action should be executed');
      assert.strictEqual(router.stats.criticalEvents, 1);
      done();
    }, 50);
  });

  it('should aggregate high-frequency market price updates', (t, done) => {
    let aggregationReceived = false;

    router.on('aggregation', (agg) => {
      assert.strictEqual(agg.type, ReflexActionType.AGGREGATE);
      assert.strictEqual(agg.eventType, 'market_price');
      assert.ok(agg.count >= 3);
      assert.ok(agg.data.min);
      assert.ok(agg.data.max);
      assert.ok(agg.data.avg);
      aggregationReceived = true;
    });

    // Emit 3 price updates
    for (let i = 0; i < 3; i++) {
      globalEventBus.publish('perception:market:price', {
        price: 0.40 + i * 0.01,
        timestamp: Date.now(),
      }, { source: 'test' });
    }

    // Wait for 5s aggregation flush
    setTimeout(() => {
      assert.ok(aggregationReceived, 'Aggregation should be flushed');
      done();
    }, 5100);
  });

  it('should track latency stats', (t, done) => {
    // Emit critical event
    globalEventBus.publish('perception:market:price_alert', {
      direction: 'down',
      priceChangePercent: 70,
      severity: 'critical',
      price: 0.30,
    }, { source: 'test' });

    setTimeout(() => {
      // Verify latency tracking works (actual latency should be <100ms)
      assert.ok(router.stats.avgLatency !== undefined, 'Should track average latency');
      assert.ok(router.stats.avgLatency < 100, 'Latency should be under limit for fast events');
      done();
    }, 50);
  });

  it('should assess priority correctly', () => {
    // Critical: market alert with high severity
    const criticalPriority = router._assessPriority(
      { payload: { severity: 'critical', priceChangePercent: 80 } },
      'market_alert'
    );
    assert.strictEqual(criticalPriority, 'critical');

    // High: Solana anomaly with medium risk
    const highPriority = router._assessPriority(
      { payload: { risk: 'medium', anomalyScore: 0.5 } },
      'solana_anomaly'
    );
    assert.strictEqual(highPriority, 'high');

    // Normal: unknown event type
    const normalPriority = router._assessPriority(
      { payload: {} },
      'unknown_event'
    );
    assert.strictEqual(normalPriority, 'normal');
  });

  it('should provide health status with φ-aligned score', () => {
    // Emit some successful events
    for (let i = 0; i < 10; i++) {
      globalEventBus.publish('perception:market:price', {
        price: 0.40,
        timestamp: Date.now(),
      }, { source: 'test' });
    }

    setTimeout(() => {
      const health = router.getHealth();
      assert.ok(health.status === 'healthy' || health.status === 'degraded');
      assert.ok(health.score <= PHI_INV, 'Health score should be φ-bounded');
      assert.ok(health.avgLatency !== undefined);
    }, 100);
  });

  it('should escalate high-priority non-critical events', (t, done) => {
    let escalationReceived = false;

    const unsubscribe = globalEventBus.subscribe(
      'fast_router:escalate',
      (event) => {
        assert.ok(event.payload.eventType);
        assert.ok(event.payload.reason);
        escalationReceived = true;
      }
    );

    // Emit Solana anomaly with medium risk (high priority but not critical)
    globalEventBus.publish('perception:solana:tx_anomaly', {
      risk: 'medium',
      anomalyScore: 0.5,
    }, { source: 'test' });

    setTimeout(() => {
      unsubscribe();
      assert.ok(escalationReceived, 'Should escalate high-priority event');
      assert.ok(router.stats.escalations > 0);
      done();
    }, 100);
  });

  it('should record event history with ring buffer', (t, done) => {
    // Emit more than maxHistory (89) events
    for (let i = 0; i < 100; i++) {
      globalEventBus.publish('perception:market:price_alert', {
        direction: 'up',
        priceChangePercent: 70,
        severity: 'critical',
        price: 0.40,
      }, { source: 'test' });
    }

    setTimeout(() => {
      const history = router.getHistory();
      assert.ok(history.length <= 89, 'History should respect ring buffer limit (F(11) = 89)');
      assert.ok(history.length >= 21, 'Should have at least 21 recent entries');
      done();
    }, 100);
  });

  it('should handle rapid events without dropping', (t, done) => {
    // Emit 50 rapid events
    for (let i = 0; i < 50; i++) {
      globalEventBus.publish('perception:market:price', {
        price: 0.40 + i * 0.001,
        timestamp: Date.now(),
      }, { source: 'test' });
    }

    setTimeout(() => {
      assert.ok(router.stats.totalEvents >= 50, 'Should process all events');
      done();
    }, 100);
  });

  it('should clean up subscriptions on stop', () => {
    const initialListeners = globalEventBus.listenerCount('perception:market:price_alert');
    router.stop();
    const afterListeners = globalEventBus.listenerCount('perception:market:price_alert');
    assert.ok(
      afterListeners < initialListeners,
      'Should unsubscribe on stop'
    );
  });
});
