/**
 * CrossScaleRouter Tests
 *
 * Tests cross-domain feedback routing via the event bus.
 *
 * @module @cynic/node/test/cross-scale-router
 */

import { describe, it, beforeEach, afterEach } from 'node:test';
import assert from 'node:assert/strict';

import { createCrossScaleRouter, DEFAULT_INFLUENCES } from '../src/services/cross-scale-router.js';
import { globalEventBus } from '@cynic/core';
import { PHI_INV, PHI_INV_2 } from '@cynic/core';

// =============================================================================
// Helpers
// =============================================================================

function collectEvents(eventName) {
  const events = [];
  const unsub = globalEventBus.subscribe(eventName, (event) => {
    events.push(event.payload || event);
  });
  return { events, cleanup: unsub };
}

// =============================================================================
// Creation
// =============================================================================

describe('createCrossScaleRouter', () => {
  it('creates router with all methods', () => {
    const router = createCrossScaleRouter();
    assert.equal(typeof router.start, 'function');
    assert.equal(typeof router.stop, 'function');
    assert.equal(typeof router.getMatrix, 'function');
    assert.equal(typeof router.getInfluence, 'function');
    assert.equal(typeof router.updateInfluence, 'function');
    assert.equal(typeof router.reinforce, 'function');
    assert.equal(typeof router.decay, 'function');
    assert.equal(typeof router.getStats, 'function');
    assert.equal(typeof router.getSummary, 'function');
    assert.equal(typeof router.reset, 'function');
  });

  it('starts not started', () => {
    const router = createCrossScaleRouter();
    assert.equal(router.isStarted(), false);
  });

  it('default matrix has routes', () => {
    const router = createCrossScaleRouter();
    const stats = router.getStats();
    assert.ok(stats.routeCount > 15);
  });
});

// =============================================================================
// Influence matrix
// =============================================================================

describe('influence matrix', () => {
  let router;

  beforeEach(() => {
    router = createCrossScaleRouter();
  });

  it('default influences are φ-bounded', () => {
    for (const [, , weight] of DEFAULT_INFLUENCES) {
      assert.ok(weight <= PHI_INV, `weight ${weight} exceeds φ⁻¹`);
    }
  });

  it('getInfluence returns known weight', () => {
    assert.ok(router.getInfluence('solana', 'market') > 0);
    assert.ok(router.getInfluence('code', 'solana') > 0);
  });

  it('getInfluence returns 0 for unknown route', () => {
    assert.equal(router.getInfluence('code', 'market'), 0);
  });

  it('updateInfluence changes weight', () => {
    router.updateInfluence('code', 'market', 0.3);
    assert.equal(router.getInfluence('code', 'market'), 0.3);
  });

  it('updateInfluence caps at φ⁻¹', () => {
    router.updateInfluence('code', 'market', 0.9);
    assert.equal(router.getInfluence('code', 'market'), PHI_INV);
  });

  it('updateInfluence floors at 0', () => {
    router.updateInfluence('code', 'market', -0.5);
    assert.equal(router.getInfluence('code', 'market'), 0);
  });

  it('custom influences override defaults', () => {
    const custom = createCrossScaleRouter({
      influences: [
        ['alpha', 'beta', 0.5, 'test'],
        ['beta', 'gamma', 0.3, 'test'],
      ],
    });
    assert.equal(custom.getInfluence('alpha', 'beta'), 0.5);
    assert.equal(custom.getInfluence('solana', 'market'), 0); // Default not present
  });
});

// =============================================================================
// Reinforcement learning
// =============================================================================

describe('reinforcement', () => {
  let router;

  beforeEach(() => {
    router = createCrossScaleRouter({
      influences: [['a', 'b', 0.3, 'test']],
    });
  });

  it('reinforce increases weight', () => {
    const before = router.getInfluence('a', 'b');
    router.reinforce('a', 'b');
    const after = router.getInfluence('a', 'b');
    assert.ok(after > before);
  });

  it('reinforce never exceeds φ⁻¹', () => {
    for (let i = 0; i < 20; i++) {
      router.reinforce('a', 'b');
    }
    assert.ok(router.getInfluence('a', 'b') <= PHI_INV);
  });

  it('decay decreases weight', () => {
    const before = router.getInfluence('a', 'b');
    router.decay('a', 'b');
    const after = router.getInfluence('a', 'b');
    assert.ok(after < before);
  });

  it('decay never goes negative', () => {
    for (let i = 0; i < 20; i++) {
      router.decay('a', 'b');
    }
    assert.ok(router.getInfluence('a', 'b') >= 0);
  });

  it('reinforce has diminishing returns', () => {
    const w0 = router.getInfluence('a', 'b');
    router.reinforce('a', 'b');
    const delta1 = router.getInfluence('a', 'b') - w0;

    router.reinforce('a', 'b');
    const delta2 = router.getInfluence('a', 'b') - (w0 + delta1);

    assert.ok(delta2 < delta1, 'second reinforcement should be smaller');
  });

  it('reinforcement tracked in route stats', () => {
    router.reinforce('a', 'b');
    router.reinforce('a', 'b');
    router.decay('a', 'b');

    const summary = router.getSummary();
    const route = summary.find(r => r.source === 'a' && r.target === 'b');
    assert.equal(route.reinforcements, 2);
    assert.equal(route.decays, 1);
  });
});

// =============================================================================
// Event routing
// =============================================================================

describe('event routing', () => {
  let router;

  afterEach(() => {
    if (router) router.stop();
  });

  it('propagates feedback on domain action', () => {
    router = createCrossScaleRouter({
      influences: [['src', 'tgt', 0.5, 'test']],
      cooldownMs: 0,
    });

    const { events, cleanup } = collectEvents('feedback:tgt');
    router.start(['src']);

    globalEventBus.publish('src:action', { type: 'TEST', value: 42 });

    cleanup();
    assert.equal(events.length, 1);
    assert.equal(events[0].fromDomain, 'src');
    assert.equal(events[0].toDomain, 'tgt');
    assert.equal(events[0].weight, 0.5);
  });

  it('does not propagate below minWeight', () => {
    router = createCrossScaleRouter({
      influences: [['low', 'tgt', 0.05, 'too low']],
      minWeight: 0.1,
      cooldownMs: 0,
    });

    const { events, cleanup } = collectEvents('feedback:tgt');
    router.start(['low']);

    globalEventBus.publish('low:action', { type: 'TEST' });

    cleanup();
    assert.equal(events.length, 0);
    assert.equal(router.getStats().feedbacksBlocked, 1);
  });

  it('respects cooldown', () => {
    router = createCrossScaleRouter({
      influences: [['cool', 'tgt', 0.5, 'test']],
      cooldownMs: 60000, // 60s cooldown
    });

    const { events, cleanup } = collectEvents('feedback:tgt');
    router.start(['cool']);

    globalEventBus.publish('cool:action', { first: true });
    globalEventBus.publish('cool:action', { second: true }); // Should be cooled down

    cleanup();
    assert.equal(events.length, 1);
    assert.equal(router.getStats().feedbacksCooledDown, 1);
  });

  it('propagates to multiple targets', () => {
    router = createCrossScaleRouter({
      influences: [
        ['multi', 'alpha', 0.5, 'test'],
        ['multi', 'beta', 0.4, 'test'],
        ['multi', 'gamma', 0.3, 'test'],
      ],
      cooldownMs: 0,
    });

    const colA = collectEvents('feedback:alpha');
    const colB = collectEvents('feedback:beta');
    const colG = collectEvents('feedback:gamma');

    router.start(['multi']);
    globalEventBus.publish('multi:action', { type: 'BROADCAST' });

    colA.cleanup();
    colB.cleanup();
    colG.cleanup();

    assert.equal(colA.events.length, 1);
    assert.equal(colB.events.length, 1);
    assert.equal(colG.events.length, 1);
    assert.equal(router.getStats().feedbacksPropagated, 3);
  });

  it('judgment events propagate at half weight', () => {
    router = createCrossScaleRouter({
      influences: [['jd', 'tgt', 0.6, 'test']],
      cooldownMs: 0,
    });

    const { events, cleanup } = collectEvents('feedback:tgt');
    router.start(['jd']);

    globalEventBus.publish('jd:judgment', { verdict: 'GROWL' });

    cleanup();
    assert.equal(events.length, 1);
    assert.ok(Math.abs(events[0].weight - 0.3) < 0.001, 'judgment weight should be halved');
  });

  it('chain propagation with attenuation', () => {
    router = createCrossScaleRouter({
      influences: [
        ['a', 'b', 0.5, 'test'],
        ['b', 'c', 0.5, 'test'],
      ],
      cooldownMs: 0,
      maxPropagationDepth: 2,
    });

    const colB = collectEvents('feedback:b');
    const colC = collectEvents('feedback:c');

    router.start(['a', 'b']);
    globalEventBus.publish('a:action', { origin: 'a' });

    colB.cleanup();
    colC.cleanup();

    assert.equal(colB.events.length, 1);
    // Chain: a→b (0.5) then b→c (0.5 * 0.5 * φ⁻¹ = attenuated)
    assert.ok(colC.events.length >= 1, 'chain should propagate to c');

    if (colC.events.length > 0) {
      assert.ok(colC.events[0].weight < colB.events[0].weight, 'chain should be attenuated');
    }

    assert.ok(router.getStats().chainsPropagated >= 1);
  });

  it('stops routing after stop()', () => {
    router = createCrossScaleRouter({
      influences: [['stop', 'tgt', 0.5, 'test']],
      cooldownMs: 0,
    });

    router.start(['stop']);
    router.stop();

    const { events, cleanup } = collectEvents('feedback:tgt');
    globalEventBus.publish('stop:action', { after: true });

    cleanup();
    assert.equal(events.length, 0);
  });
});

// =============================================================================
// Summary and stats
// =============================================================================

describe('summary and stats', () => {
  let router;

  afterEach(() => {
    if (router) router.stop();
  });

  it('getSummary returns sorted routes', () => {
    router = createCrossScaleRouter();
    const summary = router.getSummary();
    assert.ok(summary.length > 0);

    // Check sorted by weight descending
    for (let i = 1; i < summary.length; i++) {
      assert.ok(summary[i - 1].weight >= summary[i].weight);
    }
  });

  it('summary includes source, target, weight, reason', () => {
    router = createCrossScaleRouter();
    const summary = router.getSummary();

    for (const route of summary) {
      assert.ok('source' in route);
      assert.ok('target' in route);
      assert.ok('weight' in route);
      assert.ok('reason' in route);
    }
  });

  it('stats track propagations', () => {
    router = createCrossScaleRouter({
      influences: [['stat', 'tgt', 0.5, 'test']],
      cooldownMs: 0,
    });

    router.start(['stat']);
    globalEventBus.publish('stat:action', {});
    globalEventBus.publish('stat:action', {});
    router.stop();

    const stats = router.getStats();
    assert.equal(stats.feedbacksPropagated, 2);
  });

  it('reset clears stats but keeps matrix', () => {
    router = createCrossScaleRouter({
      influences: [['res', 'tgt', 0.5, 'test']],
      cooldownMs: 0,
    });

    router.start(['res']);
    globalEventBus.publish('res:action', {});
    router.stop();

    assert.equal(router.getStats().feedbacksPropagated, 1);
    router.reset();
    assert.equal(router.getStats().feedbacksPropagated, 0);
    assert.equal(router.getInfluence('res', 'tgt'), 0.5); // Matrix preserved
  });
});

// =============================================================================
// Default influences coverage
// =============================================================================

describe('DEFAULT_INFLUENCES', () => {
  it('covers all 7 domains as sources', () => {
    const sources = new Set(DEFAULT_INFLUENCES.map(([s]) => s));
    for (const domain of ['code', 'solana', 'market', 'social', 'human', 'cynic', 'cosmos']) {
      assert.ok(sources.has(domain), `missing source: ${domain}`);
    }
  });

  it('all weights are positive and φ-bounded', () => {
    for (const [source, target, weight] of DEFAULT_INFLUENCES) {
      assert.ok(weight > 0, `${source}→${target} weight must be positive`);
      assert.ok(weight <= PHI_INV, `${source}→${target} weight ${weight} exceeds φ⁻¹`);
    }
  });

  it('no self-loops', () => {
    for (const [source, target] of DEFAULT_INFLUENCES) {
      assert.notEqual(source, target, `self-loop: ${source}→${target}`);
    }
  });

  it('all reasons are non-empty', () => {
    for (const [source, target, , reason] of DEFAULT_INFLUENCES) {
      assert.ok(reason && reason.length > 0, `${source}→${target} missing reason`);
    }
  });
});
